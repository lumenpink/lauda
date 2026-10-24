use lauda_server::adapters::maildir::{
    MaildirBackend, dispatch_smtp_raw, extract_clean_email, extract_recipient_emails,
    normalize_smtp_data,
};
use lauda_server::domain::ports::MailBackend;
use lauda_types::ComposeForm;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

async fn run_mock_smtp_server(listener: TcpListener, fail_on_rcpt: bool) {
    if let Ok((mut socket, _)) = listener.accept().await {
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // 1. Banner
            writer
                .write_all(b"220 mock.smtp.local ESMTP Mock\r\n")
                .await
                .unwrap();

            // 2. Dialog Loop
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
                    if fail_on_rcpt {
                        writer
                            .write_all(b"550 5.1.1 Recipient rejected\r\n")
                            .await
                            .unwrap();
                    } else {
                        writer
                            .write_all(b"250 2.1.5 Recipient OK\r\n")
                            .await
                            .unwrap();
                    }
                } else if trimmed == "DATA" {
                    writer
                        .write_all(b"354 Start mail input; end with <CRLF>.<CRLF>\r\n")
                        .await
                        .unwrap();
                    // Read until single dot
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
async fn test_smtp_dispatch_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(run_mock_smtp_server(listener, false));

    let recipients = vec!["dest@2lp.in".to_string(), "cc@2lp.in".to_string()];
    let raw = "From: lumen@2lp.in\r\nTo: dest@2lp.in\r\nSubject: Test\r\n\r\nCorpo da mensagem";

    let res = dispatch_smtp_raw("127.0.0.1", port, "lumen@2lp.in", &recipients, raw).await;
    assert!(
        res.is_ok(),
        "Despacho SMTP deve suceder com mock ativo: {:?}",
        res
    );
}

#[tokio::test]
async fn test_smtp_dispatch_connection_refused() {
    let recipients = vec!["dest@2lp.in".to_string()];
    let raw = "From: lumen@2lp.in\r\nTo: dest@2lp.in\r\nSubject: Test\r\n\r\nCorpo";

    // Porta improvável de estar em uso
    let res = dispatch_smtp_raw("127.0.0.1", 1, "lumen@2lp.in", &recipients, raw).await;
    assert!(
        res.is_err(),
        "Despacho SMTP deve falhar quando servidor estiver offline"
    );
}

#[tokio::test]
async fn test_smtp_dispatch_recipient_rejected_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(run_mock_smtp_server(listener, true));

    let recipients = vec!["invalido@2lp.in".to_string()];
    let raw = "From: lumen@2lp.in\r\nTo: invalido@2lp.in\r\nSubject: Test\r\n\r\nCorpo";

    let res = dispatch_smtp_raw("127.0.0.1", port, "lumen@2lp.in", &recipients, raw).await;
    assert!(
        res.is_err(),
        "Despacho deve propagar erro se RCPT TO for recusado com 550"
    );
    let err_msg = res.err().unwrap();
    assert!(
        err_msg.contains("550"),
        "Mensagem de erro deve conter código 550: {}",
        err_msg
    );
}

#[test]
fn test_email_parsing_helpers() {
    assert_eq!(extract_clean_email("lumen@2lp.in"), "lumen@2lp.in");
    assert_eq!(extract_clean_email("Lumen <lumen@2lp.in>"), "lumen@2lp.in");
    assert_eq!(
        extract_clean_email("  <destino@exemplo.com>  "),
        "destino@exemplo.com"
    );

    let recs = extract_recipient_emails("lumen@2lp.in, Destinatário <dest@2lp.in>, copy@2lp.in");
    assert_eq!(recs, vec!["lumen@2lp.in", "dest@2lp.in", "copy@2lp.in"]);
}

#[test]
fn test_smtp_dot_stuffing_normalization() {
    let raw = "Linha 1\n.Linha que comeca com ponto\n..Duplo ponto\nNormal";
    let normalized = normalize_smtp_data(raw);
    assert_eq!(
        normalized,
        "Linha 1\r\n..Linha que comeca com ponto\r\n...Duplo ponto\r\nNormal\r\n"
    );
}

#[tokio::test]
async fn test_send_message_cleans_sent_file_on_dispatch_failure() {
    let tmp = tempdir().unwrap();
    let backend = MaildirBackend::new(tmp.path());

    // Configura SMTP para porta inválida e força despacho
    unsafe {
        std::env::set_var("LAUDA_FORCE_DISPATCH", "1");
        std::env::set_var("LAUDA_SMTP_HOST", "127.0.0.1");
        std::env::set_var("LAUDA_SMTP_PORT", "1"); // porta fechada
    }

    let form = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "dest@2lp.in".to_string(),
        cc: None,
        subject: "Falha intencional".to_string(),
        body: "Teste de limpeza".to_string(),
        attachments: vec![],
    };

    let result = backend.send_message("lumen", &form).await;

    // Remove variáveis para não afetar outros testes
    unsafe {
        std::env::remove_var("LAUDA_FORCE_DISPATCH");
        std::env::remove_var("LAUDA_SMTP_HOST");
        std::env::remove_var("LAUDA_SMTP_PORT");
    }

    assert!(
        result.is_err(),
        "send_message deve retornar Err quando o SMTP falha"
    );

    // Verifica que o diretório .Sent/cur não contém nenhum arquivo salvo
    let sent_cur = tmp.path().join(".Sent").join("cur");
    if sent_cur.exists() {
        let entries = std::fs::read_dir(sent_cur).unwrap();
        let count = entries.count();
        assert_eq!(
            count, 0,
            "Nenhum arquivo provisório deve permanecer em .Sent se o despacho falhar"
        );
    }
}
