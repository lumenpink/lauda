use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use tempfile::tempdir;

#[test]
fn test_search_bm25_does_not_join_indexed_documents() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("search_plan_test.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    // Inserir candidato diretamente em inverted_index sem registrar em indexed_documents
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");
        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["orcamento", "INBOX", 4242, 5],
        )
        .expect("insert inverted_index");
    }

    let results = store
        .search_hybrid("INBOX", "orcamento", 10)
        .expect("search hybrid");

    // Na implementação com JOIN, este assert falha pois a linha não existe em indexed_documents.
    assert!(
        results.iter().any(|r| r.uid == 4242),
        "A busca BM25 deve recuperar o UID diretamente de inverted_index sem depender de JOIN com indexed_documents"
    );
}
