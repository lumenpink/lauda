use lauda_server::adapters::maildir::MaildirBackend;
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_server::domain::ports::MailBackend;
use std::collections::HashSet;
use tempfile::tempdir;

#[tokio::test]
async fn test_maildir_sanitizes_markdown_noise_and_underlines() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let msg_filename = "1728000001.M123P456.test:2,S";
    let eml_content = concat!(
        "From: newsletter@exemplo.org\n",
        "To: lumen@2lp.in\n",
        "Subject: Novidades do Canal\n",
        "Date: Mon, 05 Oct 2026 12:00:00 +0000\n",
        "MIME-Version: 1.0\n",
        "Content-Type: text/html; charset=utf-8\n",
        "\n",
        "<html><body>\n",
        "<h1>Destaques da Semana</h1>\n",
        "<p>Primeiro paragrafo substantivo da noticia.</p>\n",
        "<p>__________________________________________________</p>\n",
        "<p>Divisor com underlines repetitivos _____ no meio do texto.</p>\n",
        "</body></html>\n"
    );

    std::fs::write(root.join("cur").join(msg_filename), eml_content).unwrap();
    let uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    let msg = backend
        .get_message("lumen", "INBOX", 100)
        .await
        .expect("Mensagem deve ser lida");

    let txt_auto = msg.txt_auto.expect("txt_auto deve ser gerado do HTML");

    // O sanitizador do Urupema deve eliminar a linha isolada de underlines e colapsar underlines repetidos
    assert!(
        !txt_auto.contains("__________________________________________________"),
        "txt_auto nao deve conter linha ruidosa de underlines: {txt_auto}"
    );
    assert!(
        !txt_auto.contains("_____"),
        "txt_auto nao deve conter sequencias de underlines repetitivos: {txt_auto}"
    );
    assert!(
        txt_auto.contains("Primeiro paragrafo substantivo da noticia."),
        "txt_auto deve preservar o conteudo substantivo: {txt_auto}"
    );
}

#[test]
fn test_sqlite_index_clear_mailbox_and_reindex_reconciliation() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("index.db");

    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    // Indexar documento inicial
    let mut index = lauda_index::MessageIndex::new();
    let provider = lauda_index::SimpleCpuEmbeddingProvider;

    let msg = lauda_types::Message {
        envelope: lauda_types::MessageEnvelope {
            uid: 501,
            subject: "Email Antigo".to_string(),
            sender: "a@b.com".to_string(),
            date: "2026-10-01T10:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Corpo antigo com ruido.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    index.index_message_semantic("INBOX", &msg, &provider);
    for doc in index.documents().values() {
        store.save_document("INBOX", doc, 1728000000).unwrap();
    }

    let indexed_uids = store.get_indexed_uids("INBOX").unwrap();
    assert_eq!(indexed_uids.len(), 1);
    assert!(indexed_uids.contains(&501));

    // Forçar limpeza da caixa postal para reindexação
    store
        .clear_mailbox_index("INBOX")
        .expect("limpar indice da caixa postal");

    let uids_after_clear = store.get_indexed_uids("INBOX").unwrap();
    assert!(
        uids_after_clear.is_empty(),
        "Apos clear_mailbox_index, todos os UIDs devem ter sido expurgados para reindexacao"
    );

    // Reconciliação posterior deve enxergar 501 como 'to_index'
    let mut active = HashSet::new();
    active.insert(501);
    let reconcile = store.reconcile_mailbox("INBOX", &active).unwrap();
    assert_eq!(
        reconcile.to_index,
        vec![501],
        "UID 501 deve ser marcado para reindexacao completa"
    );
}
