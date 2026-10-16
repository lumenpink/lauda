use lauda_server::adapters::MaildirBackend;
use lauda_server::domain::ports::MailBackend;

#[tokio::test]
async fn test_maildir_backend_read_mailboxes_and_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();
    std::fs::create_dir_all(root.join(".Sent").join("cur")).unwrap();

    let msg1 = "1728000001.M123P456.test:2,S";
    let msg2 = "1728000002.M123P456.test:2,";
    let content = "From: remetente@2lp.in\nTo: dest@2lp.in\nSubject: Teste Sintetico\nDate: Mon, 05 Oct 2026 12:00:00 +0000\nContent-Type: text/plain; charset=utf-8\n\nOla Mundo!\n";
    std::fs::write(root.join("cur").join(msg1), content).unwrap();
    std::fs::write(root.join("cur").join(msg2), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n101 :1728000002.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    // Autenticação
    assert!(backend.authenticate("lumen", "senha123").await.is_ok());
    assert!(backend.authenticate("lumen", "errada").await.is_err());

    // Caixas postais
    let mailboxes = backend
        .list_mailboxes("lumen")
        .await
        .expect("Listar caixas");
    assert!(!mailboxes.is_empty());
    let inbox = mailboxes
        .iter()
        .find(|m| m.name == "INBOX")
        .expect("INBOX existe");
    assert_eq!(inbox.total_messages, 2);

    // Listar mensagens
    let messages = backend
        .list_messages("lumen", "INBOX")
        .await
        .expect("Listar mensagens");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages.len() as u32, inbox.total_messages);

    // Mensagens devem estar ordenadas de forma decrescente por UID
    for window in messages.windows(2) {
        assert!(
            window[0].uid >= window[1].uid,
            "Mensagens devem estar em ordem decrescente de UID"
        );
    }

    // Carregar mensagem completa
    let first_uid = messages[0].uid;
    let full_msg = backend
        .get_message("lumen", "INBOX", first_uid)
        .await
        .expect("Carregar mensagem");
    assert_eq!(full_msg.envelope.uid, first_uid);
    assert!(
        full_msg.txt_original.is_some()
            || full_msg.html_sanitized.is_some()
            || full_msg.txt_auto.is_some()
    );
}

#[tokio::test]
async fn test_hermetic_maildir_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Estrutura Maildir básica
    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();
    std::fs::create_dir_all(root.join(".Sent").join("cur")).unwrap();
    std::fs::create_dir_all(root.join(".Trash").join("cur")).unwrap();

    // Mensagem de teste em cur/
    let msg_filename = "1728000000.M123P456.test:2,";
    let msg_content = "From: remetente@2lp.in\nTo: hi@lumen.pink\nSubject: Teste Hermetico\nDate: Mon, 05 Oct 2026 12:00:00 +0000\nContent-Type: text/plain; charset=utf-8\n\nOla Mundo!\n";
    std::fs::write(root.join("cur").join(msg_filename), msg_content).unwrap();

    // dovecot-uidlist
    let uidlist_content =
        "3 V1790640297 N2 Ga836741ba900bb6a090600002f3acf59\n100 :1728000000.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist_content).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("testuser", "senha123");

    // 1. Listar caixas
    let boxes = backend.list_mailboxes("testuser").await.unwrap();
    assert!(boxes.iter().any(|b| b.name == "INBOX"));
    assert!(boxes.iter().any(|b| b.name == "Sent"));
    assert!(boxes.iter().any(|b| b.name == "Trash"));

    let inbox = boxes.iter().find(|b| b.name == "INBOX").unwrap();
    assert_eq!(inbox.total_messages, 1);
    assert_eq!(inbox.unread_messages, 1);

    // 2. Listar mensagens
    let msgs = backend.list_messages("testuser", "INBOX").await.unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].uid, 100);
    assert_eq!(msgs[0].subject, "Teste Hermetico");
    assert!(!msgs[0].is_read);

    // 3. Obter mensagem
    let full = backend.get_message("testuser", "INBOX", 100).await.unwrap();
    assert_eq!(full.envelope.subject, "Teste Hermetico");
    assert_eq!(full.txt_original.as_deref(), Some("Ola Mundo!\n"));

    // 4. Marcar como lida
    backend
        .mark_message_read("testuser", "INBOX", 100, true)
        .await
        .unwrap();
    let msgs_after_read = backend.list_messages("testuser", "INBOX").await.unwrap();
    assert!(msgs_after_read[0].is_read);

    // 5. Enviar mensagem
    let compose = lauda_types::ComposeForm {
        from: "testuser@2lp.in".to_string(),
        to: "dest@2lp.in".to_string(),
        cc: None,
        subject: "Resposta de Teste".to_string(),
        body: "Texto enviado.".to_string(),
        attachments: vec![],
    };
    backend.send_message("testuser", &compose).await.unwrap();
    let sent_msgs = backend.list_messages("testuser", "Sent").await.unwrap();
    assert_eq!(sent_msgs.len(), 1);
    assert_eq!(sent_msgs[0].subject, "Resposta de Teste");

    // 6. Excluir mensagem (move para Trash)
    backend
        .delete_message("testuser", "INBOX", 100)
        .await
        .unwrap();
    let inbox_after_del = backend.list_messages("testuser", "INBOX").await.unwrap();
    assert!(inbox_after_del.is_empty());

    let trash_msgs = backend.list_messages("testuser", "Trash").await.unwrap();
    assert_eq!(trash_msgs.len(), 1);

    // 7. Excluir permanentemente da Lixeira
    let trash_uid = trash_msgs[0].uid;
    backend
        .delete_message("testuser", "Trash", trash_uid)
        .await
        .unwrap();
    let trash_after_del = backend.list_messages("testuser", "Trash").await.unwrap();
    assert!(trash_after_del.is_empty());
}

