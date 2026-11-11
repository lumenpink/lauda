use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{LoginRequest, MailboxInfo, TokenResponse};
use tower::ServiceExt;

const TEST_SECRET: &str = "segredo-super-secreto-websocket-test-123456";

fn setup_test_backend() -> Arc<InMemoryMailBackend> {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 0,
            unread_messages: 0,
        },
    );
    backend
}

async fn get_valid_token(app: &axum::Router) -> String {
    let login_body = serde_json::to_string(&LoginRequest {
        username: "lumen".to_string(),
        password: "senha123".to_string(),
        client_type: None,
    })
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&bytes).unwrap();
    token_resp.token
}

#[tokio::test]
async fn test_ws_handshake_rejects_missing_or_invalid_token() {
    let backend = setup_test_backend();
    let app = app_router(backend, TEST_SECRET);

    // 1. Sem token algum
    let req_no_token = Request::builder()
        .method("GET")
        .uri("/ws")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req_no_token).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Acesso ao /ws sem token DEVE retornar 401 Unauthorized"
    );

    // 2. Com token inválido
    let req_invalid_token = Request::builder()
        .method("GET")
        .uri("/ws?token=token_invalido_com_lixo_assinado")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let resp_invalid = app.clone().oneshot(req_invalid_token).await.unwrap();
    assert_eq!(
        resp_invalid.status(),
        StatusCode::UNAUTHORIZED,
        "Acesso ao /ws com token inválido DEVE retornar 401 Unauthorized"
    );
}

#[tokio::test]
async fn test_ws_handshake_accepts_valid_token() {
    let backend = setup_test_backend();
    let app = app_router(backend, TEST_SECRET);
    let token = get_valid_token(&app).await;

    let req_valid = Request::builder()
        .method("GET")
        .uri(format!("/ws?token={}", token))
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req_valid).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::SWITCHING_PROTOCOLS,
        "Handshake do /ws com token válido DEVE retornar 101 Switching Protocols"
    );
}
