use lauda_index::IndexedDoc;
use lauda_server::adapters::sqlite_index::{
    SqliteIndexStore, decode_segment_embeddings, encode_segment_embeddings,
};
use std::collections::HashMap;
use tempfile::tempdir;
use urupema::domain::SegmentKind;

#[test]
fn test_encode_decode_768_dimensions_roundtrip() {
    let mut vec_768 = Vec::with_capacity(768);
    for i in 0..768 {
        vec_768.push((i as f32) * 0.001);
    }

    let original = vec![
        (SegmentKind::Header, vec_768.clone()),
        (SegmentKind::Body, vec_768.clone()),
    ];

    let encoded = encode_segment_embeddings(&original);
    let decoded = decode_segment_embeddings(&encoded);

    assert_eq!(decoded.len(), 2, "Deve preservar a quantidade de segmentos");
    assert_eq!(decoded[0].0, SegmentKind::Header);
    assert_eq!(decoded[0].1.len(), 768, "Vetor do cabeçalho deve manter 768 dimensões");
    assert_eq!(decoded[1].0, SegmentKind::Body);
    assert_eq!(decoded[1].1.len(), 768, "Vetor do corpo deve manter 768 dimensões");
    assert_eq!(decoded[0].1, vec_768, "Valores dos floats devem ser idênticos");
}

#[test]
fn test_decode_legacy_384_dimensions_backward_compatibility() {
    // Simula blob gravado na V1 (sem header mágico V2, 1 + 384 * 4 bytes por segmento)
    let vec_384 = vec![0.5f32; 384];
    let mut legacy_blob = Vec::new();
    // SegmentKind::Body = 1
    legacy_blob.push(1u8);
    for &val in &vec_384 {
        legacy_blob.extend_from_slice(&val.to_le_bytes());
    }

    let decoded = decode_segment_embeddings(&legacy_blob);
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].0, SegmentKind::Body);
    assert_eq!(decoded[0].1.len(), 384, "Blob legado V1 deve ser decodificado com 384 dimensões");
    assert_eq!(decoded[0].1, vec_384);
}

#[test]
fn test_sqlite_index_store_supports_768_dimensions_knn_search() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_768.db");
    let store = SqliteIndexStore::open(&db_path).unwrap();
    store.init_schema().unwrap();

    let mut vec_target = vec![0.0f32; 768];
    vec_target[0] = 1.0; // Vetor unitário no eixo 0

    let mut vec_other = vec![0.0f32; 768];
    vec_other[1] = 1.0; // Vetor unitário no eixo 1

    let doc_target = IndexedDoc {
        uid: 100,
        mailbox: "INBOX".to_string(),
        sender: "coordenador@empresa.com".to_string(),
        subject: "Projeto Base 768".to_string(),
        date: "2026-11-23T10:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        has_attachments: false,
        subject_terms: Default::default(),
        sender_terms: Default::default(),
        term_frequencies: HashMap::new(),
        doc_len: 10,
        segment_embeddings: vec![(SegmentKind::Body, vec_target.clone())],
    };

    let doc_other = IndexedDoc {
        uid: 200,
        mailbox: "INBOX".to_string(),
        sender: "outro@empresa.com".to_string(),
        subject: "Outro assunto".to_string(),
        date: "2026-11-23T11:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        has_attachments: false,
        subject_terms: Default::default(),
        sender_terms: Default::default(),
        term_frequencies: HashMap::new(),
        doc_len: 10,
        segment_embeddings: vec![(SegmentKind::Body, vec_other)],
    };

    store.save_document("INBOX", &doc_target, 0).unwrap();
    store.save_document("INBOX", &doc_other, 0).unwrap();

    // Mock provider de busca que retorna consulta de 768 dimensões idêntica ao doc_target
    struct MockProvider768(Vec<f32>);
    impl lauda_index::EmbeddingProvider for MockProvider768 {
        fn embed(&self, _text: &str) -> Vec<f32> {
            self.0.clone()
        }
    }

    let provider = MockProvider768(vec_target);
    let results = store
        .search_hybrid_with_provider("INBOX", "qualquer query", 10, &provider)
        .unwrap();

    assert!(
        !results.is_empty(),
        "Busca híbrida/vetorial deve retornar resultados para modelo de 768 dimensões"
    );
    assert_eq!(results[0].uid, 100, "Top 1 deve ser o doc_target alinhado");
}
