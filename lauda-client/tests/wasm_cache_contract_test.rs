use lauda_client::domain::cache::LocalMessageCache;
use lauda_index::{IndexedDoc, MessageIndex};
use std::collections::{HashMap, HashSet};

#[test]
fn test_wasm_cache_has_rexie_store_contract() {
    assert_eq!(LocalMessageCache::IDB_DATABASE_NAME, "lauda_messages");
    assert_eq!(LocalMessageCache::IDB_VERSION, 2);
    assert_eq!(LocalMessageCache::IDB_STORE_MESSAGES, "messages");
    assert_eq!(LocalMessageCache::IDB_STORE_ACTIONS, "actions");
    assert_eq!(LocalMessageCache::IDB_STORE_INDEX_SLICES, "index_slices");
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

#[test]
fn test_wasm_local_search_bm25_mutation_ranking_and_terms() {
    let mut index = MessageIndex::new();

    // Documento com alta frequência do termo (TF=10)
    let doc_high_tf = IndexedDoc {
        uid: 501,
        mailbox: "INBOX".to_string(),
        subject: "Urgente Urgente Urgente".to_string(),
        sender: "alerta@empresa.com".to_string(),
        date: "2026-11-19T10:00:00Z".to_string(),
        has_attachments: false,
        is_read: true,
        is_flagged: false,
        doc_len: 10,
        subject_terms: HashSet::from(["urgente".to_string()]),
        sender_terms: HashSet::from(["alerta".to_string()]),
        term_frequencies: HashMap::from([("urgente".to_string(), 10)]),
        segment_embeddings: vec![],
    };

    // Documento com baixa frequência do termo (TF=1)
    let doc_low_tf = IndexedDoc {
        uid: 502,
        mailbox: "INBOX".to_string(),
        subject: "Comunicado normal".to_string(),
        sender: "comunicacao@empresa.com".to_string(),
        date: "2026-11-19T10:05:00Z".to_string(),
        has_attachments: false,
        is_read: true,
        is_flagged: false,
        doc_len: 20,
        subject_terms: HashSet::from(["comunicado".to_string(), "normal".to_string()]),
        sender_terms: HashSet::from(["comunicacao".to_string()]),
        term_frequencies: HashMap::from([("urgente".to_string(), 1)]),
        segment_embeddings: vec![],
    };

    index.insert_doc(doc_high_tf);
    index.insert_doc(doc_low_tf);

    let ranked = index.search("urgente");
    assert_eq!(ranked.len(), 2);
    assert_eq!(
        ranked[0], 501,
        "Documento com maior TF deve vir em primeiro no ranking BM25 do WASM"
    );
    assert_eq!(ranked[1], 502, "Documento com menor TF deve vir em segundo");
}

#[test]
fn test_queued_action_idb_serde_contract() {
    use lauda_client::domain::sync::{ActionType, QueuedAction};

    let action = QueuedAction {
        id: "INBOX_42_1700000000".to_string(),
        uid: 42,
        mailbox: "INBOX".to_string(),
        action: ActionType::MarkRead { read: true },
        created_at_unix: 1700000000,
    };

    let json_str = serde_json::to_string(&action).expect("Serialização da QueuedAction deve ter sucesso");
    let deserialized: QueuedAction = serde_json::from_str(&json_str).expect("Desserialização deve ter sucesso");

    assert_eq!(deserialized.id, action.id);
    assert_eq!(deserialized.uid, 42);
    assert_eq!(deserialized.mailbox, "INBOX");
    assert_eq!(deserialized.action, ActionType::MarkRead { read: true });
    assert_eq!(deserialized.created_at_unix, 1700000000);
}

#[test]
fn test_cache_mailbox_slug_sanitization() {
    assert_eq!(LocalMessageCache::sanitize_slug("INBOX"), "INBOX");
    assert_eq!(LocalMessageCache::sanitize_slug("Sent Items"), "Sent_Items");
    assert_eq!(LocalMessageCache::sanitize_slug("Archive/2026"), "Archive_2026");
    assert_eq!(LocalMessageCache::sanitize_slug("Caixa de Entrada!"), "Caixa_de_Entrada_");
}
