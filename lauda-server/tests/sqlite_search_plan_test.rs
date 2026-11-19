use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use tempfile::tempdir;

#[test]
fn test_search_bm25_query_plan_uses_no_join() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("search_plan_test.db");
    let _store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");

    // Query plan da busca direta em inverted_index
    let sql = "EXPLAIN QUERY PLAN SELECT i.uid FROM inverted_index i WHERE i.mailbox = ?1 AND i.term IN (?2) GROUP BY i.uid ORDER BY SUM(i.tf) DESC, i.uid DESC LIMIT 50";
    let mut stmt = conn.prepare(sql).expect("prepare explain");
    let rows = stmt
        .query_map(rusqlite::params!["INBOX", "orcamento"], |row| {
            row.get::<_, String>(3)
        })
        .expect("query map");

    let mut details = Vec::new();
    for r in rows.flatten() {
        details.push(r);
    }
    let full_plan = details.join(" | ");

    assert!(
        !full_plan.contains("indexed_documents"),
        "O plano de execução da busca BM25 não deve referenciar a tabela indexed_documents: {full_plan}"
    );
    assert!(
        full_plan.contains("inverted_index"),
        "O plano de execução deve utilizar a tabela inverted_index diretamente: {full_plan}"
    );
}

#[test]
fn test_search_bm25_retrieval_and_ranking() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("search_retrieval_test.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    // Inserir documento e termo correspondente
    {
        let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");
        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["INBOX", 4242, "Orcamento Aprovado", "financeiro@lohn.in", "2026-11-19T10:00:00Z"],
        ).expect("insert indexed_documents");

        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["orcamento", "INBOX", 4242, 5],
        ).expect("insert inverted_index");
    }

    let results = store
        .search_hybrid("INBOX", "orcamento", 10)
        .expect("search hybrid");
    assert!(
        results.iter().any(|r| r.uid == 4242),
        "A busca BM25 deve recuperar o UID 4242 indexado"
    );
}
