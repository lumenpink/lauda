use std::sync::Arc;
use tempfile::tempdir;

use lauda_client::adapters::in_memory::InMemoryApiClient;
use lauda_client::domain::cache::LocalMessageCache;
use lauda_client::domain::sync::prefetch_missing_bodies;
use lauda_types::{Message, MessageEnvelope};

#[test]
fn test_parse_date_robust_rfc3339_and_rfc2822() {
    // 1. RFC 3339 com Z
    let ts1 = LocalMessageCache::parse_iso_date_to_unix("2026-10-30T16:00:00Z");
    assert!(ts1.is_some(), "Deveria parsear RFC 3339 com Z");

    // 2. RFC 3339 com timezone offset (-03:00)
    let ts2 = LocalMessageCache::parse_iso_date_to_unix("2026-10-30T13:00:00-03:00");
    assert!(ts2.is_some(), "Deveria parsear RFC 3339 com offset negativo");
    assert_eq!(
        ts1.unwrap(),
        ts2.unwrap(),
        "16:00 UTC deve equivaler a 13:00 -03:00"
    );

    // 3. RFC 3339 com timezone offset positivo (+00:00)
    let ts3 = LocalMessageCache::parse_iso_date_to_unix("2026-10-30T16:00:00+00:00");
    assert!(ts3.is_some(), "Deveria parsear RFC 3339 com +00:00");
    assert_eq!(ts1.unwrap(), ts3.unwrap());

    // 4. RFC 2822 padrão emitido por servidores de correio
    let ts4 = LocalMessageCache::parse_iso_date_to_unix("Fri, 30 Oct 2026 13:00:00 -0300");
    assert!(ts4.is_some(), "Deveria parsear RFC 2822 com dia da semana");
    assert_eq!(ts1.unwrap(), ts4.unwrap());
}

#[tokio::test]
async fn test_eager_prefetch_bodies_within_90_days() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::new(temp.path());
    let client = Arc::new(InMemoryApiClient::new());

    let now_unix: u64 = 1793376000; // 30 Oct 2026 16:00:00 UTC

    // Envelope 1: Recente (5 dias atrás) - SEM corpo no cache
    let env_recent = MessageEnvelope {
        uid: 101,
        subject: "Novo Projeto".to_string(),
        sender: "alice@example.com".to_string(),
        date: "2026-10-25T16:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let msg_recent = Message {
        envelope: env_recent.clone(),
        txt_auto: Some("Corpo do novo projeto".to_string()),
        txt_original: Some("Corpo do novo projeto".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    client.add_message("INBOX", msg_recent);

    // Envelope 2: Fora da janela de 90 dias (100 dias atrás) - SEM corpo no cache
    let env_old = MessageEnvelope {
        uid: 102,
        subject: "Arquivo Antigo".to_string(),
        sender: "old@example.com".to_string(),
        date: "2026-07-01T10:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let msg_old = Message {
        envelope: env_old.clone(),
        txt_auto: Some("Mensagem antiga".to_string()),
        txt_original: Some("Mensagem antiga".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    client.add_message("INBOX", msg_old);

    // Envelope 3: Recente (10 dias atrás) - JÁ PRESENTE no cache
    let env_cached = MessageEnvelope {
        uid: 103,
        subject: "Já em cache".to_string(),
        sender: "bob@example.com".to_string(),
        date: "2026-10-20T16:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let msg_cached = Message {
        envelope: env_cached.clone(),
        txt_auto: Some("Já cacheado".to_string()),
        txt_original: Some("Já cacheado".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    cache.save_message("INBOX", &msg_cached).expect("save 103");

    let envelopes = vec![env_recent, env_old, env_cached];

    // Executa prefetch eager com concorrência 5
    let prefetched_count = prefetch_missing_bodies(
        &*client,
        "dummy_token",
        "INBOX",
        &envelopes,
        &cache,
        5,
        now_unix,
    )
    .await
    .expect("prefetch success");

    // Deve ter pré-carregado exatamente 1 mensagem (101). A 102 é antiga (>90d), a 103 já estava em cache.
    assert_eq!(prefetched_count, 1, "Deveria pré-carregar apenas a mensagem 101");

    // Verifica se a mensagem 101 agora reside em cache SQLite (acesso atômico 0ms)
    let loaded_101 = cache
        .load_message("INBOX", 101)
        .expect("load")
        .expect("mensagem 101 deve estar em cache");
    assert_eq!(
        loaded_101.txt_auto.as_deref(),
        Some("Corpo do novo projeto")
    );

    // Mensagem antiga (102) não deve ter sido baixada
    let loaded_102 = cache.load_message("INBOX", 102).expect("load");
    assert!(loaded_102.is_none(), "Mensagem com mais de 90 dias não deve ter corpo em cache");
}
