use lauda_client::domain::calendar_cache::LocalCalendarCache;
use lauda_types::calendar::{CalendarEvent, EventStatus, EventTime};
use tempfile::tempdir;

fn sample_event(id: &str, summary: &str, start_ts: i64) -> CalendarEvent {
    CalendarEvent {
        uid: id.to_string(),
        summary: summary.to_string(),
        description: None,
        location: Some("Sala 101".to_string()),
        start: EventTime::DateTimeUtc(start_ts),
        end: EventTime::DateTimeUtc(start_ts + 3600),
        is_all_day: false,
        rrule: None,
        status: EventStatus::Confirmed,
        organizer: None,
        attendees: vec![],
        sequence: 1,
        created_at: Some(start_ts),
        updated_at: Some(start_ts),
    }
}

#[test]
fn test_calendar_local_cache_lifecycle_and_search() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalCalendarCache::open(temp.path()).expect("open cache");

    // 1. Inicialmente vazio
    assert_eq!(cache.count_events().unwrap(), 0);
    assert_eq!(cache.load_events().unwrap().len(), 0);

    // 2. Inserção de evento local (Quick-Add)
    let ev1 = sample_event("ev-1", "Almoço com Marcelo", 1791968400);
    cache.upsert_event(&ev1).expect("upsert ev1");
    assert_eq!(cache.count_events().unwrap(), 1);

    // 3. Busca por título e local
    let found_marcelo = cache.search_events("Marcelo").unwrap();
    assert_eq!(found_marcelo.len(), 1);
    assert_eq!(found_marcelo[0].uid, "ev-1");

    let found_loc = cache.search_events("Sala 101").unwrap();
    assert_eq!(found_loc.len(), 1);

    // 4. Aplicar delta de sincronização
    let ev2 = sample_event("ev-2", "Dentista", 1792000000);
    cache
        .apply_sync_delta(std::slice::from_ref(&ev2), &[], "ts:1792000000")
        .expect("apply delta");
    assert_eq!(cache.count_events().unwrap(), 2);
    assert_eq!(
        cache.get_sync_token().unwrap().as_deref(),
        Some("ts:1792000000")
    );

    // 5. Exclusão e tombstone
    let deleted = cache.delete_event("ev-1").expect("delete ev1");
    assert!(deleted);
    assert_eq!(cache.count_events().unwrap(), 1);

    // Carregar eventos ativos
    let active = cache.load_events().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].uid, "ev-2");
}

#[test]
fn test_detect_ics_invite_in_message() {
    let ics_body = "Olá! Segue o convite da reunião:\r\n\r\n\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:invite-999\r\n\
SUMMARY:Alinhamento Geral\r\n\
DTSTART:20261014T150000Z\r\n\
DTEND:20261014T160000Z\r\n\
LOCATION:Auditório\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let detected = lauda_client::domain::calendar_cache::detect_ics_invite(ics_body);
    assert!(detected.is_some());
    let ev = detected.unwrap();
    assert_eq!(ev.uid, "invite-999");
    assert_eq!(ev.summary, "Alinhamento Geral");
    assert_eq!(ev.location.as_deref(), Some("Auditório"));
}

#[test]
fn test_calendar_event_json_serialization_roundtrip() {
    let ev = sample_event("ev-json-1", "Alinhamento Arquitetural", 1791968400);
    let json_str = serde_json::to_string(&ev).expect("serialize event to json");
    let restored: CalendarEvent =
        serde_json::from_str(&json_str).expect("deserialize event from json");
    assert_eq!(ev.uid, restored.uid);
    assert_eq!(ev.summary, restored.summary);
    assert_eq!(ev.start, restored.start);
    assert_eq!(ev.status, restored.status);
    assert_eq!(ev.location, restored.location);
}
