use dioxus::prelude::*;
use lauda_types::{MailboxInfo, Message, MessageEnvelope, ViewMode};

pub mod compose_view;
pub mod contacts_view;
pub mod inbox_view;
pub mod login_view;
pub mod message_view;
pub mod state;

use compose_view::ComposeView;
use contacts_view::ContactsView;
use inbox_view::InboxView;
use login_view::LoginView;
use message_view::MessageView;
pub use state::Screen;

use crate::adapters::HttpApiClient;
use crate::domain::ports::MailApiClient;
use crate::domain::session::load_session;

/// Componente raiz do smolclient Lauda com estética Trash Polka e navegação mobile-first.
#[component]
pub fn App() -> Element {
    let initial_session = load_session(None).unwrap_or_default();
    let initial_screen = if initial_session.token.is_some() {
        Screen::Inbox
    } else {
        Screen::Login
    };

    let mut screen = use_signal(|| initial_screen);
    let session = use_signal(|| initial_session);
    let mut mailboxes = use_signal(Vec::<MailboxInfo>::new);
    let current_mailbox = use_signal(|| "INBOX".to_string());
    let mut messages = use_signal(Vec::<MessageEnvelope>::new);
    let active_message = use_signal(|| None::<Message>);
    let mut current_view_mode = use_signal(|| ViewMode::TxtAuto);

    let mut update_notice = use_signal(|| None::<crate::domain::update::UpdateManifest>);

    // Interceptar navegação de histórico / popstate (suporte a gestos Android e botão Voltar)
    use_effect(move || {
        let mut eval = dioxus::document::eval(
            r#"
            window.__lauda_can_go_back = false;
            window.__lauda_handle_back = function() {
                if (window.__lauda_can_go_back) {
                    dioxus.send("back");
                    return true;
                }
                return false;
            };
            window.addEventListener('popstate', function() {
                dioxus.send("back");
            });
        "#,
        );
        spawn(async move {
            while let Ok(msg) = eval.recv::<String>().await {
                if msg == "back" {
                    let current = screen.read().clone();
                    match current {
                        Screen::MessageDetail(_) | Screen::Compose { .. } | Screen::Contacts => {
                            screen.set(Screen::Inbox);
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    // Sincronizar estado do histórico web com a tela ativa
    use_effect(move || {
        let current = screen.read().clone();
        let is_subscreen = matches!(
            current,
            Screen::MessageDetail(_) | Screen::Compose { .. } | Screen::Contacts
        );
        let js = if is_subscreen {
            r#"
                window.__lauda_can_go_back = true;
                if (window.history && (!window.history.state || window.history.state.screen !== 'sub')) {
                    window.history.pushState({ screen: 'sub' }, '');
                }
            "#
        } else {
            r#"
                window.__lauda_can_go_back = false;
            "#
        };
        let _ = dioxus::document::eval(js);
    });

    let mut is_online = use_signal(|| true);
    #[allow(unused_mut)]
    let mut outbox_pending_count = use_signal(|| 0usize);

    #[cfg(not(target_arch = "wasm32"))]
    // Carregamento inicial SWR do cache local no desktop/mobile (0ms bloqueio)
    use_effect(move || {
        let cache_dir = crate::domain::cache::get_default_cache_dir();
        let cache = crate::domain::cache::LocalMessageCache::new(&cache_dir);
        let outbox = crate::domain::outbox::OutboxQueue::new(&cache_dir);

        if let Ok(Some(boxes)) = cache.load_mailboxes()
            && !boxes.is_empty()
        {
            mailboxes.set(boxes);
        }
        let initial_box = current_mailbox.read().clone();
        if let Ok(Some(cached_msgs)) = cache.load_envelopes(&initial_box)
            && !cached_msgs.is_empty()
        {
            messages.set(cached_msgs);
        }
        if let Ok(count) = outbox.pending_count() {
            outbox_pending_count.set(count);
        }
    });

    // Ciclo SWR em segundo plano: revalidação do servidor e flush de outbox/ações
    use_effect(move || {
        let sess = session.read().clone();
        if let Some(token) = sess.token {
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                let cache_dir = crate::domain::cache::get_default_cache_dir();
                #[cfg(not(target_arch = "wasm32"))]
                let cache = crate::domain::cache::LocalMessageCache::new(&cache_dir);
                #[cfg(not(target_arch = "wasm32"))]
                let outbox = crate::domain::outbox::OutboxQueue::new(&cache_dir);
                #[cfg(not(target_arch = "wasm32"))]
                let actions = crate::domain::sync::ActionsQueue::new(&cache_dir);

                let client = HttpApiClient::new(sess.server_url.clone());

                loop {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // 1. Flush de Ações Pendentes
                        if let Ok(pending_actions) = actions.list() {
                            for act in pending_actions {
                                let res = match act.action {
                                    crate::domain::sync::ActionType::MarkRead { read } => {
                                        client.mark_read(&token, &act.mailbox, act.uid, read).await
                                    }
                                    crate::domain::sync::ActionType::MarkFlagged { flagged } => {
                                        client
                                            .mark_flagged(&token, &act.mailbox, act.uid, flagged)
                                            .await
                                    }
                                    crate::domain::sync::ActionType::Move { ref dest_box } => {
                                        client
                                            .move_message(&token, act.uid, &act.mailbox, dest_box)
                                            .await
                                    }
                                    crate::domain::sync::ActionType::Delete => {
                                        client.delete_message(&token, &act.mailbox, act.uid).await
                                    }
                                };
                                if res.is_ok() {
                                    let _ = actions.remove(&act.id);
                                }
                            }
                        }

                        // 2. Flush de Outbox
                        if let Ok(pending_items) = outbox.list() {
                            for item in pending_items {
                                let _ = outbox.update_status(
                                    &item.id,
                                    crate::domain::outbox::OutboxStatus::Sending,
                                    false,
                                );
                                match client.send_message(&token, &item.form).await {
                                    Ok(_) => {
                                        let _ = outbox.remove(&item.id);
                                    }
                                    Err(e) => {
                                        let _ = outbox.update_status(
                                            &item.id,
                                            crate::domain::outbox::OutboxStatus::Failed(
                                                e.to_string(),
                                            ),
                                            true,
                                        );
                                    }
                                }
                            }
                        }

                        if let Ok(count) = outbox.pending_count() {
                            outbox_pending_count.set(count);
                        }
                    }

                    // 3. Revalidação das caixas e mensagens com o servidor
                    let active_box = current_mailbox.read().clone();
                    match client.list_mailboxes(&token).await {
                        Ok(boxes) => {
                            is_online.set(true);
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = cache.save_mailboxes(&boxes);
                            mailboxes.set(boxes);
                        }
                        Err(_) => {
                            is_online.set(false);
                        }
                    }

                    if let Ok(server_msgs) = client.list_messages(&token, &active_box).await {
                        is_online.set(true);
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = cache.save_envelopes(&active_box, &server_msgs);
                        messages.set(server_msgs);
                    }

                    // Pausa assíncrona segura em todas as plataformas (Tokio em desktop/Android, setTimeout em WASM)
                    crate::domain::time::async_sleep(std::time::Duration::from_secs(10)).await;
                    if session.read().token.is_none() {
                        break;
                    }
                }
            });
        }
    });

    // Verificação de atualização assíncrona
    use_effect(move || {
        spawn(async move {
            let manifest_url = if env!("CARGO_PKG_VERSION").contains("nightly") {
                "https://lauda.2lp.in/version-nightly.json"
            } else {
                "https://lauda.2lp.in/version.json"
            };
            if let Ok(manifest) = crate::domain::update::fetch_remote_manifest(manifest_url).await
                && let crate::domain::update::UpdateStatus::Available(m)
                | crate::domain::update::UpdateStatus::Required(m) =
                    crate::domain::update::check_update(env!("CARGO_PKG_VERSION"), &manifest)
            {
                update_notice.set(Some(m));
            }
        });
    });

    let active_token_exists = session.read().token.is_some();

    rsx! {
        div {
            style: "background-color: #f4efe6; color: #0a0a0a; font-family: monospace; min-height: 100vh; padding: 1rem; box-sizing: border-box;",

            // Header Principal Trash Polka com Alternador Trifocal
            header {
                style: "border-bottom: 3px solid #0a0a0a; padding-bottom: 0.8rem; margin-bottom: 1.2rem; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.6rem;",
                div {
                    style: "cursor: pointer;",
                    onclick: move |_| {
                        if active_token_exists {
                            screen.set(Screen::Inbox);
                        }
                    },
                    h1 {
                        style: "font-size: 2rem; margin: 0; color: #0a0a0a; display: inline-block; vertical-align: middle;",
                        "LAUDA"
                    }
                    span {
                        style: "background: #dc143c; color: #fff; padding: 0.2rem 0.5rem; font-weight: bold; font-size: 0.8rem; margin-left: 0.5rem; vertical-align: middle;",
                        "SMOLCLIENT"
                    }
                    span {
                        style: "font-size: 0.7rem; color: #666; margin-left: 0.5rem; vertical-align: middle; border: 1px solid #ccc; padding: 0.1rem 0.4rem; background: #fff;",
                        "v{env!(\"LAUDA_APP_VERSION\")}"
                    }
                }

                // Bloco direito do Header: Status de Rede, Outbox e Alternador Trifocal
                div {
                    style: "display: flex; align-items: center; gap: 0.6rem;",
                    if *outbox_pending_count.read() > 0 {
                        span {
                            style: "background: #dc143c; color: #fff; padding: 0.3rem 0.6rem; font-weight: bold; font-size: 0.75rem; border: 1px solid #0a0a0a;",
                            "OUTBOX: {outbox_pending_count.read()}"
                        }
                    }
                    if *is_online.read() {
                        span {
                            style: "background: #e8f5e9; color: #2e7d32; padding: 0.3rem 0.5rem; font-size: 0.75rem; font-weight: bold; border: 1px solid #2e7d32;",
                            "● ONLINE"
                        }
                    } else {
                        span {
                            style: "background: #ffebee; color: #c62828; padding: 0.3rem 0.5rem; font-size: 0.75rem; font-weight: bold; border: 1px solid #c62828;",
                            "○ OFFLINE (CACHE)"
                        }
                    }

                    // Alternador Trifásico Global (Botão Único Rotativo)
                    button {
                        style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c; white-space: nowrap;",
                        onclick: move |_| {
                            let next_mode = current_view_mode.read().next();
                            current_view_mode.set(next_mode);
                        },
                        "{current_view_mode.read().label()}"
                    }
                }
            }

            // Banner Trash Polka de Atualização Disponível
            if let Some(manifest) = update_notice.read().clone() {
                div {
                    style: "background: #dc143c; color: #fff; padding: 0.6rem 1rem; margin-bottom: 1.2rem; display: flex; justify-content: space-between; align-items: center; font-size: 0.85rem; font-weight: bold; border: 2px solid #0a0a0a; box-shadow: 3px 3px 0 #0a0a0a; flex-wrap: wrap; gap: 0.5rem;",
                    span { "NOVA VERSÃO DISPONÍVEL: v{manifest.version}" }
                    a {
                        href: "{manifest.apk_url}",
                        target: "_blank",
                        style: "background: #fff; color: #0a0a0a; padding: 0.3rem 0.7rem; text-decoration: none; font-size: 0.8rem; font-weight: bold; border: 1px solid #0a0a0a; white-space: nowrap;",
                        "ATUALIZAR APK"
                    }
                }
            }

            // Exibição da Tela Conforme o Estado de Navegação
            main {
                match screen.read().clone() {
                    Screen::Login => rsx! {
                        LoginView {
                            screen,
                            session,
                            mailboxes,
                            current_mailbox,
                            messages,
                        }
                    },
                    Screen::Inbox => rsx! {
                        InboxView {
                            screen,
                            session,
                            mailboxes,
                            current_mailbox,
                            messages,
                            active_message,
                        }
                    },
                    Screen::MessageDetail(_uid) => rsx! {
                        MessageView {
                            screen,
                            session,
                            current_mailbox,
                            messages,
                            mailboxes,
                            active_message,
                            current_view_mode,
                        }
                    },
                    Screen::Compose {
                        reply_to,
                        reply_cc,
                        reply_subject,
                        initial_body,
                    } => rsx! {
                        ComposeView {
                            screen,
                            session,
                            reply_to,
                            reply_cc,
                            reply_subject,
                            initial_body,
                            current_mailbox,
                            messages,
                        }
                    },
                    Screen::Contacts => rsx! {
                        ContactsView {
                            screen,
                            session,
                        }
                    },
                }
            }
        }
    }
}
