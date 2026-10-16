use std::sync::Arc;

use base64::prelude::*;
use ishcondidim::autocrypt::AutocryptHeader;
use ishcondidim::config::Config as IshConfig;
use ishcondidim::crypto::{decrypt_message, generate_autocrypt_keypair, public_key_to_binary};
use ishcondidim::keystore::KeyStore;
use ishcondidim::mime::{
    build_security_banner_html, build_security_banner_text, encrypt_email_to_pgpmime,
};
use lauda_client::adapters::HttpApiClient;
use lauda_client::domain::ports::MailApiClient;
use lauda_client::domain::view::parse_markdown_to_html;
use lauda_server::adapters::in_memory::InMemoryMailBackend;
use lauda_server::app_router;
use lauda_types::{ComposeForm, LoginRequest, Message, MessageEnvelope};
use papeli::config::Config as PapeliConfig;
use papeli::mime::ParsedEmail;
use papeli::signatures::{collapse_historical_signatures_html, default_short_html};
use papeli::templates::TemplateEngine;
use urupema::adapters::InMemorySink;
use urupema::application::Pipeline;
use urupema::domain::RawEmail;

const TEST_SECRET: &str = "jwt-secret-full-chain-sovereign-mail-123456";
const SECURITY_WORD: &str = "CAFE_COM_PAO_DE_QUEIJO_SUL_MINAS_2026";

/// 1. Testes do Papéli: Envelopamento com papéis de carta anos 80, assinaturas e bypass
#[tokio::test]
async fn test_papeli_enveloping_themes_and_bypass() {
    let toml_config = r#"
[server]
listen = "unix:/run/papeli/papeli.sock"

[default]
header_title = "Lumen Pink"
company_name = "2LP"
template_strategy = "static"

[domains."2lp.in"]
header_title = "Lumen Pink Soberano"
company_name = "2LP"

[senders."diretoria@2lp.in"]
header_title = "Diretoria Executiva"
"#;

    let cfg = PapeliConfig::from_toml(toml_config).expect("Config do Papéli válida");

    // A. Remetente Gerenciado -> Envelopamento dinâmico (Tema Anos 80 Moranguinho)
    let managed_sender = "lumen@2lp.in";
    let resolved_managed = cfg.resolve(Some(managed_sender));
    assert!(
        resolved_managed.is_managed,
        "Remetente lumen@2lp.in deve ser marcado como gerenciado"
    );

    let raw_email = b"From: Lumen <lumen@2lp.in>\r\n\
To: contato@cliente.com\r\n\
Subject: [tema: moranguinho] Feliz Aniversario!\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Parabens pelo seu dia especial!";

    let parsed = ParsedEmail::parse(raw_email).expect("Deve parsear mensagem MIME no Papéli");
    assert_eq!(parsed.sender_email.as_deref(), Some("lumen@2lp.in"));

    let template_engine = TemplateEngine::new();
    let mjml_theme = template_engine.resolve_mjml_template("anos80_moranguinho", None);
    assert!(
        mjml_theme.contains("Moranguinho")
            || mjml_theme.contains("moranguinho")
            || mjml_theme.contains("mjml"),
        "Template MJML do tema deve ser resolvido"
    );

    let context = minijinja::context! {
        subject => "Feliz Aniversario!",
        header_title => resolved_managed.header_title,
        content_html => "<p>Parabens pelo seu dia especial!</p>",
        signature_html => "<div class=\"papeli-sig-long\" data-papeli=\"signature\">— Lumen (2LP)</div>"
    };

    let rendered_html = template_engine
        .render_html(&mjml_theme, &context)
        .expect("Renderização do template MJML em HTML deve suceder");
    assert!(
        rendered_html.contains("Parabens pelo seu dia especial!"),
        "HTML final deve conter o corpo da mensagem"
    );

    let reconstructed = parsed
        .reconstruct(
            "Parabens pelo seu dia especial!\n\n--\nLumen",
            &rendered_html,
        )
        .expect("MIME deve ser reconstruído com sucesso");

    let reconstructed_str = String::from_utf8_lossy(&reconstructed);
    assert!(
        reconstructed_str.contains("X-Papeli-Processed: true"),
        "Mensagem reconstruída deve receber header X-Papeli-Processed"
    );

    // B. Remetente Não Gerenciado -> Bypass / Ignore
    let unmanaged_sender = "estranho@dominioexterno.com";
    let resolved_unmanaged = cfg.resolve(Some(unmanaged_sender));
    assert!(
        !resolved_unmanaged.is_managed,
        "Remetente externo deve ser ignorado (is_managed = false)"
    );

    // C. Compactação de Assinaturas Antigas em Thread de Resposta
    let historical_html = r#"
<blockquote class="gmail_quote">
  <p>Mensagem anterior citada</p>
  <div class="papeli-sig-long" data-papeli="signature"><p>Assinatura Velha Gigante</p></div>
</blockquote>
"#;
    let fallback_short = default_short_html("Lumen", "2LP");
    let collapsed_html = collapse_historical_signatures_html(historical_html, &fallback_short);

    assert!(
        !collapsed_html.contains("Assinatura Velha Gigante"),
        "Assinatura dentro do blockquote deve ser compactada"
    );
    assert!(
        collapsed_html.contains("papeli-sig-short"),
        "Assinatura compactada deve conter classe papeli-sig-short"
    );
    assert!(
        collapsed_html.contains("— Lumen"),
        "Assinatura curta deve conter o nome formatado"
    );
}

