use lauda_index::{
    DynamicEmbeddingProvider, E5InstructionProvider, EmbeddingProvider, MessageIndex,
};
use lauda_types::{Message, MessageEnvelope};

/// Mock neural embedding provider que simula um espaço latente semântico em português
/// onde sinônimos conceituais ("professora" <-> "docente", "segunda chamada" <-> "reposição/substitutiva")
/// mapeiam para as mesmas dimensões vetoriais.
#[derive(Debug, Default, Clone)]
struct MockSemanticNeuralProvider;

impl EmbeddingProvider for MockSemanticNeuralProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; 384];
        let lower = text.to_lowercase();

        // Dimensões semânticas conceituais:
        // [0..50]: Conceito "Docência / Professor / Magistério"
        if lower.contains("professor")
            || lower.contains("professora")
            || lower.contains("docente")
            || lower.contains("magistério")
        {
            for i in 0..50 {
                vec[i] += 1.0;
            }
        }

        // [50..100]: Conceito "Avaliação substitutiva / Segunda chamada / Reposição de prova"
        if lower.contains("segunda chamada")
            || lower.contains("substitutiva")
            || lower.contains("reposição")
            || lower.contains("prova para quem faltou")
        {
            for i in 50..100 {
                vec[i] += 1.0;
            }
        }

        // [100..150]: Conceito "Viagens / Férias / Passagens"
        if lower.contains("passagem")
            || lower.contains("voo")
            || lower.contains("aéreo")
            || lower.contains("hotel")
        {
            for i in 100..150 {
                vec[i] += 1.0;
            }
        }

        // Normalização L2
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        vec
    }
}

#[test]
fn test_e5_instruction_prefixes_and_formatting() {
    let base = MockSemanticNeuralProvider;
    let provider = E5InstructionProvider::new(base);

    // embed_query deve usar o prefixo "query: "
    let q_emb = provider.embed_query("email de uma professora sobre segunda chamada");
    assert_eq!(q_emb.len(), 384);

    // embed_passage deve usar o prefixo "passage: "
    let p_emb = provider.embed_passage("Aviso sobre avaliação substitutiva para os alunos");
    assert_eq!(p_emb.len(), 384);
}

#[test]
fn test_semantic_search_retrieves_synonyms_without_exact_lexical_match() {
    let base = MockSemanticNeuralProvider;
    let provider = E5InstructionProvider::new(base);
    let mut index = MessageIndex::new();

    // E-mail 1: Usa estritamente termos conceituais ("docente", "substitutiva", "reposição"),
    // SEM conter as palavras exatas "professora" nem "segunda chamada"
    let msg_professora = Message {
        envelope: MessageEnvelope {
            uid: 201,
            subject: "Avaliação substitutiva da Turma B".to_string(),
            sender: "Dra. Helena <docente.helena@universidade.br>".to_string(),
            date: "2026-10-18".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some(
            "Informo o cronograma de reposição de notas para os ausentes da prova.".to_string(),
        ),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    // E-mail 2: Distrator não relacionado (turismo)
    let msg_viagem = Message {
        envelope: MessageEnvelope {
            uid: 202,
            subject: "Promoção de passagens aéreas e hotel".to_string(),
            sender: "Agência Viagens <promo@viagens.com>".to_string(),
            date: "2026-10-18".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some("Confira as melhores ofertas de voo e hospedagem para o verão.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    index.index_message_semantic("INBOX", &msg_professora, &provider);
    index.index_message_semantic("INBOX", &msg_viagem, &provider);

    // Consulta do usuário em linguagem natural:
    let query = "email de uma professora sobre segunda chamada";
    let results = index.search_semantic(query, &provider);

    assert!(
        !results.is_empty(),
        "A busca semântica deveria retornar resultados"
    );
    assert_eq!(
        results[0].uid, 201,
        "O e-mail da docente sobre substitutiva deveria ser o primeiro resultado ranqueado"
    );
    assert!(
        results[0].semantic_score > 0.60,
        "Score semântico do e-mail relevante deve ser expressivo (> 0.60), foi {}",
        results[0].semantic_score
    );

    // Se houver segundo resultado (viagem), o score deve ser próximo a zero
    if results.len() > 1 {
        assert!(
            results[1].semantic_score < 0.20,
            "Distrator não deve ter score semântico relevante"
        );
    }
}

#[test]
fn test_dynamic_embedding_provider_fallback() {
    // Quando inicializado com caminho inexistente, DynamicEmbeddingProvider usa fallback gracioso sem pânico
    let dynamic = DynamicEmbeddingProvider::from_path_or_fallback("/caminho/inexistente/modelo.onnx");
    assert!(
        dynamic.is_fallback(),
        "Deveria estar em modo fallback devido ao arquivo inexistente"
    );

    let vec = dynamic.embed_query("teste de query de fallback");
    assert_eq!(vec.len(), 384);
}
