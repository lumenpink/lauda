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
    let parsed =
        CalendarIntentParser::parse("Almoço com Marcelo amanhã às 12h30 no Fasano", &ref_time)
            .expect("parsing failed");

    assert_eq!(parsed.summary, "Almoço");
    assert_eq!(parsed.attendees.len(), 1);
    assert_eq!(parsed.attendees[0].name.as_deref(), Some("Marcelo"));
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
    let parsed_bi = CalendarIntentParser::parse("Sincronização a cada 15 dias às 10h", &ref_time)
        .expect("parsing biweekly failed");
    let rrule_bi = parsed_bi.rrule.expect("esperava rrule biweekly");
    assert_eq!(rrule_bi.freq, RecurrenceFreq::Weekly);
    assert_eq!(rrule_bi.interval, 2);

    let parsed_month = CalendarIntentParser::parse("Fechamento mensal todo mês dia 5", &ref_time)
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
    let parsed = CalendarIntentParser::parse("Plantão no Hospital das 19h às 07h", &ref_time)
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

#[test]
fn test_parse_colloquial_times_and_dates() {
    let ref_time = sample_ref_time();

    // 1. Minutos inversos: "15 pras 2" -> 13:45 (presunção comercial 1..6 -> tarde)
    let p1 = CalendarIntentParser::parse("Almoço 15 pras 2", &ref_time).expect("p1");
    assert_eq!(p1.summary, "Almoço");
    assert!(!p1.is_all_day);
    if let EventTime::DateTimeUtc(ts) = p1.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 13);
        assert_eq!(m, 45);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 2. Minutos inversos com termo fixo: "20 pra meia noite" -> 23:40
    let p2 = CalendarIntentParser::parse("Pizza 20 pra meia noite", &ref_time).expect("p2");
    assert_eq!(p2.summary, "Pizza");
    assert!(!p2.is_all_day);
    if let EventTime::DateTimeUtc(ts) = p2.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 23);
        assert_eq!(m, 40);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 3. Extenso e híbrido com turno explícito: "duas e meia da tarde" -> 14:30
    let p3 = CalendarIntentParser::parse("Reunião duas e meia da tarde", &ref_time).expect("p3");
    assert_eq!(p3.summary, "Reunião");
    if let EventTime::DateTimeUtc(ts) = p3.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 14);
        assert_eq!(m, 30);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 4. Termo nominal: "meio-dia" -> 12:00
    let p4 = CalendarIntentParser::parse("Alinhamento ao meio-dia", &ref_time).expect("p4");
    assert_eq!(p4.summary, "Alinhamento");
    if let EventTime::DateTimeUtc(ts) = p4.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 12);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 5. Escape literal com aspas: "Clube 12 de Agosto" não vira data
    let p5 =
        CalendarIntentParser::parse("Festa no \"Clube 12 de Agosto\" amanhã às 20h", &ref_time)
            .expect("p5");
    assert_eq!(p5.summary, "Festa");
    assert_eq!(p5.location.as_deref(), Some("Clube 12 de Agosto"));
    if let EventTime::DateTimeUtc(ts) = p5.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 10);
        assert_eq!(d, 15);
        assert_eq!(h, 20);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 6. Data ordinal: "primeiro de novembro" -> 01/11/2026
    let p6 =
        CalendarIntentParser::parse("Entrega primeiro de novembro às 10h", &ref_time).expect("p6");
    assert_eq!(p6.summary, "Entrega");
    if let EventTime::DateTimeUtc(ts) = p6.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 11);
        assert_eq!(d, 1);
        assert_eq!(h, 10);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 7. Data de 3 letras / barra: "1/nov às 14h"
    let p7 = CalendarIntentParser::parse("Plantão 1/nov às 14h", &ref_time).expect("p7");
    assert_eq!(p7.summary, "Plantão");
    if let EventTime::DateTimeUtc(ts) = p7.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 11);
        assert_eq!(d, 1);
        assert_eq!(h, 14);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 8. Feriado: "natal às 21h" -> 25/12/2026
    let p8 = CalendarIntentParser::parse("Ceia de natal às 21h", &ref_time).expect("p8");
    assert_eq!(p8.summary, "Ceia");
    if let EventTime::DateTimeUtc(ts) = p8.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 12);
        assert_eq!(d, 25);
        assert_eq!(h, 21);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // 9. Coloquial: "findi às 12h" -> Sábado mais próximo (17/10/2026)
    let p9 = CalendarIntentParser::parse("Churrasco no findi às 12h", &ref_time).expect("p9");
    assert_eq!(p9.summary, "Churrasco");
    if let EventTime::DateTimeUtc(ts) = p9.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 10);
        assert_eq!(d, 18);
        assert_eq!(h, 12);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }
}

