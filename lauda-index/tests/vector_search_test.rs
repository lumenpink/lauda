use lauda_index::{EmbeddingProvider, MessageIndex, SearchResult, cosine_similarity};
use lauda_types::{Message, MessageEnvelope};
use urupema::domain::SegmentKind;

/// Mock EmbeddingProvider determinístico de 4 dimensões para testes herméticos.
struct MockEmbeddingProvider;

impl EmbeddingProvider for MockEmbeddingProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        // Vetor 5D sintético: [armamento, saude, juridico, major, coronel]
        let mut vec = vec![0.0f32; 5];

        if lower.contains("arma") || lower.contains("tiro") || lower.contains("ig-10") {
            vec[0] += 1.0;
        }
        if lower.contains("saúde") || lower.contains("médico") || lower.contains("hospital") {
            vec[1] += 1.0;
        }
        if lower.contains("lei") || lower.contains("norma") || lower.contains("artigo") {
            vec[2] += 1.0;
        }
        if lower.contains("major") {
            vec[3] += 1.0;
        }
        if lower.contains("coronel") {
            vec[4] += 1.0;
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
fn test_cosine_similarity() {
    let a = vec![1.0, 0.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0, 0.0];
    let c = vec![0.0, 1.0, 0.0, 0.0];

    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-5);
    assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-5);
}

#[test]
fn test_semantic_max_pooling_and_best_segment() {
    let mut index = MessageIndex::new();
    let provider = MockEmbeddingProvider;

    // E-mail 1: Fala de armas no corpo, e assinatura é de um Coronel
    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 101,
            subject: "Publicação de Norma".to_string(),
            sender: "chem@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some(
            "Informo que está aprovada a nova regulamentação de armas e tiro da corporação.\n\n\
             Atenciosamente,\n\
             Coronel da Silva"
                .to_string(),
        ),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    // E-mail 2: Fala de saúde e perícia médica, e assinatura de Major Médico
    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 102,
            subject: "Encaminhamento Pericial".to_string(),
            sender: "dsps@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some(
            "Encaminho o servidor para inspeção de saúde no hospital militar.\n\n\
             Atenciosamente,\n\
             Major Médico Den Silveira"
                .to_string(),
        ),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    index.index_message_semantic("INBOX", &msg1, &provider);
    index.index_message_semantic("INBOX", &msg2, &provider);

    // 1. Busca semântica por "armamento" deve ranquear msg 1 em primeiro
    let res_armas = index.search_semantic("armamento", &provider);
    assert!(!res_armas.is_empty());
    assert_eq!(res_armas[0].uid, 101);
    assert_eq!(res_armas[0].matched_segment, Some(SegmentKind::Body));

    // 2. Busca semântica por "médico" deve ranquear msg 2 em primeiro
    let res_saude = index.search_semantic("médico e hospital", &provider);
    assert!(!res_saude.is_empty());
    assert_eq!(res_saude[0].uid, 102);

    // 3. Busca por assinatura de posto militar ("Major")
    let res_major = index.search_semantic("Major", &provider);
    assert!(!res_major.is_empty());
    assert_eq!(res_major[0].uid, 102);
    assert_eq!(res_major[0].matched_segment, Some(SegmentKind::Signature));
}

#[test]
fn test_hybrid_search_fusion() {
    let mut index = MessageIndex::new();
    let provider = MockEmbeddingProvider;

    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 201,
            subject: "LEI 14751 VIGENTE".to_string(),
            sender: "legislativo@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some("Texto da nova lei publicada no diário oficial.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    index.index_message_semantic("INBOX", &msg1, &provider);

    // Busca híbrida combinando termo exato "14751" (BM25) com "lei" (Semântico)
    let results: Vec<SearchResult> = index.search_hybrid("14751 lei", &provider, 0.5);
    assert!(!results.is_empty());
    assert_eq!(results[0].uid, 201);
    assert!(results[0].bm25_score > 0.0);
    assert!(results[0].semantic_score > 0.0);
    assert!(results[0].score > 0.0);
}

#[test]
fn test_hybrid_search_alpha_extremes() {
    let mut index = MessageIndex::new();
    let provider = MockEmbeddingProvider;

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 301,
            subject: "Vacinação Militar".to_string(),
            sender: "saude@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            ..Default::default()
        },
        txt_auto: Some("Campanha de saúde no hospital geral militar.".to_string()),
        ..Default::default()
    };

    index.index_message_semantic("INBOX", &msg, &provider);

    // Alpha = 1.0 (somente BM25): score semântico deve ser 0.0
    let res_bm25 = index.search_hybrid("Vacinação", &provider, 1.0);
    assert!(!res_bm25.is_empty());
    assert_eq!(res_bm25[0].uid, 301);
    assert!(res_bm25[0].bm25_score > 0.0);
    assert_eq!(res_bm25[0].semantic_score, 0.0);

    // Alpha = 0.0 (somente Semântica): bm25 score deve ser 0.0
    let res_sem = index.search_hybrid("médico", &provider, 0.0);
    assert!(!res_sem.is_empty());
    assert_eq!(res_sem[0].uid, 301);
    assert_eq!(res_sem[0].bm25_score, 0.0);
    assert!(res_sem[0].semantic_score > 0.0);
}

#[test]
fn test_semantic_search_empty_and_no_match() {
    let mut index = MessageIndex::new();
    let provider = MockEmbeddingProvider;

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 401,
            subject: "Sem Conteúdo".to_string(),
            sender: "null@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            ..Default::default()
        },
        txt_auto: None,
        txt_original: None,
        ..Default::default()
    };

    index.index_message_semantic("INBOX", &msg, &provider);

    // Termo desconhecido sem similaridade
    let res = index.search_semantic("termo_inexistente_xyz_123", &provider);
    assert!(res.is_empty());

    // Query vazia
    let res_empty = index.search_semantic("", &provider);
    assert!(res_empty.is_empty());
}

#[test]
fn test_semantic_index_persistence_roundtrip() {
    let mut index = MessageIndex::new();
    let provider = MockEmbeddingProvider;

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 501,
            subject: "Aviso Geral".to_string(),
            sender: "comando@exemplo.gov.br".to_string(),
            date: "2026-10-15".to_string(),
            ..Default::default()
        },
        txt_auto: Some("Diretrizes de armamento e estande de tiro militar.".to_string()),
        ..Default::default()
    };

    index.index_message_semantic("INBOX", &msg, &provider);

    let temp_file = tempfile::NamedTempFile::new().expect("temp file");
    index.save_to_file(temp_file.path()).expect("save index");

    let loaded = MessageIndex::load_from_file(temp_file.path())
        .expect("load index")
        .expect("index exists");

    let results = loaded.search_semantic("armamento", &provider);
    assert!(!results.is_empty());
    assert_eq!(results[0].uid, 501);
    assert_eq!(results[0].matched_segment, Some(SegmentKind::Body));
}
