use lauda_types::calendar::{
    CalendarEvent, EventAttendee, EventStatus, EventTime, ItipMethod, RsvpStatus,
    generate_itip_payload,
};

#[test]
fn test_itip_method_codes() {
    assert_eq!(ItipMethod::Request.to_ical_str(), "REQUEST");
    assert_eq!(ItipMethod::Reply.to_ical_str(), "REPLY");
    assert_eq!(ItipMethod::Cancel.to_ical_str(), "CANCEL");
}

#[test]
fn test_generate_itip_request_payload_conformance() {
    let organizer = EventAttendee {
        email: "organizador@2lp.in".to_string(),
        name: Some("Organizador do Evento".to_string()),
        status: RsvpStatus::Accepted,
    };
    let attendee = EventAttendee {
        email: "convidado@dominio.com".to_string(),
        name: Some("Convidado Especial".to_string()),
        status: RsvpStatus::NeedsAction,
    };

    let event = CalendarEvent {
        uid: "inv-test-uid-100".to_string(),
        summary: "Reunião de Arquitetura".to_string(),
        description: Some("Alinhamento técnico da RFC de convites".to_string()),
        location: Some("Sala 3 / Virtual".to_string()),
        start: EventTime::DateTimeUtc(1700000000),
        end: EventTime::DateTimeUtc(1700003600),
        is_all_day: false,
        rrule: None,
        status: EventStatus::Confirmed,
        organizer: Some(organizer.clone()),
        attendees: vec![attendee],
        sequence: 1,
        created_at: Some(1700000000),
        updated_at: Some(1700000000),
    };

    let payload = generate_itip_payload(&event, ItipMethod::Request, &organizer);

    // Verificações estruturais iCalendar RFC 5545 e iTIP RFC 5546
    assert!(payload.contains("BEGIN:VCALENDAR"), "Deve conter cabeçalho VCALENDAR");
    assert!(payload.contains("VERSION:2.0"), "Versão deve ser 2.0");
    assert!(payload.contains("METHOD:REQUEST"), "Deve declarar METHOD:REQUEST");
    assert!(payload.contains("BEGIN:VEVENT"), "Deve conter bloco VEVENT");
    assert!(payload.contains("UID:inv-test-uid-100"), "UID deve ser preservado");
    assert!(payload.contains("SUMMARY:Reunião de Arquitetura"), "Summary deve ser preservado");
    assert!(payload.contains("ORGANIZER;CN=\"Organizador do Evento\":mailto:organizador@2lp.in"), "Organizer formatado");
    assert!(payload.contains("ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE;CN=\"Convidado Especial\":mailto:convidado@dominio.com"), "Attendee formatado com RSVP");
    assert!(payload.contains("SEQUENCE:1"), "Sequence deve ser incluído");
    assert!(payload.contains("END:VEVENT"), "Deve fechar VEVENT");
    assert!(payload.contains("END:VCALENDAR"), "Deve fechar VCALENDAR");
}

#[test]
fn test_generate_itip_cancel_payload() {
    let organizer = EventAttendee {
        email: "organizador@2lp.in".to_string(),
        name: None,
        status: RsvpStatus::Accepted,
    };
    let event = CalendarEvent {
        uid: "inv-cancel-uid".to_string(),
        summary: "Reunião Cancelada".to_string(),
        description: None,
        location: None,
        start: EventTime::DateTimeUtc(1700000000),
        end: EventTime::DateTimeUtc(1700003600),
        is_all_day: false,
        rrule: None,
        status: EventStatus::Cancelled,
        organizer: Some(organizer.clone()),
        attendees: vec![],
        sequence: 2,
        created_at: Some(1700000000),
        updated_at: Some(1700000000),
    };

    let payload = generate_itip_payload(&event, ItipMethod::Cancel, &organizer);
    assert!(payload.contains("METHOD:CANCEL"));
    assert!(payload.contains("STATUS:CANCELLED"));
    assert!(payload.contains("SEQUENCE:2"));
}
