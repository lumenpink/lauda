use std::sync::Arc;
use tempfile::tempdir;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lauda_index::{MessageIndex, SimpleCpuEmbeddingProvider};
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_server::app_router;
use lauda_types::{
    LoginRequest, MailboxInfo, Message, MessageEnvelope, SearchResult, TokenResponse,
};
use tower::ServiceExt;

#[test]
fn test_sqlite_index_store_sync_status_calculation() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("sync_status.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    // 1. Estado inicial: 0 documentos indexados em caixa com 10 mensagens
    let status_initial = store
        .get_sync_status("INBOX", 10)
        .expect("calcular sync status inicial");
    assert!(
        !status_initial.is_synced,
        "Base vazia não deve constar como sincronizada"
    );
    assert_eq!(status_initial.indexed_count, 0);
    assert_eq!(status_initial.total_count, 10);
    assert_eq!(status_initial.remaining_count, 10);

    // 2. Indexar 2 documentos
    let mut index = MessageIndex::new();
    let provider = SimpleCpuEmbeddingProvider;

    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 1,
            subject: "Boleto de Energia".to_string(),
            sender: "contas@energia.com".to_string(),
            date: "2026-10-10T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Boleto para pagamento da fatura de energia elétrica.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 2,
            subject: "Extrato Bancário".to_string(),
            sender: "extratos@banco.com".to_string(),
            date: "2026-10-11T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: false,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Extrato consolidado mensal da conta corrente.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    index.index_message_semantic("INBOX", &msg1, &provider);
    index.index_message_semantic("INBOX", &msg2, &provider);

    for doc in index.documents().values() {
        store
            .save_document("INBOX", doc, 1000)
            .expect("salvar documento");
    }

    // 3. Status com sincronização parcial (2 de 5)
    let status_partial = store
        .get_sync_status("INBOX", 5)
        .expect("calcular status parcial");
    assert!(
        !status_partial.is_synced,
        "2 de 5 não deve constar como sincronizado"
    );
    assert_eq!(status_partial.indexed_count, 2);
    assert_eq!(status_partial.total_count, 5);
    assert_eq!(status_partial.remaining_count, 3);

    // 4. Status com sincronização 100% concluída (2 de 2)
    let status_complete = store
        .get_sync_status("INBOX", 2)
        .expect("calcular status completo");
    assert!(
        status_complete.is_synced,
        "2 de 2 deve constar como sincronizado"
    );
    assert_eq!(status_complete.indexed_count, 2);
    assert_eq!(status_complete.total_count, 2);
    assert_eq!(status_complete.remaining_count, 0);
}

#[test]
fn test_sqlite_index_store_direct_bm25_search() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("bm25_search.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let mut index = MessageIndex::new();
    let provider = SimpleCpuEmbeddingProvider;

    let msg1 = Message {
        envelope: MessageEnvelope {
            uid: 101,
            subject: "Fatura Cartao de Credito".to_string(),
            sender: "faturas@cartao.com".to_string(),
            date: "2026-10-12T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some(
            "A fatura do seu cartao de credito vence no dia 20. Pague o cartao em dia.".to_string(),
        ),
        txt_original: None,
        html_sanitized: None,
    };

    let msg2 = Message {
        envelope: MessageEnvelope {
            uid: 102,
            subject: "Comprovante PIX Recebido".to_string(),
            sender: "pix@banco.com".to_string(),
            date: "2026-10-13T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: true,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some(
            "Voce recebeu um pix no valor de R$ 150,00 atraves da chave cartao fidelidade."
                .to_string(),
        ),
        txt_original: None,
        html_sanitized: None,
    };

    let msg3 = Message {
        envelope: MessageEnvelope {
            uid: 103,
            subject: "Boletim de Notícias".to_string(),
            sender: "newsletter@tech.com".to_string(),
            date: "2026-10-14T12:00:00Z".to_string(),
            cc: None,
            has_attachments: true,
            is_read: false,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Principais noticias de tecnologia e seguranca da semana.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    index.index_message_semantic("INBOX", &msg1, &provider);
    index.index_message_semantic("INBOX", &msg2, &provider);
    index.index_message_semantic("INBOX", &msg3, &provider);

    for doc in index.documents().values() {
        store
            .save_document("INBOX", doc, 1000)
            .expect("salvar documento");
    }

    // Busca BM25 direta via SQL
    let results_cartao = store
        .search_bm25("INBOX", "cartao", 10)
        .expect("busca direta bm25 cartao");
    assert_eq!(
        results_cartao.len(),
        2,
        "Deve encontrar msg1 e msg2 que contêm cartao"
    );
    // msg1 tem termo 'cartao' repetido tanto no subject quanto no corpo, logo deve ter maior score
    assert_eq!(results_cartao[0].uid, 101);
    assert_eq!(results_cartao[1].uid, 102);

    let results_pix = store
        .search_bm25("INBOX", "pix", 10)
        .expect("busca direta bm25 pix");
    assert_eq!(results_pix.len(), 1);
    assert_eq!(results_pix[0].uid, 102);

    let results_inexistente = store
        .search_bm25("INBOX", "palavra_desconhecida_xyz", 10)
        .expect("busca inexistente");
    assert!(results_inexistente.is_empty());
}

#[tokio::test]
async fn test_search_api_returns_sync_status_headers_and_payload() {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 5,
            unread_messages: 2,
        },
    );

    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 101,
                subject: "Alerta de Seguranca".to_string(),
                sender: "sec@lohn.in".to_string(),
                date: "2026-10-18T10:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: true,
                is_flagged: false,
                crypto_meta: None,
            },
            attachments: vec![],
            txt_auto: Some("Alerta de login detectado a partir de novo IP.".to_string()),
            txt_original: None,
            html_sanitized: None,
        },
    );

    let secret = "teste-segredo-jwt-bm25-status";
    let app = app_router(backend.clone(), secret);

    // Login
    let login_body = serde_json::to_string(&LoginRequest {
        username: "lumen".to_string(),
        password: "senha123".to_string(),
    })
    .unwrap();

    let req_login = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body))
        .unwrap();

    let resp_login = app.clone().oneshot(req_login).await.unwrap();
    let token_resp: TokenResponse =
        serde_json::from_slice(&resp_login.into_body().collect().await.unwrap().to_bytes())
            .unwrap();
    let token = token_resp.token;

    // Busca com status de sincronização
    let req_search = Request::builder()
        .method("GET")
        .uri("/messages/search?box=INBOX&q=login")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp_search = app.clone().oneshot(req_search).await.unwrap();
    assert_eq!(resp_search.status(), StatusCode::OK);

    // Verificar cabeçalhos de sincronização
    let headers = resp_search.headers();
    assert!(headers.contains_key("x-lauda-index-synced"));
    assert!(headers.contains_key("x-lauda-index-indexed"));
    assert!(headers.contains_key("x-lauda-index-total"));
    assert!(headers.contains_key("x-lauda-index-remaining"));

    // Verificar corpo JSON estruturado SearchResult
    let bytes = resp_search.into_body().collect().await.unwrap().to_bytes();
    let result: SearchResult = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result.messages.len(), 1);
    assert_eq!(result.messages[0].uid, 101);
    assert_eq!(result.sync_status.total_count, 5);
}
