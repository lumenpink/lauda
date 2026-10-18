use lauda_client::adapters::in_memory::InMemoryApiClient;
use lauda_client::domain::ports::MailApiClient;
use lauda_client::domain::view::{
    ViewMode, clean_comment_tags, filter_aliases, parse_markdown_to_html,
};
use lauda_types::{ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope};

#[test]
fn test_clean_comment_tags() {
    let raw_long = "=== 2LP ===\n\nSegue contrato.\n\n--\n<!--papeli:sig:start:long-->\nLumen Pink\n2LP\n<!--papeli:sig:end:long-->";
    let cleaned = clean_comment_tags(raw_long);
    assert_eq!(
        cleaned,
        "=== 2LP ===\n\nSegue contrato.\n\n--\nLumen Pink\n2LP\n"
    );

    let raw_short = "<!--papeli:sig:start:short-->\n— Lumen (2LP)\n<!--papeli:sig:end:short-->";
    let cleaned_short = clean_comment_tags(raw_short);
    assert_eq!(cleaned_short, "— Lumen (2LP)\n");

    let raw_variant =
        "Mensagem <!--papeli-sig-start-long>comentario<!--papeli-sig-end-long> normal";
    let cleaned_variant = clean_comment_tags(raw_variant);
    assert_eq!(cleaned_variant, "Mensagem comentario normal");
}

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
fn test_markdown_preserves_single_line_breaks_and_paragraphs() {
    let md = "Linha 1\nLinha 2\nLinha 3\n\nOutro Paragrafo";
    let html = parse_markdown_to_html(md);

    // Linhas consecutivas em um mesmo bloco devem preservar a quebra visual com <br />
    assert!(
        html.contains("<br") || html.contains("<br/>") || html.contains("<br />"),
        "Linhas dentro de um bloco devem gerar quebra de linha visual: {}",
        html
    );
    // Parágrafos separados por linha em branco devem ser parágrafos distintos
    assert!(html.contains("<p>Outro Paragrafo</p>"));
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
                crypto_meta: None,
            },
            attachments: Vec::new(),
            txt_auto: Some("# Teste".to_string()),
            txt_original: Some("Teste".to_string()),
            html_sanitized: None,
        },
    );

    let list = client
        .list_messages(token, "INBOX", None, None)
        .await
        .unwrap();
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
    let list_after = client
        .list_messages(token, "INBOX", None, None)
        .await
        .unwrap();
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
                crypto_meta: None,
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
    let inbox_msgs = client
        .list_messages(token, "INBOX", None, None)
        .await
        .unwrap();
    assert!(inbox_msgs.is_empty());
    let archive_msgs = client
        .list_messages(token, "Archive", None, None)
        .await
        .unwrap();
    assert_eq!(archive_msgs.len(), 1);
    assert_eq!(archive_msgs[0].uid, 102);

    // 10. List Contacts
    let contacts = client.list_contacts(token).await.unwrap();
    assert_eq!(contacts.len(), 2);
    assert!(contacts.iter().any(|c| c.email == "colleague@2lp.in"));
    assert!(contacts.iter().any(|c| c.email == "a@b.com"));

    // 11. Search Messages
    let search_res = client
        .search_messages(token, "Archive", "relatorio")
        .await
        .unwrap();
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].uid, 102);
}

#[test]
fn test_fast_filter_envelopes_instant_and_accurate() {
    use lauda_client::domain::fast_filter_envelopes;
    let env1 = MessageEnvelope {
        uid: 1,
        subject: "Projeto Alfa 2026".to_string(),
        sender: "dev@2lp.in".to_string(),
        ..Default::default()
    };
    let env2 = MessageEnvelope {
        uid: 2,
        subject: "Relatório Financeiro".to_string(),
        sender: "finance@2lp.in".to_string(),
        ..Default::default()
    };
    let env3 = MessageEnvelope {
        uid: 3,
        subject: "Urgente: deploy".to_string(),
        sender: "alfa-lead@2lp.in".to_string(),
        ..Default::default()
    };

    let list = vec![env1, env2, env3];

    let res = fast_filter_envelopes(&list, "alfa");
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].uid, 1);
    assert_eq!(res[1].uid, 3);

    let empty = fast_filter_envelopes(&list, "inexistente");
    assert!(empty.is_empty());
}

