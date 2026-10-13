use lauda_types::sieve::{
    ConditionField, MatchType, SieveAction, SieveCondition, SieveRule, generate_sieve_script,
    parse_sieve_script, predict_rule_from_envelope,
};
use lauda_types::MessageEnvelope;

#[test]
fn test_generate_and_parse_sieve_script_roundtrip() {
    let rules = vec![
        SieveRule {
            id: "rule_1".to_string(),
            name: "Mover Noticias".to_string(),
            enabled: true,
            match_all: true,
            conditions: vec![SieveCondition {
                field: ConditionField::From,
                match_type: MatchType::Contains,
                value: "news@example.com".to_string(),
            }],
            actions: vec![
                SieveAction::FileInto("Noticias".to_string()),
                SieveAction::Stop,
            ],
        },
        SieveRule {
            id: "rule_2".to_string(),
            name: "Spam por Assunto".to_string(),
            enabled: true,
            match_all: false,
            conditions: vec![
                SieveCondition {
                    field: ConditionField::Subject,
                    match_type: MatchType::Contains,
                    value: "URGENTE".to_string(),
                },
                SieveCondition {
                    field: ConditionField::Subject,
                    match_type: MatchType::Contains,
                    value: "PROMOÇÃO".to_string(),
                },
            ],
            actions: vec![SieveAction::Discard, SieveAction::Stop],
        },
    ];

    let script = generate_sieve_script(&rules);
    assert!(script.contains("require [\"fileinto\", \"imap4flags\"];"));
    assert!(script.contains("# rule:[Mover Noticias]"));
    assert!(script.contains("header :contains \"from\" \"news@example.com\""));
    assert!(script.contains("fileinto \"Noticias\";"));

    let parsed_rules = parse_sieve_script(&script);
    assert_eq!(parsed_rules.len(), 2);
    assert_eq!(parsed_rules[0].name, "Mover Noticias");
    assert_eq!(parsed_rules[0].actions, vec![
        SieveAction::FileInto("Noticias".to_string()),
        SieveAction::Stop
    ]);
    assert_eq!(parsed_rules[1].name, "Spam por Assunto");
    assert_eq!(parsed_rules[1].conditions.len(), 2);
}

#[test]
fn test_predict_rule_from_envelope_with_prefix_subject() {
    let env = MessageEnvelope {
        uid: 101,
        sender: "dev-team@project.org".to_string(),
        subject: "[lauda-dev] Atualização do release".to_string(),
        date: "2026-10-13".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let predicted = predict_rule_from_envelope(&env);
    assert_eq!(predicted.name, "Regra para [lauda-dev]");
    assert!(predicted.conditions.iter().any(|c| c.field == ConditionField::Subject && c.value == "[lauda-dev]"));
    assert_eq!(predicted.actions, vec![
        SieveAction::FileInto("Listas/lauda-dev".to_string()),
        SieveAction::Stop,
    ]);
}

#[test]
fn test_predict_rule_from_envelope_sender_fallback() {
    let env = MessageEnvelope {
        uid: 102,
        sender: "chefe@empresa.com".to_string(),
        subject: "Relatório Mensal".to_string(),
        date: "2026-10-13".to_string(),
        is_read: false,
        is_flagged: false,
        cc: None,
        has_attachments: false,
        crypto_meta: None,
    };

    let predicted = predict_rule_from_envelope(&env);
    assert_eq!(predicted.name, "Regra para chefe@empresa.com");
    assert_eq!(predicted.conditions[0].field, ConditionField::From);
    assert_eq!(predicted.conditions[0].value, "chefe@empresa.com");
    assert_eq!(predicted.actions, vec![
        SieveAction::FileInto("Contatos/chefe".to_string()),
        SieveAction::Stop,
    ]);
}
