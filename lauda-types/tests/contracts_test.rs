use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse, ViewMode,
};

#[test]
fn test_login_request_roundtrip() {
    let req = LoginRequest {
        username: "lumen".to_string(),
        password: "secret_password".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let decoded: LoginRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req, decoded);
}

#[test]
fn test_token_response_roundtrip() {
    let resp = TokenResponse {
        token: "jwt.token.here".to_string(),
        expires_at: 1727800000,
        username: "lumen".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let decoded: TokenResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(resp, decoded);
}

#[test]
fn test_mailbox_info_roundtrip() {
    let mailbox = MailboxInfo {
        name: "INBOX".to_string(),
        total_messages: 42,
        unread_messages: 3,
    };
    let json = serde_json::to_string(&mailbox).unwrap();
    let decoded: MailboxInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(mailbox, decoded);
}

#[test]
fn test_message_envelope_roundtrip() {
    let envelope = MessageEnvelope {
        uid: 1042,
        subject: "Reunião de Alinhamento".to_string(),
        sender: "camarada@2lp.in".to_string(),
        date: "2026-10-01T20:00:00Z".to_string(),
        is_read: false,
        is_flagged: true,
        cc: None,
        has_attachments: false,
    };
    let json = serde_json::to_string(&envelope).unwrap();
    let decoded: MessageEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope, decoded);
}

#[test]
fn test_message_with_optional_bodies() {
    let msg = Message {
        envelope: MessageEnvelope {
            uid: 1043,
            subject: "Boletim Semanal".to_string(),
            sender: "news@2lp.in".to_string(),
            date: "2026-10-01T21:00:00Z".to_string(),
            is_read: true,
            is_flagged: false,
            cc: Some("equipe@2lp.in".to_string()),
            has_attachments: false,
        },
        txt_auto: Some("# Titulo\nTexto limpo.".to_string()),
        txt_original: Some("Titulo\nTexto limpo.".to_string()),
        html_sanitized: Some("<h1>Titulo</h1><p>Texto limpo.</p>".to_string()),
        attachments: vec![lauda_types::AttachmentMeta {
            id: 0,
            filename: "relatorio.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 1024,
        }],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, decoded);
}

#[test]
fn test_compose_form_roundtrip() {
    let form = ComposeForm {
        from: "lumen@2lp.in".to_string(),
        to: "destinatario@exemplo.org".to_string(),
        cc: Some("copia@exemplo.org".to_string()),
        subject: "Nova Mensagem".to_string(),
        body: "Texto digitado no smolclient.".to_string(),
        attachments: vec![lauda_types::AttachmentUpload {
            filename: "doc.txt".to_string(),
            content_type: "text/plain".to_string(),
            data_base64: "aGVsbG8=".to_string(),
        }],
    };
    let json = serde_json::to_string(&form).unwrap();
    let decoded: ComposeForm = serde_json::from_str(&json).unwrap();
    assert_eq!(form, decoded);
}

#[test]
fn test_contact_info_roundtrip() {
    let contact = lauda_types::ContactInfo {
        email: "alice@example.com".to_string(),
        name: Some("Alice".to_string()),
        message_count: 5,
        has_autocrypt: true,
        autocrypt_fingerprint: Some("ABCD 1234".to_string()),
    };
    let json = serde_json::to_string(&contact).unwrap();
    let decoded: lauda_types::ContactInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(contact, decoded);
}

#[test]
fn test_view_mode_serialization() {
    assert_eq!(
        serde_json::to_string(&ViewMode::TxtAuto).unwrap(),
        "\"txt_auto\""
    );
    assert_eq!(
        serde_json::to_string(&ViewMode::TxtOriginal).unwrap(),
        "\"txt_original\""
    );
    assert_eq!(serde_json::to_string(&ViewMode::Html).unwrap(), "\"html\"");
}

#[test]
fn test_view_mode_cycle() {
    assert_eq!(ViewMode::TxtAuto.next(), ViewMode::Html);
    assert_eq!(ViewMode::Html.next(), ViewMode::TxtOriginal);
    assert_eq!(ViewMode::TxtOriginal.next(), ViewMode::TxtAuto);

    assert_eq!(ViewMode::TxtAuto.label(), "TXT AUTO ⟳");
    assert_eq!(ViewMode::Html.label(), "HTML ⟳");
    assert_eq!(ViewMode::TxtOriginal.label(), "ORIGINAL ⟳");
}
