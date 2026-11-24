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
    assert_eq!(mode.label(), "SIMPLIFICADO ⟳");

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
            client_type: None,
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
                bcc: None,
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
    assert_eq!(search_res.messages.len(), 1);
    assert_eq!(search_res.messages[0].uid, 102);
    assert!(search_res.sync_status.is_synced);
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
async fn test_client_offline_index_slice_and_bm25_search() {
    use lauda_client::domain::LocalMessageCache;
    use lauda_index::MessageIndex;
    use tempfile::tempdir;

    let temp = tempdir().expect("tempdir");
    let cache = LocalMessageCache::open(temp.path()).expect("abrir cache local");

    // Cria documento indexado simulado vindo do servidor
    let mut server_index = MessageIndex::new();

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

    server_index.index_message("INBOX", &msg);
    let docs: Vec<_> = server_index.documents().values().cloned().collect();

    // 1. Salvar fatia de 90 dias no SQLite do cliente
    cache
        .save_index_slice("INBOX", &docs)
        .expect("salvar fatia local");

    // 2. Carregar fatia offline e executar busca BM25
    let local_index = cache
        .load_index_slice("INBOX")
        .expect("carregar fatia local")
        .expect("indice deve existir");

    let results = local_index.search("pix");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], 777);
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

#[test]
fn test_view_mode_default_and_serde_mutation_coverage() {
    // 1. O modo padrão universal deve ser impreterivelmente TxtAuto (SIMPLIFICADO)
    let default_mode = ViewMode::default();
    assert_eq!(default_mode, ViewMode::TxtAuto);
    assert_eq!(default_mode.label(), "SIMPLIFICADO ⟳");

    // 2. Cobertura de mutação do ciclo de estados
    let first = ViewMode::TxtAuto;
    let second = first.next();
    let third = second.next();
    let loop_back = third.next();
    assert_eq!(second, ViewMode::Html);
    assert_eq!(third, ViewMode::TxtOriginal);
    assert_eq!(loop_back, ViewMode::TxtAuto);

    // 3. Desserialização serde de todas as variantes snake_case
    let parsed_auto: ViewMode = serde_json::from_str("\"txt_auto\"").expect("txt_auto valido");
    let parsed_html: ViewMode = serde_json::from_str("\"html\"").expect("html valido");
    let parsed_orig: ViewMode =
        serde_json::from_str("\"txt_original\"").expect("txt_original valido");
    assert_eq!(parsed_auto, ViewMode::TxtAuto);
    assert_eq!(parsed_html, ViewMode::Html);
    assert_eq!(parsed_orig, ViewMode::TxtOriginal);

    // 4. Rejeição de valores espúrios
    let invalid = serde_json::from_str::<ViewMode>("\"invalido\"");
    assert!(invalid.is_err(), "Valor inexistente deve falhar no parse");
}

#[test]
fn test_find_neighbor_uids_navigation_contract() {
    use lauda_client::domain::view::find_neighbor_uids;

    let envs = vec![
        MessageEnvelope {
            uid: 300,
            subject: "Msg 300".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 200,
            subject: "Msg 200".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 100,
            subject: "Msg 100".to_string(),
            ..Default::default()
        },
    ];

    // Elemento intermediário (uid 200): anterior é 300, próximo é 100
    let (prev, next) = find_neighbor_uids(&envs, 200);
    assert_eq!(prev, Some(300));
    assert_eq!(next, Some(100));

    // Primeiro elemento (uid 300): anterior é None, próximo é 200
    let (prev_first, next_first) = find_neighbor_uids(&envs, 300);
    assert_eq!(prev_first, None);
    assert_eq!(next_first, Some(200));

    // Último elemento (uid 100): anterior é 200, próximo é None
    let (prev_last, next_last) = find_neighbor_uids(&envs, 100);
    assert_eq!(prev_last, Some(200));
    assert_eq!(next_last, None);
}

#[test]
fn test_find_neighbor_uids_edge_cases_mutation() {
    use lauda_client::domain::view::find_neighbor_uids;

    // 1. Lista completamente vazia
    assert_eq!(find_neighbor_uids(&[], 42), (None, None));

    // 2. UID não existente na lista
    let single = vec![MessageEnvelope {
        uid: 10,
        ..Default::default()
    }];
    assert_eq!(find_neighbor_uids(&single, 999), (None, None));

    // 3. Lista de tamanho único contendo o elemento (não há anterior nem próximo)
    assert_eq!(find_neighbor_uids(&single, 10), (None, None));

    // 4. Lista de 2 elementos
    let duo = vec![
        MessageEnvelope {
            uid: 2,
            ..Default::default()
        },
        MessageEnvelope {
            uid: 1,
            ..Default::default()
        },
    ];
    assert_eq!(find_neighbor_uids(&duo, 2), (None, Some(1)));
    assert_eq!(find_neighbor_uids(&duo, 1), (Some(2), None));
}

