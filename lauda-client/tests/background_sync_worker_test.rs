use std::sync::Arc;
use tempfile::tempdir;

use lauda_client::adapters::in_memory::InMemoryApiClient;
use lauda_client::domain::cache::LocalMessageCache;
use lauda_client::domain::calendar_cache::LocalCalendarCache;
use lauda_client::domain::contacts_cache::LocalContactsCache;
use lauda_client::domain::ports::CalendarSyncResponse;
use lauda_client::domain::sync::{
    ActionType, ActionsQueue, BackgroundSyncWorker, SyncCycleResult, SyncWorkerConfig,
    calculate_next_backoff, detect_new_unread_messages, sync_all_cycle,
};
use lauda_types::calendar::{CalendarEvent, EventStatus, EventTime};
use lauda_types::contact::ContactEntry;
use lauda_types::{Message, MessageEnvelope};

#[test]
fn test_backoff_algorithm_exponential_growth_and_reset() {
    let config = SyncWorkerConfig {
        poll_interval_secs: 60,
        min_backoff_secs: 5,
        max_backoff_secs: 80,
        retention_days: 90,
        prefetch_concurrency: 5,
        notify_on_new: true,
    };

    // 1. Falha inicial parte de 0 -> min_backoff (5s)
    let b1 = calculate_next_backoff(0, false, &config);
    assert_eq!(b1, 5);

    // 2. Falhas consecutivas dobram
    let b2 = calculate_next_backoff(b1, false, &config);
    assert_eq!(b2, 10);

    let b3 = calculate_next_backoff(b2, false, &config);
    assert_eq!(b3, 20);

    let b4 = calculate_next_backoff(b3, false, &config);
    assert_eq!(b4, 40);

    let b5 = calculate_next_backoff(b4, false, &config);
    assert_eq!(b5, 80);

    // 3. Respeita o teto máximo de 80s
    let b6 = calculate_next_backoff(b5, false, &config);
    assert_eq!(b6, 80);

    // 4. Sucesso restaura o intervalo nominal de 60s
    let reset = calculate_next_backoff(b6, true, &config);
    assert_eq!(reset, 60);
}

#[test]
fn test_detect_new_unread_messages() {
    let env_cached = MessageEnvelope {
        uid: 10,
        subject: "Antigo lido".to_string(),
        sender: "a@ex.com".to_string(),
        date: "2026-10-01T00:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let env_cached2 = MessageEnvelope {
        uid: 11,
        subject: "Antigo não lido".to_string(),
        sender: "b@ex.com".to_string(),
        date: "2026-10-02T00:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let env_new_read = MessageEnvelope {
        uid: 12,
        subject: "Novo mas já lido".to_string(),
        sender: "c@ex.com".to_string(),
        date: "2026-10-30T10:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let env_new_unread = MessageEnvelope {
        uid: 13,
        subject: "Novo e não lido urgente".to_string(),
        sender: "d@ex.com".to_string(),
        date: "2026-10-30T11:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let cached = vec![env_cached.clone(), env_cached2.clone()];
    let remote = vec![
        env_cached,
        env_cached2,
        env_new_read,
        env_new_unread.clone(),
    ];

    let notifs = detect_new_unread_messages(&remote, &cached);
    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].uid, 13);
    assert_eq!(notifs[0].sender, "d@ex.com");
    assert_eq!(notifs[0].subject, "Novo e não lido urgente");
}

