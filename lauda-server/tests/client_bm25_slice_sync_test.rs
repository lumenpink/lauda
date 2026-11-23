use lauda_index::{IndexedDoc, SimpleCpuEmbeddingProvider};
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_types::{Message, MessageEnvelope};
use std::collections::HashSet;
use tempfile::tempdir;

#[test]
fn test_get_recent_slice_purges_segment_embeddings_for_client_sync() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_index.db");
    let store = SqliteIndexStore::open(&db_path).unwrap();
    store.init_schema().unwrap();

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 101,
            subject: "Reunião de Alinhamento Semanal".to_string(),
            sender: "coordenacao@empresa.com".to_string(),
            date: "2026-11-20T10:00:00Z".to_string(),
            is_read: true,
            is_flagged: false,
            has_attachments: false,
            ..Default::default()
        },
        txt_auto: Some("Pauta sobre cronograma e entregas prioritárias.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    let provider = SimpleCpuEmbeddingProvider;
    let mut temp_index = lauda_index::MessageIndex::new();
    temp_index.index_message_semantic("INBOX", &msg, &provider);

    let doc = temp_index.documents().get(&101).unwrap();
    assert!(
        !doc.segment_embeddings.is_empty(),
        "Documento indexado no servidor deve conter embeddings de segmentos"
    );

    store.save_document("INBOX", doc, 0).unwrap();

    // Ao exportar a fatia para sincronização com o cliente, os embeddings DEVEM ser purgados
    let slice = store.get_recent_slice("INBOX", 30).unwrap();
    assert_eq!(slice.len(), 1);
    let exported = &slice[0];

    assert!(
        exported.segment_embeddings.is_empty(),
        "Fatia exportada para o cliente DEVE conter segment_embeddings vazios para poupar banda e storage"
    );
}

#[test]
fn test_client_local_search_bm25_operates_without_embeddings() {
    let mut index = lauda_index::MessageIndex::new();
    let mut terms = std::collections::HashMap::new();
    terms.insert("diploma".to_string(), 2);
    terms.insert("universidade".to_string(), 1);

    let doc = IndexedDoc {
        uid: 50,
        mailbox: "INBOX".to_string(),
        sender: "secretaria@universidade.br".to_string(),
        subject: "Emissão de Diploma".to_string(),
        date: "2026-11-21T09:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        has_attachments: false,
        subject_terms: HashSet::from_iter(vec!["emissao".to_string(), "diploma".to_string()]),
        sender_terms: HashSet::from_iter(vec!["secretaria".to_string(), "universidade".to_string()]),
        term_frequencies: terms,
        doc_len: 20,
        segment_embeddings: Vec::new(), // Zero embeddings
    };

    index.insert_doc(doc);

    // Busca léxica local BM25 pura
    let matches = index.search("Diploma");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], 50);
}
