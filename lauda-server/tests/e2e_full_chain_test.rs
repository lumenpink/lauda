use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use lauda_client::adapters::HttpApiClient;
use lauda_client::domain::ports::MailApiClient;
use lauda_client::domain::view::parse_markdown_to_html;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{ComposeForm, LoginRequest, Message, MessageEnvelope};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use urupema::adapters::InMemorySink;
use urupema::application::Pipeline;
use urupema::domain::RawEmail;

async fn wait_for_tcp_ready(addr: SocketAddr) {
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let probe = b"GET /auth/login HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
            if stream.write_all(probe).await.is_ok() {
                use tokio::io::AsyncReadExt;
                let mut buf = [0u8; 16];
                if let Ok(n) = stream.read(&mut buf).await
                    && n > 0
                {
                    return;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
    panic!("Timeout aguardando prontidão TCP/HTTP em {addr}");
}

const TEST_SECRET: &str = "jwt-secret-full-chain-sovereign-mail-123456";
const SECURITY_WORD: &str = "CAFE_COM_PAO_DE_QUEIJO_SUL_MINAS_2026";

/// 1. Testes de Integração de Ponta a Ponta: Lauda Server & Lauda Client
#[tokio::test]
async fn test_full_integrated_mail_chain_with_in_memory_backend() {
    let backend = Arc::new(InMemoryMailBackend::new());

    // Popula usuário e caixas postais
    backend.add_user("lumen", "senha_super_secreta_99");
    backend.add_mailbox(
        "lumen",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );

    // Popula mensagem de teste com as três visões
    let sample_msg = Message {
        envelope: MessageEnvelope {
            uid: 777,
            subject: "Relatorio Financeiro e Comunicado".to_string(),
            sender: "financeiro@2lp.in".to_string(),
            date: "2026-10-01T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: false,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: Vec::new(),
        txt_auto: Some(
            "# Relatorio Mensal\n\n- Lucro: +35%\n- Servidores: 100% no ar.".to_string(),
        ),
        txt_original: Some("Relatorio Mensal\nLucro: +35%\nServidores: 100% no ar.".to_string()),
        html_sanitized: Some(
            "<div class=\"papeli-sheet\"><h1>Relatorio Mensal</h1><p>Lucro: +35%</p></div>"
                .to_string(),
        ),
    };
    backend.add_message("lumen", "INBOX", sample_msg);

    // 1. Sobe o servidor Axum escutando em porta TCP efêmera real
    let app = app_router(backend.clone(), TEST_SECRET);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Deve vincular porta efêmera");
    let local_addr = listener.local_addr().expect("Local addr");
    let base_url = format!("http://{}", local_addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    wait_for_tcp_ready(local_addr).await;

    // 2. Instancia o HttpApiClient apontando para a URL TCP real
    let client = HttpApiClient::new(&base_url);

    // A. Falha de Login com credencial errada
    let bad_login = LoginRequest {
        username: "lumen".to_string(),
        password: "senha_incorreta".to_string(),
    };
    let err = client
        .login(&bad_login)
        .await
        .expect_err("Login incorreto deve falhar");
    assert!(matches!(
        err,
        lauda_client::domain::ports::ClientError::AuthFailed
    ));

    // B. Sucesso no Login
    let good_login = LoginRequest {
        username: "lumen".to_string(),
        password: "senha_super_secreta_99".to_string(),
    };
    let token_resp = client.login(&good_login).await.expect("Login bem sucedido");
    let token = token_resp.token;
    assert!(!token.is_empty());

    // C. Listagem de Caixas
    let boxes = client
        .list_mailboxes(&token)
        .await
        .expect("Listagem de caixas postais");
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].name, "INBOX");

    // D. Listagem de Mensagens
    let messages = client
        .list_messages(&token, "INBOX", None, None)
        .await
        .expect("Listagem de mensagens");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].uid, 777);

    // E. Recuperação e Validação do Visualizador Trifásico
    let full_msg = client
        .get_message(&token, "INBOX", 777)
        .await
        .expect("Obter mensagem UID 777");

    // Modo 1: txt auto (Markdown)
    let txt_auto = full_msg.txt_auto.expect("txt_auto presente");
    let rendered_markdown = parse_markdown_to_html(&txt_auto);
    assert!(
        rendered_markdown.contains("<h1>Relatorio Mensal</h1>"),
        "Markdown deve renderizar titulo H1"
    );
    assert!(
        rendered_markdown.contains("<li>Lucro: +35%</li>"),
        "Markdown deve renderizar lista de itens"
    );

    // Modo 2: txt original
    let txt_orig = full_msg.txt_original.expect("txt_original presente");
    assert!(txt_orig.contains("Lucro: +35%"));

    // Modo 3: html sanitizado
    let html_san = full_msg.html_sanitized.expect("html_sanitized presente");
    assert!(html_san.contains("papeli-sheet"));

    // F. Validação de UID Inexistente (404)
    let not_found_err = client
        .get_message(&token, "INBOX", 9999)
        .await
        .expect_err("UID 9999 deve retornar 404");
    assert!(matches!(
        not_found_err,
        lauda_client::domain::ports::ClientError::NotFound
    ));

    // G. Composição e Envio via /compose com Remetente Livre
    let compose = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "diretoria@2lp.in".to_string(),
        cc: None,
        subject: "Re: Relatorio Financeiro e Comunicado".to_string(),
        body: "Confirmado recebimento via Lauda smolclient.".to_string(),
        attachments: vec![],
    };
    client
        .send_message(&token, &compose)
        .await
        .expect("Envio de mensagem via /compose bem sucedido");

    assert_eq!(
        backend.sent_count(),
        1,
        "Mensagem enviada registrada no backend"
    );
}

