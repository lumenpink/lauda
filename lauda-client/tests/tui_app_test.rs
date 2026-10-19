use lauda_client::tui::app::{TuiAction, TuiApp, TuiScreen};
use lauda_types::{MailboxInfo, Message, MessageEnvelope};

fn create_sample_envelopes() -> Vec<MessageEnvelope> {
    vec![
        MessageEnvelope {
            uid: 101,
            subject: "Reunião de Alinhamento Semanal".to_string(),
            sender: "alice@lohn.in".to_string(),
            date: "2026-10-18T10:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: false,
            is_flagged: false,
            crypto_meta: None,
        },
        MessageEnvelope {
            uid: 102,
            subject: "Atualização de Infraestrutura".to_string(),
            sender: "bob@lohn.in".to_string(),
            date: "2026-10-18T11:00:00Z".to_string(),
            cc: None,
            has_attachments: true,
            is_read: true,
            is_flagged: true,
            crypto_meta: None,
        },
        MessageEnvelope {
            uid: 103,
            subject: "Almoço da equipe na sexta".to_string(),
            sender: "carol@lohn.in".to_string(),
            date: "2026-10-18T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
    ]
}

fn create_sample_mailboxes() -> Vec<MailboxInfo> {
    vec![
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 3,
            unread_messages: 1,
        },
        MailboxInfo {
            name: "Sent".to_string(),
            total_messages: 5,
            unread_messages: 0,
        },
        MailboxInfo {
            name: "Archive".to_string(),
            total_messages: 12,
            unread_messages: 0,
        },
    ]
}

#[test]
fn test_tui_app_navigation_and_selection() {
    let mailboxes = create_sample_mailboxes();
    let envelopes = create_sample_envelopes();

    let mut app = TuiApp::new(mailboxes, envelopes);

    // Estado inicial
    assert_eq!(app.current_screen(), TuiScreen::EnvelopeList);
    assert_eq!(app.selected_envelope_index(), 0);
    assert_eq!(app.current_mailbox_name(), "INBOX");

    // Navega para baixo ('j' / Down)
    app.handle_action(TuiAction::MoveDown);
    assert_eq!(app.selected_envelope_index(), 1);

    app.handle_action(TuiAction::MoveDown);
    assert_eq!(app.selected_envelope_index(), 2);

    // Limite inferior (não deve ultrapassar o último item)
    app.handle_action(TuiAction::MoveDown);
    assert_eq!(app.selected_envelope_index(), 2);

    // Navega para cima ('k' / Up)
    app.handle_action(TuiAction::MoveUp);
    assert_eq!(app.selected_envelope_index(), 1);

    app.handle_action(TuiAction::MoveUp);
    assert_eq!(app.selected_envelope_index(), 0);

    // Limite superior (não deve ficar negativo)
    app.handle_action(TuiAction::MoveUp);
    assert_eq!(app.selected_envelope_index(), 0);
}

#[test]
fn test_tui_app_open_message_and_return() {
    let mailboxes = create_sample_mailboxes();
    let envelopes = create_sample_envelopes();
    let mut app = TuiApp::new(mailboxes, envelopes);

    // Seleciona a segunda mensagem (uid 102)
    app.handle_action(TuiAction::MoveDown);
    assert_eq!(app.selected_envelope_index(), 1);

    // Pressiona Enter para ler a mensagem
    app.handle_action(TuiAction::Enter);
    assert_eq!(app.current_screen(), TuiScreen::MessageReader(102));

    // Define mensagem ativa carregada
    let sample_msg = Message {
        envelope: create_sample_envelopes()[1].clone(),
        txt_auto: Some("Olá equipe, atualizamos os servidores.".to_string()),
        txt_original: Some("Olá equipe, atualizamos os servidores.".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    app.set_active_message(Some(sample_msg));
    assert!(app.active_message().is_some());

    // Pressiona Back/Quit para retornar à lista
    app.handle_action(TuiAction::Back);
    assert_eq!(app.current_screen(), TuiScreen::EnvelopeList);
}

#[test]
fn test_tui_app_mailbox_switch() {
    let mailboxes = create_sample_mailboxes();
    let envelopes = create_sample_envelopes();
    let mut app = TuiApp::new(mailboxes, envelopes);

    // Pressiona 'm' para abrir o seletor de caixas
    app.handle_action(TuiAction::SwitchMailboxPane);
    assert_eq!(app.current_screen(), TuiScreen::MailboxList);
    assert_eq!(app.selected_mailbox_index(), 0);

    // Navega até "Sent"
    app.handle_action(TuiAction::MoveDown);
    assert_eq!(app.selected_mailbox_index(), 1);

    // Seleciona com Enter
    app.handle_action(TuiAction::Enter);
    assert_eq!(app.current_mailbox_name(), "Sent");
    assert_eq!(app.current_screen(), TuiScreen::EnvelopeList);
}

#[test]
fn test_tui_app_search_filter() {
    let mailboxes = create_sample_mailboxes();
    let envelopes = create_sample_envelopes();
    let mut app = TuiApp::new(mailboxes, envelopes);

    // Pressiona '/' para entrar no modo de busca
    app.handle_action(TuiAction::OpenSearch);
    assert_eq!(app.current_screen(), TuiScreen::Search);

    // Digita consulta de busca
    app.update_search_query("Almoço");
    let filtered = app.filtered_envelopes();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].uid, 103);

    // Sai da busca com Back
    app.handle_action(TuiAction::Back);
    assert_eq!(app.current_screen(), TuiScreen::EnvelopeList);
    assert_eq!(app.filtered_envelopes().len(), 3);
}

#[test]
fn test_tui_app_mutation_and_edge_cases() {
    // Lista vazia não entra em pânico
    let mut empty_app = TuiApp::new(vec![], vec![]);
    empty_app.handle_action(TuiAction::MoveDown);
    empty_app.handle_action(TuiAction::MoveUp);
    empty_app.handle_action(TuiAction::Enter);
    assert_eq!(empty_app.selected_envelope_index(), 0);

    // Sair da raiz define flag de encerramento
    empty_app.handle_action(TuiAction::Quit);
    assert!(empty_app.should_quit());
}
