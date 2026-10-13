use lauda_types::message::{CryptoMeta, Message, MessageEnvelope, SignatureStatus};

#[test]
fn test_envelope_and_message_crypto_metadata() {
    let crypto = CryptoMeta {
        is_encrypted: true,
        signature_status: SignatureStatus::Valid,
        signer_key_id: Some("0x1234567890ABCDEF".to_string()),
        verified_sender: Some("alice@secmail.org".to_string()),
        security_word: Some("SEGREDO_TOP".to_string()),
    };

    let envelope = MessageEnvelope {
        uid: 999,
        subject: "Segredo de Estado".to_string(),
        sender: "alice@secmail.org".to_string(),
        date: "2026-10-12T12:00:00Z".to_string(),
        is_read: false,
        is_flagged: true,
        cc: None,
        has_attachments: false,
        crypto_meta: Some(crypto.clone()),
    };

    let msg = Message {
        envelope: envelope.clone(),
        txt_auto: Some("Conteúdo limpo".to_string()),
        txt_original: Some("Conteúdo limpo".to_string()),
        html_sanitized: Some("<p>Conteúdo limpo</p>".to_string()),
        attachments: vec![],
    };

    assert!(msg.envelope.crypto_meta.is_some());
    let extracted = msg.envelope.crypto_meta.unwrap();
    assert!(extracted.is_encrypted);
    assert_eq!(extracted.signature_status, SignatureStatus::Valid);
    assert_eq!(
        extracted.signer_key_id.as_deref(),
        Some("0x1234567890ABCDEF")
    );
    assert_eq!(
        extracted.verified_sender.as_deref(),
        Some("alice@secmail.org")
    );
    assert_eq!(extracted.security_word.as_deref(), Some("SEGREDO_TOP"));
}
