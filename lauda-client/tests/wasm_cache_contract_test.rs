use lauda_client::domain::cache::LocalMessageCache;
use lauda_index::{IndexedDoc, MessageIndex};
use std::collections::{HashMap, HashSet};

#[test]
fn test_wasm_cache_has_rexie_store_contract() {
    assert_eq!(LocalMessageCache::IDB_DATABASE_NAME, "lauda_messages");
}

#[test]
fn test_wasm_local_search_bm25_pure() {
    let mut index = MessageIndex::new();
    let doc1 = IndexedDoc {
        uid: 101,
        mailbox: "INBOX".to_string(),
        subject: "Relatório de Despesas".to_string(),
        sender: "financeiro@empresa.com".to_string(),
        date: "2026-11-19T10:00:00Z".to_string(),
        has_attachments: true,
        is_read: true,
        is_flagged: false,
        doc_len: 20,
        subject_terms: HashSet::from(["relatorio".to_string(), "despesas".to_string()]),
        sender_terms: HashSet::from(["financeiro".to_string(), "empresa".to_string()]),
        term_frequencies: HashMap::from([
            ("relatorio".to_string(), 3),
            ("despesas".to_string(), 2),
        ]),
        segment_embeddings: vec![],
    };
    let doc2 = IndexedDoc {
        uid: 102,
        mailbox: "INBOX".to_string(),
        subject: "Reunião de Alinhamento".to_string(),
        sender: "gestao@empresa.com".to_string(),
        date: "2026-11-19T11:00:00Z".to_string(),
        has_attachments: false,
        is_read: false,
        is_flagged: true,
        doc_len: 15,
        subject_terms: HashSet::from(["reuniao".to_string(), "alinhamento".to_string()]),
        sender_terms: HashSet::from(["gestao".to_string(), "empresa".to_string()]),
        term_frequencies: HashMap::from([
            ("reuniao".to_string(), 4),
            ("alinhamento".to_string(), 1),
        ]),
        segment_embeddings: vec![],
    };

    index.insert_doc(doc1);
    index.insert_doc(doc2);

    let matched_uids = index.search("relatorio");
    assert_eq!(
        matched_uids,
        vec![101],
        "Busca BM25 no WASM deve encontrar o documento 101"
    );

    let matched_uids_reuniao = index.search("reuniao");
    assert_eq!(
        matched_uids_reuniao,
        vec![102],
        "Busca BM25 no WASM deve encontrar o documento 102"
    );
}
