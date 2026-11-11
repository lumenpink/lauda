use lauda_client::domain::realtime::{RealtimeEvent, build_ws_url, parse_server_realtime_event};

#[test]
fn test_build_ws_url_variants() {
    assert_eq!(
        build_ws_url("http://127.0.0.1:8432", "jwt.token.abc"),
        "ws://127.0.0.1:8432/ws?token=jwt.token.abc"
    );
    assert_eq!(
        build_ws_url("https://lauda.2lp.in", "jwt.token.xyz"),
        "wss://lauda.2lp.in/ws?token=jwt.token.xyz"
    );
    assert_eq!(
        build_ws_url("http://localhost:8080/", "abc"),
        "ws://localhost:8080/ws?token=abc"
    );
    assert_eq!(
        build_ws_url("127.0.0.1:8432", "tok"),
        "ws://127.0.0.1:8432/ws?token=tok"
    );
}

#[test]
fn test_parse_server_realtime_event_success() {
    // 1. Evento mailbox_updated com campo "mailbox"
    let ev1 = parse_server_realtime_event(r#"{"event":"mailbox_updated","mailbox":"INBOX"}"#);
    assert_eq!(
        ev1,
        Some(RealtimeEvent::MailboxUpdated {
            mailbox: "INBOX".to_string()
        })
    );

    // 2. Evento mailbox_updated com alias "box" legado
    let ev2 = parse_server_realtime_event(r#"{"event":"mailbox_updated","box":"Sent"}"#);
    assert_eq!(
        ev2,
        Some(RealtimeEvent::MailboxUpdated {
            mailbox: "Sent".to_string()
        })
    );

    // 3. Evento read_update com uid
    let ev3 = parse_server_realtime_event(r#"{"event":"read_update","box":"INBOX","uid":105}"#);
    assert_eq!(
        ev3,
        Some(RealtimeEvent::MessageAction {
            action: "read_update".to_string(),
            uid: Some(105),
        })
    );

    // 4. Evento flag_update
    let ev4 = parse_server_realtime_event(r#"{"event":"flag_update","box":"INBOX","uid":200}"#);
    assert_eq!(
        ev4,
        Some(RealtimeEvent::MessageAction {
            action: "flag_update".to_string(),
            uid: Some(200),
        })
    );

    // 5. Evento delete
    let ev5 = parse_server_realtime_event(r#"{"event":"delete","box":"INBOX","uid":300}"#);
    assert_eq!(
        ev5,
        Some(RealtimeEvent::MessageAction {
            action: "delete".to_string(),
            uid: Some(300),
        })
    );
}

#[test]
fn test_mutation_detection_on_realtime_events_and_urls() {
    // 1. Mutante: JSON quebrado ou campos nulos não podem disparar evento espúrio
    assert_eq!(parse_server_realtime_event(""), None);
    assert_eq!(parse_server_realtime_event("{not valid json}"), None);
    assert_eq!(parse_server_realtime_event(r#"{"unknown":"event"}"#), None);
    assert_eq!(
        parse_server_realtime_event(r#"{"event":"ping"}"#),
        None,
        "Mutante: ping de controle não deve ser emitido como evento de domínio"
    );

    // 2. Mutante: https deve virar wss, nunca ws inseguro
    let https_url = build_ws_url("https://secure.2lp.in", "tok");
    assert!(
        https_url.starts_with("wss://"),
        "Mutante: https DEVE ser convertido para wss://"
    );
    assert!(
        !https_url.starts_with("ws://"),
        "Mutante sobreviveu: wss inseguro"
    );

    // 3. Mutante: barra duplicada na URL
    let slash_url = build_ws_url("http://127.0.0.1:8432/", "tok");
    assert!(
        !slash_url.contains("//ws"),
        "Mutante sobreviveu: URL gerou barra duplicada //ws: {}",
        slash_url
    );
}
