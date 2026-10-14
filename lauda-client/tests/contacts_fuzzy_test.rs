use lauda_client::domain::contacts_cache::{LocalContactsCache, create_local_contact};
use lauda_client::domain::fuzzy::{calculate_fuzzy_match_score, normalize_for_fuzzy};
use lauda_types::calendar::{CalendarEvent, EventAttendee, EventStatus, EventTime, RsvpStatus};
use tempfile::tempdir;

#[test]
fn test_normalize_for_fuzzy_diacritics_and_case() {
    assert_eq!(normalize_for_fuzzy("João"), "joao");
    assert_eq!(normalize_for_fuzzy("Débora"), "debora");
    assert_eq!(normalize_for_fuzzy("Ação & Criação!"), "acao criacao");
    assert_eq!(normalize_for_fuzzy("  Éric   "), "eric");
    assert_eq!(normalize_for_fuzzy("MÜLLER"), "muller");
}

#[test]
fn test_fuzzy_score_ranking() {
    let exact = calculate_fuzzy_match_score("marcelo", "Marcelo Santos", "marcelo@domain.org");
    let prefix = calculate_fuzzy_match_score("marc", "Marcelo Santos", "marcelo@domain.org");
    let typo = calculate_fuzzy_match_score("marsselo", "Marcelo Santos", "marcelo@domain.org");
    let none = calculate_fuzzy_match_score(
        "totalmentediferente",
        "Marcelo Santos",
        "marcelo@domain.org",
    );

    assert!(exact > prefix, "Match exato deve pontuar mais que prefixo");
    assert!(prefix > typo, "Prefixo deve pontuar mais que typo");
    assert!(typo > 0, "Typo leve deve receber score positivo");
    assert_eq!(none, 0, "Termo sem relação deve ter score 0");
}

#[test]
fn test_search_contacts_fuzzy_cache() {
    let temp = tempdir().expect("create temp dir");
    let cache = LocalContactsCache::open(temp.path()).expect("open cache");

    let c1 = create_local_contact("user1", Some("Beatriz Lima"), "beatriz@example.com", None);
    let c2 = create_local_contact("user2", Some("Marcelo Santos"), "marcelo@domain.org", None);
    let c3 = create_local_contact(
        "user3",
        Some("Débora Martins"),
        "debora@empresa.com.br",
        None,
    );

    cache
        .apply_sync_delta(&[c1, c2, c3], &[], "tok-1")
        .expect("insert contacts");

    // 1. Match exato
    let res = cache.search_contacts_fuzzy("marcelo", 5).expect("search");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].display_name.as_deref(), Some("Marcelo Santos"));

    // 2. Insensibilidade a acentos (sem acento na busca buscando com acento)
    let res = cache.search_contacts_fuzzy("debora", 5).expect("search");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].display_name.as_deref(), Some("Débora Martins"));

    // 3. Insensibilidade a acentos (com acento na busca)
    let res = cache.search_contacts_fuzzy("Débora", 5).expect("search");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].display_name.as_deref(), Some("Débora Martins"));

    // 4. Tolerância a typo leve (Beatiz -> Beatriz)
    let res = cache.search_contacts_fuzzy("Beatiz", 5).expect("search");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].display_name.as_deref(), Some("Beatriz Lima"));

    // 5. Tolerância a typo leve (Marsselo -> Marcelo)
    let res = cache.search_contacts_fuzzy("marsselo", 5).expect("search");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].display_name.as_deref(), Some("Marcelo Santos"));

    // 6. Termo inexistente
    let res = cache
        .search_contacts_fuzzy("xyz_desconhecido", 5)
        .expect("search");
    assert!(
        res.is_empty(),
        "Termo inexistente deve retornar lista vazia"
    );
}

#[test]
fn test_calendar_attendee_heuristic_linking_flow() {
    let temp = tempdir().expect("create temp dir");
    let cache = LocalContactsCache::open(temp.path()).expect("open cache");

    let contact = create_local_contact(
        "u42",
        Some("Beatriz de Oliveira"),
        "beatriz.oliveira@empresa.org",
        None,
    );
    cache
        .apply_sync_delta(&[contact], &[], "tok-cal")
        .expect("save contact");

    // Simula participante textual extraído do parser (ex.: "com Beatriz")
    let raw_attendee_name = "Beatriz";
    let matches = cache
        .search_contacts_fuzzy(raw_attendee_name, 5)
        .expect("search");
    assert!(!matches.is_empty());

    let matched_contact = &matches[0];
    let mut attendee = EventAttendee {
        name: Some(raw_attendee_name.to_string()),
        email: String::new(),
        status: RsvpStatus::NeedsAction,
    };

    // Vinculação heurística com contato selecionado
    attendee.name = matched_contact.display_name.clone();
    attendee.email = matched_contact.primary_email.clone();

    assert_eq!(attendee.name.as_deref(), Some("Beatriz de Oliveira"));
    assert_eq!(attendee.email, "beatriz.oliveira@empresa.org");

    let event = CalendarEvent {
        uid: "ev-test-link-1".to_string(),
        summary: "Alinhamento de Projeto".to_string(),
        description: None,
        location: None,
        start: EventTime::DateTimeUtc(1700000000),
        end: EventTime::DateTimeUtc(1700003600),
        is_all_day: false,
        rrule: None,
        status: EventStatus::Confirmed,
        organizer: None,
        attendees: vec![attendee],
        sequence: 1,
        created_at: Some(1700000000),
        updated_at: Some(1700000000),
    };

    let cal_cache = lauda_client::domain::calendar_cache::LocalCalendarCache::open(temp.path())
        .expect("open cal cache");
    cal_cache.upsert_event(&event).expect("save event");

    let loaded = cal_cache.load_events().expect("load");
    let loaded_ev = loaded
        .into_iter()
        .find(|e| e.uid == "ev-test-link-1")
        .expect("found");
    assert_eq!(loaded_ev.attendees.len(), 1);
    assert_eq!(
        loaded_ev.attendees[0].name.as_deref(),
        Some("Beatriz de Oliveira")
    );
    assert_eq!(loaded_ev.attendees[0].email, "beatriz.oliveira@empresa.org");
}