#[test]
fn test_search_threshold_and_feedback_rules() {
    use lauda_client::domain::{
        MIN_SEARCH_QUERY_LEN, fast_filter_envelopes, is_search_active, should_show_search_hint,
    };

    assert_eq!(MIN_SEARCH_QUERY_LEN, 3);

    // Queries com menos de 3 caracteres não devem ativar a busca nem filtrar a listagem
    assert!(!is_search_active(""));
    assert!(!is_search_active("a"));
    assert!(!is_search_active("al"));
    assert!(!is_search_active("  al  "));

    // Queries com 3 ou mais caracteres ativam a busca
    assert!(is_search_active("pix"));
    assert!(is_search_active("PIX"));
    assert!(is_search_active("doc"));
    assert!(is_search_active("  dev  "));
    assert!(is_search_active("relatorio"));

    // O aviso visual (hint) de digitação mínima só aparece para 1 ou 2 caracteres
    assert!(!should_show_search_hint(""));
    assert!(!should_show_search_hint("   "));
    assert!(should_show_search_hint("a"));
    assert!(should_show_search_hint("al"));
    assert!(should_show_search_hint("  al  "));
    assert!(!should_show_search_hint("pix"));
    assert!(!should_show_search_hint("relatorio"));

    // Validação de fast_filter_envelopes com limiar
    let env1 = MessageEnvelope {
        uid: 1,
        subject: "PIX Recebido".to_string(),
        sender: "banco@2lp.in".to_string(),
        ..Default::default()
    };
    let env2 = MessageEnvelope {
        uid: 2,
        subject: "Relatório Mensal".to_string(),
        sender: "gestao@2lp.in".to_string(),
        ..Default::default()
    };
    let list = vec![env1, env2];

    // Queries curtas (< 3) NÃO filtram o vetor (mantêm a lista original intacta, evitando falso negativo)
    let under_threshold = fast_filter_envelopes(&list, "p");
    assert_eq!(
        under_threshold.len(),
        2,
        "1 caractere não deve esvaziar a lista"
    );

    let under_threshold2 = fast_filter_envelopes(&list, "pi");
    assert_eq!(
        under_threshold2.len(),
        2,
        "2 caracteres não devem esvaziar a lista"
    );

    // Query de 3 caracteres aciona o filtro
    let exact_threshold = fast_filter_envelopes(&list, "pix");
    assert_eq!(exact_threshold.len(), 1);
    assert_eq!(exact_threshold[0].uid, 1);
}

#[test]
fn test_format_search_hint_dynamic_countdown() {
    use lauda_client::domain::format_search_hint;

    // 0 caracteres ou espaços em branco -> None
    assert_eq!(format_search_hint(""), None);
    assert_eq!(format_search_hint("   "), None);

    // 1 caractere digitado -> faltam 2
    assert_eq!(
        format_search_hint("a"),
        Some("FALTAM 2 CARACTERES PARA PESQUISAR...".to_string())
    );
    assert_eq!(
        format_search_hint("  p  "),
        Some("FALTAM 2 CARACTERES PARA PESQUISAR...".to_string())
    );

    // 2 caracteres digitados -> falta 1 (singular)
    assert_eq!(
        format_search_hint("ab"),
        Some("FALTA 1 CARACTERE PARA PESQUISAR...".to_string())
    );
    assert_eq!(
        format_search_hint("  pi "),
        Some("FALTA 1 CARACTERE PARA PESQUISAR...".to_string())
    );

    // 3 ou mais caracteres -> None (busca ativa)
    assert_eq!(format_search_hint("abc"), None);
    assert_eq!(format_search_hint("pix"), None);
    assert_eq!(format_search_hint("relatorio"), None);
}

#[tokio::test]
async fn test_client_offline_index_slice_and_hybrid_search() {
    use lauda_client::domain::LocalMessageCache;
    use lauda_index::{MessageIndex, SimpleCpuEmbeddingProvider};
    use tempfile::tempdir;

    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::open(temp.path()).expect("abrir cache local");

    // Cria documento indexado simulado vindo do servidor
    let mut server_index = MessageIndex::new();
    let provider = SimpleCpuEmbeddingProvider;

    let msg = Message {
        envelope: MessageEnvelope {
            uid: 777,
            subject: "Comprovante PIX Salário".to_string(),
            sender: "empresa@rh.com".to_string(),
            date: "2026-10-15T12:00:00Z".to_string(),
            cc: None,
            has_attachments: false,
            is_read: true,
            is_flagged: false,
            crypto_meta: None,
        },
        attachments: vec![],
        txt_auto: Some("Depósito PIX efetuado com sucesso.".to_string()),
        txt_original: None,
        html_sanitized: None,
    };

    server_index.index_message_semantic("INBOX", &msg, &provider);
    let docs: Vec<_> = server_index.documents().values().cloned().collect();

    // 1. Salvar fatia de 90 dias no SQLite do cliente
    cache
        .save_index_slice("INBOX", &docs)
        .expect("salvar fatia local");

    // 2. Carregar fatia offline e executar busca híbrida
    let local_index = cache
        .load_index_slice("INBOX")
        .expect("carregar fatia local")
        .expect("indice deve existir");

    let results = local_index.search_hybrid("pix", &provider, 0.5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uid, 777);
    assert!(results[0].semantic_score > 0.0);
}