#[tokio::test]
async fn test_hermetic_maildir_flag_move_and_attachments() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();
    std::fs::create_dir_all(root.join(".Archive").join("cur")).unwrap();

    // Mensagem MIME com anexo em cur/
    let mime_msg = "From: remetente@2lp.in\n\
To: hi@lumen.pink\n\
Cc: team@2lp.in\n\
Subject: Proposta com Anexo\n\
Date: Mon, 05 Oct 2026 14:00:00 +0000\n\
MIME-Version: 1.0\n\
Content-Type: multipart/mixed; boundary=\"BOUNDARY123\"\n\
\n\
--BOUNDARY123\n\
Content-Type: text/plain; charset=utf-8\n\
\n\
Segue proposta anexada.\n\
\n\
--BOUNDARY123\n\
Content-Type: application/pdf; name=\"proposta.pdf\"\n\
Content-Disposition: attachment; filename=\"proposta.pdf\"\n\
Content-Transfer-Encoding: base64\n\
\n\
SGVsbG8gTGF1ZGEh\n\
--BOUNDARY123--\n";

    let msg_filename = "1728000001.M999P888.test:2,";
    std::fs::write(root.join("cur").join(msg_filename), mime_msg).unwrap();

    let uidlist_content =
        "3 V1790640297 N2 Ga836741ba900bb6a090600002f3acf59\n200 :1728000001.M999P888.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist_content).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("testuser", "senha123");

    // 1. Obter detalhes e verificar anexo e CC
    let full = backend.get_message("testuser", "INBOX", 200).await.unwrap();
    assert_eq!(full.envelope.subject, "Proposta com Anexo");
    assert_eq!(full.envelope.cc.as_deref(), Some("team@2lp.in"));
    assert_eq!(full.attachments.len(), 1);
    assert_eq!(full.attachments[0].filename, "proposta.pdf");
    assert_eq!(full.attachments[0].content_type, "application/pdf");

    // 2. Extrair anexo diretamente
    let (att_name, att_type, att_bytes) = backend
        .get_attachment("testuser", "INBOX", 200, 0)
        .await
        .unwrap();
    assert_eq!(att_name, "proposta.pdf");
    assert_eq!(att_type, "application/pdf");
    assert_eq!(att_bytes, b"Hello Lauda!");

    // 3. Marcar como Favorito (Flagged)
    backend
        .mark_message_flagged("testuser", "INBOX", 200, true)
        .await
        .unwrap();
    let msgs = backend.list_messages("testuser", "INBOX").await.unwrap();
    assert!(msgs[0].is_flagged);

    // 4. Mover para a pasta Archive
    backend
        .move_message("testuser", "INBOX", "Archive", 200)
        .await
        .unwrap();
    let inbox_after = backend.list_messages("testuser", "INBOX").await.unwrap();
    assert!(inbox_after.is_empty());

    let archive_after = backend.list_messages("testuser", "Archive").await.unwrap();
    assert_eq!(archive_after.len(), 1);
    assert_eq!(archive_after[0].subject, "Proposta com Anexo");

    // 5. Listar contatos
    let contacts = backend.list_contacts("testuser").await.unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].email, "remetente@2lp.in");

    // 6. Enviar mensagem com anexo e CC
    let compose_with_att = lauda_types::ComposeForm {
        from: "testuser@2lp.in".to_string(),
        to: "socio@empresa.com".to_string(),
        cc: Some("diretoria@2lp.in".to_string()),
        subject: "Contrato Assinado".to_string(),
        body: "Segue em anexo o contrato em PDF.".to_string(),
        attachments: vec![lauda_types::AttachmentUpload {
            filename: "contrato.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            data_base64: "SGVsbG8gTGF1ZGEh".to_string(),
        }],
    };
    backend
        .send_message("testuser", &compose_with_att)
        .await
        .unwrap();
    let sent_msgs = backend.list_messages("testuser", "Sent").await.unwrap();
    assert_eq!(sent_msgs.len(), 1);
    assert_eq!(sent_msgs[0].subject, "Contrato Assinado");
    assert!(sent_msgs[0].has_attachments);
}