/// 2. Testes do Ishcondidim: Autocrypt L1, Criptografia Oportunística, Decifragem Inbound e Anti-Lockout
#[tokio::test]
async fn test_ishcondidim_autocrypt_full_lifecycle() {
    let temp_dir = tempfile::tempdir().expect("Deve criar diretório temporário");
    let keys_dir = temp_dir.path().join("keys");
    let peers_dir = temp_dir.path().join("peers");
    let keystore = KeyStore::new(&keys_dir, &peers_dir).expect("Keystore deve inicializar");

    let is_config = r#"
[server]
listen = "unix:/run/ishcondidim/ishcondidim.sock"

[default]
prefer_encrypt = "mutual"
auto_generate_keys = true
security_word = "CAFE_COM_PAO_DE_QUEIJO_SUL_MINAS_2026"

[domains."2lp.in"]
prefer_encrypt = "mutual"
auto_generate_keys = true

[domains."externo.com"]
prefer_encrypt = "nopreference"
"#;
    let cfg = IshConfig::from_toml(is_config).expect("Config Ishcondidim válida");

    // A. Bootstrap de Chaves Ed25519/X25519 e Injeção do Cabeçalho Autocrypt
    let local_email = "lumen@2lp.in";
    let resolved_local = cfg.resolve(Some(local_email));
    assert!(
        resolved_local.is_managed,
        "lumen@2lp.in deve ser gerenciado"
    );

    let (lumen_sec, lumen_pub) = keystore
        .get_or_generate_keypair(local_email, true)
        .expect("Deve gerar chave local")
        .expect("Chave deve existir");

    let lumen_pub_bin = public_key_to_binary(&lumen_pub).expect("Chave binária");
    let lumen_keydata_b64 = BASE64_STANDARD.encode(&lumen_pub_bin);

    let autocrypt_header_val = format!(
        "addr={}; prefer-encrypt=mutual; keydata={}",
        local_email, lumen_keydata_b64
    );
    let parsed_header = AutocryptHeader::parse(&autocrypt_header_val).expect("Parse Autocrypt");
    assert_eq!(parsed_header.addr, local_email);
    assert_eq!(parsed_header.prefer_encrypt, "mutual");

    // B. Remetente Não Gerenciado -> Bypass
    let unmanaged = "invasor@alheio.com";
    let resolved_ext = cfg.resolve(Some(unmanaged));
    assert!(
        !resolved_ext.is_managed,
        "Domínio não cadastrado deve ser ignorado"
    );

    // C. Aprendizado de Pares (Peer Discovery) Inbound
    let alice_email = "alice@parceiro.com";
    let (alice_sec, alice_pub) =
        generate_autocrypt_keypair(alice_email).expect("Gera par da Alice");
    let alice_pub_bin = public_key_to_binary(&alice_pub).expect("Bin da Alice");
    let alice_b64 = BASE64_STANDARD.encode(&alice_pub_bin);

    let alice_header_str = format!(
        "addr={}; prefer-encrypt=mutual; keydata={}",
        alice_email, alice_b64
    );
    let alice_header = AutocryptHeader::parse(&alice_header_str).expect("Parse Alice");

    let now_ts = chrono::Utc::now().timestamp();
    keystore
        .update_peer_state(alice_email, Some(&alice_header), Some(now_ts))
        .expect("Atualiza peer state");

    let peer_state = keystore
        .get_peer_state(alice_email)
        .expect("Alice deve estar no keystore");
    assert!(peer_state.autocrypt_enabled);
    assert_eq!(peer_state.prefer_encrypt, "mutual");

    // D. Criptografia Oportunística Outbound (PGP/MIME RFC 3156)
    let outbound_clear = b"From: lumen@2lp.in\r\n\
To: alice@parceiro.com\r\n\
Subject: Documento Ultra Secreto\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Conteudo altamente sigiloso para a Alice.";

    let loaded_alice_pub = keystore
        .get_peer_public_key(alice_email)
        .expect("Chave da Alice encontrada");

    let pgp_mime_encrypted =
        encrypt_email_to_pgpmime(outbound_clear, &lumen_sec, &[&loaded_alice_pub])
            .expect("Cifragem PGP/MIME deve funcionar");

    let enc_str = String::from_utf8_lossy(&pgp_mime_encrypted);
    assert!(
        enc_str.contains("multipart/encrypted"),
        "Mensagem cifrada deve ser multipart/encrypted"
    );
    assert!(
        enc_str.contains("-----BEGIN PGP MESSAGE-----"),
        "Deve conter payload cifrado OpenPGP"
    );
    assert!(
        !enc_str.contains("Conteudo altamente sigiloso"),
        "Texto claro original não deve vazar"
    );

    // E. Decifragem Inbound & Selo com Palavra Secreta
    // Inbound: Alice envia e-mail cifrado para Lumen
    let alice_to_lumen = b"From: alice@parceiro.com\r\n\
To: lumen@2lp.in\r\n\
Subject: Resposta Cifrada da Alice\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Combinado! Aqui esta a senha do cofre: 987654.";

    let encrypted_for_lumen = encrypt_email_to_pgpmime(alice_to_lumen, &alice_sec, &[&lumen_pub])
        .expect("Alice cifra para Lumen");

    // Extrai o payload PGP
    let enc_str_lumen = String::from_utf8_lossy(&encrypted_for_lumen);
    let pgp_start = enc_str_lumen
        .find("-----BEGIN PGP MESSAGE-----")
        .expect("PGP start");
    let pgp_end = enc_str_lumen
        .find("-----END PGP MESSAGE-----")
        .expect("PGP end")
        + "-----END PGP MESSAGE-----".len();
    let pgp_payload = &enc_str_lumen[pgp_start..pgp_end];

    let decrypted = decrypt_message(pgp_payload.as_bytes(), &lumen_sec, None)
        .expect("Lumen decifra com sua chave privada");
    let decrypted_text = String::from_utf8_lossy(&decrypted.content);
    assert!(
        decrypted_text.contains("Combinado! Aqui esta a senha do cofre: 987654."),
        "Texto claro decifrado com fidelidade"
    );

    // Geração do banner de autenticidade com a palavra secreta
    let banner_html = build_security_banner_html(
        alice_email,
        decrypted.is_signed,
        decrypted.is_signature_valid,
        decrypted.signer_key_id.as_deref(),
        Some(SECURITY_WORD),
    );
    assert!(
        banner_html.contains(SECURITY_WORD),
        "Banner de segurança deve conter a Palavra Secreta"
    );
    assert!(
        banner_html.contains("Mensagem PGP Decifrada"),
        "Banner deve certificar a decifragem pelo servidor"
    );

    let banner_text = build_security_banner_text(
        alice_email,
        decrypted.is_signed,
        decrypted.is_signature_valid,
        decrypted.signer_key_id.as_deref(),
        Some(SECURITY_WORD),
    );
    assert!(
        banner_text.contains(SECURITY_WORD),
        "Banner em texto puro deve conter a Palavra Secreta"
    );

    // F. Prevenção Anti-Lockout (Downgrade para texto claro)
    // Se a Alice responder mais tarde sem cabeçalho Autocrypt:
    keystore
        .update_peer_state(alice_email, None, Some(now_ts + 500))
        .expect("Desativa Autocrypt no peer");

    let alice_updated = keystore.get_peer_state(alice_email).unwrap();
    assert!(
        !alice_updated.autocrypt_enabled,
        "Autocrypt deve ser desabilitado para evitar lockout"
    );
}

