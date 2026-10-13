use lauda_types::calendar::{
    CalendarEvent, CalendarIntentParser, CalendarReferenceTime, EventAttendee, EventStatus,
    EventTime, RecurrenceFreq, RecurrenceRule, RsvpStatus, Weekday,
};

fn sample_ref_time() -> CalendarReferenceTime {
    // Terça-feira, 14 de Outubro de 2026, 09:00 UTC (timestamp: 1791968400)
    CalendarReferenceTime {
        year: 2026,
        month: 10,
        day: 14,
        hour: 9,
        minute: 0,
        weekday: Weekday::Tuesday,
        timestamp_utc: 1791968400,
    }
}

#[test]
fn test_parse_simple_time_and_summary() {
    let ref_time = sample_ref_time();
    let parsed = CalendarIntentParser::parse(
        "Almoço com Marcelo amanhã às 12h30 no Fasano",
        &ref_time,
    )
    .expect("parsing failed");

    assert_eq!(parsed.summary, "Almoço com Marcelo");
    assert_eq!(parsed.location.as_deref(), Some("Fasano"));
    assert!(!parsed.is_all_day);

    // Amanhã é 15/10/2026
    if let EventTime::DateTimeUtc(ts) = parsed.start {
        // 15/10/2026 12:30 UTC
        let expected = 1791968400 + 86400 + (3 * 3600) + (30 * 60);
        assert_eq!(ts, expected);
    } else {
        panic!("esperava DateTimeUtc para início");
    }

    // Duração presumida: 1 hora (13:30)
    if let EventTime::DateTimeUtc(ts) = parsed.end {
        let expected = 1791968400 + 86400 + (4 * 3600) + (30 * 60);
        assert_eq!(ts, expected);
    } else {
        panic!("esperava DateTimeUtc para término");
    }
}

#[test]
fn test_parse_time_range_and_location() {
    let ref_time = sample_ref_time();
    let parsed = CalendarIntentParser::parse(
        "Reunião de alinhamento sexta das 14h às 15h30 na Sala 2",
        &ref_time,
    )
    .expect("parsing failed");

    assert_eq!(parsed.summary, "Reunião de alinhamento");
    assert_eq!(parsed.location.as_deref(), Some("Sala 2"));
    assert!(!parsed.is_all_day);
}

#[test]
fn test_parse_all_day_event() {
    let ref_time = sample_ref_time();
    let parsed = CalendarIntentParser::parse("Aniversário da Clara amanhã", &ref_time)
        .expect("parsing failed");

    assert_eq!(parsed.summary, "Aniversário da Clara");
    assert!(parsed.is_all_day);
    assert!(parsed.location.is_none());
}

#[test]
fn test_parse_recurring_weekly() {
    let ref_time = sample_ref_time();
    let parsed = CalendarIntentParser::parse("Dentista toda terça às 14h", &ref_time)
        .expect("parsing failed");

    assert_eq!(parsed.summary, "Dentista");
    assert!(!parsed.is_all_day);
    let rrule = parsed.rrule.expect("esperava rrule");
    assert_eq!(rrule.freq, RecurrenceFreq::Weekly);
    assert_eq!(rrule.interval, 1);
    assert_eq!(rrule.by_day, vec![Weekday::Tuesday]);
}

#[test]
fn test_parse_biweekly_and_monthly() {
    let ref_time = sample_ref_time();
    let parsed_bi = CalendarIntentParser::parse(
        "Sincronização a cada 15 dias às 10h",
        &ref_time,
    )
    .expect("parsing biweekly failed");
    let rrule_bi = parsed_bi.rrule.expect("esperava rrule biweekly");
    assert_eq!(rrule_bi.freq, RecurrenceFreq::Weekly);
    assert_eq!(rrule_bi.interval, 2);

    let parsed_month =
        CalendarIntentParser::parse("Fechamento mensal todo mês dia 5", &ref_time)
            .expect("parsing monthly failed");
    let rrule_month = parsed_month.rrule.expect("esperava rrule mensal");
    assert_eq!(rrule_month.freq, RecurrenceFreq::Monthly);
}

