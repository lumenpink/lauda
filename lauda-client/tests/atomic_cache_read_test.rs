use lauda_client::domain::LocalMessageCache;
use lauda_client::domain::sync::{ActionType, ActionsQueue};
use lauda_types::{Message, MessageEnvelope};
use tempfile::tempdir;

#[test]
fn test_mark_envelope_read_updates_cache_and_message() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    let envelope = MessageEnvelope {
        uid: 201,
        subject: "Alinhamento Estratégico".to_string(),
        sender: "maria@example.com".to_string(),
        date: "2026-10-15T10:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    cache
        .save_envelopes("INBOX", std::slice::from_ref(&envelope))
        .expect("save envelopes");

    let full_msg = Message {
        envelope: envelope.clone(),
        txt_auto: Some("Conteúdo da mensagem importante".to_string()),
        txt_original: Some("Conteúdo da mensagem importante".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };

    cache
        .save_message("INBOX", &full_msg)
        .expect("save full message");

    // Executa a marcação de leitura atômica no cache local
    cache
        .mark_envelope_read("INBOX", 201, true)
        .expect("mark envelope read");

    // 1. Verifica se o envelope no SQLite/cache reflete is_read = true
    let envelopes = cache
        .load_envelopes("INBOX")
        .expect("load envelopes")
        .expect("some envelopes");
    assert_eq!(envelopes.len(), 1);
    assert!(
        envelopes[0].is_read,
        "Envelope deve estar marcado como lido no cache"
    );

    // 2. Verifica se o corpo da mensagem em cache_messages também reflete is_read = true
    let cached_msg = cache
        .load_message("INBOX", 201)
        .expect("load message")
        .expect("message must exist");
    assert!(
        cached_msg.envelope.is_read,
        "Mensagem completa deve ter o envelope atualizado para is_read = true"
    );
}

#[test]
fn test_atomic_offline_message_retrieval_within_90_days() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    let envelope = MessageEnvelope {
        uid: 301,
        subject: "Relatório Financeiro".to_string(),
        sender: "financeiro@empresa.com".to_string(),
        date: "2026-10-20T14:30:00Z".to_string(),
        is_read: true,
        is_flagged: true,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let full_msg = Message {
        envelope,
        txt_auto: Some("DRE do trimestre consolidada.".to_string()),
        txt_original: Some("DRE do trimestre consolidada.".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };

    // Salva no cache local
    cache
        .save_message("INBOX", &full_msg)
        .expect("salvar mensagem");

    // Recuperação imediata do SQLite (0ms de rede)
    let loaded = cache
        .load_message("INBOX", 301)
        .expect("load_message")
        .expect("deve encontrar mensagem em cache");

    assert_eq!(loaded.envelope.uid, 301);
    assert_eq!(
        loaded.txt_auto.as_deref(),
        Some("DRE do trimestre consolidada.")
    );
}

#[test]
fn test_mark_read_resilience_actions_queue() {
    let temp = tempdir().expect("tempdir");
    let actions = ActionsQueue::new(temp.path());

    // Enfileira ação de leitura em cenário offline/instável
    let queued = actions
        .enqueue(
            401,
            "INBOX",
            ActionType::MarkRead { read: true },
            1790800000,
        )
        .expect("enqueue mark read");

    let list = actions.list().expect("list actions");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, queued.id);
    assert_eq!(list[0].mailbox, "INBOX");
    assert_eq!(list[0].uid, 401);
    assert_eq!(list[0].action, ActionType::MarkRead { read: true });
}

#[test]
fn test_mutation_detection_on_cache_and_90d_cutoff() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    // Mutante 1: Marcar como não lida (rollback/toggle)
    let envelope = MessageEnvelope {
        uid: 501,
        subject: "Mutante Test".to_string(),
        sender: "tester@example.com".to_string(),
        date: "2026-10-20T12:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    cache
        .save_envelopes("INBOX", std::slice::from_ref(&envelope))
        .expect("save envelopes");

    cache
        .mark_envelope_read("INBOX", 501, false)
        .expect("mark unread");
    let loaded = cache
        .load_envelopes("INBOX")
        .expect("load")
        .expect("exists");
    assert!(
        !loaded[0].is_read,
        "Mutante detectado: is_read deve ser false após toggle"
    );

    // Mutante 2: Janela de corte de 90 dias
    let now_unix = 1792000000u64;
    let cutoff = now_unix.saturating_sub(90 * 86400);

    let recent_date = "2026-10-15T00:00:00Z";
    let parsed_recent =
        LocalMessageCache::parse_iso_date_to_unix(recent_date).expect("parsed recent");
    assert!(
        parsed_recent >= cutoff,
        "Mensagem recente deve estar dentro da janela de 90 dias"
    );

    let old_date = "2026-06-01T00:00:00Z";
    let parsed_old = LocalMessageCache::parse_iso_date_to_unix(old_date).expect("parsed old");
    assert!(
        parsed_old < cutoff,
        "Mensagem com mais de 90 dias deve ficar fora do corte de sincronização"
    );
}