#[test]
fn test_keyboard_shortcuts_contract() {
    use lauda_client::domain::view::{ShortcutAction, parse_shortcut};

    // Quando input está focado, NENHUM atalho deve disparar
    assert_eq!(parse_shortcut("j", true), None);
    assert_eq!(parse_shortcut("Enter", true), None);
    assert_eq!(parse_shortcut("r", true), None);

    // Quando input NÃO está focado:
    assert_eq!(
        parse_shortcut("j", false),
        Some(ShortcutAction::NextMessage)
    );
    assert_eq!(
        parse_shortcut("ArrowDown", false),
        Some(ShortcutAction::NextMessage)
    );
    assert_eq!(
        parse_shortcut("k", false),
        Some(ShortcutAction::PrevMessage)
    );
    assert_eq!(
        parse_shortcut("ArrowUp", false),
        Some(ShortcutAction::PrevMessage)
    );
    assert_eq!(
        parse_shortcut("Enter", false),
        Some(ShortcutAction::OpenMessage)
    );
    assert_eq!(
        parse_shortcut("o", false),
        Some(ShortcutAction::OpenMessage)
    );
    assert_eq!(
        parse_shortcut("u", false),
        Some(ShortcutAction::BackToInbox)
    );
    assert_eq!(
        parse_shortcut("Escape", false),
        Some(ShortcutAction::BackToInbox)
    );
    assert_eq!(parse_shortcut("c", false), Some(ShortcutAction::Compose));
    assert_eq!(parse_shortcut("r", false), Some(ShortcutAction::Reply));
    assert_eq!(parse_shortcut("a", false), Some(ShortcutAction::ReplyAll));
    assert_eq!(parse_shortcut("f", false), Some(ShortcutAction::Forward));
    assert_eq!(parse_shortcut("m", false), Some(ShortcutAction::ToggleMode));
    assert_eq!(parse_shortcut("s", false), Some(ShortcutAction::ToggleFlag));
    assert_eq!(parse_shortcut("e", false), Some(ShortcutAction::Archive));
    assert_eq!(parse_shortcut("#", false), Some(ShortcutAction::Delete));
    assert_eq!(
        parse_shortcut("Delete", false),
        Some(ShortcutAction::Delete)
    );
}

#[test]
fn test_keyboard_shortcuts_mutation_coverage() {
    use lauda_client::domain::view::{ShortcutAction, parse_shortcut};

    // 1. Variantes em caixa alta (Shift acidental ou CapsLock)
    assert_eq!(
        parse_shortcut("J", false),
        Some(ShortcutAction::NextMessage)
    );
    assert_eq!(
        parse_shortcut("K", false),
        Some(ShortcutAction::PrevMessage)
    );
    assert_eq!(
        parse_shortcut("O", false),
        Some(ShortcutAction::OpenMessage)
    );
    assert_eq!(
        parse_shortcut("U", false),
        Some(ShortcutAction::BackToInbox)
    );
    assert_eq!(parse_shortcut("C", false), Some(ShortcutAction::Compose));
    assert_eq!(parse_shortcut("R", false), Some(ShortcutAction::Reply));
    assert_eq!(parse_shortcut("A", false), Some(ShortcutAction::ReplyAll));
    assert_eq!(parse_shortcut("F", false), Some(ShortcutAction::Forward));
    assert_eq!(parse_shortcut("M", false), Some(ShortcutAction::ToggleMode));
    assert_eq!(parse_shortcut("S", false), Some(ShortcutAction::ToggleFlag));
    assert_eq!(parse_shortcut("E", false), Some(ShortcutAction::Archive));

    // 2. Teclas não vinculadas devem retornar rigorosamente None
    assert_eq!(parse_shortcut("x", false), None);
    assert_eq!(parse_shortcut("1", false), None);
    assert_eq!(parse_shortcut("Tab", false), None);
    assert_eq!(parse_shortcut("", false), None);

    // 3. Quando focado em input, absolutamente NENHUM caractere pode acionar ação
    for k in [
        "J",
        "K",
        "O",
        "U",
        "C",
        "R",
        "A",
        "F",
        "M",
        "S",
        "E",
        "/",
        "#",
        "Delete",
        "ArrowDown",
    ] {
        assert_eq!(
            parse_shortcut(k, true),
            None,
            "Tecla {} nao deve acionar atalho em input",
            k
        );
    }
}

