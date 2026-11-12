use std::sync::{Arc, Mutex};
use tempfile::tempdir;

use lauda_client::adapters::InMemoryApiClient;
use lauda_client::domain::cache::LocalMessageCache;
use lauda_client::domain::sync::{
    SyncWorkerConfig, prefetch_missing_bodies_with_progress, sync_all_cycle_with_progress,
};
use lauda_types::{Message, MessageEnvelope};

fn make_msg(uid: u32, date: &str) -> Message {
    Message {
        envelope: MessageEnvelope {
            uid,
            subject: format!("Mensagem {}", uid),
            sender: "remetente@2lp.in".to_string(),
            date: date.to_string(),
            is_read: false,
            is_flagged: false,
            cc: None,
            has_attachments: false,
            crypto_meta: None,
        },
        txt_auto: Some(format!("Corpo da mensagem {}", uid)),
        txt_original: None,
        html_sanitized: None,
        attachments: Vec::new(),
    }
}

#[tokio::test]
async fn test_prefetch_missing_bodies_with_progress_decrements_progress_to_zero() {
    let tmp = tempdir().unwrap();
    let cache = LocalMessageCache::new(tmp.path());
    let client = InMemoryApiClient::new();

    let mut envelopes = Vec::new();
    for uid in 1..=12 {
        let msg = make_msg(uid, "2026-11-12T10:00:00Z");
        client.add_message("INBOX", msg.clone());
        envelopes.push(msg.envelope);
    }

    let progress_events = Arc::new(Mutex::new(Vec::new()));
    let progress_events_clone = Arc::clone(&progress_events);

    let now_unix = 1794400000; // ~Nov 2026
    let prefetched = prefetch_missing_bodies_with_progress(
        &client,
        "token",
        "INBOX",
        &envelopes,
        &cache,
        5, // concurrency limit 5 -> chunks de 5, 5, 2
        now_unix,
        move |remaining| {
            progress_events_clone.lock().unwrap().push(remaining);
        },
    )
    .await
    .expect("prefetch");

    assert_eq!(prefetched, 12);
    let events = progress_events.lock().unwrap().clone();
    // Deve emitir: inicial (12), pós chunk 1 (7), pós chunk 2 (2), pós chunk 3 (0)
    assert_eq!(events, vec![12, 7, 2, 0]);
}

#[tokio::test]
async fn test_sync_all_cycle_with_progress_emits_telemetry() {
    let tmp = tempdir().unwrap();
    let cache_dir = tmp.path().to_path_buf();
    let client = InMemoryApiClient::new();

    for uid in 1..=7 {
        let msg = make_msg(uid, "2026-11-12T10:00:00Z");
        client.add_message("INBOX", msg);
    }

    let progress_events = Arc::new(Mutex::new(Vec::new()));
    let progress_events_clone = Arc::clone(&progress_events);

    let now_unix = 1794400000;
    let config = SyncWorkerConfig {
        prefetch_concurrency: 5,
        ..Default::default()
    };

    let result = sync_all_cycle_with_progress(
        &client,
        "token",
        "INBOX",
        &cache_dir,
        &config,
        now_unix,
        move |remaining| {
            progress_events_clone.lock().unwrap().push(remaining);
        },
    )
    .await
    .expect("sync_all_cycle");

    assert_eq!(result.messages_prefetched, 7);
    let events = progress_events.lock().unwrap().clone();
    // 7 mensagens com concorrência 5 -> chunks de 5, 2 -> [7, 2, 0]
    assert_eq!(events, vec![7, 2, 0]);
}
