use std::sync::Arc;
use tempfile::tempdir;

use lauda_index::{MessageIndex, SimpleCpuEmbeddingProvider};
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_types::{Message, MessageEnvelope};

#[test]
fn test_sqlite_search_hybrid_conceptual_and_bm25() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("hybrid_test.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let provider = SimpleCpuEmbeddingProvider;

    // E-mail 1: Laudo laboratorial (sem as palavras exatas "resultado de exame de sangue")
    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 10,
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

    // E-mail 2: Notificação de fatura (irrelevante)
    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 20,
            subject: "Sua fatura de cartão de crédito fechou".to_string(),
            sender: "cartao@banco.com".to_string(),
            date: "2026-10-16T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: false,
            is_flagged: false,
            crypto_meta: None,
        },
        txt_auto: Some("A fatura do seu cartão com vencimento no dia 25 está disponível para pagamento.".to_string()),
        txt_original: None,
        html_sanitized: None,
        attachments: vec![],
    };

    let mut idx = MessageIndex::new();
    idx.index_message_semantic("INBOX", &msg1, &provider);
    idx.index_message_semantic("INBOX", &msg2, &provider);

    for (_, doc) in idx.documents() {
        store.save_document("INBOX", doc, 0).expect("salvar doc");
    }

    // Deve chamar search_hybrid e retornar o e-mail 10 no topo
    let results = store
        .search_hybrid("INBOX", "resultado de exame de sangue", 10)
        .expect("executar search_hybrid no SQLite");

    assert!(!results.is_empty(), "Busca híbrida no SQLite deveria retornar resultados");
    assert_eq!(results[0].uid, 10, "Top 1 deveria ser o laudo de hemograma (UID 10)");
}