#[tokio::test]
async fn test_dovecot_maildir_flag_parsing_with_size_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let email_raw = "From: teste@2lp.in\nTo: hi@lumen.pink\nSubject: Dovecot Test\nDate: Mon, 05 Oct 2026 12:00:00 +0000\nContent-Type: text/plain\n\nCorpo\n";

    // 1. Não lido em cur/ com metadados ,S=1099,W=1125:2,
    let f1 = "1790640297.M460601P1545.mail.lumen.pink,S=1099,W=1125:2,";
    std::fs::write(root.join("cur").join(f1), email_raw).unwrap();

    // 2. Lido em cur/ com metadados ,S=2048,W=2100:2,S
    let f2 = "1790640298.M460602P1545.mail.lumen.pink,S=2048,W=2100:2,S";
    std::fs::write(root.join("cur").join(f2), email_raw).unwrap();

    // 3. Lido e Favorito em cur/ com metadados ,S=500,W=510:2,FS
    let f3 = "1790640299.M460603P1545.mail.lumen.pink,S=500,W=510:2,FS";
    std::fs::write(root.join("cur").join(f3), email_raw).unwrap();

    // 4. Não lido em new/ sem sufixo de flags
    let f4 = "1790640300.M460604P1545.mail.lumen.pink,S=1200,W=1250";
    std::fs::write(root.join("new").join(f4), email_raw).unwrap();

    // dovecot-uidlist associando uids 10, 20, 30, 40
    let uidlist = "3 V1790640297 N5 Ga836741ba900bb6a090600002f3acf59\n10 :1790640297.M460601P1545.mail.lumen.pink,S=1099,W=1125\n20 :1790640298.M460602P1545.mail.lumen.pink,S=2048,W=2100\n30 :1790640299.M460603P1545.mail.lumen.pink,S=500,W=510\n40 :1790640300.M460604P1545.mail.lumen.pink,S=1200,W=1250\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("testuser", "senha123");

    // Verificar contagem de mailboxes
    let boxes = backend.list_mailboxes("testuser").await.unwrap();
    let inbox = boxes.iter().find(|b| b.name == "INBOX").unwrap();
    assert_eq!(inbox.total_messages, 4);
    assert_eq!(inbox.unread_messages, 2); // f1 e f4 não lidos

    // Verificar mensagens individuais
    let msgs = backend.list_messages("testuser", "INBOX").await.unwrap();
    let m1 = msgs.iter().find(|m| m.uid == 10).unwrap();
    assert!(!m1.is_read);
    assert!(!m1.is_flagged);

    let m2 = msgs.iter().find(|m| m.uid == 20).unwrap();
    assert!(m2.is_read);
    assert!(!m2.is_flagged);

    let m3 = msgs.iter().find(|m| m.uid == 30).unwrap();
    assert!(m3.is_read);
    assert!(m3.is_flagged);

    let m4 = msgs.iter().find(|m| m.uid == 40).unwrap();
    assert!(!m4.is_read);
    assert!(!m4.is_flagged);

    // Marcar m1 como lida
    backend
        .mark_message_read("testuser", "INBOX", 10, true)
        .await
        .unwrap();
    let boxes_after = backend.list_mailboxes("testuser").await.unwrap();
    let inbox_after = boxes_after.iter().find(|b| b.name == "INBOX").unwrap();
    assert_eq!(inbox_after.unread_messages, 1);

    // Marcar m1 de volta como não lida
    backend
        .mark_message_read("testuser", "INBOX", 10, false)
        .await
        .unwrap();
    let boxes_unread = backend.list_mailboxes("testuser").await.unwrap();
    let inbox_unread = boxes_unread.iter().find(|b| b.name == "INBOX").unwrap();
    assert_eq!(inbox_unread.unread_messages, 2);
}

