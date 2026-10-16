use std::collections::HashSet;
use std::path::Path;
use tempfile::tempdir;

use lauda_index::{IndexedDoc, MessageIndex, SimpleCpuEmbeddingProvider};
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_types::{Message, MessageEnvelope};

#[test]
fn test_sqlite_index_store_crud_and_reconciliation() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("test_index.db");

    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    // 1. Criar e indexar mensagens semânticas
    let mut index = MessageIndex::new();
    let provider = SimpleCpuEmbeddingProvider;

    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 201,
            subject: "Comprovante PIX Banco do Brasil".to_string(),
            sender: "financeiro@bb.com.br".to_string(),
            date: "2026-10-10T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Transferência recebida no valor de R$ 500,00 via chave CPF.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 202,
            subject: "Relatório de Atividades Mensais".to_string(),
            sender: "gestao@empresa.com".to_string(),
            date: "2026-10-11T14:00:00Z".to_string(),
            cc: None,
            has_attachments: true,
            is_read: false,
            is_flagged: true,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Segue em anexo o relatório referente ao terceiro trimestre.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    index.index_message_semantic("INBOX", &msg1, &provider);
    index.index_message_semantic("INBOX", &msg2, &provider);

    // Salvar documentos indexados no SQLite
    for (&uid, doc) in index.documents() {
        store
            .save_document("INBOX", doc, 1728560000)
            .expect("salvar documento no sqlite");
    }

    // 2. Verificar UIDs indexados
    let indexed_uids = store.get_indexed_uids("INBOX").expect("obter uids");
    assert_eq!(indexed_uids.len(), 2);
    assert!(indexed_uids.contains(&201));
    assert!(indexed_uids.contains(&202));

    // 3. Reconciliação com dovecot-uidlist
    // Suponha que o dovecot-uidlist contenha os UIDs 201 e 203 (202 foi apagado, 203 é novo)
    let mut active_uids = HashSet::new();
    active_uids.insert(201);
    active_uids.insert(203);

    let reconcile = store
        .reconcile_mailbox("INBOX", &active_uids)
        .expect("reconciliar");
    assert_eq!(reconcile.to_index, vec![203], "203 deve ser indexado");
    assert_eq!(reconcile.to_remove, vec![202], "202 deve ser removido");

    // Aplica a remoção
    store
        .remove_document("INBOX", 202)
        .expect("remover documento");
    let uids_after_removal = store.get_indexed_uids("INBOX").expect("obter uids pós remoção");
    assert_eq!(uids_after_removal.len(), 1);
    assert!(uids_after_removal.contains(&201));
    assert!(!uids_after_removal.contains(&202));

    // 4. Carregar índice para busca do SQLite
    let loaded_index = store
        .load_mailbox_index("INBOX")
        .expect("carregar indice do sqlite");
    let results = loaded_index.search_hybrid("pix", &provider, 0.5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uid, 201);

    // 5. Obter fatia recente de 90 dias
    let slice = store
        .get_recent_slice("INBOX", 90)
        .expect("obter fatia de 90 dias");
    assert_eq!(slice.len(), 1);
    assert_eq!(slice[0].uid, 201);
}