#[test]
fn test_ical_serialization_roundtrip() {
    let event = CalendarEvent {
        uid: "event-sovereign-1234@twolp.in".to_string(),
        summary: "Alinhamento Estratégico".to_string(),
        description: Some("Reunião de alinhamento do ecossistema Lauda".to_string()),
        location: Some("Sala 101".to_string()),
        start: EventTime::DateTimeUtc(1791968400),
        end: EventTime::DateTimeUtc(1791972000),
        is_all_day: false,
        rrule: Some(RecurrenceRule {
            freq: RecurrenceFreq::Weekly,
            interval: 1,
            count: Some(10),
            until: None,
            by_day: vec![Weekday::Monday],
        }),
        status: EventStatus::Confirmed,
        organizer: Some(EventAttendee {
            email: "lumen@2lp.in".to_string(),
            name: Some("Lumen Pink".to_string()),
            status: RsvpStatus::Accepted,
        }),
        attendees: vec![EventAttendee {
            email: "marcelo@2lp.in".to_string(),
            name: Some("Marcelo".to_string()),
            status: RsvpStatus::NeedsAction,
        }],
        sequence: 1,
        created_at: Some(1791900000),
        updated_at: Some(1791900000),
    };

    let ical_str = event.to_ical();
    assert!(ical_str.contains("BEGIN:VCALENDAR"));
    assert!(ical_str.contains("BEGIN:VEVENT"));
    assert!(ical_str.contains("UID:event-sovereign-1234@twolp.in"));
    assert!(ical_str.contains("SUMMARY:Alinhamento Estratégico"));
    assert!(ical_str.contains("LOCATION:Sala 101"));
    assert!(ical_str.contains("RRULE:FREQ=WEEKLY;INTERVAL=1;COUNT=10;BYDAY=MO"));
    assert!(ical_str.contains("STATUS:CONFIRMED"));
    assert!(ical_str.contains("ORGANIZER;CN=Lumen Pink:mailto:lumen@2lp.in"));
    assert!(ical_str.contains("ATTENDEE;PARTSTAT=NEEDS-ACTION;CN=Marcelo:mailto:marcelo@2lp.in"));
    assert!(ical_str.contains("END:VEVENT"));
    assert!(ical_str.contains("END:VCALENDAR"));

    let parsed = CalendarEvent::from_ical(&ical_str).expect("deserialization failed");
    assert_eq!(parsed.uid, event.uid);
    assert_eq!(parsed.summary, event.summary);
    assert_eq!(parsed.location, event.location);
    assert_eq!(parsed.status, event.status);
    assert_eq!(parsed.start, event.start);
    assert_eq!(parsed.end, event.end);
    assert_eq!(parsed.rrule, event.rrule);
}

#[test]
fn test_parse_overnight_span() {
    let ref_time = sample_ref_time();
    let parsed = CalendarIntentParser::parse(
        "Plantão no Hospital das 19h às 07h",
        &ref_time,
    )
    .expect("parsing failed");

    assert_eq!(parsed.summary, "Plantão");
    assert_eq!(parsed.location.as_deref(), Some("Hospital"));
    assert!(!parsed.is_all_day);

    if let (EventTime::DateTimeUtc(start_ts), EventTime::DateTimeUtc(end_ts)) =
        (parsed.start, parsed.end)
    {
        // O término deve ser exatamente 12 horas depois (19h até 07h do dia seguinte)
        assert_eq!(end_ts - start_ts, 12 * 3600);
    } else {
        panic!("esperava timestamps UTC");
    }
}

#[test]
fn test_ical_unfolding() {
    let raw = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:unfolded-123\r\n\
DTSTART:20261014T100000Z\r\n\
DTEND:20261014T110000Z\r\n\
SUMMARY:Este é um resumo muito longo que foi dobrado pelo servidor c\r\n \
alDAV para respeitar o limite de 75 octetos da RFC 5545\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let event = CalendarEvent::from_ical(raw).expect("deve desdobrar linhas");
    assert_eq!(
        event.summary,
        "Este é um resumo muito longo que foi dobrado pelo servidor calDAV para respeitar o limite de 75 octetos da RFC 5545"
    );
}

#[test]
fn test_parse_empty_input_error() {
    let ref_time = sample_ref_time();
    let result = CalendarIntentParser::parse("   ", &ref_time);
    assert!(result.is_err());
}

