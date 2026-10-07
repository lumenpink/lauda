use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};
use tower::ServiceExt;

const TEST_SECRET: &str = "segredo-super-secreto-de-teste-123456";

fn setup_test_backend() -> Arc<InMemoryMailBackend> {
    let backend = Arc::new(InMemoryMailBackend::new());

    // Popula dados para teste
    backend.add_user("lumen", "senha123");
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );
    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 1001,
                subject: "Bem-vindo ao Lauda".to_string(),
                sender: "sistema@2lp.in".to_string(),
                date: "2026-10-02T06:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: true,
            },
            attachments: Vec::new(),
            txt_auto: Some("# Bem-vindo\nLeitura em texto puro rápida.".to_string()),
            txt_original: Some("Bem-vindo\nLeitura em texto puro rápida.".to_string()),
            html_sanitized: Some("<h1>Bem-vindo</h1>".to_string()),
        },
    );

    backend
}

#[tokio::test]
async fn test_login_success_and_protected_flow() {
    let backend = setup_test_backend();
    let app = app_router(backend.clone(), TEST_SECRET);

    // 1. Login com sucesso
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
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(token_resp.username, "lumen");
    assert!(!token_resp.token.is_empty());

    let token = token_resp.token;

    // 2. Acesso à rota protegida /mailboxes
    let req = Request::builder()
        .method("GET")
        .uri("/mailboxes")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let boxes: Vec<MailboxInfo> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].name, "INBOX");

    // 3. Acesso à listagem /messages
    let req = Request::builder()
        .method("GET")
        .uri("/messages?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let envelopes: Vec<MessageEnvelope> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0].uid, 1001);

    // 4. Detalhes de /messages/1001
    let req = Request::builder()
        .method("GET")
        .uri("/messages/1001?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let msg: Message = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(msg.envelope.uid, 1001);
    assert_eq!(
        msg.txt_auto.as_deref(),
        Some("# Bem-vindo\nLeitura em texto puro rápida.")
    );

    // 5. Envio de mensagem /compose
    let compose_body = serde_json::to_string(&ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "amigo@exemplo.org".to_string(),
        cc: None,
        subject: "Olá do Smolclient".to_string(),
        body: "Testando envio via lauda-server.".to_string(),
        attachments: vec![],
    })
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/compose")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(compose_body))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(backend.sent_count(), 1);

    // 6. Marcar como lida /messages/1001/read
    let req = Request::builder()
        .method("PATCH")
        .uri("/messages/1001/read?box=INBOX&read=true")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verificar se mensagem agora é lida
    let req = Request::builder()
        .method("GET")
        .uri("/messages/1001?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let msg: Message = serde_json::from_slice(&body_bytes).unwrap();
    assert!(msg.envelope.is_read);

    // 7. Excluir mensagem DELETE /messages/1001
    let req = Request::builder()
        .method("DELETE")
        .uri("/messages/1001?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verificar se mensagem sumiu da INBOX
    let req = Request::builder()
        .method("GET")
        .uri("/messages?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let envelopes: Vec<MessageEnvelope> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(envelopes.is_empty());
}

#[tokio::test]
async fn test_unauthorized_access() {
    let backend = setup_test_backend();
    let app = app_router(backend, TEST_SECRET);

    // Tentativa de login com senha errada
    let login_body = serde_json::to_string(&LoginRequest {
        username: "lumen".to_string(),
        password: "senha_errada".to_string(),
    })
    .unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Tentativa de rota protegida sem header Authorization
    let req = Request::builder()
        .method("GET")
        .uri("/mailboxes")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mua_flag_move_and_attachment_endpoints() {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha123");
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );
    backend.add_mailbox(
        "lumen",
        MailboxInfo {
            name: "Archive".to_string(),
            total_messages: 0,
            unread_messages: 0,
        },
    );
    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 5001,
                subject: "Documento Contratual".to_string(),
                sender: "advogado@2lp.in".to_string(),
                date: "2026-10-06T11:00:00Z".to_string(),
                cc: Some("socio@2lp.in".to_string()),
                has_attachments: true,
                is_read: false,
                is_flagged: false,
            },
            attachments: vec![lauda_types::AttachmentMeta {
                id: 0,
                filename: "minuta.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 512,
            }],
            txt_auto: Some("Segue minuta em anexo.".to_string()),
            txt_original: Some("Segue minuta em anexo.".to_string()),
            html_sanitized: None,
        },
    );

    let app = app_router(backend.clone(), TEST_SECRET);

    // Login
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

    // 1. Alternar flag para true: PATCH /messages/5001/flag?box=INBOX&flagged=true
    let req = Request::builder()
        .method("PATCH")
        .uri("/messages/5001/flag?box=INBOX&flagged=true")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verificar se mensagem agora é flagged
    let req = Request::builder()
        .method("GET")
        .uri("/messages/5001?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let msg: Message = serde_json::from_slice(&body_bytes).unwrap();
    assert!(msg.envelope.is_flagged);
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].filename, "minuta.pdf");

    // 2. Download de anexo com Header Authorization: GET /messages/5001/attachments/0?box=INBOX
    let req = Request::builder()
        .method("GET")
        .uri("/messages/5001/attachments/0?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "application/pdf"
    );

    // 3. Download de anexo com token via Query Param (Link direto de download): GET /messages/5001/attachments/0?box=INBOX&token=...
    let req = Request::builder()
        .method("GET")
        .uri(format!(
            "/messages/5001/attachments/0?box=INBOX&token={token}"
        ))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let att_bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(!att_bytes.is_empty());

    // 4. Mover mensagem de INBOX para Archive: POST /messages/5001/move?src=INBOX&dest=Archive
    let req = Request::builder()
        .method("POST")
        .uri("/messages/5001/move?src=INBOX&dest=Archive")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verificar se INBOX ficou vazia e Archive contém a mensagem
    let req = Request::builder()
        .method("GET")
        .uri("/messages?box=INBOX")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let inbox_msgs: Vec<MessageEnvelope> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(inbox_msgs.is_empty());

    let req = Request::builder()
        .method("GET")
        .uri("/messages?box=Archive")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let archive_msgs: Vec<MessageEnvelope> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(archive_msgs.len(), 1);
    assert_eq!(archive_msgs[0].uid, 5001);

    // 5. Testar endpoint /contacts
    let req = Request::builder()
        .method("GET")
        .uri("/contacts")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let contacts: Vec<lauda_types::ContactInfo> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].email, "advogado@2lp.in");

    // 6. Testar endpoint /events (SSE) com token via query param
    let req = Request::builder()
        .method("GET")
        .uri(format!("/events?token={token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/event-stream"
    );
}