#[tokio::test]
async fn test_sync_all_cycle_triad_mail_contacts_calendar() {
    let temp = tempdir().expect("tempdir");
    let cache_dir = temp.path();

    let client = Arc::new(InMemoryApiClient::new());
    let now_unix: u64 = 1793376000; // 30 Oct 2026 16:00:00 UTC

    // 1. Preparar fila de ações pendentes (Outbox)
    let queue = ActionsQueue::new(cache_dir);
    queue
        .enqueue(55, "INBOX", ActionType::MarkRead { read: true }, now_unix)
        .expect("enqueue");
    assert_eq!(queue.count().unwrap(), 1);

    // 2. Preparar mensagens no servidor
    let env_recent = MessageEnvelope {
        uid: 201,
        subject: "Reunião de Alinhamento".to_string(),
        sender: "coordenacao@example.com".to_string(),
        date: "2026-10-29T10:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let msg_recent = Message {
        envelope: env_recent.clone(),
        txt_auto: Some("Pauta da reunião".to_string()),
        txt_original: Some("Pauta da reunião".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    client.add_message("INBOX", msg_recent);

    // Mensagem expirada antiga no cache (> 90 dias) para verificar purga
    let msg_cache = LocalMessageCache::new(cache_dir);
    let env_expired = MessageEnvelope {
        uid: 99,
        subject: "Email antigo de maio".to_string(),
        sender: "old@example.com".to_string(),
        date: "2026-05-01T10:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };
    let msg_expired = Message {
        envelope: env_expired.clone(),
        txt_auto: Some("corpo antigo".to_string()),
        txt_original: Some("corpo antigo".to_string()),
        html_sanitized: None,
        attachments: vec![],
    };
    msg_cache
        .save_message("INBOX", &msg_expired)
        .expect("save expired");

    // 3. Inicializar cache de contatos e mock de sincronização
    let contacts_cache = LocalContactsCache::open(cache_dir).expect("contacts cache");
    assert_eq!(contacts_cache.count_contacts().unwrap(), 0);

    let new_contact = ContactEntry {
        id: "c-1".to_string(),
        username: "lumen".to_string(),
        display_name: Some("Lisiane Professora".to_string()),
        primary_email: "lisiane@example.com".to_string(),
        secondary_emails: vec![],
        phone_numbers: vec![],
        birthday: None,
        notes: None,
        vcard_raw: String::new(),
        has_autocrypt: false,
        autocrypt_fingerprint: None,
        etag: "etag-c1".to_string(),
        version: 1,
        created_at: now_unix,
        updated_at: now_unix,
        deleted_at: None,
    };
    client.set_contacts_sync_response(lauda_types::contact::ContactSyncResponse {
        sync_token: "ts:contacts-1".to_string(),
        updated: vec![new_contact],
        deleted_ids: vec![],
        has_more: false,
    });

    // 4. Inicializar cache de calendário e mock de sincronização
    let cal_cache = LocalCalendarCache::open(cache_dir).expect("cal cache");
    assert_eq!(cal_cache.count_events().unwrap(), 0);

    let new_event = CalendarEvent {
        uid: "ev-1".to_string(),
        summary: "Banca de TCC".to_string(),
        description: None,
        location: Some("Sala 402".to_string()),
        start: EventTime::DateTimeUtc(now_unix as i64 + 3600),
        end: EventTime::DateTimeUtc(now_unix as i64 + 7200),
        is_all_day: false,
        rrule: None,
        status: EventStatus::Confirmed,
        organizer: None,
        attendees: vec![],
        sequence: 0,
        created_at: Some(now_unix as i64),
        updated_at: Some(now_unix as i64),
    };
    client.set_calendar_sync_response(CalendarSyncResponse {
        sync_token: "ts:calendar-1".to_string(),
        updated: vec![new_event],
        deleted_ids: vec![],
    });

    // 5. Configuração do worker
    let config = SyncWorkerConfig {
        poll_interval_secs: 60,
        min_backoff_secs: 5,
        max_backoff_secs: 300,
        retention_days: 90,
        prefetch_concurrency: 5,
        notify_on_new: false, // Desliga spawn de notify-send em testes
    };

    // Executa ciclo unificado
    let summary: SyncCycleResult = sync_all_cycle(
        client.as_ref(),
        "mock-token",
        "INBOX",
        cache_dir,
        &config,
        now_unix,
    )
    .await
    .expect("sync_all_cycle deve ter sucesso");

    // Validações de correio:
    assert_eq!(
        summary.actions_flushed, 1,
        "Deveria ter esvaziado 1 ação pendente"
    );
    assert_eq!(queue.count().unwrap(), 0, "Fila de ações deve estar zerada");
    assert_eq!(
        summary.messages_prefetched, 1,
        "Deveria ter baixado o corpo da msg 201"
    );
    assert_eq!(
        summary.new_unread_messages, 1,
        "Deveria ter detectado 1 nova msg não lida"
    );
    assert_eq!(
        summary.messages_purged, 1,
        "Deveria ter expurgado a msg de maio (> 90 dias)"
    );

    // O corpo da msg 201 deve estar presente no cache local
    let loaded = msg_cache.load_message("INBOX", 201).unwrap();
    assert!(loaded.is_some());
    assert_eq!(
        loaded.unwrap().txt_auto.as_deref(),
        Some("Pauta da reunião")
    );

    // O corpo da msg 99 foi purgado
    assert!(msg_cache.load_message("INBOX", 99).unwrap().is_none());

    // Validações de contatos e agenda:
    assert_eq!(
        summary.contacts_updated, 1,
        "Deveria ter reconciliado 1 contato"
    );
    assert_eq!(contacts_cache.count_contacts().unwrap(), 1);
    assert_eq!(
        summary.calendar_updated, 1,
        "Deveria ter reconciliado 1 evento de agenda"
    );
    assert_eq!(cal_cache.count_events().unwrap(), 1);
}

#[tokio::test]
async fn test_background_sync_worker_trigger_and_shutdown() {
    let temp = tempdir().expect("tempdir");
    let cache_dir = temp.path();
    let client = Arc::new(InMemoryApiClient::new());

    let config = SyncWorkerConfig {
        poll_interval_secs: 1000, // Alto para não rodar pelo timer
        min_backoff_secs: 1,
        max_backoff_secs: 5,
        retention_days: 90,
        prefetch_concurrency: 2,
        notify_on_new: false,
    };

    let (worker, mut handle) = BackgroundSyncWorker::new(
        client.clone(),
        "mock-token".to_string(),
        "INBOX".to_string(),
        cache_dir.to_path_buf(),
        config,
    );

    // Spawna worker em segundo plano
    let join_handle = tokio::spawn(async move {
        worker.run().await;
    });

    // Dispara sincronização imediata (como faria o onResume do Android ou Fast Resume)
    handle.trigger_sync().await.expect("trigger");

    // Aguarda um instante para a rodada terminar e envia shutdown
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    handle.shutdown().await.expect("shutdown");

    // Worker deve finalizar de forma limpa
    let res = tokio::time::timeout(tokio::time::Duration::from_millis(500), join_handle).await;
    assert!(
        res.is_ok(),
        "Worker deve terminar dentro do timeout pós shutdown"
    );
}

#[test]
fn test_backoff_mutations_boundary_and_overflow() {
    let config = SyncWorkerConfig {
        poll_interval_secs: 45,
        min_backoff_secs: 3,
        max_backoff_secs: 100,
        retention_days: 90,
        prefetch_concurrency: 5,
        notify_on_new: true,
    };

    // Mutação 1: Falha com backoff inicial 0 deve sempre resultar em min_backoff_secs
    assert_eq!(calculate_next_backoff(0, false, &config), 3);

    // Mutação 2: Falha com valor próximo do teto (60 * 2 = 120 > 100) deve cravar em max_backoff_secs
    assert_eq!(calculate_next_backoff(60, false, &config), 100);

    // Mutação 3: Falha com valor extremo (u64::MAX) não deve entrar em pânico nem dar overflow
    assert_eq!(calculate_next_backoff(u64::MAX, false, &config), 100);

    // Mutação 4: Sucesso após qualquer valor (inclusive extremo) deve restaurar exatamente poll_interval_secs
    assert_eq!(calculate_next_backoff(u64::MAX, true, &config), 45);
    assert_eq!(calculate_next_backoff(0, true, &config), 45);
}

#[test]
fn test_detect_new_unread_messages_strict_invariants() {
    let env_cached_read = MessageEnvelope {
        uid: 1,
        subject: "Msg 1".to_string(),
        sender: "u1@ex.com".to_string(),
        date: "2026-10-01T00:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let env_cached_unread = MessageEnvelope {
        uid: 2,
        subject: "Msg 2".to_string(),
        sender: "u2@ex.com".to_string(),
        date: "2026-10-02T00:00:00Z".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let env_new_read = MessageEnvelope {
        uid: 3,
        subject: "Msg 3".to_string(),
        sender: "u3@ex.com".to_string(),
        date: "2026-10-30T00:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let cached = vec![env_cached_read, env_cached_unread.clone()];
    // remote possui as mesmas mais uma nova já lida
    let remote = vec![env_cached_unread, env_new_read];

    let notifs = detect_new_unread_messages(&remote, &cached);
    // Nenhuma notificação deve ser emitida (uma já era conhecida e a outra é lida)
    assert!(
        notifs.is_empty(),
        "Não deve notificar mensagens já conhecidas ou já lidas"
    );
}
