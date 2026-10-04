use std::sync::Arc;

use base64::prelude::*;
use ishcondidim::autocrypt::{AutocryptError, AutocryptHeader};
use ishcondidim::crypto::{
    decrypt_message, generate_autocrypt_keypair, public_key_to_binary, sign_and_encrypt,
};
use ishcondidim::keystore::KeyStore;
use jsonwebtoken::{EncodingKey, Header, encode};
use lauda_client::adapters::HttpApiClient;
use lauda_client::domain::ports::{ClientError, MailApiClient};
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::LoginRequest;
use papeli::sanitizer::sanitize_html;
use papeli::templates::{TemplateEngine, canonical_theme_name, extract_theme_tag};
use serde::{Deserialize, Serialize};
use urupema::adapters::InMemorySink;
use urupema::application::Pipeline;
use urupema::domain::{ProcessResult, RawEmail};

const TEST_SECRET: &str = "jwt-secret-resilience-testing-sovereign-mail-999";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

// ---------------------------------------------------------------------------
// 1. Autocrypt: Rotação de Chaves e Defesa Anti-Replay
// ---------------------------------------------------------------------------
#[test]
fn test_autocrypt_key_rotation_and_anti_replay() {
    let temp_dir = tempfile::tempdir().expect("Diretório temporário");
    let keys_dir = temp_dir.path().join("keys");
    let peers_dir = temp_dir.path().join("peers");
    let keystore = KeyStore::new(&keys_dir, &peers_dir).expect("Keystore inicializada");

    let alice_email = "alice@empresa.com";

    // 1. Alice publica Chave A em t = 1000
    let (_alice_sec_a, alice_pub_a) =
        generate_autocrypt_keypair(alice_email).expect("Gera Chave A");
    let alice_a_b64 = BASE64_STANDARD.encode(public_key_to_binary(&alice_pub_a).unwrap());
    let header_a = AutocryptHeader::parse(&format!(
        "addr={}; prefer-encrypt=mutual; keydata={}",
        alice_email, alice_a_b64
    ))
    .unwrap();

    keystore
        .update_peer_state(alice_email, Some(&header_a), Some(1000))
        .unwrap();

    let state_a = keystore.get_peer_state(alice_email).unwrap();
    assert_eq!(state_a.effective_date, Some(1000));
    assert!(state_a.autocrypt_enabled);

    // 2. Alice rotaciona para Chave B em t = 2000
    let (_alice_sec_b, alice_pub_b) =
        generate_autocrypt_keypair(alice_email).expect("Gera Chave B");
    let alice_b_b64 = BASE64_STANDARD.encode(public_key_to_binary(&alice_pub_b).unwrap());
    let header_b = AutocryptHeader::parse(&format!(
        "addr={}; prefer-encrypt=mutual; keydata={}",
        alice_email, alice_b_b64
    ))
    .unwrap();

    keystore
        .update_peer_state(alice_email, Some(&header_b), Some(2000))
        .unwrap();

    let state_b = keystore.get_peer_state(alice_email).unwrap();
    assert_eq!(state_b.effective_date, Some(2000));
    let current_key_bin = public_key_to_binary(
        &keystore
            .get_peer_public_key(alice_email)
            .expect("Chave B ativa"),
    )
    .unwrap();
    assert_eq!(
        current_key_bin,
        public_key_to_binary(&alice_pub_b).unwrap(),
        "A chave ativa deve ser a Chave B mais recente"
    );

    // 3. Ataque de Replay: Mensagem antiga entregue fora de ordem com Chave A em t = 500
    keystore
        .update_peer_state(alice_email, Some(&header_a), Some(500))
        .unwrap();

    let state_after_replay = keystore.get_peer_state(alice_email).unwrap();
    assert_eq!(
        state_after_replay.effective_date,
        Some(2000),
        "A data efetiva NÃO deve sofrer rollback por mensagem antiga"
    );
    let key_after_replay_bin = public_key_to_binary(
        &keystore
            .get_peer_public_key(alice_email)
            .expect("Chave permanece B"),
    )
    .unwrap();
    assert_eq!(
        key_after_replay_bin,
        public_key_to_binary(&alice_pub_b).unwrap(),
        "A chave ativa deve permanecer estritamente a Chave B"
    );
}

// ---------------------------------------------------------------------------
// 2. Autocrypt: Múltiplos Remetentes e Aliases Independentes
// ---------------------------------------------------------------------------
#[test]
fn test_multiple_sender_aliases_independent_keys() {
    let temp_dir = tempfile::tempdir().expect("Diretório temporário");
    let keys_dir = temp_dir.path().join("keys");
    let peers_dir = temp_dir.path().join("peers");
    let keystore = KeyStore::new(&keys_dir, &peers_dir).expect("Keystore inicializada");

    let aliases = ["lumen@2lp.in", "contato@2lp.in", "postmaster@2lp.in"];

    let mut pub_keys = Vec::new();

    for &alias in &aliases {
        let (sec, pub_k) = keystore
            .get_or_generate_keypair(alias, true)
            .unwrap()
            .unwrap();

        let bin = public_key_to_binary(&pub_k).unwrap();
        pub_keys.push(bin);

        // Verifica que o arquivo .sec e .pub foram criados
        let loaded = keystore.get_local_keypair(alias).unwrap().unwrap();
        assert_eq!(
            public_key_to_binary(&loaded.1).unwrap(),
            public_key_to_binary(&sec.to_public_key()).unwrap()
        );
    }

    // Garante que todas as chaves geradas para cada alias são mutuamente exclusivas e únicas
    assert_ne!(pub_keys[0], pub_keys[1]);
    assert_ne!(pub_keys[1], pub_keys[2]);
    assert_ne!(pub_keys[0], pub_keys[2]);
}