#[test]
fn test_resolve_navigation_envelopes_with_search_context() {
    use lauda_client::domain::view::{find_neighbor_uids, resolve_navigation_envelopes};

    let mailbox = vec![
        MessageEnvelope {
            uid: 500,
            subject: "Msg 500".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 400,
            subject: "Importante: Alvo da busca 400".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 300,
            subject: "Msg 300".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 200,
            subject: "Msg 200".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 100,
            subject: "Importante: Alvo da busca 100".to_string(),
            ..Default::default()
        },
    ];

    let search_results = vec![
        MessageEnvelope {
            uid: 400,
            subject: "Importante: Alvo da busca 400".to_string(),
            ..Default::default()
        },
        MessageEnvelope {
            uid: 100,
            subject: "Importante: Alvo da busca 100".to_string(),
            ..Default::default()
        },
    ];

    // 1. Sem busca ativa: navegação deve usar mailbox completa
    let normal_candidates = resolve_navigation_envelopes(None, &mailbox);
    assert_eq!(normal_candidates.len(), 5);
    let (prev_norm, next_norm) = find_neighbor_uids(normal_candidates, 400);
    assert_eq!(prev_norm, Some(500));
    assert_eq!(next_norm, Some(300));

    // 2. Com busca ativa: navegação deve usar estritamente os resultados da busca
    let search_candidates = resolve_navigation_envelopes(Some(&search_results), &mailbox);
    assert_eq!(search_candidates.len(), 2);

    let (prev_search_400, next_search_400) = find_neighbor_uids(search_candidates, 400);
    assert_eq!(prev_search_400, None);
    assert_eq!(next_search_400, Some(100)); // Salta o 300 e 200!

    let (prev_search_100, next_search_100) = find_neighbor_uids(search_candidates, 100);
    assert_eq!(prev_search_100, Some(400)); // Volta direto para o 400!
    assert_eq!(next_search_100, None);

    // 3. Busca vazia (zero resultados): deve retornar slice vazio de busca
    let empty_search: Vec<MessageEnvelope> = Vec::new();
    let empty_candidates = resolve_navigation_envelopes(Some(&empty_search), &mailbox);
    assert_eq!(empty_candidates.len(), 0);
    assert_eq!(find_neighbor_uids(empty_candidates, 400), (None, None));
}

#[test]
fn test_resolve_navigation_envelopes_mutation_coverage() {
    use lauda_client::domain::view::{find_neighbor_uids, resolve_navigation_envelopes};

    let mailbox = vec![
        MessageEnvelope {
            uid: 50,
            ..Default::default()
        },
        MessageEnvelope {
            uid: 30,
            ..Default::default()
        },
        MessageEnvelope {
            uid: 10,
            ..Default::default()
        },
    ];

    // Ordem personalizada de relevância da busca (diferente da caixa)
    let ranked_search = vec![
        MessageEnvelope {
            uid: 10,
            ..Default::default()
        },
        MessageEnvelope {
            uid: 50,
            ..Default::default()
        },
        MessageEnvelope {
            uid: 30,
            ..Default::default()
        },
    ];

    let candidates = resolve_navigation_envelopes(Some(&ranked_search), &mailbox);
    assert_eq!(find_neighbor_uids(candidates, 10), (None, Some(50)));
    assert_eq!(find_neighbor_uids(candidates, 50), (Some(10), Some(30)));
    assert_eq!(find_neighbor_uids(candidates, 30), (Some(50), None));

    // UID que existe na caixa (uid 30) mas não está no resultado da busca
    let filtered_subset = vec![MessageEnvelope {
        uid: 10,
        ..Default::default()
    }];
    let subset_candidates = resolve_navigation_envelopes(Some(&filtered_subset), &mailbox);
    // Como 30 não está no resultado da busca, não pode vazar vizinhança da caixa
    assert_eq!(find_neighbor_uids(subset_candidates, 30), (None, None));

    // Ambos vazios
    let empty_mailbox: Vec<MessageEnvelope> = Vec::new();
    assert_eq!(resolve_navigation_envelopes(None, &empty_mailbox).len(), 0);
    assert_eq!(
        resolve_navigation_envelopes(Some(&[]), &empty_mailbox).len(),
        0
    );
}
