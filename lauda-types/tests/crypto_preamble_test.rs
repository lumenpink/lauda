use lauda_types::message::{
    SignatureStatus, clean_ishcondidim_preamble_html, clean_ishcondidim_preamble_text,
};

#[test]
fn test_clean_ishcondidim_preamble_text() {
    let raw_text = "┌── 🔒 [Ixcondidim: Mensagem PGP Decifrada pelo Servidor] ──────────────\n\
│ Palavra Secreta de Validação: [CENOURA_VERDE]\n\
│ Remetente: alice@example.com\n\
│ Assinatura Digital: VÁLIDA (Assinatura digital confirmada)\n\
│ ID da Chave: 0x9876543210ABCDEF\n\
└───────────────────────────────────────────────────────────────────────\n\n\
Olá Alice, este é o corpo legítimo da mensagem secreta!\n\
Até logo.";

    let (cleaned, meta) = clean_ishcondidim_preamble_text(raw_text);

    assert_eq!(
        cleaned,
        "Olá Alice, este é o corpo legítimo da mensagem secreta!\nAté logo."
    );

    let meta = meta.expect("Deveria ter extraído metadados do banner de texto");
    assert!(meta.is_encrypted);
    assert_eq!(meta.signature_status, SignatureStatus::Valid);
    assert_eq!(meta.signer_key_id.as_deref(), Some("0x9876543210ABCDEF"));
    assert_eq!(meta.verified_sender.as_deref(), Some("alice@example.com"));
    assert_eq!(meta.security_word.as_deref(), Some("CENOURA_VERDE"));
}

#[test]
fn test_clean_ishcondidim_preamble_html() {
    let raw_html = r#"<div style="border-left: 4px solid #10b981; background: #f0fdf4; padding: 12px 16px; margin: 0 0 16px 0;">
  <div style="font-weight: bold;">
    🔒 [Ixcondidim: Mensagem PGP Decifrada pelo Servidor]
  </div>
  <table>
    <tr><td>Palavra Secreta:</td><td><strong>[BATATA_DOCE]</strong></td></tr>
    <tr><td>Remetente:</td><td><code>bob@example.com</code></td></tr>
    <tr><td>Assinatura:</td><td><span>✗ INVÁLIDA (Falha na verificação da assinatura)</span></td></tr>
    <tr><td>ID da Chave:</td><td><code>0xDEADBEEFCAFE</code></td></tr>
  </table>
</div>
<p>Este é o parágrafo legítimo do HTML.</p>"#;

    let (cleaned, meta) = clean_ishcondidim_preamble_html(raw_html);

    assert_eq!(
        cleaned.trim(),
        "<p>Este é o parágrafo legítimo do HTML.</p>"
    );

    let meta = meta.expect("Deveria ter extraído metadados do banner HTML");
    assert!(meta.is_encrypted);
    assert_eq!(meta.signature_status, SignatureStatus::Invalid);
    assert_eq!(meta.signer_key_id.as_deref(), Some("0xDEADBEEFCAFE"));
    assert_eq!(meta.verified_sender.as_deref(), Some("bob@example.com"));
    assert_eq!(meta.security_word.as_deref(), Some("BATATA_DOCE"));
}
