use std::path::PathBuf;

#[test]
fn test_android_syncadapter_resources_contract() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let res_xml_dir = manifest_dir.join("android/res/xml");

    // 1. Validar authenticator.xml
    let auth_xml_path = res_xml_dir.join("authenticator.xml");
    assert!(auth_xml_path.exists(), "authenticator.xml deve existir");
    let auth_content = std::fs::read_to_string(&auth_xml_path).unwrap();
    assert!(
        auth_content.contains("android:accountType=\"in.twolp.lauda\""),
        "authenticator.xml deve definir a conta in.twolp.lauda"
    );

    // 2. Validar syncadapter.xml (Contatos)
    let sync_xml_path = res_xml_dir.join("syncadapter.xml");
    assert!(sync_xml_path.exists(), "syncadapter.xml deve existir");
    let sync_content = std::fs::read_to_string(&sync_xml_path).unwrap();
    assert!(
        sync_content.contains("android:contentAuthority=\"com.android.contacts\""),
        "syncadapter.xml deve apontar para com.android.contacts"
    );
    assert!(
        sync_content.contains("android:accountType=\"in.twolp.lauda\""),
        "syncadapter.xml deve pertencer a in.twolp.lauda"
    );

    // 3. Validar calendar_syncadapter.xml (Calendário)
    let cal_xml_path = res_xml_dir.join("calendar_syncadapter.xml");
    assert!(
        cal_xml_path.exists(),
        "calendar_syncadapter.xml deve existir"
    );
    let cal_content = std::fs::read_to_string(&cal_xml_path).unwrap();
    assert!(
        cal_content.contains("android:contentAuthority=\"com.android.calendar\""),
        "calendar_syncadapter.xml deve apontar para com.android.calendar"
    );
    assert!(
        cal_content.contains("android:accountType=\"in.twolp.lauda\""),
        "calendar_syncadapter.xml deve pertencer a in.twolp.lauda"
    );
}

#[test]
fn test_android_kotlin_services_exist() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let android_dir = manifest_dir.join("android");

    let auth_kt = android_dir.join("LaudaAuthenticatorService.kt");
    let sync_kt = android_dir.join("LaudaSyncService.kt");
    let cal_kt = android_dir.join("LaudaCalendarSyncService.kt");

    assert!(
        auth_kt.exists(),
        "LaudaAuthenticatorService.kt deve existir"
    );
    assert!(sync_kt.exists(), "LaudaSyncService.kt deve existir");
    assert!(cal_kt.exists(), "LaudaCalendarSyncService.kt deve existir");

    let auth_content = std::fs::read_to_string(&auth_kt).unwrap();
    assert!(
        auth_content.contains("package `in`.twolp.lauda"),
        "Pacote canônico `in`.twolp.lauda com escape de palavra reservada"
    );
    assert!(
        auth_content.contains("class LaudaAuthenticatorService"),
        "Declaração de LaudaAuthenticatorService"
    );

    let sync_content = std::fs::read_to_string(&sync_kt).unwrap();
    assert!(
        sync_content.contains("package `in`.twolp.lauda"),
        "Pacote canônico `in`.twolp.lauda em LaudaSyncService"
    );
    assert!(
        sync_content.contains("class LaudaSyncService"),
        "Declaração de LaudaSyncService"
    );

    let cal_content = std::fs::read_to_string(&cal_kt).unwrap();
    assert!(
        cal_content.contains("package `in`.twolp.lauda"),
        "Pacote canônico `in`.twolp.lauda em LaudaCalendarSyncService"
    );
}

#[test]
fn test_publish_nightly_script_injects_syncadapters_and_permissions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script_path = manifest_dir.join("../scripts/publish-nightly.sh");
    assert!(script_path.exists(), "publish-nightly.sh deve existir");

    let script_content = std::fs::read_to_string(&script_path).unwrap();

    // 1. Deve copiar os serviços Kotlin para o app Gradle
    assert!(
        script_content.contains("LaudaAuthenticatorService.kt")
            && script_content.contains("LaudaSyncService.kt")
            && script_content.contains("LaudaCalendarSyncService.kt"),
        "publish-nightly.sh deve sincronizar os serviços Kotlin do Lauda"
    );

    // 2. Deve injetar as permissões obrigatórias no AndroidManifest.xml
    let required_permissions = [
        "android.permission.READ_CONTACTS",
        "android.permission.WRITE_CONTACTS",
        "android.permission.READ_CALENDAR",
        "android.permission.WRITE_CALENDAR",
        "android.permission.GET_ACCOUNTS",
        "android.permission.MANAGE_ACCOUNTS",
        "android.permission.AUTHENTICATE_ACCOUNTS",
        "android.permission.READ_SYNC_SETTINGS",
        "android.permission.WRITE_SYNC_SETTINGS",
    ];

    for perm in required_permissions {
        assert!(
            script_content.contains(perm),
            "publish-nightly.sh deve injetar a permissão {}",
            perm
        );
    }

    // 3. Deve injetar as tags de serviço no AndroidManifest.xml
    assert!(
        script_content.contains("in.twolp.lauda.LaudaAuthenticatorService"),
        "publish-nightly.sh deve registrar LaudaAuthenticatorService no manifesto"
    );
    assert!(
        script_content.contains("in.twolp.lauda.LaudaSyncService"),
        "publish-nightly.sh deve registrar LaudaSyncService no manifesto"
    );
    assert!(
        script_content.contains("in.twolp.lauda.LaudaCalendarSyncService"),
        "publish-nightly.sh deve registrar LaudaCalendarSyncService no manifesto"
    );
}
