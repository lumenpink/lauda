use lauda_server::adapters::MaildirBackend;
use lauda_server::domain::ports::MailBackend;
use std::path::PathBuf;

#[tokio::test]
async fn test_maildir_backend_read_mailboxes_and_messages() {
    let maildir = PathBuf::from("/home/lumen/stacks/mail/data/dms/mail-data/lumen.pink/hi");
    if !maildir.exists() {
        return;
    }

    let backend = MaildirBackend::new(&maildir);
    backend.add_user("lumen", "senha123");

    // Autenticação
    assert!(backend.authenticate("lumen", "senha123").await.is_ok());
    assert!(backend.authenticate("lumen", "errada").await.is_err());

    // Caixas postais
    let mailboxes = backend.list_mailboxes("lumen").await.expect("Listar caixas");
    assert!(!mailboxes.is_empty());
    let inbox = mailboxes.iter().find(|m| m.name == "INBOX").expect("INBOX existe");
    assert!(inbox.total_messages >= 40, "Deve haver mais de 40 mensagens na INBOX");

    // Listar mensagens
    let messages = backend.list_messages("lumen", "INBOX").await.expect("Listar mensagens");
    assert!(!messages.is_empty());
    assert_eq!(messages.len() as u32, inbox.total_messages);

    // Mensagens devem estar ordenadas de forma decrescente por UID
    for window in messages.windows(2) {
        assert!(window[0].uid >= window[1].uid, "Mensagens devem estar em ordem decrescente de UID");
    }

    // Carregar mensagem completa
    let first_uid = messages[0].uid;
    let full_msg = backend.get_message("lumen", "INBOX", first_uid).await.expect("Carregar mensagem");
    assert_eq!(full_msg.envelope.uid, first_uid);
    assert!(full_msg.txt_original.is_some() || full_msg.html_sanitized.is_some() || full_msg.txt_auto.is_some());
}
