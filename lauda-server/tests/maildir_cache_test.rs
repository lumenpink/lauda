use lauda_server::adapters::MaildirBackend;
use lauda_server::domain::ports::MailBackend;

#[tokio::test]
async fn test_maildir_cache_lookup_and_file_resolution() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let msg1 = "1728000001.M123P456.test:2,S";
    let msg2 = "1728000002.M123P456.test:2,";
    let msg3 = "1728000003.M123P456.test:2,F";
    let content = "From: remetente@2lp.in\nTo: dest@2lp.in\nSubject: Teste Cache\nDate: Mon, 05 Oct 2026 12:00:00 +0000\nContent-Type: text/plain; charset=utf-8\n\nOla Mundo!\n";

    std::fs::write(root.join("cur").join(msg1), content).unwrap();
    std::fs::write(root.join("cur").join(msg2), content).unwrap();
    std::fs::write(root.join("new").join(msg3), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n101 :1728000002.M123P456.test\n102 :1728000003.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    // 1. Primeiro lookup: popula o cache
    let res100 = MaildirBackend::find_message_file_static(root, 100);
    assert!(res100.is_some(), "UID 100 deve ser encontrado");
    let (path100, fname100, read100, flag100) = res100.unwrap();
    assert_eq!(fname100, msg1);
    assert!(read100);
    assert!(!flag100);
    assert!(path100.exists());

    // 2. Segundo lookup: deve ser resolvido instantaneamente via cache
    let res101 = MaildirBackend::find_message_file_static(root, 101);
    assert!(res101.is_some(), "UID 101 deve ser encontrado");
    let (_, fname101, read101, flag101) = res101.unwrap();
    assert_eq!(fname101, msg2);
    assert!(!read101);
    assert!(!flag101);

    // 3. Terceiro lookup em pasta "new":
    let res102 = MaildirBackend::find_message_file_static(root, 102);
    assert!(res102.is_some(), "UID 102 deve ser encontrado em new");
    let (_, fname102, read102, flag102) = res102.unwrap();
    assert_eq!(fname102, msg3);
    assert!(!read102);
    assert!(flag102);

    // 4. Invalidação por renomeação de arquivo externa
    // Simulando Dovecot renomeando msg2 de unread para read (:2,S)
    let new_msg2 = "1728000002.M123P456.test:2,S";
    std::fs::rename(root.join("cur").join(msg2), root.join("cur").join(new_msg2)).unwrap();

    let res101_after = MaildirBackend::find_message_file_static(root, 101);
    assert!(
        res101_after.is_some(),
        "UID 101 deve ser encontrado após renomeação"
    );
    let (_, fname101_after, read101_after, _) = res101_after.unwrap();
    assert_eq!(fname101_after, new_msg2);
    assert!(read101_after, "UID 101 agora deve refletir flag de lido");

    // 5. Invalidação e leitura através do método do trait backend.get_message()
    let msg_obj = backend
        .get_message("lumen", "INBOX", 101)
        .await
        .expect("Mensagem 101 recuperada");
    assert_eq!(msg_obj.envelope.uid, 101);
    assert!(msg_obj.envelope.is_read);
}

#[tokio::test]
async fn test_maildir_cache_invalidation_on_dovecot_uidlist_change() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();

    let msg1 = "1728000001.M123P456.test:2,S";
    let content = "From: a@2lp.in\nTo: b@2lp.in\nSubject: Inicial\n\nCorpo\n";
    std::fs::write(root.join("cur").join(msg1), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    assert!(MaildirBackend::find_message_file_static(root, 100).is_some());
    assert!(MaildirBackend::find_message_file_static(root, 200).is_none());

    // Escrever nova mensagem e atualizar dovecot-uidlist
    let msg2 = "1728000002.M123P456.test:2,";
    std::fs::write(root.join("cur").join(msg2), content).unwrap();
    let updated_uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n200 :1728000002.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), updated_uidlist).unwrap();

    let res200 = MaildirBackend::find_message_file_static(root, 200);
    assert!(
        res200.is_some(),
        "UID 200 deve ser encontrado após atualização do dovecot-uidlist"
    );
    let (_, fname200, _, _) = res200.unwrap();
    assert_eq!(fname200, msg2);
}

#[tokio::test]
async fn test_maildir_cache_invalidation_on_backend_mutations() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::create_dir_all(root.join("cur")).unwrap();
    std::fs::create_dir_all(root.join("new")).unwrap();
    std::fs::create_dir_all(root.join(".Trash").join("cur")).unwrap();

    let msg1 = "1728000001.M123P456.test:2,";
    let content = "From: a@2lp.in\nTo: b@2lp.in\nSubject: Teste Mutacao\n\nCorpo\n";
    std::fs::write(root.join("cur").join(msg1), content).unwrap();

    let uidlist = "3 V1728000000 N100 Gabcdef\n100 :1728000001.M123P456.test\n";
    std::fs::write(root.join("dovecot-uidlist"), uidlist).unwrap();

    let backend = MaildirBackend::new(root);
    backend.add_user("lumen", "senha123");

    // Verificar não lido inicialmente
    let (_, _, is_read_initial, is_flagged_initial) =
        MaildirBackend::find_message_file_static(root, 100).expect("Encontrada");
    assert!(!is_read_initial);
    assert!(!is_flagged_initial);

    // Marcar como lida
    backend
        .mark_message_read("lumen", "INBOX", 100, true)
        .await
        .expect("Marcar lida");
    let (_, _, is_read_after, _) =
        MaildirBackend::find_message_file_static(root, 100).expect("Encontrada");
    assert!(is_read_after, "Deve refletir lido imediatamente");

    // Marcar como flag
    backend
        .mark_message_flagged("lumen", "INBOX", 100, true)
        .await
        .expect("Marcar flag");
    let (_, _, _, is_flagged_after) =
        MaildirBackend::find_message_file_static(root, 100).expect("Encontrada");
    assert!(is_flagged_after, "Deve refletir flag imediatamente");

    // Excluir mensagem (move para Trash)
    backend
        .delete_message("lumen", "INBOX", 100)
        .await
        .expect("Deletar");
    let lookup_deleted = MaildirBackend::find_message_file_static(root, 100);
    assert!(
        lookup_deleted.is_none(),
        "Mensagem excluída não deve mais ser encontrada na INBOX"
    );
}
