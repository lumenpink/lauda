use lauda_server::adapters::maildir::MaildirBackend;
use lauda_server::domain::ports::MailBackend;
use lauda_types::ComposeForm;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

async fn run_mock_smtp_server(listener: TcpListener) {
    if let Ok((mut socket, _)) = listener.accept().await {
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            writer
                .write_all(b"220 mock.smtp.local ESMTP Mock\r\n")
                .await
                .unwrap();

            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                let trimmed = line.trim();
                if trimmed.starts_with("EHLO") || trimmed.starts_with("HELO") {
                    writer
                        .write_all(b"250-mock.smtp.local\r\n250 OK\r\n")
                        .await
                        .unwrap();
                } else if trimmed.starts_with("MAIL FROM:") {
                    writer.write_all(b"250 2.1.0 Sender OK\r\n").await.unwrap();
                } else if trimmed.starts_with("RCPT TO:") {
                    writer.write_all(b"250 2.1.5 Recipient OK\r\n").await.unwrap();
                } else if trimmed == "DATA" {
                    writer
                        .write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n")
                        .await
                        .unwrap();
                    let mut data_line = String::new();
                    while reader.read_line(&mut data_line).await.unwrap_or(0) > 0 {
                        if data_line.trim() == "." {
                            break;
                        }
                        data_line.clear();
                    }
                    writer
                        .write_all(b"250 2.0.0 OK: message queued\r\n")
                        .await
                        .unwrap();
                } else if trimmed == "QUIT" {
                    writer.write_all(b"221 2.0.0 Bye\r\n").await.unwrap();
                    break;
                } else {
                    writer
                        .write_all(b"500 5.5.1 Unrecognized command\r\n")
                        .await
                        .unwrap();
                }
                line.clear();
            }
        });
    }
}

#[tokio::test]
async fn test_send_message_delivers_to_ephemeral_mock_smtp() {
    let tmp = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(run_mock_smtp_server(listener));

    let backend = MaildirBackend::with_smtp_endpoint(tmp.path(), "127.0.0.1", port);

    let form = ComposeForm {
        from: "sender@example.test".to_string(),
        to: "recipient@example.test".to_string(),
        cc: None,
        subject: "Teste Ephemeral SMTP".to_string(),
        body: "Corpo do e-mail entregue com sucesso.".to_string(),
        attachments: vec![],
    };

    let result = backend.send_message("testuser", &form).await;
    assert!(
        result.is_ok(),
        "Envio de mensagem deve suceder contra mock SMTP efêmero: {:?}",
        result
    );
}

#[tokio::test]
async fn test_send_message_with_cc_and_attachments_to_ephemeral_mock_smtp() {
    let tmp = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(run_mock_smtp_server(listener));

    let backend = MaildirBackend::with_smtp_endpoint(tmp.path(), "127.0.0.1", port);

    let form = ComposeForm {
        from: "Alícia <alicia@example.org>".to_string(),
        to: "dest1@example.test, dest2@example.test".to_string(),
        cc: Some("cc1@example.test".to_string()),
        subject: "Teste com Anexo e Múltiplos Destinatários".to_string(),
        body: "Texto com anexo".to_string(),
        attachments: vec![lauda_types::AttachmentUpload {
            filename: "doc.txt".to_string(),
            content_type: "text/plain".to_string(),
            data_base64: "SGVsbG8gTGF1ZGEh".to_string(),
        }],
    };

    let result = backend.send_message("testuser", &form).await;
    assert!(
        result.is_ok(),
        "Envio com múltiplos destinatários deve suceder: {:?}",
        result
    );
}

#[tokio::test]
async fn test_message_id_domain_isolation() {
    let tmp = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(run_mock_smtp_server(listener));

    let backend = MaildirBackend::with_smtp_endpoint(tmp.path(), "127.0.0.1", port);

    let form = ComposeForm {
        from: "Usuario Isolado <usuario@empresa-teste.org>".to_string(),
        to: "destino@empresa-teste.org".to_string(),
        cc: None,
        subject: "Checagem de Message-ID".to_string(),
        body: "Verificando se o domínio não vaza".to_string(),
        attachments: vec![],
    };

    backend.send_message("testuser", &form).await.unwrap();

    let sent_cur = tmp.path().join(".Sent").join("cur");
    let mut files = std::fs::read_dir(sent_cur).unwrap();
    let entry = files.next().expect("Arquivo deve existir em .Sent").unwrap();
    let raw = std::fs::read_to_string(entry.path()).unwrap();

    assert!(
        raw.contains("Message-ID: <"),
        "Deve conter cabeçalho Message-ID"
    );
    assert!(
        raw.contains("@empresa-teste.org>"),
        "Message-ID deve usar o domínio do remetente e não vazar @2lp.in: {}",
        raw
    );
}
