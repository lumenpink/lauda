use lauda_client::adapters::in_memory::InMemoryApiClient;
use lauda_client::domain::ports::MailApiClient;
use lauda_client::domain::view::{ViewMode, filter_aliases, parse_markdown_to_html};
use lauda_types::{ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope};

#[test]
fn test_markdown_parser_conversion() {
    let md = "# Titulo Principal\n\nEste e um paragrafo com [link](https://2lp.in) e `codigo`.";
    let html = parse_markdown_to_html(md);

    assert!(html.contains("<h1>Titulo Principal</h1>"));
    assert!(html.contains("<p>Este e um paragrafo"));
    assert!(html.contains("<a href=\"https://2lp.in\">link</a>"));
    assert!(html.contains("<code>codigo</code>"));
}

#[test]
fn test_view_mode_switching() {
    let mut mode = ViewMode::TxtAuto;
    assert_eq!(mode, ViewMode::TxtAuto);
    assert_eq!(mode.label(), "TXT AUTO ⟳");

    mode = mode.next();
    assert_eq!(mode, ViewMode::Html);
    assert_eq!(mode.label(), "HTML ⟳");

    mode = mode.next();
    assert_eq!(mode, ViewMode::TxtOriginal);
    assert_eq!(mode.label(), "ORIGINAL ⟳");

    mode = mode.next();
    assert_eq!(mode, ViewMode::TxtAuto);
}

#[test]
fn test_alias_filtering() {
    let aliases = vec![
        "lumen@2lp.in".to_string(),
        "contato@2lp.in".to_string(),
        "postmaster@2lp.in".to_string(),
    ];

    let matches = filter_aliases(&aliases, "con");
    assert_eq!(matches, vec!["contato@2lp.in"]);

    let matches_empty = filter_aliases(&aliases, "");
    assert_eq!(matches_empty.len(), 3);
}

#[tokio::test]
async fn test_in_memory_api_client() {
    let client = InMemoryApiClient::new();

    // 1. Login
    let login_resp = client
        .login(&LoginRequest {
            username: "lumen".to_string(),
            password: "secret".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(login_resp.username, "lumen");
    let token = &login_resp.token;

    // 2. Mailboxes
    client.add_mailbox(MailboxInfo {
        name: "INBOX".to_string(),
        total_messages: 1,
        unread_messages: 1,
    });
    let boxes = client.list_mailboxes(token).await.unwrap();
    assert_eq!(boxes.len(), 1);

    // 3. Messages
    client.add_message(
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 101,
                subject: "Teste".to_string(),
                sender: "a@b.com".to_string(),
                date: "2026-10-02T06:00:00Z".to_string(),
                cc: None,
                has_attachments: false,
                is_read: false,
                is_flagged: false,
            },
            attachments: Vec::new(),
            txt_auto: Some("# Teste".to_string()),
            txt_original: Some("Teste".to_string()),
            html_sanitized: None,
        },
    );

    let list = client.list_messages(token, "INBOX").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].uid, 101);

    let msg = client.get_message(token, "INBOX", 101).await.unwrap();
    assert_eq!(msg.envelope.subject, "Teste");

    // 4. Send
    let result = client
        .send_message(
            token,
            &ComposeForm {
                from: "lumen@2lp.in".to_string(),
                to: "dest@test.com".to_string(),
                cc: None,
                subject: "Envio".to_string(),
                body: "Corpo".to_string(),
                attachments: vec![],
            },
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(client.sent_count(), 1);

    // 5. Mark Read
    let mark_res = client.mark_read(token, "INBOX", 101, true).await;
    assert!(mark_res.is_ok());
    let updated = client.get_message(token, "INBOX", 101).await.unwrap();
    assert!(updated.envelope.is_read);

    // 6. Delete Message
    let del_res = client.delete_message(token, "INBOX", 101).await;
    assert!(del_res.is_ok());
    let list_after = client.list_messages(token, "INBOX").await.unwrap();
    assert!(list_after.is_empty());

    // 7. Flagging
    client.add_message(
        "INBOX",
        Message {
            envelope: MessageEnvelope {
                uid: 102,
                subject: "Mensagem com Anexo".to_string(),
                sender: "colleague@2lp.in".to_string(),
                date: "2026-10-06T10:00:00Z".to_string(),
                cc: Some("team@2lp.in".to_string()),
                has_attachments: true,
                is_read: false,
                is_flagged: false,
            },
            attachments: vec![lauda_types::AttachmentMeta {
                id: 0,
                filename: "relatorio.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 1024,
            }],
            txt_auto: Some("Veja o anexo.".to_string()),
            txt_original: Some("Veja o anexo.".to_string()),
            html_sanitized: None,
        },
    );

    let flag_res = client.mark_flagged(token, "INBOX", 102, true).await;
    assert!(flag_res.is_ok());
    let flagged_msg = client.get_message(token, "INBOX", 102).await.unwrap();
    assert!(flagged_msg.envelope.is_flagged);
    assert_eq!(flagged_msg.envelope.cc.as_deref(), Some("team@2lp.in"));

    // 8. Attachment Fetch
    let (att_name, att_type, att_bytes) = client
        .fetch_attachment(token, "INBOX", 102, 0)
        .await
        .unwrap();
    assert_eq!(att_name, "relatorio.pdf");
    assert_eq!(att_type, "application/pdf");
    assert!(!att_bytes.is_empty());

    // 9. Move Message
    let move_res = client.move_message(token, 102, "INBOX", "Archive").await;
    assert!(move_res.is_ok());
    let inbox_msgs = client.list_messages(token, "INBOX").await.unwrap();
    assert!(inbox_msgs.is_empty());
    let archive_msgs = client.list_messages(token, "Archive").await.unwrap();
    assert_eq!(archive_msgs.len(), 1);
    assert_eq!(archive_msgs[0].uid, 102);

    // 10. List Contacts
    let contacts = client.list_contacts(token).await.unwrap();
    assert_eq!(contacts.len(), 2);
    assert!(contacts.iter().any(|c| c.email == "colleague@2lp.in"));
    assert!(contacts.iter().any(|c| c.email == "a@b.com"));
}