/// 3. Teste E2E Integrado com Socket TCP Real: Lauda Server <-> HttpApiClient <-> Trifocal & Compose
#[tokio::test]
async fn test_live_tcp_lauda_server_client_trifocal_and_compose() {
    let backend = Arc::new(InMemoryMailBackend::new());
    backend.add_user("lumen", "senha_super_secreta_99");
    backend.add_mailbox(
        "lumen",
        lauda_types::MailboxInfo {
            name: "INBOX".to_string(),
            total_messages: 1,
            unread_messages: 1,
        },
    );

    let sample_msg = Message {
        envelope: MessageEnvelope {
            uid: 777,
            subject: "Relatorio Financeiro e Comunicado".to_string(),
            sender: "diretoria@2lp.in".to_string(),
            date: "2026-10-02T09:30:00Z".to_string(),
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

/// 4. O Grande Ciclo Integrado (The Grand Sovereign Loop):
/// Papéli (MJML/Papel de Carta) -> Ishcondidim (Autocrypt) -> Urupema (Sanitizer) -> Lauda Server/Client
#[tokio::test]
async fn test_grand_sovereign_mail_pipeline_full_loop() {
    let temp_dir = tempfile::tempdir().expect("Diretório temporário");
    let keys_dir = temp_dir.path().join("keys");
    let peers_dir = temp_dir.path().join("peers");
    let keystore = KeyStore::new(&keys_dir, &peers_dir).expect("Keystore inicializada");

    // 1. Configuração do Remetente (Lumen) e Destinatário (Bob)
    let (lumen_sec, _lumen_pub) = keystore
        .get_or_generate_keypair("lumen@2lp.in", true)
        .expect("Gera chave Lumen")
        .unwrap();

    let (bob_sec, bob_pub) = generate_autocrypt_keypair("bob@outro.org").expect("Gera chave Bob");

    // Registra Bob no keystore como peer com Autocrypt ativo
    let bob_pub_bin = public_key_to_binary(&bob_pub).unwrap();
    let bob_b64 = BASE64_STANDARD.encode(&bob_pub_bin);
    let bob_header = AutocryptHeader::parse(&format!(
        "addr=bob@outro.org; prefer-encrypt=mutual; keydata={}",
        bob_b64
    ))
    .unwrap();
    keystore
        .update_peer_state(
            "bob@outro.org",
            Some(&bob_header),
            Some(chrono::Utc::now().timestamp()),
        )
        .unwrap();

    // 2. Passo 1 (Papéli): Envelopamento com papel de carta responsivo anos 80
    let template_engine = TemplateEngine::new();
    let mjml_theme = template_engine.resolve_mjml_template("anos80_rosinha_frufru", None);
    let context = minijinja::context! {
        subject => "Proposta Estrategica Soberana",
        header_title => "Lumen Pink",
        content_html => "<p>Proposta com link <a href=\"https://empresa.com/doc\">ver documento</a> e <script>alert('xss')</script> vetor.</p>",
        signature_html => "<div class=\"papeli-sig-long\" data-papeli=\"signature\">— Lumen</div>"
    };
    let papeli_html = template_engine.render_html(&mjml_theme, &context).unwrap();

    let raw_email = format!(
        "From: Lumen <lumen@2lp.in>\r\n\
To: bob@outro.org\r\n\
Subject: Proposta Estrategica Soberana\r\n\
MIME-Version: 1.0\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
{}",
        papeli_html
    );

    // 3. Passo 2 (Ishcondidim Outbound): Cifragem em PGP/MIME porque Bob é peer conhecido
    let bob_peer_pub = keystore.get_peer_public_key("bob@outro.org").unwrap();
    let pgp_mime_email =
        encrypt_email_to_pgpmime(raw_email.as_bytes(), &lumen_sec, &[&bob_peer_pub]).unwrap();

    let encrypted_str = String::from_utf8_lossy(&pgp_mime_email);
    assert!(encrypted_str.contains("multipart/encrypted"));

    // 4. Passo 3 (Ishcondidim Inbound): Bob recebe no seu MDA e decifra
    let start = encrypted_str.find("-----BEGIN PGP MESSAGE-----").unwrap();
    let end = encrypted_str.find("-----END PGP MESSAGE-----").unwrap()
        + "-----END PGP MESSAGE-----".len();
    let pgp_block = &encrypted_str[start..end];

    let decrypted_for_bob = decrypt_message(pgp_block.as_bytes(), &bob_sec, None).unwrap();
    let decrypted_html = String::from_utf8_lossy(&decrypted_for_bob.content);
    assert!(decrypted_html.contains("Proposta com link"));

    // Injeção do banner de segurança com palavra secreta
    let security_banner = build_security_banner_html(
        "lumen@2lp.in",
        decrypted_for_bob.is_signed,
        decrypted_for_bob.is_signature_valid,
        decrypted_for_bob.signer_key_id.as_deref(),
        Some(SECURITY_WORD),
    );
    let final_inbound_html = format!("{}\n{}", security_banner, decrypted_html);

    // 5. Passo 4 (Urupema): Higienização e geração de txt_auto
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

    // 6. Passo 5 (Lauda Server & Client): Entrega no backend e consumo no smolclient
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
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

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
