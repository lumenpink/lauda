use lauda_client::domain::contacts_cache::{
    LocalContactsCache, create_local_contact, format_contact_recipient,
};
use tempfile::tempdir;

#[test]
fn test_contacts_ui_autocomplete_helpers_and_crud() {
    // 1. Formatação de destinatário
    assert_eq!(
        format_contact_recipient(Some("Maria Firmina"), "maria@letras.org"),
        "Maria Firmina <maria@letras.org>"
    );
    assert_eq!(
        format_contact_recipient(None, "anon@letras.org"),
        "anon@letras.org"
    );
    assert_eq!(
        format_contact_recipient(Some(""), "semnome@letras.org"),
        "semnome@letras.org"
    );

    // 2. Criação de contato local e persistência no cache
    let temp = tempdir().expect("tempdir");
    let cache = LocalContactsCache::open(temp.path()).expect("open cache");

    let entry = create_local_contact(
        "lumen",
        Some("Carolina Maria de Jesus"),
        "carolina@quarto.org",
        Some("Escritora e cronista"),
    );

    assert_eq!(entry.username, "lumen");
    assert_eq!(entry.primary_email, "carolina@quarto.org");
    assert!(entry.vcard_raw.contains("Carolina Maria de Jesus"));

    // Persistir contato
    cache
        .apply_sync_delta(std::slice::from_ref(&entry), &[], "ts:local-1")
        .expect("persist contact");

    assert_eq!(cache.count_contacts().unwrap(), 1);

    // 3. Autocomplete busca pelo prefixo
    let suggestions = cache.search_contacts("carolina", 5).unwrap();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(
        format_contact_recipient(
            suggestions[0].display_name.as_deref(),
            &suggestions[0].primary_email
        ),
        "Carolina Maria de Jesus <carolina@quarto.org>"
    );
}
