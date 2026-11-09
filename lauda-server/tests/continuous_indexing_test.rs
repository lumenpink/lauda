use std::collections::HashSet;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

use lauda_server::adapters::maildir::MaildirBackend;
use lauda_server::adapters::sqlite_index::SqliteIndexStore;
use lauda_server::domain::ports::MailBackend;

#[test]
fn test_reconcile_mailbox_orders_pending_uids_descending() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("test_index.db");
    let store = SqliteIndexStore::open(&db_path).expect("abrir sqlite index");

    let mut active = HashSet::new();
    active.insert(10);
    active.insert(50);
    active.insert(200);
    active.insert(15);
    active.insert(500);

    let reconcile = store
        .reconcile_mailbox("INBOX", &active)
        .expect("reconciliar");

    // Requisitos do Ticket #95: mensagens mais recentes (maiores UIDs) devem vir primeiro!
    assert_eq!(
        reconcile.to_index,
        vec![500, 200, 50, 15, 10],
        "A fila to_index DEVE estar ordenada de forma decrescente para priorizar e-mails novos"
    );
}

#[tokio::test]
async fn test_maildir_continuous_indexer_syncs_automatically() {
    let temp = tempdir().expect("tempdir");
    let base_dir = temp.path().join("maildir");
    let cur_dir = base_dir.join("cur");
    fs::create_dir_all(&cur_dir).expect("criar pasta cur");

    // Criar dovecot-uidlist com dois e-mails
    let uidlist_content = "3 V1234567890 N1 G0\n101 :101.mail.test:2,S\n102 :102.mail.test:2,S\n";
    fs::write(base_dir.join("dovecot-uidlist"), uidlist_content).expect("escrever uidlist");

    // Criar os arquivos de e-mail
    let raw_msg_101 = "From: teste1@2lp.in\r\nTo: hi@lumen.pink\r\nSubject: Teste 101\r\nDate: Mon, 9 Nov 2026 12:00:00 -0300\r\n\r\nCorpo da mensagem 101";
    let raw_msg_102 = "From: teste2@2lp.in\r\nTo: hi@lumen.pink\r\nSubject: Teste 102\r\nDate: Mon, 9 Nov 2026 13:00:00 -0300\r\n\r\nCorpo da mensagem 102";
    fs::write(cur_dir.join("101.mail.test:2,S"), raw_msg_101).expect("write msg 101");
    fs::write(cur_dir.join("102.mail.test:2,S"), raw_msg_102).expect("write msg 102");

    let backend = Arc::new(MaildirBackend::new(&base_dir));

    // Antes de iniciar o indexador, o SQLite não tem nenhum documento
    let store = backend.sqlite_index().expect("sqlite_index deve existir");
    let initial_uids = store.get_indexed_uids("INBOX").expect("uids");
    assert!(initial_uids.is_empty());

    // Disparar o indexador contínuo com intervalo curto (50ms)
    backend.clone().start_continuous_indexer(1);

    // Aguardar até 2 segundos para o background worker indexar automaticamente os 2 e-mails
    let mut synced = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(uids) = store.get_indexed_uids("INBOX") {
            if uids.len() == 2 && uids.contains(&101) && uids.contains(&102) {
                synced = true;
                break;
            }
        }
    }

    assert!(
        synced,
        "O indexador contínuo em background DEVE sincronizar as mensagens automaticamente sem necessidade de busca"
    );
}

#[tokio::test]
async fn test_maildir_delete_message_removes_from_sqlite_index_immediately() {
    let temp = tempdir().expect("tempdir");
    let base_dir = temp.path().join("maildir");
    let cur_dir = base_dir.join("cur");
    fs::create_dir_all(&cur_dir).expect("criar pasta cur");

    let uidlist_content = "3 V1234567890 N1 G0\n301 :301.mail.test:2,S\n";
    fs::write(base_dir.join("dovecot-uidlist"), uidlist_content).expect("escrever uidlist");

    let raw_msg_301 = "From: deletar@2lp.in\r\nTo: hi@lumen.pink\r\nSubject: Mensagem Para Deletar\r\nDate: Mon, 9 Nov 2026 12:00:00 -0300\r\n\r\nTexto de teste para exclusao.";
    fs::write(cur_dir.join("301.mail.test:2,S"), raw_msg_301).expect("write msg 301");

    let backend = Arc::new(MaildirBackend::new(&base_dir));
    let store = backend.sqlite_index().expect("sqlite_index");

    // Sincroniza indexando a mensagem 301
    backend.clone().start_continuous_indexer(1);

    let mut indexed = false;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(uids) = store.get_indexed_uids("INBOX") {
            if uids.contains(&301) {
                indexed = true;
                break;
            }
        }
    }
    assert!(indexed, "Mensagem 301 deve ter sido indexada");

    // Executa a exclusão da mensagem
    backend
        .delete_message("lumen", "INBOX", 301)
        .await
        .expect("deletar mensagem");

    // Verifica que foi expurgada do índice SQLite imediatamente
    let uids_after_delete = store.get_indexed_uids("INBOX").expect("uids");
    assert!(
        !uids_after_delete.contains(&301),
        "A mensagem excluída DEVE ser removida imediatamente da base vetorial do SQLite"
    );
}
