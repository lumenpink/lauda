use lauda_types::android_calendar_sync::{
    AndroidCalendarBatch, AndroidCalendarOp, LAUDA_CALENDAR_AUTHORITY,
    STATUS_CANCELED, STATUS_CONFIRMED, STATUS_TENTATIVE,
    build_calendar_sync_batch, event_status_to_android,
};
use lauda_types::calendar::{
    CalendarEvent, EventAttendee, EventStatus, EventTime, RecurrenceFreq, RecurrenceRule,
};

fn sample_calendar_event(uid: &str, start_ts: i64, is_all_day: bool) -> CalendarEvent {
    CalendarEvent {
        uid: uid.to_string(),
        summary: "Reunião de Alinhamento".to_string(),
        description: Some("Pauta estratégica".to_string()),
        location: Some("Sala 42".to_string()),
        start: EventTime::DateTimeUtc(start_ts),
        end: EventTime::DateTimeUtc(start_ts + 3600),
        is_all_day,
        rrule: Some(RecurrenceRule {
            freq: RecurrenceFreq::Weekly,
            interval: 1,
            count: None,
            until: None,
            by_day: vec![],
        }),
        status: EventStatus::Confirmed,
        organizer: Some(EventAttendee {
            email: "lumen@lauda.in".to_string(),
            name: Some("Lumen".to_string()),
            status: lauda_types::calendar::RsvpStatus::Accepted,
        }),
        attendees: vec![],
        sequence: 1,
        created_at: Some(start_ts),
        updated_at: Some(start_ts),
    }
}

#[test]
fn test_android_calendar_sync_batch_generation() {
    let ev1 = sample_calendar_event("ev-101", 1791968400, false);
    let mut ev2 = sample_calendar_event("ev-102", 1792000000, true);
    ev2.status = EventStatus::Tentative;
    ev2.summary = "Dia de Foco".to_string();

    let account_name = "hi@lumen.pink";
    let calendar_id = 1;
    let deleted_uids = vec!["ev-999".to_string()];

    let batch: AndroidCalendarBatch = build_calendar_sync_batch(
        account_name,
        calendar_id,
        &[ev1.clone(), ev2.clone()],
        &deleted_uids,
    );

    assert_eq!(batch.account_name, "hi@lumen.pink");
    assert_eq!(batch.operations.len(), 3);

    // Operação 1: Inserção de ev1 com milissegundos
    match &batch.operations[0] {
        AndroidCalendarOp::InsertEvent {
            calendar_id: cid,
            account_name: acc,
            account_type: acc_type,
            sync_id,
            title,
            description,
            location,
            dtstart_ms,
            dtend_ms,
            all_day,
            rrule,
            status,
        } => {
            assert_eq!(*cid, 1);
            assert_eq!(acc, "hi@lumen.pink");
            assert_eq!(acc_type, "in.twolp.lauda");
            assert_eq!(sync_id, "ev-101");
            assert_eq!(title, "Reunião de Alinhamento");
            assert_eq!(description.as_deref(), Some("Pauta estratégica"));
            assert_eq!(location.as_deref(), Some("Sala 42"));
            assert_eq!(*dtstart_ms, 1791968400 * 1000);
            assert_eq!(*dtend_ms, (1791968400 + 3600) * 1000);
            assert!(!all_day);
            assert_eq!(rrule.as_deref(), Some("FREQ=WEEKLY"));
            assert_eq!(*status, STATUS_CONFIRMED);
        }
        _ => panic!("Esperada operação InsertEvent para ev1"),
    }

    // Operação 2: Inserção de ev2 (all_day e Tentative)
    match &batch.operations[1] {
        AndroidCalendarOp::InsertEvent {
            sync_id,
            all_day,
            status,
            ..
        } => {
            assert_eq!(sync_id, "ev-102");
            assert!(*all_day);
            assert_eq!(*status, STATUS_TENTATIVE);
        }
        _ => panic!("Esperada operação InsertEvent para ev2"),
    }

    // Operação 3: Tombstone de deleção
    match &batch.operations[2] {
        AndroidCalendarOp::DeleteEvent { sync_id } => {
            assert_eq!(sync_id, "ev-999");
        }
        _ => panic!("Esperada operação DeleteEvent para ev-999"),
    }
}

#[test]
fn test_status_mapping_and_authority() {
    assert_eq!(LAUDA_CALENDAR_AUTHORITY, "com.android.calendar");
    assert_eq!(event_status_to_android(EventStatus::Confirmed), STATUS_CONFIRMED);
    assert_eq!(event_status_to_android(EventStatus::Tentative), STATUS_TENTATIVE);
    assert_eq!(event_status_to_android(EventStatus::Cancelled), STATUS_CANCELED);
}
