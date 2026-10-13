use lauda_server::adapters::sqlite_contacts::SqliteContactsStore;
use lauda_types::contact::{ContactEntry, ContactSyncRequest};

#[tokio::test]
async fn test_sqlite_contacts_crud_and_incremental_sync() {
    let temp_dir = tempfile::tempdir().expect("Criar tempdir");
    let db_path = temp_dir.path().join("contacts.db");

    let store = SqliteContactsStore::open(&db_path).expect("Inicializar banco SQLite de contatos");

    // 1. Inserir ou atualizar contato
    let contact1 = ContactEntry {
        id: "c1-uuid-1234".to_string(),
        username: "lumen".to_string(),
        display_name: Some("Alexandra Kollontai".to_string()),
        primary_email: "kollontai@soviet.org".to_string(),
        secondary_emails: vec!["alexandra@diplomacy.su".to_string()],
        phone_numbers: vec!["+7-495-000-0001".to_string()],
        birthday: Some("1872-03-31".to_string()),
        notes: Some("Diplomata e revolucionária".to_string()),
        vcard_raw: "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Alexandra Kollontai\r\nBDAY:1872-03-31\r\nEMAIL:kollontai@soviet.org\r\nEND:VCARD".to_string(),
        has_autocrypt: true,
        autocrypt_fingerprint: Some("KOLLONTAI-KEY-FINGERPRINT".to_string()),
        etag: "etag-v1".to_string(),
        version: 1,
        created_at: 1728700000,
        updated_at: 1728700000,
        deleted_at: None,
    };

    store.upsert_contact(&contact1).expect("Inserir contato 1");

    // 2. Recuperar por email
    let fetched = store
        .get_contact_by_email("lumen", "kollontai@soviet.org")
        .expect("Buscar por email")
        .expect("Contato deve existir");
    assert_eq!(fetched.display_name.as_deref(), Some("Alexandra Kollontai"));
    assert!(fetched.has_autocrypt);

    // 3. Sync Inicial (sem token)
    let sync_req_initial = ContactSyncRequest {
        sync_token: None,
        limit: Some(100),
    };
    let initial_sync = store
        .sync_contacts("lumen", &sync_req_initial)
        .expect("Sync inicial");
    assert_eq!(initial_sync.updated.len(), 1);
    assert_eq!(initial_sync.deleted_ids.len(), 0);
    let token_1 = initial_sync.sync_token;

    // 4. Inserir contato 2 e atualizar contato 1
    let contact2 = ContactEntry {
        id: "c2-uuid-5678".to_string(),
        username: "lumen".to_string(),
        display_name: Some("Rosa Luxemburgo".to_string()),
        primary_email: "rosa@spartakus.de".to_string(),
        secondary_emails: vec![],
        phone_numbers: vec![],
        birthday: Some("1871-03-05".to_string()),
        notes: None,
        vcard_raw: "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Rosa Luxemburgo\r\nBDAY:1871-03-05\r\nEMAIL:rosa@spartakus.de\r\nEND:VCARD".to_string(),
        has_autocrypt: false,
        autocrypt_fingerprint: None,
        etag: "etag-rosa-v1".to_string(),
        version: 1,
        created_at: 1728700050,
        updated_at: 1728700050,
        deleted_at: None,
    };
    store.upsert_contact(&contact2).expect("Inserir contato 2");

    // 5. Sync Incremental com token anterior
    let sync_req_inc = ContactSyncRequest {
        sync_token: Some(token_1.clone()),
        limit: Some(100),
    };
    let inc_sync = store
        .sync_contacts("lumen", &sync_req_inc)
        .expect("Sync incremental");
    assert_eq!(
        inc_sync.updated.len(),
        1,
        "Apenas contato 2 deve vir como novo/alterado"
    );
    assert_eq!(inc_sync.updated[0].id, "c2-uuid-5678");
    let token_2 = inc_sync.sync_token;

    // 6. Soft-delete contato 1
    store
        .delete_contact("lumen", "c1-uuid-1234", 1728700100)
        .expect("Soft-delete contato 1");

    // 7. Sync Incremental detectando tombstone
    let sync_req_tombstone = ContactSyncRequest {
        sync_token: Some(token_2),
        limit: Some(100),
    };
    let tombstone_sync = store
        .sync_contacts("lumen", &sync_req_tombstone)
        .expect("Sync tombstone");
    assert_eq!(tombstone_sync.updated.len(), 0);
    assert_eq!(tombstone_sync.deleted_ids, vec!["c1-uuid-1234".to_string()]);
}