#[test]
fn test_parse_colloquial_mutation_and_edge_cases() {
    let ref_time = sample_ref_time(); // 14/10/2026

    // Minutos inversos com turno explícito: "dez pras oito da manhã" -> 07:50
    let m1 = CalendarIntentParser::parse("Revisão dez pras oito da manhã", &ref_time).expect("m1");
    assert_eq!(m1.summary, "Revisão");
    if let EventTime::DateTimeUtc(ts) = m1.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 7);
        assert_eq!(m, 50);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // Minutos inversos comercial: "5 pras 6" -> 17:55
    let m2 = CalendarIntentParser::parse("Happy hour 5 pras 6", &ref_time).expect("m2");
    assert_eq!(m2.summary, "Happy hour");
    if let EventTime::DateTimeUtc(ts) = m2.start {
        let (_, _, _, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(h, 17);
        assert_eq!(m, 55);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // Feriado passado no ano corrente avança para ano seguinte: "tiradentes às 9h" -> 21/04/2027
    let m3 = CalendarIntentParser::parse("Desfile de tiradentes às 9h", &ref_time).expect("m3");
    assert_eq!(m3.summary, "Desfile");
    if let EventTime::DateTimeUtc(ts) = m3.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2027);
        assert_eq!(mo, 4);
        assert_eq!(d, 21);
        assert_eq!(h, 9);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // Data passada no ano corrente avança para ano seguinte: "01/10 às 15h" -> 01/10/2027
    let m4 = CalendarIntentParser::parse("Conferência 01/10 às 15h", &ref_time).expect("m4");
    assert_eq!(m4.summary, "Conferência");
    if let EventTime::DateTimeUtc(ts) = m4.start {
        let (y, mo, d, h, m, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2027);
        assert_eq!(mo, 10);
        assert_eq!(d, 1);
        assert_eq!(h, 15);
        assert_eq!(m, 0);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // Proteção de título com aspas: "'Reunião de Natal' amanhã às 14h"
    let m5 =
        CalendarIntentParser::parse("'Reunião de Natal' amanhã às 14h", &ref_time).expect("m5");
    assert_eq!(m5.summary, "Reunião de Natal");
    if let EventTime::DateTimeUtc(ts) = m5.start {
        let (y, mo, d, h, _, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 10);
        assert_eq!(d, 15); // amanhã
        assert_eq!(h, 14);
    } else {
        panic!("esperava DateTimeUtc");
    }

    // Heurística de logradouro sem aspas: "Almoço no clube 12 de agosto amanhã às 12h"
    let m6 = CalendarIntentParser::parse("Almoço no clube 12 de agosto amanhã às 12h", &ref_time)
        .expect("m6");
    assert_eq!(m6.summary, "Almoço");
    assert_eq!(m6.location.as_deref(), Some("clube 12 de agosto"));
    if let EventTime::DateTimeUtc(ts) = m6.start {
        let (y, mo, d, h, _, _) = lauda_types::calendar::timestamp_to_utc_parts(ts);
        assert_eq!(y, 2026);
        assert_eq!(mo, 10);
        assert_eq!(d, 15); // amanhã
        assert_eq!(h, 12);
    } else {
        panic!("esperava DateTimeUtc");
    }
}

#[test]
fn test_parse_attendees_extraction() {
    let ref_time = sample_ref_time();

    // 1. Participante simples com conectivo "com"
    let p1 =
        CalendarIntentParser::parse("Almoço com Marcelo amanhã às 12h30", &ref_time).expect("p1");
    assert_eq!(p1.summary, "Almoço");
    assert_eq!(p1.attendees.len(), 1);
    assert_eq!(p1.attendees[0].name.as_deref(), Some("Marcelo"));
    assert_eq!(p1.attendees[0].status, RsvpStatus::NeedsAction);

    // 2. Múltiplos participantes: "Reunião com Alice e Bob sexta às 15h"
    let p2 =
        CalendarIntentParser::parse("Reunião com Alice e Bob sexta às 15h", &ref_time).expect("p2");
    assert_eq!(p2.summary, "Reunião");
    assert_eq!(p2.attendees.len(), 2);
    assert_eq!(p2.attendees[0].name.as_deref(), Some("Alice"));
    assert_eq!(p2.attendees[1].name.as_deref(), Some("Bob"));

    // 3. Conectivo composto: "Alinhamento junto a Carlos, Daniel e Elisa"
    let p3 = CalendarIntentParser::parse("Alinhamento junto a Carlos, Daniel e Elisa", &ref_time)
        .expect("p3");
    assert_eq!(p3.summary, "Alinhamento");
    assert_eq!(p3.attendees.len(), 3);
    assert_eq!(p3.attendees[0].name.as_deref(), Some("Carlos"));
    assert_eq!(p3.attendees[1].name.as_deref(), Some("Daniel"));
    assert_eq!(p3.attendees[2].name.as_deref(), Some("Elisa"));

    // 4. Participante com e-mail: "Sync com dev@lohn.in amanhã às 11h"
    let p4 =
        CalendarIntentParser::parse("Sync com dev@lohn.in amanhã às 11h", &ref_time).expect("p4");
    assert_eq!(p4.summary, "Sync");
    assert_eq!(p4.attendees.len(), 1);
    assert_eq!(p4.attendees[0].email, "dev@lohn.in");

    // 5. Escape com aspas: "'Almoço com Marcelo' amanhã"
    let p5 = CalendarIntentParser::parse("'Almoço com Marcelo' amanhã", &ref_time).expect("p5");
    assert_eq!(p5.summary, "Almoço com Marcelo");
    assert!(p5.attendees.is_empty());
}