#[tokio::test]
async fn test_client_attachment_save_and_status_formatting() {
    use lauda_client::domain::view::{
        AttachmentSaveState, format_attachment_status, save_attachment_bytes,
    };
    use tempfile::tempdir;

    let temp = tempdir().expect("tempdir");
    let target = temp.path().join("documento.pdf");

    // 1. Estados e formatação de feedback explicativo
    let s_downloading = format_attachment_status("documento.pdf", AttachmentSaveState::Downloading);
    assert_eq!(s_downloading, "⌛ BAIXANDO ANEXO: documento.pdf...");

    let s_saving = format_attachment_status("documento.pdf", AttachmentSaveState::Saving);
    assert_eq!(s_saving, "💾 SALVANDO EM DISCO: documento.pdf...");

    let s_saved = format_attachment_status(
        "documento.pdf",
        AttachmentSaveState::Saved(target.display().to_string()),
    );
    assert!(s_saved.starts_with("✓ Salvo em "));

    let s_failed = format_attachment_status(
        "documento.pdf",
        AttachmentSaveState::Failed("Sem permissão".to_string()),
    );
    assert_eq!(s_failed, "Falha no salvamento: Sem permissão");

    // 2. Gravação assíncrona desacoplada de bytes
    let payload = b"%PDF-1.4 Relatorio Teste".to_vec();
    let res = save_attachment_bytes(&target, &payload).await;
    assert!(res.is_ok());
    assert!(target.exists());
    let read_back = std::fs::read(&target).expect("ler arquivo gravado");
    assert_eq!(read_back, payload);
}

#[tokio::test]
async fn test_client_async_contact_search_non_blocking() {
    use lauda_client::domain::contacts_cache::LocalContactsCache;
    use lauda_client::domain::view::{search_contacts_async, search_contacts_fuzzy_async};
    use tempfile::tempdir;

    let temp = tempdir().expect("tempdir");
    let cache = LocalContactsCache::open(temp.path()).expect("abrir cache contatos");

    let c1 = lauda_client::domain::contacts_cache::create_local_contact(
        "maria",
        Some("Maria Silva"),
        "maria@empresa.com",
        None,
    );
    cache
        .apply_sync_delta(std::slice::from_ref(&c1), &[], "tok:1")
        .expect("gravar contato");

    // 1. Busca assíncrona desacoplada da UI
    let results = search_contacts_async(temp.path().to_path_buf(), "maria".to_string(), 5).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].primary_email, "maria@empresa.com");

    // 2. Busca fuzzy assíncrona desacoplada da UI
    let fuzzy_results =
        search_contacts_fuzzy_async(temp.path().to_path_buf(), "marya".to_string(), 5).await;
    assert_eq!(fuzzy_results.len(), 1);
    assert_eq!(fuzzy_results[0].id, c1.id);
}

#[test]
fn test_session_default_server_url_mutation_resilience() {
    use lauda_client::domain::session::{SessionConfig, detect_default_server_url};

    // 1. Salvar estado original de LAUDA_SERVER_URL
    let original = std::env::var("LAUDA_SERVER_URL").ok();

    // 2. Com LAUDA_SERVER_URL definida, deve respeitar estritamente a variável
    unsafe {
        std::env::set_var("LAUDA_SERVER_URL", "https://mail.example.org:9000");
    }
    assert_eq!(detect_default_server_url(), "https://mail.example.org:9000");

    // 3. Com LAUDA_SERVER_URL removida, deve retornar deterministicamente http://localhost:8432
    unsafe {
        std::env::remove_var("LAUDA_SERVER_URL");
    }
    assert_eq!(detect_default_server_url(), "http://localhost:8432");

    let cfg = SessionConfig::default();
    assert_eq!(cfg.server_url, "http://localhost:8432");
    assert!(
        !cfg.server_url.contains("lohn.in"),
        "Nao deve conter dominio fixo de producao"
    );
    assert_eq!(cfg.server_url, detect_default_server_url());

    // 4. Restaurar estado original
    if let Some(val) = original {
        unsafe {
            std::env::set_var("LAUDA_SERVER_URL", val);
        }
    }
}