/// 2. O Grande Ciclo Integrado (The Grand Sovereign Loop):
/// E-mail com papel de carta e banner de segurança -> Urupema (Sanitizer Crate) -> Lauda Server/Client
#[tokio::test]
async fn test_grand_sovereign_mail_pipeline_full_loop() {
    let papeli_html = r#"<div class="papeli-sheet">
      <h1>Lumen Pink</h1>
      <p>Proposta com link <a href="https://empresa.com/doc">ver documento</a> e <script>alert('xss')</script> vetor.</p>
      <div class="papeli-sig-long" data-papeli="signature">— Lumen</div>
    </div>"#;

    let security_banner = format!(
        "<div class=\"autocrypt-banner\">[Assinatura PGP Válida: lumen@2lp.in (Palavra Secreta: {SECURITY_WORD})]</div>"
    );
    let final_inbound_html = format!("{}\n{}", security_banner, papeli_html);

    // Passo: Higienização pelo Urupema e geração de txt_auto
    let inbound_raw = format!(
        "From: lumen@2lp.in\r\n\
To: bob@outro.org\r\n\
Subject: Proposta Estrategica Soberana\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
{}",
        final_inbound_html
    );

    let raw = RawEmail::new(inbound_raw.into_bytes());
    let mut urupema_sink = InMemorySink::new();
    let (sanitized, _) =
        Pipeline::process(&raw, "lumen@2lp.in", "bob@outro.org", &mut urupema_sink)
            .expect("Urupema processa e-mail higienizado");

    let parts = sanitized.parts;
    let txt_auto = parts.markdown_auto.expect("txt_auto Markdown gerado");
    let html_sanitized = parts.sanitized_html.expect("HTML higienizado gerado");

    // Validações de segurança do Urupema
    assert!(
        !html_sanitized.contains("<script>"),
        "Urupema deve purificar scripts dentro do layout envelopado"
    );
    assert!(
        html_sanitized.contains(SECURITY_WORD),
        "HTML higienizado deve manter a palavra secreta do banner"
    );
    assert!(
        txt_auto.contains("Proposta com link"),
        "txt_auto deve conter o texto estruturado"
    );

    // Passo: Entrega no Lauda Server e consumo no Lauda Smolclient
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("bob", "bob_senha_forte_2026");
    backend.add_mailbox(
        "bob",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );

    let uid = 101;
    backend.add_message(
        "bob",
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid,
                subject: "Proposta Estrategica Soberana".to_string(),
                sender: "lumen@2lp.in".to_string(),
                date: "2026-10-02T09:45:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: true,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some(txt_auto.clone()),
            txt_original: parts.original_plain,
            html_sanitized: Some(html_sanitized.clone()),
        },
    );

    let app = app_router(backend.clone(), TEST_SECRET);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", local_addr);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    wait_for_tcp_ready(local_addr).await;

    let client = HttpApiClient::new(base_url);
    let token = client
        .login(&LoginRequest {
            username: "bob".to_string(),
            password: "bob_senha_forte_2026".to_string(),
        })
        .await
        .unwrap()
        .token;

    let fetched = client.get_message(&token, "INBOX", uid).await.unwrap();

    // Renderização no smolclient via Dioxus/Markdown
    let client_rendered = parse_markdown_to_html(&fetched.txt_auto.unwrap());
    assert!(
        client_rendered.contains("Proposta com link"),
        "Cliente Lauda renderiza com sucesso o Markdown no visualizador txt auto"
    );
    assert!(
        !client_rendered.contains("<script>"),
        "Cliente Lauda seguro sem vulnerabilidade de XSS"
    );
}
