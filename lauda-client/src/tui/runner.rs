use std::io::stdout;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::adapters::HttpApiClient;
use crate::domain::cache::{LocalMessageCache, get_default_cache_dir};
use crate::domain::ports::MailApiClient;
use crate::domain::session::load_session;
use crate::tui::app::{TuiAction, TuiApp, TuiScreen};
use crate::tui::ui;

/// Executa o loop principal da TUI no terminal ativo.
pub async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Carrega sessão salva e cache local
    let session = load_session(None).unwrap_or_default();
    let cache_dir = get_default_cache_dir();
    let cache = LocalMessageCache::new(&cache_dir);

    let initial_mailboxes = cache.load_mailboxes()?.unwrap_or_default();
    let initial_mailbox_name = initial_mailboxes
        .first()
        .map(|m| m.name.clone())
        .unwrap_or_else(|| "INBOX".to_string());
    let initial_envelopes = cache
        .load_envelopes(&initial_mailbox_name)?
        .unwrap_or_default();

    let mut app = TuiApp::new(initial_mailboxes, initial_envelopes);

    // 2. Configura terminal no modo raw com alternate screen
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app_loop(&mut terminal, &mut app, &session, &cache).await;

    // 3. Restauração garantida do terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut TuiApp,
    session: &crate::domain::session::SessionConfig,
    cache: &LocalMessageCache,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpApiClient::new(session.server_url.clone());
    let token = session.token.clone();

    // Sincronização inicial assíncrona se houver token
    if let Some(ref tok) = token {
        let current_box = app.current_mailbox_name().to_string();
        if let Ok(boxes) = client.list_mailboxes(tok).await {
            let _ = cache.save_mailboxes(&boxes);
            app.set_mailboxes(boxes);
        }
        if let Ok(envs) = client
            .list_messages(tok, &current_box, Some(50), Some(0))
            .await
        {
            let _ = cache.save_envelopes(&current_box, &envs);
            app.set_envelopes(envs);
        }
    }

    loop {
        terminal.draw(|f| ui::render(app, f))?;

        if app.should_quit() {
            break;
        }

        // Se uma mensagem estiver selecionada para leitura e ainda não carregada
        if let TuiScreen::MessageReader(uid) = app.current_screen()
            && app.active_message().is_none()
        {
            let current_box = app.current_mailbox_name().to_string();
            let mut target_msg = None;
            if let Ok(Some(cached_msg)) = cache.load_message(&current_box, uid) {
                target_msg = Some(cached_msg);
            } else if let Some(ref tok) = token
                && let Ok(server_msg) = client.get_message(tok, &current_box, uid).await
            {
                let _ = cache.save_message(&current_box, &server_msg);
                target_msg = Some(server_msg);
            }

            if let Some(mut msg) = target_msg {
                if !msg.envelope.is_read {
                    msg.envelope.is_read = true;
                    app.mark_envelope_read(uid, true);
                    let _ = cache.mark_envelope_read(&current_box, uid, true);
                    if let Some(ref tok) = token {
                        let _ = client.mark_read(tok, &current_box, uid, true).await;
                    }
                }
                app.set_active_message(Some(msg));
            }
        }

        // Poll de eventos de teclado com timeout para manter UI responsiva
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Modo de busca com captura de texto digitado
            if app.current_screen() == TuiScreen::Search {
                match key.code {
                    KeyCode::Esc => {
                        app.handle_action(TuiAction::Back);
                    }
                    KeyCode::Enter => {
                        app.handle_action(TuiAction::Enter);
                    }
                    KeyCode::Backspace => {
                        let mut q = app.search_query().to_string();
                        q.pop();
                        app.update_search_query(&q);
                    }
                    KeyCode::Char(c) => {
                        let mut q = app.search_query().to_string();
                        q.push(c);
                        app.update_search_query(&q);
                    }
                    KeyCode::Up => {
                        app.handle_action(TuiAction::MoveUp);
                    }
                    KeyCode::Down => {
                        app.handle_action(TuiAction::MoveDown);
                    }
                    _ => {}
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.handle_action(TuiAction::Back);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    app.handle_action(TuiAction::MoveDown);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.handle_action(TuiAction::MoveUp);
                }
                KeyCode::Enter => {
                    let prev_box = app.current_mailbox_name().to_string();
                    app.handle_action(TuiAction::Enter);
                    // Se trocou de caixa postal, recarrega envelopes
                    let new_box = app.current_mailbox_name().to_string();
                    if prev_box != new_box {
                        if let Ok(Some(cached_envs)) = cache.load_envelopes(&new_box) {
                            app.set_envelopes(cached_envs);
                        }
                        if let Some(ref tok) = token
                            && let Ok(server_envs) =
                                client.list_messages(tok, &new_box, Some(50), Some(0)).await
                        {
                            let _ = cache.save_envelopes(&new_box, &server_envs);
                            app.set_envelopes(server_envs);
                        }
                    }
                }
                KeyCode::Char('m') => {
                    app.handle_action(TuiAction::SwitchMailboxPane);
                }
                KeyCode::Char('/') => {
                    app.handle_action(TuiAction::OpenSearch);
                }
                KeyCode::Char('r') => {
                    app.handle_action(TuiAction::Refresh);
                    if let Some(ref tok) = token {
                        let current_box = app.current_mailbox_name().to_string();
                        if let Ok(boxes) = client.list_mailboxes(tok).await {
                            let _ = cache.save_mailboxes(&boxes);
                            app.set_mailboxes(boxes);
                        }
                        if let Ok(envs) = client
                            .list_messages(tok, &current_box, Some(50), Some(0))
                            .await
                        {
                            let _ = cache.save_envelopes(&current_box, &envs);
                            app.set_envelopes(envs);
                        }
                    }
                    app.status_message = None;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
