use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse};
use tower::ServiceExt;

const TEST_SECRET: &str = "segredo-super-secreto-de-teste-123456";

fn setup_search_backend() -> Arc<InMemoryMailBackend> {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 2,
            unread_messages: 1,
        },
    );

    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 101,
                subject: "Parecer Jurídico".to_string(),
                sender: "juridico@exemplo.gov.br".to_string(),
                date: "2026-10-15T10:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: true,
                is_flagged: false,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some(
                "Aprovada a norma sobre armamento e tiro para os membros da corporação.\n\n\
                 Atenciosamente,\n\
                 Comandante Geral"
                    .to_string(),
            ),
            txt_original: None,
            html_sanitized: None,
        },
    );

    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 102,
                subject: "Encaminhamento Pericial".to_string(),
                sender: "saude@exemplo.gov.br".to_string(),
                date: "2026-10-15T11:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: true,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some(
                "Encaminhamos o servidor para inspeção de saúde e perícia no hospital militar.\n\n\
                 Atenciosamente,\n\
                 Major Médico Den Silveira"
                    .to_string(),
            ),
            txt_original: None,
            html_sanitized: None,
        },
    );

    backend
}

#[tokio::test]
async fn test_server_search_endpoint() {
    let backend = setup_search_backend();
    let app = app_router(backend.clone(), TEST_SECRET);

    // 1. Obter Token
    let login_body = serde_json::to_string(&LoginRequest {
        username: "lumen".to_string(),
        password: "senha123".to_string(),
    })
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&body_bytes).unwrap();
    let token = token_resp.token;

    // 2. Busca por "tiro" via /messages/search?box=INBOX&q=tiro
    let req_tiro = Request::builder()
        .method("GET")
        .uri("/messages/search?box=INBOX&q=tiro")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp_tiro = app.clone().oneshot(req_tiro).await.unwrap();
    assert_eq!(resp_tiro.status(), StatusCode::OK);

    let bytes = resp_tiro.into_body().collect().await.unwrap().to_bytes();
    let results: Vec<MessageEnvelope> = serde_json::from_slice(&bytes).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].uid, 101);

    // 3. Busca por "hospital" via /mailboxes/INBOX/search?q=hospital
    let req_hosp = Request::builder()
        .method("GET")
        .uri("/mailboxes/INBOX/search?q=hospital")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp_hosp = app.clone().oneshot(req_hosp).await.unwrap();
    assert_eq!(resp_hosp.status(), StatusCode::OK);

    let bytes = resp_hosp.into_body().collect().await.unwrap().to_bytes();
    let results_hosp: Vec<MessageEnvelope> = serde_json::from_slice(&bytes).unwrap();
    assert!(!results_hosp.is_empty());
    assert_eq!(results_hosp[0].uid, 102);
}

#[tokio::test]
async fn test_search_edge_cases_and_unauthorized() {
    let backend = setup_search_backend();
    let app = app_router(backend.clone(), TEST_SECRET);

    // 1. Acesso sem token deve retornar 401 Unauthorized
    let req_unauth = Request::builder()
        .method("GET")
        .uri("/messages/search?box=INBOX&q=armamento")
        .body(Body::empty())
        .unwrap();

    let resp_unauth = app.clone().oneshot(req_unauth).await.unwrap();
    assert_eq!(resp_unauth.status(), StatusCode::UNAUTHORIZED);

    // 2. Com token válido
    let login_body = serde_json::to_string(&LoginRequest {
        username: "lumen".to_string(),
        password: "senha123".to_string(),
    })
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let token_resp: TokenResponse = serde_json::from_slice(
        &response.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    let token = token_resp.token;

    // 3. Termo que não casa com nenhuma mensagem
    let req_nomatch = Request::builder()
        .method("GET")
        .uri("/messages/search?box=INBOX&q=xkz987qwv123")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp_nomatch = app.clone().oneshot(req_nomatch).await.unwrap();
    assert_eq!(resp_nomatch.status(), StatusCode::OK);
    let bytes = resp_nomatch.into_body().collect().await.unwrap().to_bytes();
    let results: Vec<MessageEnvelope> = serde_json::from_slice(&bytes).unwrap();
    assert!(results.is_empty());

    // 4. Query vazia deve retornar todas as mensagens da caixa
    let req_empty = Request::builder()
        .method("GET")
        .uri("/messages/search?box=INBOX&q=")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let resp_empty = app.clone().oneshot(req_empty).await.unwrap();
    assert_eq!(resp_empty.status(), StatusCode::OK);
    let bytes = resp_empty.into_body().collect().await.unwrap().to_bytes();
    let results_empty: Vec<MessageEnvelope> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(results_empty.len(), 2);
}

