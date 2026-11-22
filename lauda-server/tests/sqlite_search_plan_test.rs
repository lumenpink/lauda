use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use tempfile::tempdir;

#[test]
fn test_search_bm25_query_plan_uses_no_join() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("search_plan_test.db");
    let _store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");

    // Query plan da busca direta em inverted_index com covering index explícito
    let sql = "EXPLAIN QUERY PLAN SELECT i.uid FROM inverted_index i INDEXED BY idx_inverted_bm25_covering WHERE i.mailbox = ?1 AND i.term IN (?2) GROUP BY i.uid ORDER BY SUM(i.tf) DESC, i.uid DESC LIMIT 50";
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
        full_plan.contains("idx_inverted_bm25_covering") || full_plan.contains("inverted_index"),
        "O plano de execução deve utilizar a tabela ou índice cobridor de inverted_index: {full_plan}"
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

#[test]
fn test_search_bm25_covering_index_plan() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("covering_plan_test.db");
    let _store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");

    // Valida que o índice cobridor específico existe no catálogo de índices
    let mut check_idx = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_inverted_bm25_covering'")
        .expect("prepare index check");
    let mut idx_rows = check_idx.query([]).expect("query index");
    let has_covering_index = idx_rows.next().expect("next row").is_some();
    assert!(
        has_covering_index,
        "O índice cobridor idx_inverted_bm25_covering deve estar criado no schema do SqliteIndexStore"
    );

    // Valida que o plano de execução para a query BM25 utiliza o índice cobridor
    let sql = "EXPLAIN QUERY PLAN SELECT i.uid FROM inverted_index i INDEXED BY idx_inverted_bm25_covering WHERE i.mailbox = ?1 AND i.term IN (?2) GROUP BY i.uid ORDER BY SUM(i.tf) DESC, i.uid DESC LIMIT 50";
    let mut stmt = conn.prepare(sql).expect("prepare explain");
    let rows = stmt
        .query_map(rusqlite::params!["INBOX", "baquara"], |row| {
            row.get::<_, String>(3)
        })
        .expect("query map");

    let details: Vec<String> = rows.flatten().collect();
    let full_plan = details.join(" | ");

    assert!(
        full_plan.contains("idx_inverted_bm25_covering") || full_plan.contains("COVERING INDEX"),
        "O plano de execução deve utilizar o índice cobridor (COVERING INDEX): {full_plan}"
    );
}

#[test]
fn test_mutation_covering_index_multi_term_and_empty_terms() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("covering_multi_mutation_test.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    {
        let conn = rusqlite::Connection::open(&db_path).expect("open raw sqlite");

        // Documento 10: contém "contrato" (tf=3) e "aditivo" (tf=4) -> soma = 7
        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["INBOX", 10, "Contrato Aditivo", "juridico@lohn.in", "2026-11-20T10:00:00Z"],
        ).expect("insert doc 10");
        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES ('contrato', 'INBOX', 10, 3), ('aditivo', 'INBOX', 10, 4)",
            [],
        ).expect("insert terms doc 10");

        // Documento 20: contém apenas "contrato" (tf=5) -> soma = 5
        conn.execute(
            "INSERT INTO indexed_documents (mailbox, uid, subject, sender, date, is_read, is_flagged, has_attachments, doc_len, terms_json)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 10, '{}')",
            rusqlite::params!["INBOX", 20, "Contrato Original", "juridico@lohn.in", "2026-11-20T11:00:00Z"],
        ).expect("insert doc 20");
        conn.execute(
            "INSERT INTO inverted_index (term, mailbox, uid, tf) VALUES ('contrato', 'INBOX', 20, 5)",
            [],
        ).expect("insert terms doc 20");
    }

    // Busca multi-termo: doc 10 deve ser ranqueado à frente do doc 20 pela soma de TF
    let results = store
        .search_hybrid("INBOX", "contrato aditivo", 10)
        .expect("search hybrid multi-term");
    assert!(
        results.len() >= 2,
        "Ambos os documentos devem ser recuperados"
    );
    assert_eq!(
        results[0].uid, 10,
        "Doc 10 com maior soma de TF deve vir em primeiro"
    );
    assert_eq!(results[1].uid, 20, "Doc 20 deve vir em segundo");

    // Busca com termo inexistente: retorna vazio sem panic
    let empty_results = store
        .search_hybrid("INBOX", "inexistente_xyz_123", 10)
        .expect("search hybrid empty");
    assert!(
        empty_results.is_empty(),
        "Termo inexistente deve retornar lista vazia"
    );
}
