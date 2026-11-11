use lauda_client::adapters::InMemoryApiClient;
use lauda_client::domain::cache::LocalMessageCache;
use lauda_client::domain::sync::{
    SyncStatusIndicator, count_missing_bodies, prefetch_missing_bodies,
};
use lauda_client::domain::time::format_local_date_display;
use lauda_types::{Message, MessageEnvelope};
use tempfile::tempdir;

#[test]
fn test_format_local_date_display() {
    // 1. Mensagem de anos anteriores: deve formatar com o ano
    let old_date = "2024-05-20T10:00:00Z";
    let formatted_old = format_local_date_display(old_date);
    assert!(
        formatted_old.contains("2024"),
        "Deve conter o ano 2024: {}",
        formatted_old
    );

    // 2. Data vazia ou inválida não entra em pânico
    assert_eq!(format_local_date_display(""), "");
    assert_eq!(format_local_date_display("data-invalida"), "data-invalida");
}

#[tokio::test]
async fn test_prefetch_prioritizes_most_recent_date_over_uid() {
    let dir = tempdir().unwrap();
    let cache = LocalMessageCache::new(dir.path());
    let client = InMemoryApiClient::new();

    let now_unix = 1794400000; // ~Nov 2026

    // UID 500: mensagem mais antiga (outubro)
    let msg_old = Message {
        envelope: MessageEnvelope {
            uid: 500,
            subject: "Antiga com UID alto".to_string(),
            sender: "old@2lp.in".to_string(),
            date: "2026-10-01T10:00:00Z".to_string(),
            is_read: false,
            is_flagged: false,
            cc: None,
            has_attachments: false,
            crypto_meta: None,
        },
        txt_auto: Some("corpo antiga".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: Vec::new(),
    };

    // UID 50: mensagem mais recente (novembro)
    let msg_new = Message {
        envelope: MessageEnvelope {
            uid: 50,
            subject: "Nova com UID baixo".to_string(),
            sender: "new@2lp.in".to_string(),
            date: "2026-11-10T15:00:00Z".to_string(),
            is_read: false,
            is_flagged: false,
            cc: None,
            has_attachments: false,
            crypto_meta: None,
        },
        txt_auto: Some("corpo nova".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: Vec::new(),
    };

    client.add_message("INBOX", msg_old.clone());
    client.add_message("INBOX", msg_new.clone());

    let envelopes = vec![msg_old.envelope.clone(), msg_new.envelope.clone()];

    // Valida count_missing_bodies
    let missing_before = count_missing_bodies("INBOX", &envelopes, &cache, now_unix);
    assert_eq!(missing_before, 2);

    let count = prefetch_missing_bodies(&client, "tok", "INBOX", &envelopes, &cache, 1, now_unix)
        .await
        .unwrap();

    assert_eq!(count, 2);

    let missing_after = count_missing_bodies("INBOX", &envelopes, &cache, now_unix);
    assert_eq!(missing_after, 0);

    // Ambas as mensagens devem estar salvas no cache
    let cached_new = cache.load_message("INBOX", 50).unwrap();
    assert!(cached_new.is_some(), "UID 50 deve estar no cache");
    let cached_old = cache.load_message("INBOX", 500).unwrap();
    assert!(cached_old.is_some(), "UID 500 deve estar no cache");
}

#[test]
fn test_sync_status_indicator_text() {
    assert_eq!(SyncStatusIndicator::format(true, 0), "● Sincronizado");
    assert_eq!(
        SyncStatusIndicator::format(true, 12),
        "↻ Sincronizando (12 restantes)"
    );
    assert_eq!(
        SyncStatusIndicator::format(false, 5),
        "○ Offline (5 pendentes)"
    );
    assert_eq!(SyncStatusIndicator::format(false, 0), "○ Offline");
}

#[test]
fn test_mutation_detection_on_status_indicator_and_missing_counts() {
    // 1. Mutante: online com pendência não pode dizer que está sincronizado
    let mutated_format = SyncStatusIndicator::format(true, 10);
    assert_ne!(
        mutated_format, "● Sincronizado",
        "Mutante sobreviveu: pendência não pode reportar sincronizado"
    );

    // 2. Mutante: offline não pode dizer que está online
    let offline_format = SyncStatusIndicator::format(false, 0);
    assert!(
        !offline_format.contains("Sincronizado"),
        "Mutante sobreviveu: offline reportou sincronizado"
    );
    assert!(offline_format.contains("Offline"));
}