// ---------------------------------------------------------------------------
// 3. Autocrypt: Simulação de Erros, Cabeçalhos Malformados e Payloads Adulterados
// ---------------------------------------------------------------------------
#[test]
fn test_autocrypt_error_simulation_and_tampered_payloads() {
    // A. Atributos ausentes no cabeçalho
    let missing_addr = "prefer-encrypt=mutual; keydata=AAAA";
    let err_addr = AutocryptHeader::parse(missing_addr).expect_err("Deve falhar sem addr");
    assert!(matches!(err_addr, AutocryptError::MissingAttribute(ref a) if a == "addr"));

    let missing_keydata = "addr=lumen@2lp.in; prefer-encrypt=mutual";
    let err_key = AutocryptHeader::parse(missing_keydata).expect_err("Deve falhar sem keydata");
    assert!(matches!(err_key, AutocryptError::MissingAttribute(ref k) if k == "keydata"));

    // B. Base64 corrompido
    let bad_b64 = "addr=lumen@2lp.in; prefer-encrypt=mutual; keydata=%%INVALID_BASE64%%";
    let err_b64 = AutocryptHeader::parse(bad_b64).expect_err("Deve falhar com base64 inválido");
    assert!(matches!(err_b64, AutocryptError::Base64Error(_)));

    // C. Payload PGP adulterado (tampered ciphertext)
    let (sec_sender, _pub_sender) = generate_autocrypt_keypair("remetente@2lp.in").unwrap();
    let (_sec_recip, pub_recip) = generate_autocrypt_keypair("destinatario@2lp.in").unwrap();
    let (sec_wrong, _pub_wrong) = generate_autocrypt_keypair("intruso@outro.com").unwrap();

    let plaintext = b"Mensagem secreta autentica.";
    let encrypted = sign_and_encrypt(plaintext, &sec_sender, &[&pub_recip]).unwrap();

    // 1. Tentar decifrar com a chave errada deve falhar graciosamente
    let dec_err = decrypt_message(encrypted.as_bytes(), &sec_wrong, None);
    assert!(dec_err.is_err(), "Decifragem com chave errada deve falhar");

    // 2. Adulteração de bytes do ciphertext
    let mut tampered = encrypted.clone().into_bytes();
    if tampered.len() > 100 {
        tampered[80] ^= 0xFF; // Inverte bits no meio da mensagem
        tampered[81] ^= 0xAA;
    }
    let tampered_err = decrypt_message(&tampered, &sec_sender, None);
    assert!(
        tampered_err.is_err(),
        "Ciphertext adulterado deve ser rejeitado com erro"
    );
}

// ---------------------------------------------------------------------------
// 4. Papéli: Resiliência contra HTML Imperfeito, Tags Quebradas e Fallback de Temas
// ---------------------------------------------------------------------------
#[test]
fn test_papeli_resilience_broken_html_and_theme_fallback() {
    // A. Fragmento de HTML quebrado/imperfeito com tags não fechadas
    let broken_html = r#"
<div class="header">
  <p>Comunicado importante sem fechar tags
  <b>Negrito esquecido
  <table border="1">
    <tr><td>Celula solta
"#;
    let sanitized = sanitize_html(broken_html);
    assert!(
        sanitized.contains("Comunicado importante"),
        "O texto útil deve ser recuperado"
    );
    assert!(
        sanitized.contains("<b>") && sanitized.contains("</b>"),
        "Sanitizador deve fechar tags desbalanceadas"
    );

    // B. Fallback limpo de temas dinâmicos desconhecidos
    let (clean_subject, chosen_theme) =
        extract_theme_tag("[tema: cyberpunk_inexistente_2099] Reunião de Pauta");
    assert_eq!(clean_subject, "Reunião de Pauta");
    assert_eq!(chosen_theme.as_deref(), Some("cyberpunk_inexistente_2099"));

    // O resolvedor de tema deve fazer fallback para o padrão seguro
    let engine = TemplateEngine::new();
    let template = engine.resolve_mjml_template("cyberpunk_inexistente_2099", None);
    assert!(
        template.contains("<mjml>") && template.contains("papeli-sheet"),
        "Tema desconhecido deve recorrer com segurança ao template padrão"
    );

    // C. Mapeamento canônico de apelidos
    assert_eq!(canonical_theme_name("frufru"), "anos80_rosinha_frufru");
    assert_eq!(canonical_theme_name("morango"), "anos80_moranguinho");
    assert_eq!(canonical_theme_name("desconhecido"), "default");
}

// ---------------------------------------------------------------------------
// 5. Urupema: Vetores Avançados de Evasão XSS e Prevenção Rígida de Loops
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
// 6. Lauda Gateway: Rejeição de Tokens Forjados, Assinaturas Falsas e Expiradas
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
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

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
