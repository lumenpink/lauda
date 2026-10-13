use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use lauda_client::domain::view::parse_markdown_to_html;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{ComposeForm, LoginRequest, Message, MessageEnvelope, TokenResponse};
use tower::ServiceExt;
use urupema::adapters::InMemorySink;
use urupema::application::Pipeline;
use urupema::domain::RawEmail;

const TEST_SECRET: &str = "jwt-secret-para-testes-integrados-e2e-123456";

#[tokio::test]
async fn test_full_loop_urupema_sanitizer_to_lauda_client_and_compose() {
    // -------------------------------------------------------------------------
    // 1. Ingestão e Higienização via Urupema Pipeline
    // -------------------------------------------------------------------------
    let raw_rfc5322 = b"From: remetente@perigoso.com\r\n\
To: lumen@2lp.in\r\n\
Subject: Comunicado com Tracker e XSS\r\n\
MIME-Version: 1.0\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html>\
<body>\
<script>alert('pwned')</script>\
<h1>Aviso Importante de Seguranca</h1>\
<p>Verifique o relatorio em <a href=\"https://empresa.com/doc\">documento oficial</a>.</p>\
<img src=\"https://tracker.spy/pixel.gif\" width=\"1\" height=\"1\" />\
</body>\
</html>";

    let raw = RawEmail::new(raw_rfc5322.to_vec());
    let mut urupema_sink = InMemorySink::new();
    let sender = "remetente@perigoso.com";
    let recipient = "lumen@2lp.in";

    let (sanitized, _result) = Pipeline::process(&raw, sender, recipient, &mut urupema_sink)
        .expect("Urupema pipeline deve processar sem erros");

    // Validações no sink de saída do Urupema
    assert_eq!(urupema_sink.deliveries.len(), 1);
    let delivery = &urupema_sink.deliveries[0];
    assert_eq!(delivery.sender, sender);
    assert_eq!(delivery.recipient, recipient);

    let delivered_str = String::from_utf8_lossy(&delivery.payload);
    assert!(
        delivered_str.contains("X-Urupema-Processed: 1"),
        "Deve conter cabecalho loop prevention do Urupema"
    );
    assert!(
        !delivered_str.contains("<script>"),
        "HTML entregue nao deve conter tags script"
    );

    let parts = sanitized.parts;
    let txt_auto = parts
        .markdown_auto
        .expect("Urupema deve gerar txt_auto em Markdown");
    let html_sanitized = parts
        .sanitized_html
        .expect("Urupema deve gerar HTML higienizado");

    assert!(
        txt_auto.contains("Aviso Importante"),
        "txt_auto deve conter o titulo em Markdown"
    );
    assert!(
        !html_sanitized.contains("<script>"),
        "html_sanitized nao deve conter scripts"
    );

    // -------------------------------------------------------------------------
    // 2. Armazenamento no Backend do Lauda Server
    // -------------------------------------------------------------------------
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha_forte_123");
    backend.add_mailbox(
        "lumen",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );

    let mail_uid = 42;
    backend.add_message(
        "lumen",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: mail_uid,
                subject: "Comunicado com Tracker e XSS".to_string(),
                sender: sender.to_string(),
                date: "2026-10-02T08:30:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: false,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some(txt_auto.clone()),
            txt_original: parts.original_plain,
            html_sanitized: Some(html_sanitized),
        },
    );

    let app = app_router(backend.clone(), TEST_SECRET);

    // -------------------------------------------------------------------------
    // 3. Autenticação e Consulta de Mensagem via API Lauda
    // -------------------------------------------------------------------------
    let login_req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&LoginRequest {
                username: "lumen".to_string(),
                password: "senha_forte_123".to_string(),
            })
            .unwrap(),
        ))
        .unwrap();

    let login_resp = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);

    let login_bytes = login_resp.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&login_bytes).unwrap();
    let token = token_resp.token;

    let get_msg_req = Request::builder()
        .method("GET")
        .uri(format!("/messages/{mail_uid}?box=INBOX"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let msg_resp = app.clone().oneshot(get_msg_req).await.unwrap();
    assert_eq!(msg_resp.status(), StatusCode::OK);

    let msg_bytes = msg_resp.into_body().collect().await.unwrap().to_bytes();
    let fetched_msg: Message = serde_json::from_slice(&msg_bytes).unwrap();
    assert_eq!(fetched_msg.envelope.uid, mail_uid);

    // -------------------------------------------------------------------------
    // 4. Renderização Gráfica do Smolclient (Dioxus / Markdown)
    // -------------------------------------------------------------------------
    let client_txt_auto = fetched_msg.txt_auto.expect("Mensagem deve conter txt_auto");
    let rendered_html = parse_markdown_to_html(&client_txt_auto);

    assert!(
        rendered_html.contains("Aviso Importante de Seguranca"),
        "Visualizador do cliente deve renderizar o titulo a partir do Markdown"
    );
    assert!(
        !rendered_html.contains("<script>"),
        "Visualizador do cliente nao deve conter tags script"
    );

    // -------------------------------------------------------------------------
    // 5. Redação e Envio de Resposta Desimpedida (/compose)
    // -------------------------------------------------------------------------
    let reply_form = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: sender.to_string(),
        cc: None,
        subject: "Re: Comunicado com Tracker e XSS".to_string(),
        body: "Recebido e visualizado com sucesso no modo txt auto seguro.".to_string(),
        attachments: vec![],
    };

    let compose_req = Request::builder()
        .method("POST")
        .uri("/compose")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(serde_json::to_string(&reply_form).unwrap()))
        .unwrap();

    let compose_resp = app.clone().oneshot(compose_req).await.unwrap();
    assert_eq!(
        compose_resp.status(),
        StatusCode::CREATED,
        "Envio de mensagem via /compose deve retornar 201 Created"
    );

    // Verifica que a mensagem enviada foi registrada no backend
    assert_eq!(backend.sent_count(), 1);
}
