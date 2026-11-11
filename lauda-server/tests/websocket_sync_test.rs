use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{LoginRequest, MailboxInfo, TokenResponse};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::tungstenite::http::StatusCode as TungsteniteStatusCode;

const TEST_SECRET: &str = "segredo-super-secreto-websocket-test-123456";

fn setup_test_backend() -> Arc<InMemoryMailBackend> {
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
    backend.add_message(
        "lumen",
        "INBOX",
        lauda_types::Message {
            envelope: lauda_types::MessageEnvelope {
                uid: 42,
                subject: "Mensagem para teste de WebSocket".to_string(),
                sender: "remetente@2lp.in".to_string(),
                date: "2026-11-11T12:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: false,
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some("Conteúdo de teste".to_string()),
            txt_original: Some("Conteúdo de teste".to_string()),
            html_sanitized: None,
        },
    );
    backend
}

async fn spawn_test_server() -> (SocketAddr, String) {
    let backend = setup_test_backend();
    let app = app_router(backend, TEST_SECRET);

    // 1. Obtém token válido via login
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

    use tower::ServiceExt;
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&bytes).unwrap();
    let token = token_resp.token;

    // 2. Sobe o servidor HTTP num socket TCP efêmero local
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, token)
}

#[tokio::test]
async fn test_ws_handshake_rejects_missing_or_invalid_token() {
    let (addr, _valid_token) = spawn_test_server().await;

    // 1. Conexão sem token deve ser rejeitada com 401
    let url_no_token = format!("ws://{}/ws", addr);
    let err_no_token = tokio_tungstenite::connect_async(&url_no_token)
        .await
        .unwrap_err();
    match err_no_token {
        tokio_tungstenite::tungstenite::Error::Http(resp) => {
            assert_eq!(
                resp.status(),
                TungsteniteStatusCode::UNAUTHORIZED,
                "Handshake sem token deve responder 401 Unauthorized"
            );
        }
        other => panic!("Esperado erro HTTP 401, obtido: {:?}", other),
    }

    // 2. Conexão com token inválido deve ser rejeitada com 401
    let url_invalid = format!("ws://{}/ws?token=token_invalido_de_teste", addr);
    let err_invalid = tokio_tungstenite::connect_async(&url_invalid)
        .await
        .unwrap_err();
    match err_invalid {
        tokio_tungstenite::tungstenite::Error::Http(resp) => {
            assert_eq!(
                resp.status(),
                TungsteniteStatusCode::UNAUTHORIZED,
                "Handshake com token inválido deve responder 401 Unauthorized"
            );
        }
        other => panic!("Esperado erro HTTP 401, obtido: {:?}", other),
    }
}

#[tokio::test]
async fn test_ws_handshake_accepts_valid_token_and_receives_events() {
    let (addr, token) = spawn_test_server().await;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let (mut socket, response) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("conectar websocket com token valido");

    assert_eq!(
        response.status(),
        TungsteniteStatusCode::SWITCHING_PROTOCOLS,
        "Upgrade bem-sucedido deve retornar 101 Switching Protocols"
    );

    // Enviar mensagem Ping em formato de texto para checar reatividade
    socket
        .send(TungsteniteMessage::Text(r#"{"action":"ping"}"#.into()))
        .await
        .expect("enviar ping");

    let received = socket
        .next()
        .await
        .expect("receber resposta")
        .expect("resposta valida");

    match received {
        TungsteniteMessage::Text(text) => {
            assert!(
                text.contains("pong"),
                "Servidor deve responder ao comando de ping com evento pong"
            );
        }
        other => panic!("Esperada mensagem Text pong, recebido: {:?}", other),
    }

    // 2. Disparar ação HTTP de marcar mensagem como lida e checar difusão em tempo real
    let client = reqwest::Client::new();
    let patch_resp = client
        .patch(format!("http://{}/messages/42/read?read=true", addr))
        .bearer_auth(&token)
        .send()
        .await
        .expect("patch mark read");
    assert_eq!(patch_resp.status(), StatusCode::NO_CONTENT);

    // O socket DEVE receber o evento de read_update transmitido pelo servidor!
    let event_msg = socket
        .next()
        .await
        .expect("receber evento de broadcast")
        .expect("evento valido");
    match event_msg {
        TungsteniteMessage::Text(t) => {
            assert!(
                t.contains("read_update"),
                "Evento transmitido deve conter read_update: {}",
                t
            );
            assert!(
                t.contains("42"),
                "Evento transmitido deve conter o UID 42: {}",
                t
            );
        }
        other => panic!("Esperado evento de broadcast Text, obtido: {:?}", other),
    }
}

#[tokio::test]
async fn test_maildir_inotify_triggers_websocket_event() {
    let temp = tempfile::tempdir().expect("tempdir");
    let maildir_path = temp.path().join("maildir");
    let cur_dir = maildir_path.join("cur");
    let new_dir = maildir_path.join("new");
    std::fs::create_dir_all(&cur_dir).expect("criar cur");
    std::fs::create_dir_all(&new_dir).expect("criar new");

    let backend = Arc::new(lauda_server::adapters::MaildirBackend::new(&maildir_path));
    backend.add_user("lumen", "senha123");

    let (events_tx, _) = tokio::sync::broadcast::channel(128);
    let app =
        lauda_server::app_router_with_events_tx(backend.clone(), TEST_SECRET, events_tx.clone());

    // 1. Inicia watcher
    let _watcher_handle = backend
        .clone()
        .start_maildir_watcher(events_tx.clone())
        .expect("iniciar watcher");

    // 2. Obtém token e sobe servidor local
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

    use tower::ServiceExt;
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let token_resp: TokenResponse = serde_json::from_slice(&bytes).unwrap();
    let token = token_resp.token;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 3. Conecta WebSocket
    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let (mut socket, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("conectar websocket");

    // 4. Escreve arquivo novo em new/ simulando entrega do Postfix
    let raw_msg = "From: teste@2lp.in\r\nTo: hi@lumen.pink\r\nSubject: Novo Email via Inotify\r\n\r\nCorpo em tempo real";
    let file_path = new_dir.join("1731350000.M123456P1234.host,S=123");
    std::fs::write(&file_path, raw_msg).expect("gravar arquivo maildir");

    // 5. O WebSocket DEVE receber o evento mailbox_updated após o debounce!
    let timeout_res = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next()).await;
    let msg_res = timeout_res.expect("timeout aguardando evento do inotify");
    let msg = msg_res.expect("mensagem").expect("valida");

    match msg {
        TungsteniteMessage::Text(t) => {
            assert!(
                t.contains("mailbox_updated"),
                "Evento esperado: mailbox_updated, obtido: {}",
                t
            );
        }
        other => panic!("Esperado evento Text mailbox_updated, obtido: {:?}", other),
    }
}
