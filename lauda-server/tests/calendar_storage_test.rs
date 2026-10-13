use lauda_server::adapters::sqlite_calendar::SqliteCalendarStore;
use lauda_types::calendar::{
    CalendarEvent, EventStatus, EventTime, RecurrenceFreq, RecurrenceRule, Weekday,
};

#[test]
fn test_sqlite_calendar_crud_and_range_query() {
    let temp_dir = tempfile::tempdir().expect("Criar tempdir");
    let db_path = temp_dir.path().join("calendar.db");
    let store = SqliteCalendarStore::open(&db_path).expect("Abrir sqlite");

    let event1 = CalendarEvent {
        uid: "ev-1".to_string(),
        summary: "Reunião de Arquitetura".to_string(),
        description: Some("Alinhamento do CalDAV".to_string()),
        location: Some("Sala 42".to_string()),
        start: EventTime::DateTimeUtc(1791968400),
        end: EventTime::DateTimeUtc(1791972000),
        is_all_day: false,
        rrule: Some(RecurrenceRule {
            freq: RecurrenceFreq::Weekly,
            interval: 1,
            count: None,
            until: None,
            by_day: vec![Weekday::Tuesday],
        }),
        status: EventStatus::Confirmed,
        organizer: None,
        attendees: Vec::new(),
        sequence: 1,
        created_at: None,
        updated_at: None,
    };

    // 1. Inserção
    let etag1 = store.upsert_event("lumen", &event1).expect("upsert event1");
    assert!(!etag1.is_empty());

    // 2. Consulta direta
    let fetched = store.get_event_raw("lumen", "ev-1").expect("get_event_raw");
    assert!(fetched.is_some());
    let (ics_raw, etag) = fetched.unwrap();
    assert_eq!(etag, etag1);
    assert!(ics_raw.contains("SUMMARY:Reunião de Arquitetura"));

    // 3. Consulta por janela temporal
    let in_range = store
        .query_events_by_range("lumen", 1791960000, 1791980000)
        .expect("query_events_by_range");
    assert_eq!(in_range.len(), 1);
    assert_eq!(in_range[0].0, "ev-1");

    let out_range = store
        .query_events_by_range("lumen", 1792000000, 1792100000)
        .expect("query_events_by_range fora");
    assert_eq!(out_range.len(), 0);

    // 4. Sincronização incremental
    let sync1 = store.sync_events("lumen", None).expect("sync_events");
    assert_eq!(sync1.updated.len(), 1);
    assert_eq!(sync1.deleted_ids.len(), 0);
    let token1 = sync1.sync_token;

    // 5. Exclusão suave (tombstone)
    let deleted_ok = store.delete_event("lumen", "ev-1").expect("delete_event");
    assert!(deleted_ok);

    // Consulta após deleção não deve encontrar o evento ativo
    let fetched_deleted = store.get_event_raw("lumen", "ev-1").expect("get_event_raw");
    assert!(fetched_deleted.is_none());

    // 6. Sincronização incremental reportando tombstone
    let sync2 = store
        .sync_events("lumen", Some(&token1))
        .expect("sync_events token1");
    assert_eq!(sync2.updated.len(), 0);
    assert_eq!(sync2.deleted_ids.len(), 1);
    assert_eq!(sync2.deleted_ids[0], "ev-1");
}
