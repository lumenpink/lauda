use lauda_client::domain::{
    ActionType, ActionsQueue, LocalMessageCache, OutboxQueue,
};
use lauda_client::domain::ports::MailApiClient;
use lauda_client::adapters::InMemoryApiClient;
use lauda_types::ComposeForm;
use tempfile::tempdir;

#[tokio::test]
async fn test_swr_and_offline_sync_reconciliation() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());
    let outbox = OutboxQueue::new(temp.path());
    let actions = ActionsQueue::new(temp.path());

    // 1. Simular estado offline onde o cache local possui envelopes
    let initial_env = lauda_types::MessageEnvelope {
        uid: 50,
        subject: "Email Offline".to_string(),
        sender: "offline@example.com".to_string(),
        date: "2026-10-10T10:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
    };
    cache.save_envelopes("INBOX", &[initial_env.clone()]).expect("save envs");

    // SWR imediato carrega do cache local
    let cached = cache.load_envelopes("INBOX").expect("load").unwrap();
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].uid, 50);

    // 2. O usuário marca a mensagem como lida e favorita offline
    actions
        .enqueue(50, "INBOX", ActionType::MarkRead { read: true }, 1000)
        .expect("enqueue read");
    actions
        .enqueue(50, "INBOX", ActionType::MarkFlagged { flagged: true }, 1001)
        .expect("enqueue flag");

    assert_eq!(actions.count().unwrap(), 2);

    // 3. O usuário compõe uma mensagem para envio na Outbox offline
    let form = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "dest@example.com".to_string(),
        cc: None,
        subject: "Assunto Outbox".to_string(),
        body: "Texto outbox".to_string(),
        attachments: vec![],
    };
    let _out_item = outbox.enqueue(form.clone(), 1002).expect("enqueue outbox");
    assert_eq!(outbox.pending_count().unwrap(), 1);

    // 4. Reconexão e Flush de Reconciliação
    let mock_api = InMemoryApiClient::new();
    mock_api.add_message("INBOX", lauda_types::Message {
        envelope: initial_env.clone(),
        txt_auto: Some("Corpo".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    });
    let token = "test-token";

    // Flush de ações offline
    let pending_acts = actions.list().unwrap();
    for act in pending_acts {
        match act.action {
            ActionType::MarkRead { read } => {
                mock_api.mark_read(token, &act.mailbox, act.uid, read).await.unwrap();
                actions.remove(&act.id).unwrap();
            }
            ActionType::MarkFlagged { flagged } => {
                mock_api.mark_flagged(token, &act.mailbox, act.uid, flagged).await.unwrap();
                actions.remove(&act.id).unwrap();
            }
            ActionType::Move { dest_box } => {
                mock_api.move_message(token, act.uid, &act.mailbox, &dest_box).await.unwrap();
                actions.remove(&act.id).unwrap();
            }
            ActionType::Delete => {
                mock_api.delete_message(token, &act.mailbox, act.uid).await.unwrap();
                actions.remove(&act.id).unwrap();
            }
        }
    }
    assert_eq!(actions.count().unwrap(), 0, "Todas as ações devem ser reconciliadas");

    // Flush de Outbox
    let pending_out = outbox.list().unwrap();
    for item in pending_out {
        mock_api.send_message(token, &item.form).await.unwrap();
        outbox.remove(&item.id).unwrap();
    }
    assert_eq!(outbox.pending_count().unwrap(), 0, "Outbox deve ser limpa após envio");
}
