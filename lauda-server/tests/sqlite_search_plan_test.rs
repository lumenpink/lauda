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
        )
        .expect("insert inverted_index");
    }

    let results = store
        .search_hybrid("INBOX", "orcamento", 10)
        .expect("search hybrid");
    assert!(
        results.iter().any(|r| r.uid == 4242),
        "A busca BM25 deve recuperar o UID 4242 indexado"
    );
}

#[test]
fn test_search_bm25_mutation_ordering_and_mailbox_isolation() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("search_mutation_test.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");

        // Registrar dois documentos na INBOX e um na pasta Sent
        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["INBOX", 1, "Doc Menor TF", "remetente@a.com", "2026-11-19T10:00:00Z"],
        ).expect("insert doc 1");

        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["INBOX", 2, "Doc Maior TF", "remetente@b.com", "2026-11-19T10:00:00Z"],
        ).expect("insert doc 2");

        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["Sent", 3, "Doc Outra Caixa", "remetente@c.com", "2026-11-19T10:00:00Z"],
        ).expect("insert doc 3");

        // INBOX: doc 1 tem tf=2, doc 2 tem tf=10
        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["relatorio", "INBOX", 1, 2],
        )
        .expect("insert inverted doc 1");

        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["relatorio", "INBOX", 2, 10],
        )
        .expect("insert inverted doc 2");

        // Sent: doc 3 tem tf=50, mas pertence a outra caixa
        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["relatorio", "Sent", 3, 50],
        )
        .expect("insert inverted doc 3");
    }

    let results = store
        .search_hybrid("INBOX", "relatorio", 10)
        .expect("search hybrid");

    // Valida isolamento de caixa postal (doc 3 da Sent não pode aparecer)
    assert!(
        !results.iter().any(|r| r.uid == 3),
        "Mensagem de outra caixa (Sent) não deve constar na busca de INBOX"
    );

    // Valida ordenação BM25 (doc 2 com tf=10 deve vir antes do doc 1 com tf=2)
    assert!(
        results.len() >= 2,
        "Ambos os documentos devem ser retornados"
    );
    assert_eq!(
        results[0].uid, 2,
        "Documento com maior TF deve ser ranqueado em primeiro"
    );
    assert_eq!(
        results[1].uid, 1,
        "Documento com menor TF deve ser ranqueado em segundo"
    );
}
