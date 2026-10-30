use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::{EncodingKey, Header, encode};
use lauda_client::adapters::HttpApiClient;
use lauda_client::domain::ports::{ClientError, MailApiClient};
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::LoginRequest;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use urupema::adapters::InMemorySink;
use urupema::application::Pipeline;
use urupema::domain::{ProcessResult, RawEmail};

async fn wait_for_tcp_ready(addr: SocketAddr) {
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.shutdown().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("Timeout aguardando prontidão TCP em {addr}");
}

const TEST_SECRET: &str = "jwt-secret-resilience-testing-sovereign-mail-999";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// ---------------------------------------------------------------------------
// 1. Urupema: Vetores Avançados de Evasão XSS e Prevenção Rígida de Loops
// ---------------------------------------------------------------------------
#[test]
fn test_urupema_advanced_xss_evasion_and_loop_prevention() {
    // A. Evasão agressiva com mutações de tags e handlers de eventos
    let malicious_email = b"From: atacante@perigoso.com\r\n\
To: lumen@2lp.in\r\n\
Subject: Teste de Evasao XSS\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<html>\
<body>\
  <SCRIPT src=\"http://malware.org/payload.js\"></SCRIPT>\
  <iframe src=\"http://phishing.site/login\"></iframe>\
  <a href=\"javascript:stealCredentials()\">Clique aqui</a>\
  <img src=\"valid.jpg\" onload=\"alert(1)\" onerror=\"alert(2)\" />\
  <img src=\"http://tracker.spy/pixel.gif\" width=\"1\" height=\"1\" />\
  <h1>Mensagem Legitima</h1>\
  <p>Conteudo que deve permanecer intacto.</p>\
</body>\
</html>";

    let raw = RawEmail::new(malicious_email.to_vec());
    let mut sink = InMemorySink::new();
    let (sanitized, result) =
        Pipeline::process(&raw, "atacante@perigoso.com", "lumen@2lp.in", &mut sink)
            .expect("Processamento do Urupema deve suceder");

    assert_eq!(result, ProcessResult::Reinject);
    let parts = sanitized.parts;
    let html = parts.sanitized_html.expect("HTML higienizado gerado");

    // Todos os vetores perigosos devem ter sido expurgados
    assert!(!html.to_lowercase().contains("<script"));
    assert!(!html.to_lowercase().contains("<iframe"));
    assert!(!html.to_lowercase().contains("javascript:"));
    assert!(!html.to_lowercase().contains("onload"));
    assert!(!html.to_lowercase().contains("onerror"));
    assert!(!html.contains("tracker.spy/pixel.gif"));

    // O conteúdo seguro é estritamente preservado
    assert!(html.contains("Mensagem Legitima"));
    assert!(html.contains("Conteudo que deve permanecer intacto."));

    // B. Prevenção Rígida de Loop no Postfix
    let already_processed_email = b"From: sistema@2lp.in\r\n\
To: lumen@2lp.in\r\n\
Subject: Loop Test\r\n\
X-Urupema-Processed: 1; engine=urupema-0.1.0\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Mensagem ja tratada.";

    let raw_loop = RawEmail::new(already_processed_email.to_vec());
    let mut sink_loop = InMemorySink::new();
    let (_, result_loop) =
        Pipeline::process(&raw_loop, "sistema@2lp.in", "lumen@2lp.in", &mut sink_loop)
            .expect("Processamento deve passar");

    assert_eq!(
        result_loop,
        ProcessResult::Bypass,
        "Mensagem já processada deve resultar em Bypass imediato"
    );
    assert_eq!(
        sink_loop.deliveries.len(),
        1,
        "Deve entregar os bytes intactos no sink sem reprocessar"
    );
}

// ---------------------------------------------------------------------------
// 2. Lauda Gateway: Rejeição de Tokens Forjados, Assinaturas Falsas e Expiradas
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_lauda_jwt_tampering_and_unauthorized_attacks() {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha_autentica");
    backend.add_mailbox(
        "lumen",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 0,
            unread_messages: 0,
        },
    );

    let app = app_router(backend.clone(), TEST_SECRET);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Bind TCP");
    let local_addr = listener.local_addr().expect("Local addr");
    let base_url = format!("http://{local_addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    wait_for_tcp_ready(local_addr).await;

    let client = HttpApiClient::new(&base_url);

    // A. Token assinado com chave errada (forjamento)
    let forged_claims = Claims {
        sub: "lumen".to_string(),
        exp: (chrono::Utc::now().timestamp() + 3600) as usize,
    };
    let forged_token = encode(
        &Header::default(),
        &forged_claims,
        &EncodingKey::from_secret("chave_falsa_do_atacante".as_bytes()),
    )
    .unwrap();

    let forged_res = client.list_mailboxes(&forged_token).await;
    assert!(
        matches!(forged_res, Err(ClientError::AuthFailed)),
        "Token com assinatura adulterada deve retornar AuthFailed (401)"
    );

    // B. Token expirado
    let expired_claims = Claims {
        sub: "lumen".to_string(),
        exp: (chrono::Utc::now().timestamp() - 3600) as usize, // Expirou há 1 hora
    };
    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .unwrap();

    let expired_res = client.list_mailboxes(&expired_token).await;
    assert!(
        matches!(expired_res, Err(ClientError::AuthFailed)),
        "Token expirado deve retornar AuthFailed (401)"
    );

    // C. Token válido funciona perfeitamente
    let valid_token = client
        .login(&LoginRequest {
            username: "lumen".to_string(),
            password: "senha_autentica".to_string(),
        })
        .await
        .unwrap()
        .token;

    let boxes = client.list_mailboxes(&valid_token).await.unwrap();
    assert_eq!(boxes.len(), 1);
}
