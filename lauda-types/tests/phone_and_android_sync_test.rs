use lauda_types::contact::ContactEntry;
use lauda_types::phone::{NormalizedPhone, PhoneKind, PhoneNormalizer};
use lauda_types::android_sync::{
    AndroidContactBatch, AndroidMimeType, AndroidRawContactOp, LAUDA_ACCOUNT_TYPE,
};

#[test]
fn test_phone_normalizer_brazilian_mobile_and_ninth_digit() {
    // Celular já com 9 dígitos e DDD
    let p1 = PhoneNormalizer::normalize_br("(48) 99123-4567", None).expect("deve normalizar celular");
    assert_eq!(p1.e164, "+5548991234567");
    assert_eq!(p1.formatted, "+55 (48) 99123-4567");
    assert_eq!(p1.ddd, "48");
    assert_eq!(p1.kind, PhoneKind::Mobile);

    // Celular antigo com 8 dígitos precisando de 9º dígito (começado com 9)
    let p2 = PhoneNormalizer::normalize_br("48 9123-4567", None).expect("deve adicionar 9º dígito");
    assert_eq!(p2.e164, "+5548991234567");
    assert_eq!(p2.formatted, "+55 (48) 99123-4567");
    assert_eq!(p2.kind, PhoneKind::Mobile);

    // Celular antigo com 8 dígitos começado em 8 (ex: SP)
    let p3 = PhoneNormalizer::normalize_br("(11) 8765-4321", None).expect("deve adicionar 9º dígito");
    assert_eq!(p3.e164, "+5511987654321");
    assert_eq!(p3.formatted, "+55 (11) 98765-4321");
    assert_eq!(p3.kind, PhoneKind::Mobile);
}

#[test]
fn test_phone_normalizer_csp_carrier_selection_removal() {
    // Número com CSP da Claro (021)
    let p1 = PhoneNormalizer::normalize_br("021 (48) 99123-4567", None).expect("deve remover CSP 021");
    assert_eq!(p1.e164, "+5548991234567");
    assert_eq!(p1.ddd, "48");

    // Número com CSP da Vivo (015)
    let p2 = PhoneNormalizer::normalize_br("015 11 98888-7777", None).expect("deve remover CSP 015");
    assert_eq!(p2.e164, "+5511988887777");
    assert_eq!(p2.ddd, "11");

    // Número com CSP da TIM (041)
    let p3 = PhoneNormalizer::normalize_br("041 21 97777-6666", None).expect("deve remover CSP 041");
    assert_eq!(p3.e164, "+5521977776666");
    assert_eq!(p3.ddd, "21");
}

#[test]
fn test_phone_normalizer_landline_never_adds_ninth_digit() {
    // Telefone fixo (iniciado com 3)
    let p1 = PhoneNormalizer::normalize_br("48 3222-1234", None).expect("deve normalizar fixo");
    assert_eq!(p1.e164, "+554832221234");
    assert_eq!(p1.formatted, "+55 (48) 3222-1234");
    assert_eq!(p1.kind, PhoneKind::Landline);

    // Telefone fixo comercial (iniciado com 2)
    let p2 = PhoneNormalizer::normalize_br("(11) 2345-6789", None).expect("deve normalizar fixo comercial");
    assert_eq!(p2.e164, "+551123456789");
    assert_eq!(p2.formatted, "+55 (11) 2345-6789");
    assert_eq!(p2.kind, PhoneKind::Landline);
}

#[test]
fn test_phone_normalizer_default_ddd_fallback() {
    // Número sem DDD usando fallback de Florianópolis ("48")
    let p1 = PhoneNormalizer::normalize_br("99123-4567", Some("48")).expect("deve usar default DDD");
    assert_eq!(p1.e164, "+5548991234567");
    assert_eq!(p1.ddd, "48");
    assert_eq!(p1.kind, PhoneKind::Mobile);

    // Fixo sem DDD usando fallback
    let p2 = PhoneNormalizer::normalize_br("3221-0000", Some("48")).expect("deve usar default DDD em fixo");
    assert_eq!(p2.e164, "+554832210000");
    assert_eq!(p2.kind, PhoneKind::Landline);
}

#[test]
fn test_android_sync_adapter_batch_operations_and_birthday() {
    let entry = ContactEntry {
        id: "c7e2ecd0-0000-4000-8000-000000000001".to_string(),
        username: "hi@lumen.pink".to_string(),
        display_name: Some("Ada Lovelace".to_string()),
        primary_email: "ada@lovelace.org".to_string(),
        secondary_emails: vec![],
        phone_numbers: vec!["021 (48) 99123-4567".to_string()],
        birthday: Some("1815-12-10".to_string()),
        notes: Some("Pioneira da computação".to_string()),
        vcard_raw: "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Ada Lovelace\r\nBDAY:1815-12-10\r\nEND:VCARD".to_string(),
        has_autocrypt: false,
        autocrypt_fingerprint: None,
        etag: "etag123".to_string(),
        version: 1,
        created_at: 1000,
        updated_at: 1000,
        deleted_at: None,
    };

    let batch = AndroidContactBatch::from_contact_entry(&entry, "hi@lumen.pink");
    assert_eq!(batch.account_type, LAUDA_ACCOUNT_TYPE);
    assert_eq!(batch.account_name, "hi@lumen.pink");
    assert_eq!(batch.source_id, entry.id);

    // Deve conter operações para nome, email, telefone normalizado e aniversário
    let has_name = batch.operations.iter().any(|op| {
        matches!(op, AndroidRawContactOp::InsertData { mime_type, value1, .. }
            if mime_type == &AndroidMimeType::StructuredName && value1.as_deref() == Some("Ada Lovelace"))
    });
    assert!(has_name, "deve conter operação de nome estruturado");

    let has_email = batch.operations.iter().any(|op| {
        matches!(op, AndroidRawContactOp::InsertData { mime_type, value1, .. }
            if mime_type == &AndroidMimeType::Email && value1.as_deref() == Some("ada@lovelace.org"))
    });
    assert!(has_email, "deve conter operação de email");

    let has_phone = batch.operations.iter().any(|op| {
        matches!(op, AndroidRawContactOp::InsertData { mime_type, value1, value4, .. }
            if mime_type == &AndroidMimeType::Phone && value1.as_deref() == Some("+55 (48) 99123-4567") && value4.as_deref() == Some("+5548991234567"))
    });
    assert!(has_phone, "deve conter operação de telefone higienizado em E.164");

    let has_birthday = batch.operations.iter().any(|op| {
        matches!(op, AndroidRawContactOp::InsertData { mime_type, value1, .. }
            if mime_type == &AndroidMimeType::EventBirthday && value1.as_deref() == Some("1815-12-10"))
    });
    assert!(has_birthday, "deve conter operação de aniversário BDAY mapeado para Event");
}

#[test]
fn test_android_sync_tombstone_operation() {
    let tombstone = AndroidContactBatch::delete_tombstone(
        "c7e2ecd0-0000-4000-8000-000000000001",
        "hi@lumen.pink",
    );
    assert_eq!(tombstone.source_id, "c7e2ecd0-0000-4000-8000-000000000001");
    assert!(matches!(tombstone.operations.as_slice(), [AndroidRawContactOp::DeleteRawContact { .. }]));
}
