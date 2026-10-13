use lauda_client::domain::cache::LocalMessageCache;
use lauda_types::message::{CryptoMeta, Message, MessageEnvelope, SignatureStatus};
use lauda_types::MailboxInfo;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_sqlite_message_cache_lifecycle() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::open(temp.path()).expect("open sqlite cache");

    // 1. Mailboxes Roundtrip
    let mailboxes = vec![
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 10,
            unread_messages: 3,
        },
        MailboxInfo {
            name: "Sent".to_string(),
            total_messages: 5,
            unread_messages: 0,
        },
    ];

    cache.save_mailboxes(&mailboxes).expect("save mailboxes");
    let loaded_boxes = cache
        .load_mailboxes()
        .expect("load mailboxes")
        .expect("some mailboxes");
    assert_eq!(loaded_boxes.len(), 2);
    assert_eq!(loaded_boxes[0].name, "INBOX");
    assert_eq!(loaded_boxes[0].unread_messages, 3);

    // 2. Envelopes Roundtrip com metadados criptográficos
    let crypto = CryptoMeta {
        is_encrypted: true,
        signature_status: SignatureStatus::Valid,
        signer_key_id: Some("0x123456789".to_string()),
        verified_sender: Some("alice@secmail.org".to_string()),
        security_word: Some("PALAVRA_SECRETA".to_string()),
    };

    let envelopes = vec![
        MessageEnvelope {
            uid: 101,
            subject: "Msg Cripto 1".to_string(),
            sender: "alice@secmail.org".to_string(),
            date: "2026-10-12T12:00:00Z".to_string(),
            is_read: false,
            is_flagged: true,
            cc: None,
            has_attachments: false,
            crypto_meta: Some(crypto),
        },
        MessageEnvelope {
            uid: 102,
            subject: "Msg Normal 2".to_string(),
            sender: "bob@example.com".to_string(),
            date: "2026-10-12T10:00:00Z".to_string(),
            is_read: true,
            is_flagged: false,
            cc: Some("team@example.com".to_string()),
            has_attachments: true,
            crypto_meta: None,
        },
    ];

    cache.save_envelopes("INBOX", &envelopes).expect("save envelopes");
    let loaded_envs = cache
        .load_envelopes("INBOX")
        .expect("load envelopes")
        .expect("some envelopes");
    assert_eq!(loaded_envs.len(), 2);
    assert_eq!(loaded_envs[0].uid, 101);
    assert!(loaded_envs[0].crypto_meta.is_some());
    let loaded_crypto = loaded_envs[0].crypto_meta.as_ref().unwrap();
    assert!(loaded_crypto.is_encrypted);
    assert_eq!(loaded_crypto.signature_status, SignatureStatus::Valid);

    // 3. Message Body Roundtrip
    let msg = Message {
        envelope: envelopes[0].clone(),
        txt_auto: Some("# Corpo formatado".to_string()),
        txt_original: Some("Corpo formatado".to_string()),
        html_sanitized: Some("<p>Corpo formatado</p>".to_string()),
        attachments: vec![],
    };

    cache.save_message("INBOX", &msg).expect("save message");
    let loaded_msg = cache
        .load_message("INBOX", 101)
        .expect("load message")
        .expect("some message");
    assert_eq!(loaded_msg.envelope.uid, 101);
    assert_eq!(loaded_msg.txt_auto.as_deref(), Some("# Corpo formatado"));

    // 4. Retenção de 90 dias e Purga
    // Simular mensagem antiga (> 95 dias)
    let old_env = MessageEnvelope {
        uid: 999,
        subject: "Antiga".to_string(),
        sender: "old@example.com".to_string(),
        date: "2026-07-01T00:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let old_msg = Message {
        envelope: old_env.clone(),
        txt_auto: Some("Corpo antigo".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };
    cache.save_envelopes("INBOX", &[envelopes[0].clone(), envelopes[1].clone(), old_env]).expect("save with old");
    cache.save_message("INBOX", &old_msg).expect("save old msg");

    // Purga de mensagens com mais de 90 dias
    let purged = cache
        .purge_older_than(Duration::from_secs(90 * 86400))
        .expect("purge");
    assert!(purged >= 1, "Deveria ter purgado pelo menos a mensagem antiga");

    // A mensagem antiga não deve mais existir no cache
    let old_loaded = cache.load_message("INBOX", 999).expect("load old");
    assert!(old_loaded.is_none());

    // A mensagem recente deve continuar intacta
    let recent_loaded = cache.load_message("INBOX", 101).expect("load recent");
    assert!(recent_loaded.is_some());
}
