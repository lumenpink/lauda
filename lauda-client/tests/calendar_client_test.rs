use lauda_client::domain::calendar_cache::LocalCalendarCache;
use lauda_types::calendar::{CalendarEvent, EventAttendee, EventStatus, EventTime, RsvpStatus};
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

#[test]
fn test_calendar_event_attendees_preservation_and_cache() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalCalendarCache::open(temp.path()).expect("open cache");

    let mut ev = sample_event("ev-att-1", "Almoço", 1791968400);
    ev.attendees = vec![
        EventAttendee {
            email: "marcelo@example.com".to_string(),
            name: Some("Marcelo".to_string()),
            status: RsvpStatus::NeedsAction,
        },
        EventAttendee {
            email: String::new(),
            name: Some("Beatriz".to_string()),
            status: RsvpStatus::NeedsAction,
        },
    ];

    cache
        .upsert_event(&ev)
        .expect("upsert event with attendees");
    let loaded = cache.load_events().expect("load events");
    assert_eq!(loaded.len(), 1);
    let loaded_ev = &loaded[0];
    assert_eq!(loaded_ev.attendees.len(), 2);
    assert_eq!(loaded_ev.attendees[0].name.as_deref(), Some("Marcelo"));
    assert_eq!(loaded_ev.attendees[0].email, "marcelo@example.com");
    assert_eq!(loaded_ev.attendees[1].name.as_deref(), Some("Beatriz"));
}

#[test]
fn test_quick_add_default_placeholder_is_generic_and_parseable() {
    use lauda_client::ui::calendar_view::QUICK_ADD_DEFAULT_PLACEHOLDER;
    use lauda_types::calendar::{CalendarIntentParser, CalendarReferenceTime, Weekday};

    // 1. Constante deve ser neutra e conter o formato genérico
    assert_eq!(
        QUICK_ADD_DEFAULT_PLACEHOLDER,
        "Ex: Reunião de equipe amanhã às 14h na Sala 2"
    );

    // 2. Não deve conter termos pessoais ou locais específicos depreciados
    assert!(!QUICK_ADD_DEFAULT_PLACEHOLDER.contains("Marcelo"));
    assert!(!QUICK_ADD_DEFAULT_PLACEHOLDER.contains("Fasano"));

    // 3. O exemplo sugerido (sem o prefixo 'Ex: ') deve ser perfeitamente interpretável pelo parser
    let example_input = QUICK_ADD_DEFAULT_PLACEHOLDER
        .strip_prefix("Ex: ")
        .expect("Prefixo 'Ex: ' deve estar presente");

    let ref_time = CalendarReferenceTime {
        year: 2026,
        month: 10,
        day: 15,
        hour: 9,
        minute: 0,
        weekday: Weekday::Thursday,
        timestamp_utc: 1792054800,
    };

    let parsed = CalendarIntentParser::parse(example_input, &ref_time)
        .expect("O texto do placeholder deve ser interpretável pelo parser");

    assert!(parsed.summary.to_lowercase().contains("reunião de equipe"));
    assert_eq!(parsed.location.as_deref(), Some("Sala 2"));
    let (_y, _m, _d, h, min, _s) = match parsed.start {
        lauda_types::calendar::EventTime::DateTimeUtc(ts) => {
            lauda_types::calendar::timestamp_to_utc_parts(ts)
        }
        lauda_types::calendar::EventTime::DateOnly { year, month, day } => {
            (year, month, day, 0, 0, 0)
        }
    };
    assert_eq!(h, 14);
    assert_eq!(min, 0);
}

#[test]
fn test_quick_add_default_placeholder_mutation_and_edge_cases() {
    use lauda_client::ui::calendar_view::QUICK_ADD_DEFAULT_PLACEHOLDER;
    use lauda_types::calendar::{CalendarIntentParser, CalendarReferenceTime, Weekday};

    // Não pode ser vazio nem conter somente espaços
    assert!(!QUICK_ADD_DEFAULT_PLACEHOLDER.trim().is_empty());

    // Deve conter prefixo instrutivo 'Ex: '
    assert!(QUICK_ADD_DEFAULT_PLACEHOLDER.starts_with("Ex: "));

    // Deve cobrir os marcadores essenciais de intenção natural
    let lower = QUICK_ADD_DEFAULT_PLACEHOLDER.to_lowercase();
    assert!(lower.contains("amanhã"));
    assert!(lower.contains("14h"));
    assert!(lower.contains("sala"));

    // O parser deve rejeitar entrada vazia (mutante de omissão)
    let ref_time = CalendarReferenceTime {
        year: 2026,
        month: 10,
        day: 15,
        hour: 9,
        minute: 0,
        weekday: Weekday::Thursday,
        timestamp_utc: 1792054800,
    };
    assert!(CalendarIntentParser::parse("", &ref_time).is_err());
    assert!(CalendarIntentParser::parse("   ", &ref_time).is_err());
}

#[test]
fn test_select_upcoming_events_chronological_and_limit() {
    use lauda_client::ui::calendar_view::select_upcoming_events;

    let now_ts = 100_000;

    // 1. Lista vazia
    assert!(select_upcoming_events(&[], now_ts, 3).is_empty());

    // 2. Apenas eventos passados
    let past1 = sample_event("p1", "Passado 1", now_ts - 500);
    let past2 = sample_event("p2", "Passado 2", now_ts - 100);
    assert!(select_upcoming_events(&[past1.clone(), past2.clone()], now_ts, 3).is_empty());

    // 3. Mistura com 5 futuros fora de ordem e passados
    let fut5 = sample_event("f5", "Futuro 5", now_ts + 5000);
    let fut1 = sample_event("f1", "Futuro 1", now_ts + 1000);
    let fut4 = sample_event("f4", "Futuro 4", now_ts + 4000);
    let fut2 = sample_event("f2", "Futuro 2", now_ts + 2000);
    let fut3 = sample_event("f3", "Futuro 3", now_ts + 3000);

    let all_events = vec![
        fut5,
        past1,
        fut1,
        past2,
        fut4,
        fut2,
        fut3,
    ];

    let selected = select_upcoming_events(&all_events, now_ts, 3);
    assert_eq!(selected.len(), 3);
    assert_eq!(selected[0].uid, "f1");
    assert_eq!(selected[1].uid, "f2");
    assert_eq!(selected[2].uid, "f3");

    // 4. Se houver apenas 2 eventos futuros, retorna os 2
    let two_selected = select_upcoming_events(&all_events[..4], now_ts, 3);
    // all_events[..4] possui fut5, past1, fut1, past2 -> futuros são fut1 e fut5
    assert_eq!(two_selected.len(), 2);
    assert_eq!(two_selected[0].uid, "f1");
    assert_eq!(two_selected[1].uid, "f5");
}
