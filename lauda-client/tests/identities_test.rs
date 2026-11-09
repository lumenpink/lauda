use lauda_client::domain::session::SessionConfig;
use lauda_client::domain::view::{filter_identities, parse_identity};

#[test]
fn test_session_config_identities_lifecycle() {
    let mut config = SessionConfig {
        server_url: "https://lauda.example.com".to_string(),
        username: "lumen".to_string(),
        token: None,
        identities: vec![],
    };

    // 1. Padrão inicial derivado do username
    let defaults = config.get_or_default_identities();
    assert_eq!(defaults, vec!["lumen@2lp.in".to_string()]);

    // 2. Adição de identidades
    config.add_identity("Lumen Freitas <hi@lumen.pink>");
    config.add_identity("lumen@empresa.com");

    let ids = config.get_or_default_identities();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], "lumen@empresa.com");
    assert_eq!(ids[1], "Lumen Freitas <hi@lumen.pink>");

    // 3. Deduplicação e reordenação ao readicionar
    config.add_identity("Lumen Freitas <hi@lumen.pink>");
    let ids_dedup = config.get_or_default_identities();
    assert_eq!(ids_dedup.len(), 2);
    assert_eq!(ids_dedup[0], "Lumen Freitas <hi@lumen.pink>");

    // 4. Remoção de identidade
    config.remove_identity("lumen@empresa.com");
    let ids_after_remove = config.get_or_default_identities();
    assert_eq!(ids_after_remove.len(), 1);
    assert_eq!(ids_after_remove[0], "Lumen Freitas <hi@lumen.pink>");
}

#[test]
fn test_session_config_identities_backward_compatibility() {
    // Payload JSON legado sem o campo identities
    let legacy_json = r#"{"server_url":"https://lauda.example.com","username":"lumen","token":null}"#;
    let decoded: SessionConfig = serde_json::from_str(legacy_json).unwrap();
    assert!(decoded.identities.is_empty());
    assert_eq!(decoded.get_or_default_identities(), vec!["lumen@2lp.in"]);

    // Serialização com identidades
    let mut config = decoded;
    config.add_identity("Lumen <hi@lumen.pink>");
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("Lumen <hi@lumen.pink>"));
}

#[test]
fn test_parse_and_filter_identities() {
    // 1. Parsing RFC 5322
    let (name1, email1) = parse_identity("Lumen Freitas <hi@lumen.pink>");
    assert_eq!(name1.as_deref(), Some("Lumen Freitas"));
    assert_eq!(email1, "hi@lumen.pink");

    let (name2, email2) = parse_identity("hi@lumen.pink");
    assert_eq!(name2, None);
    assert_eq!(email2, "hi@lumen.pink");

    let (name3, email3) = parse_identity("   <diretoria@empresa.org>  ");
    assert_eq!(name3, None);
    assert_eq!(email3, "diretoria@empresa.org");

    // 2. Filtro de identidades
    let pool = vec![
        "Lumen Freitas <hi@lumen.pink>".to_string(),
        "Suporte Técnico <suporte@2lp.in>".to_string(),
        "contato@empresa.com".to_string(),
    ];

    let matches_name = filter_identities(&pool, "freitas");
    assert_eq!(matches_name.len(), 1);
    assert_eq!(matches_name[0], "Lumen Freitas <hi@lumen.pink>");

    let matches_email = filter_identities(&pool, "2lp");
    assert_eq!(matches_email.len(), 1);
    assert_eq!(matches_email[0], "Suporte Técnico <suporte@2lp.in>");

    let matches_empty = filter_identities(&pool, "");
    assert_eq!(matches_empty.len(), 3);
}
