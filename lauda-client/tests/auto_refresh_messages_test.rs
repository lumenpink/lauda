use lauda_client::domain::sync::reconcile_ui_messages;
use lauda_types::MessageEnvelope;

fn make_envelope(uid: u32, is_read: bool, subject: &str) -> MessageEnvelope {
    MessageEnvelope {
        uid,
        subject: subject.to_string(),
        sender: "test@2lp.in".to_string(),
        date: "2026-11-12T10:00:00Z".to_string(),
        is_read,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    }
}

#[test]
fn test_reconcile_ui_messages_updates_when_envelopes_differ() {
    let old_msgs: Vec<MessageEnvelope> = (1..=100)
        .rev()
        .map(|i| make_envelope(i, false, &format!("Msg {}", i)))
        .collect();

    // Novo e-mail UID 101 chegou no topo
    let mut fresh_msgs = old_msgs.clone();
    fresh_msgs.insert(0, make_envelope(101, false, "Msg 101 Nova"));
    fresh_msgs.truncate(100);

    let updated = reconcile_ui_messages("INBOX", "INBOX", &old_msgs, &fresh_msgs);
    assert!(updated.is_some(), "Deve solicitar atualização da UI");
    let res = updated.unwrap();
    assert_eq!(res.len(), 100);
    assert_eq!(res[0].uid, 101);
}

#[test]
fn test_reconcile_ui_messages_updates_when_flag_changes() {
    let old_msgs = vec![make_envelope(1, false, "Msg 1")];
    let fresh_msgs = vec![make_envelope(1, true, "Msg 1")]; // Marcado como lido

    let updated = reconcile_ui_messages("INBOX", "INBOX", &old_msgs, &fresh_msgs);
    assert!(updated.is_some());
    assert!(updated.unwrap()[0].is_read);
}

#[test]
fn test_reconcile_ui_messages_ignores_identical_envelopes() {
    let msgs = vec![make_envelope(1, false, "Msg 1")];
    let updated = reconcile_ui_messages("INBOX", "INBOX", &msgs, &msgs);
    assert!(updated.is_none(), "Não deve emitir novo sinal se idêntico");
}

#[test]
fn test_reconcile_ui_messages_ignores_when_user_switched_mailbox() {
    let old_msgs = vec![make_envelope(1, false, "Msg 1")];
    let fresh_msgs = vec![make_envelope(2, false, "Msg 2")];

    // O usuário mudou para Sent enquanto a INBOX sincronizava
    let updated = reconcile_ui_messages("Sent", "INBOX", &old_msgs, &fresh_msgs);
    assert!(
        updated.is_none(),
        "Não deve sobrescrever lista se o usuário mudou de pasta"
    );
}

#[test]
fn test_reconcile_ui_messages_ignores_empty_cache() {
    let old_msgs = vec![make_envelope(1, false, "Msg 1")];
    let empty: Vec<MessageEnvelope> = Vec::new();

    let updated = reconcile_ui_messages("INBOX", "INBOX", &old_msgs, &empty);
    assert!(
        updated.is_none(),
        "Cache vazio não deve zerar a lista da tela"
    );
}