#[tokio::test]
async fn test_maildir_txt_auto_prioritizes_html_over_plain() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let msg = "1728000010.M123P456.test:2,S";
    let content = concat!(
        "From: mailling@abvo.com.br\r\n",
        "To: hi@lumen.pink\r\n",
        "Subject: LEI 14.751/2023\r\n",
        "Content-Type: multipart/alternative; boundary=\"boundary_abc\"\r\n",
        "\r\n",
        "--boundary_abc\r\n",
        "Content-Type: text/plain; charset=utf-8\r\n",
        "\r\n",
        "CORPO EM TEXTO PURO DEGRADADO SEM QUEBRAS\r\n",
        "--boundary_abc\r\n",
        "Content-Type: text/html; charset=utf-8\r\n",
        "\r\n",
        "<p>PUBLICADA LEI 14.751/2023 LOB NACIONAL</p>\r\n",
        "<p>Prezados(as) Dirigentes de Entidades</p>\r\n",
        "<p>DIRETORIA DA FENEME</p>\r\n",
        "--boundary_abc--\r\n"
    );
    std::fs::write(root.join("cur").join(msg), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n200 :1728000010.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    let full_msg = backend
        .get_message("lumen", "INBOX", 200)
        .await
        .expect("Buscar mensagem");

    assert!(full_msg.txt_auto.is_some());
    let txt_auto = full_msg.txt_auto.unwrap();

    // txt_auto DEVE priorizar o conteúdo derivado do HTML e não o texto puro degradado
    assert!(
        !txt_auto.contains("CORPO EM TEXTO PURO DEGRADADO"),
        "txt_auto não deve usar o texto puro degradado quando há HTML válido"
    );
    assert!(
        txt_auto.contains("PUBLICADA LEI 14.751/2023 LOB NACIONAL"),
        "txt_auto deve conter o conteúdo derivado do HTML"
    );
    assert!(
        txt_auto.contains("Prezados(as) Dirigentes de Entidades"),
        "txt_auto deve conter parágrafos intermediários do HTML"
    );
}

#[tokio::test]
async fn test_maildir_txt_auto_falls_back_to_plain_when_no_html() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let msg = "1728000011.M123P456.test:2,S";
    let content = "From: cron@local\r\nTo: root@local\r\nSubject: Backup OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nRelatorio de backup executado com sucesso.\r\n";
    std::fs::write(root.join("cur").join(msg), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n201 :1728000011.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    let full_msg = backend
        .get_message("lumen", "INBOX", 201)
        .await
        .expect("Buscar mensagem");

    assert!(full_msg.txt_auto.is_some());
    let txt_auto = full_msg.txt_auto.unwrap();
    assert!(txt_auto.contains("Relatorio de backup executado com sucesso."));
}

#[tokio::test]
async fn test_maildir_pagination_and_mtime_sorting() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    // Criar 4 mensagens com mtimes distintos
    let base_time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1700000000);
    for i in 1..=4 {
        let filename = format!("170000000{i}.M123P456.test:2,S");
        let path = root.join("cur").join(&filename);
        let content = format!(
            "From: test{i}@2lp.in\nSubject: Mensagem {i}\nDate: Mon, 05 Oct 2026 12:00:00 +0000\n\nCorpo {i}"
        );
        std::fs::write(&path, content).unwrap();
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        let mtime = base_time + std::time::Duration::from_secs(i * 100);
        let times = std::fs::FileTimes::new().set_modified(mtime);
        file.set_times(times).unwrap();
    }

    // Página 1: limit 2, offset 0 -> deve retornar msg 4 e msg 3 (mtimes mais recentes)
    let p1 = backend
        .list_messages("lumen", "INBOX", Some(2), Some(0))
        .await
        .expect("pagina 1");
    assert_eq!(p1.len(), 2);
    assert_eq!(p1[0].subject, "Mensagem 4");
    assert_eq!(p1[1].subject, "Mensagem 3");

    // Página 2: limit 2, offset 2 -> deve retornar msg 2 e msg 1
    let p2 = backend
        .list_messages("lumen", "INBOX", Some(2), Some(2))
        .await
        .expect("pagina 2");
    assert_eq!(p2.len(), 2);
    assert_eq!(p2[0].subject, "Mensagem 2");
    assert_eq!(p2[1].subject, "Mensagem 1");
}

