use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use lauda_index::IndexedDoc;
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use tempfile::tempdir;

#[test]
fn test_sqlite_concurrent_read_during_write() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_index.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite store");

    // Popula documento inicial
    let mut tf = HashMap::new();
    tf.insert("lisiane".to_string(), 3);
    tf.insert("urgente".to_string(), 1);

    let mut subject_terms = HashSet::new();
    subject_terms.insert("relatorio".to_string());
    subject_terms.insert("producao".to_string());

    let mut sender_terms = HashSet::new();
    sender_terms.insert("lisiane".to_string());

    let doc = IndexedDoc {
        uid: 100,
        mailbox: "INBOX".to_string(),
        sender: "lisiane@example.com".to_string(),
        subject: "Relatório de produção".to_string(),
        date: "2026-11-20T10:00:00Z".to_string(),
        is_read: false,
        is_flagged: true,
        has_attachments: false,
        subject_terms,
        sender_terms,
        term_frequencies: tf,
        doc_len: 20,
        segment_embeddings: vec![],
    };

    store
        .save_document("INBOX", &doc, 1000)
        .expect("salvar doc inicial");

    let barrier = Arc::new(Barrier::new(2));
    let barrier_writer = barrier.clone();
    let store_writer = store.clone();

    // Thread de escrita simulando background worker pesado
    let writer_handle = thread::spawn(move || {
        barrier_writer.wait();
        for i in 101..=150 {
            let mut tf = HashMap::new();
            tf.insert(format!("termo_{}", i), 2);
            let d = IndexedDoc {
                uid: i,
                mailbox: "INBOX".to_string(),
                sender: "worker@example.com".to_string(),
                subject: format!("Mensagem em background {}", i),
                date: "2026-11-20T12:00:00Z".to_string(),
                is_read: true,
                is_flagged: false,
                has_attachments: false,
                subject_terms: HashSet::new(),
                sender_terms: HashSet::new(),
                term_frequencies: tf,
                doc_len: 10,
                segment_embeddings: vec![],
            };
            store_writer
                .save_document("INBOX", &d, 1000 + i as i64)
                .expect("salvar documento em background");
            thread::sleep(Duration::from_millis(10));
        }
    });

    barrier.wait();

    // A thread leitora executa buscas concorrentes enquanto o writer está em loop ativo
    let start = std::time::Instant::now();
    for _ in 0..10 {
        let results = store
            .search_bm25("INBOX", "lisiane", 10)
            .expect("busca bm25 concorrente deve responder");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].uid, 100);

        let status = store
            .get_sync_status("INBOX", 200)
            .expect("sync status concorrente deve responder");
        assert!(status.indexed_count >= 1);

        thread::sleep(Duration::from_millis(15));
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "Buscas concorrentes demoraram {:?}, excedendo o limite de fluidez",
        elapsed
    );

    writer_handle.join().expect("join writer thread");
}

#[test]
fn test_sqlite_read_does_not_block_on_held_write_lock() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_lock.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite store");

    let mut tf = HashMap::new();
    tf.insert("lisiane".to_string(), 5);

    let mut subject_terms = HashSet::new();
    subject_terms.insert("teste".to_string());

    let mut sender_terms = HashSet::new();
    sender_terms.insert("lisiane".to_string());

    let doc = IndexedDoc {
        uid: 42,
        mailbox: "INBOX".to_string(),
        sender: "lisiane@example.com".to_string(),
        subject: "Teste de concorrência com lock retido".to_string(),
        date: "2026-11-20T10:00:00Z".to_string(),
        is_read: true,
        is_flagged: false,
        has_attachments: false,
        subject_terms,
        sender_terms,
        term_frequencies: tf,
        doc_len: 15,
        segment_embeddings: vec![],
    };
    store
        .save_document("INBOX", &doc, 100)
        .expect("salvar documento");

    let barrier = Arc::new(Barrier::new(2));
    let barrier_holder = barrier.clone();
    let store_holder = store.clone();

    // Thread que retém o lock de escrita por 500ms simulando transação longa do indexador
    let lock_holder_thread = thread::spawn(move || {
        let write_conn = store_holder.raw_connection();
        write_conn
            .execute_batch("BEGIN IMMEDIATE;")
            .expect("iniciar transacao imediata");
        barrier_holder.wait();
        // Segura a conexão de escrita aberta
        thread::sleep(Duration::from_millis(500));
        write_conn
            .execute_batch("COMMIT;")
            .expect("commit transacao imediata");
    });

    barrier.wait();

    // Leitura em paralelo deve responder imediatamente usando a conexao de leitura independente,
    // sem ficar bloqueada pelos 500ms do mutex ou da transacao de escrita
    let read_start = std::time::Instant::now();
    let results = store
        .search_bm25("INBOX", "lisiane", 10)
        .expect("leitura deve funcionar mesmo com escrita ativa");
    let read_elapsed = read_start.elapsed();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uid, 42);
    // Deve responder em menos de 150ms, provando que não esperou os 500ms do lock de escrita
    assert!(
        read_elapsed < Duration::from_millis(150),
        "A leitura demorou {:?} esperando a conexão de escrita!",
        read_elapsed
    );

    lock_holder_thread.join().expect("join lock holder");
}

#[test]
fn test_mutation_sqlite_isolation_and_empty_search() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_mut.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite store");

    // Mutação 1: Busca em caixa postal vazia ou inexistente não trava e retorna vazio
    let res = store
        .search_bm25("NONEXISTENT", "lisiane", 10)
        .expect("busca em caixa inexistente");
    assert!(res.is_empty());

    let status = store
        .get_sync_status("NONEXISTENT", 0)
        .expect("sync status caixa inexistente");
    assert_eq!(status.indexed_count, 0);

    // Mutação 2: Query vazia deve retornar imediatamente
    let empty_res = store
        .search_bm25("INBOX", "", 10)
        .expect("busca com query vazia");
    assert!(empty_res.is_empty());
}
