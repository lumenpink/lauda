use lauda_client::domain::{LocalMessageCache, get_default_cache_dir};
use lauda_types::{MailboxInfo, Message, MessageEnvelope};
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_cache_atomic_mailboxes_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    // Inicialmente vazio
    let initial = cache.load_mailboxes().expect("load should succeed");
    assert_eq!(initial, None);

    let mailboxes = vec![
        MailboxInfo {
            name: "INBOX".to_string(),
            unread_messages: 5,
            total_messages: 20,
        },
        MailboxInfo {
            name: "Sent".to_string(),
            unread_messages: 0,
            total_messages: 15,
        },
    ];

    cache
        .save_mailboxes(&mailboxes)
        .expect("save should succeed");

    let loaded = cache
        .load_mailboxes()
        .expect("load should succeed")
        .expect("some mailboxes");
    assert_eq!(loaded, mailboxes);
}

#[test]
fn test_cache_atomic_envelopes_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    let initial = cache.load_envelopes("INBOX").expect("load should succeed");
    assert_eq!(initial, None);

    let envelopes = vec![
        MessageEnvelope {
            uid: 101,
            subject: "Reunião de Alinhamento".to_string(),
            sender: "alice@example.com".to_string(),
            date: "2026-10-10T12:00:00Z".to_string(),
            is_read: false,
            is_flagged: true,
            cc: None,
            has_attachments: false,
        },
        MessageEnvelope {
            uid: 102,
            subject: "Relatório Mensal".to_string(),
            sender: "bob@example.com".to_string(),
            date: "2026-10-09T15:30:00Z".to_string(),
            is_read: true,
            is_flagged: false,
            cc: Some("equipe@example.com".to_string()),
            has_attachments: true,
        },
    ];

    cache
        .save_envelopes("INBOX", &envelopes)
        .expect("save envelopes");

    let loaded = cache
        .load_envelopes("INBOX")
        .expect("load envelopes")
        .expect("some envelopes");
    assert_eq!(loaded, envelopes);
}

#[test]
fn test_cache_atomic_message_roundtrip() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    let initial = cache
        .load_message("INBOX", 42)
        .expect("load should succeed");
    assert_eq!(initial, None);

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 42,
            subject: "Mensagem Completa".to_string(),
            sender: "author@example.com".to_string(),
            date: "2026-10-01T08:00:00Z".to_string(),
            is_read: true,
            is_flagged: false,
            cc: None,
            has_attachments: false,
        },
        txt_auto: Some("Corpo formatado em Markdown".to_string()),
        txt_original: Some("Corpo cru em texto".to_string()),
        html_sanitized: Some("<p>HTML limpo</p>".to_string()),
        attachments: vec![],
    };

    cache.save_message("INBOX", &msg).expect("save message");

    let loaded = cache
        .load_message("INBOX", 42)
        .expect("load message")
        .expect("some message");
    assert_eq!(loaded, msg);
}

#[test]
fn test_cache_purge_90_day_retention() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());

    // Simulação: Data atual = 1791662400 (ex: 2026-10-10T12:00:00Z)
    // Mensagem Recente: 10 dias atrás -> 1790798400 (deve ser mantida)
    // Mensagem Antiga: 95 dias atrás -> 1783454400 (deve ser expurgada)
    let now_unix: u64 = 1791662400;

    let recent_msg = Message {
        envelope: MessageEnvelope {
            uid: 1,
            subject: "Recente".to_string(),
            sender: "test@example.com".to_string(),
            date: "2026-09-30T12:00:00Z".to_string(), // ~10 dias
            is_read: true,
            is_flagged: false,
            cc: None,
            has_attachments: false,
        },
        txt_auto: Some("Conteúdo recente".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    let old_msg = Message {
        envelope: MessageEnvelope {
            uid: 2,
            subject: "Antiga".to_string(),
            sender: "test@example.com".to_string(),
            date: "2026-07-07T12:00:00Z".to_string(), // ~95 dias
            is_read: true,
            is_flagged: false,
            cc: None,
            has_attachments: false,
        },
        txt_auto: Some("Conteúdo antigo que deve ser purgado".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    let envelopes = vec![recent_msg.envelope.clone(), old_msg.envelope.clone()];

    cache
        .save_envelopes("INBOX", &envelopes)
        .expect("save envelopes");
    cache
        .save_message("INBOX", &recent_msg)
        .expect("save recent");
    cache.save_message("INBOX", &old_msg).expect("save old");

    // Purga mensagens com mais de 90 dias
    let purged = cache
        .purge_expired_messages(Duration::from_secs(90 * 86400), now_unix)
        .expect("purge");

    assert_eq!(purged, 1, "Exatamente 1 mensagem antiga deve ser purgada");

    // A mensagem recente ainda tem corpo
    assert!(cache.load_message("INBOX", 1).expect("load").is_some());
    // A mensagem antiga teve seu arquivo de corpo purgado
    assert!(cache.load_message("INBOX", 2).expect("load").is_none());
    // Mas ambos os envelopes permanecem na lista
    let loaded_envs = cache.load_envelopes("INBOX").expect("load").unwrap();
    assert_eq!(loaded_envs.len(), 2);
}

#[test]
fn test_default_cache_dir_resolution() {
    let dir = get_default_cache_dir();
    assert!(!dir.as_os_str().is_empty());
}
