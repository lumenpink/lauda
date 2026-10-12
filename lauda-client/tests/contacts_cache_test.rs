use lauda_client::domain::contacts_cache::LocalContactsCache;
use lauda_types::contact::ContactEntry;
use tempfile::tempdir;

fn make_sample_contact(id: &str, name: &str, email: &str, updated_at: u64) -> ContactEntry {
    ContactEntry {
        id: id.to_string(),
        username: "lumen".to_string(),
        display_name: Some(name.to_string()),
        primary_email: email.to_string(),
        secondary_emails: vec![],
        phone_numbers: vec![],
        notes: None,
        vcard_raw: format!("BEGIN:VCARD\r\nFN:{name}\r\nEMAIL:{email}\r\nEND:VCARD"),
        has_autocrypt: false,
        autocrypt_fingerprint: None,
        etag: format!("etag-{id}"),
        version: 1,
        created_at: 1000,
        updated_at,
        deleted_at: None,
    }
}

#[test]
fn test_contacts_local_cache_lifecycle_and_search() {
    let temp = tempdir().expect("tempdir");
    let cache = LocalContactsCache::open(temp.path()).expect("open cache");

    // 1. Inicialmente vazio
    assert_eq!(cache.count_contacts().unwrap(), 0);
    assert_eq!(cache.get_sync_token().unwrap(), None);
    assert_eq!(cache.load_contacts().unwrap().len(), 0);

    // 2. Aplicar lote inicial de contatos
    let c1 = make_sample_contact("c1", "Maria Firmina", "maria@letras.org", 100);
    let c2 = make_sample_contact("c2", "Machado de Assis", "machado@academia.org", 100);

    cache
        .apply_sync_delta(&[c1.clone(), c2.clone()], &[], "ts:100")
        .expect("apply delta");

    assert_eq!(cache.count_contacts().unwrap(), 2);
    assert_eq!(cache.get_sync_token().unwrap().as_deref(), Some("ts:100"));

    // 3. Busca por prefixo/substring (display_name e primary_email)
    let search_name = cache.search_contacts("firmina", 10).unwrap();
    assert_eq!(search_name.len(), 1);
    assert_eq!(search_name[0].id, "c1");

    let search_email = cache.search_contacts("academia.org", 10).unwrap();
    assert_eq!(search_email.len(), 1);
    assert_eq!(search_email[0].id, "c2");

    let search_none = cache.search_contacts("desconhecido", 10).unwrap();
    assert_eq!(search_none.len(), 0);

    // 4. Aplicar delta incremental: atualizar c1 e remover c2 (tombstone)
    let c1_updated = make_sample_contact("c1", "Maria Firmina dos Reis", "maria@letras.org", 200);
    cache
        .apply_sync_delta(&[c1_updated], &["c2".to_string()], "ts:200")
        .expect("apply delta update and delete");

    assert_eq!(cache.count_contacts().unwrap(), 1);
    assert_eq!(cache.get_sync_token().unwrap().as_deref(), Some("ts:200"));

    let loaded = cache.load_contacts().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded[0].display_name.as_deref(),
        Some("Maria Firmina dos Reis")
    );

    // c2 não deve mais aparecer em buscas
    let search_deleted = cache.search_contacts("machado", 10).unwrap();
    assert_eq!(search_deleted.len(), 0);
}
