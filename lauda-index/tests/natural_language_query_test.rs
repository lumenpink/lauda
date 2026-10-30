use lauda_index::{MessageIndex, SimpleCpuEmbeddingProvider, parse_query};
use lauda_types::{Message, MessageEnvelope};

#[test]
fn test_natural_language_query_parsing() {
    let q = parse_query("de lisiane sobre trabalhos dos alunos");
    // Deve identificar que "lisiane" é remetente (From)
    let has_from_lisiane = q.clauses.iter().any(|c| match c {
        lauda_index::QueryClause::From(val) => val.contains("lisiane"),
        _ => false,
    });
    assert!(
        has_from_lisiane,
        "Deveria ter extraído cláusula From('lisiane') da consulta em linguagem natural"
    );

    // O restante da frase deve conter a intenção semântica "trabalhos dos alunos"
    let semantic_terms: Vec<String> = q
        .clauses
        .iter()
        .filter_map(|c| match c {
            lauda_index::QueryClause::Term(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(
        semantic_terms.iter().any(|t| t == "trabalhos" || t == "alunos"),
        "Deveria manter termos semânticos relevantes, obtido: {:?}",
        semantic_terms
    );
}

#[test]
fn test_conceptual_semantic_search_without_exact_keywords() {
    let mut index = MessageIndex::new();
    let provider = SimpleCpuEmbeddingProvider;

    // E-mail 1: Laudo laboratorial (hemograma / plaquetas)
    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 101,
            subject: "Laudo de Análises Clínicas - Hemograma Completo".to_string(),
            sender: "laboratorio@clinica.com.br".to_string(),
            date: "2026-10-15T10:00:00Z".to_string(),
            cc: None,
            has_attachments: true,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        txt_auto: Some("Prezado paciente, segue em anexo o laudo de seu hemograma completo e contagem de plaquetas.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    // E-mail 2: Convite de aniversário
    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 102,
            subject: "Comemoração de 30 anos no sábado!".to_string(),
            sender: "amigo@email.com".to_string(),
            date: "2026-10-16T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: false,
            is_flagged: true,
            crypto_meta: None,
        },
        txt_auto: Some("Venha comemorar comigo meus 30 anos! Teremos bolo, música e churrasco a partir das 19h.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    // E-mail 3: Oferta acadêmica
    let msg3 = Message {
        envelope: MessageEnvelope {
            uid: 103,
            subject: "Inscrições Abertas: Processo Seletivo de Mestrado e Doutorado".to_string(),
            sender: "posgraduacao@universidade.edu.br".to_string(),
            date: "2026-10-17T09:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        txt_auto: Some("Estão abertas as inscrições para o programa de mestrado acadêmico e especialização em computação.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    index.index_message_semantic("INBOX", &msg1, &provider);
    index.index_message_semantic("INBOX", &msg2, &provider);
    index.index_message_semantic("INBOX", &msg3, &provider);

    // Consulta 1: "resultado de exame de sangue" -> deve encontrar o E-mail 101 no topo
    let res1 = index.search_hybrid("resultado de exame de sangue", &provider, 0.4);
    assert!(!res1.is_empty(), "Deveria encontrar resultados conceituais para 'resultado de exame de sangue'");
    assert_eq!(res1[0].uid, 101, "Top 1 para 'resultado de exame de sangue' deveria ser o laudo de hemograma (UID 101)");

    // Consulta 2: "festa de aniversário" -> deve encontrar o E-mail 102 no topo
    let res2 = index.search_hybrid("festa de aniversário", &provider, 0.4);
    assert!(!res2.is_empty(), "Deveria encontrar resultados conceituais para 'festa de aniversário'");
    assert_eq!(res2[0].uid, 102, "Top 1 para 'festa de aniversário' deveria ser a comemoração com bolo (UID 102)");

    // Consulta 3: "sobre uma oferta de pós graduação" -> deve encontrar o E-mail 103 no topo
    let res3 = index.search_hybrid("sobre uma oferta de pós graduação", &provider, 0.4);
    assert!(!res3.is_empty(), "Deveria encontrar resultados para 'sobre uma oferta de pós graduação'");
    assert_eq!(res3[0].uid, 103, "Top 1 para 'sobre uma oferta de pós graduação' deveria ser a inscrição de mestrado (UID 103)");
}
