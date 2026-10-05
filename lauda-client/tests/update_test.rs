use lauda_client::domain::update::{UpdateManifest, UpdateStatus, check_update};

#[test]
fn test_parse_version_manifest() {
    let json_data = r#"{
        "version": "0.1.2",
        "min_compatible_version": "0.1.0",
        "apk_url": "https://lauda.2lp.in/lauda-latest.apk",
        "release_notes": "Correções e melhorias de performance",
        "published_at": "2026-10-02T08:00:00Z"
    }"#;

    let manifest: UpdateManifest =
        serde_json::from_str(json_data).expect("Deve desserializar manifesto");
    assert_eq!(manifest.version, "0.1.2");
    assert_eq!(manifest.min_compatible_version, "0.1.0");
    assert_eq!(manifest.apk_url, "https://lauda.2lp.in/lauda-latest.apk");
}

#[test]
fn test_check_update_available() {
    let manifest = UpdateManifest {
        version: "0.2.0".to_string(),
        min_compatible_version: "0.1.0".to_string(),
        apk_url: "https://lauda.2lp.in/lauda-latest.apk".to_string(),
        release_notes: "Nova versão disponível".to_string(),
        published_at: "2026-10-02T08:00:00Z".to_string(),
    };

    let status = check_update("0.1.5", &manifest);
    assert!(matches!(status, UpdateStatus::Available(_)));
}

#[test]
fn test_check_update_required() {
    let manifest = UpdateManifest {
        version: "0.2.0".to_string(),
        min_compatible_version: "0.2.0".to_string(),
        apk_url: "https://lauda.2lp.in/lauda-latest.apk".to_string(),
        release_notes: "Atualização obrigatória por quebra de protocolo".to_string(),
        published_at: "2026-10-02T08:00:00Z".to_string(),
    };

    let status = check_update("0.1.9", &manifest);
    assert!(matches!(status, UpdateStatus::Required(_)));
}

#[test]
fn test_check_update_up_to_date() {
    let manifest = UpdateManifest {
        version: "0.1.0".to_string(),
        min_compatible_version: "0.1.0".to_string(),
        apk_url: "https://lauda.2lp.in/lauda-latest.apk".to_string(),
        release_notes: "Mesma versão".to_string(),
        published_at: "2026-10-02T08:00:00Z".to_string(),
    };

    let status_equal = check_update("0.1.0", &manifest);
    assert_eq!(status_equal, UpdateStatus::UpToDate);

    let status_ahead = check_update("0.2.0", &manifest);
    assert_eq!(status_ahead, UpdateStatus::UpToDate);
}

#[test]
fn test_check_update_nightly_channel() {
    let manifest = UpdateManifest {
        version: "0.2.1-nightly.20261005".to_string(),
        min_compatible_version: "0.1.0".to_string(),
        apk_url: "https://lauda.2lp.in/lauda-nightly.apk".to_string(),
        release_notes: "Build experimental nightly".to_string(),
        published_at: "2026-10-05T19:35:00Z".to_string(),
    };

    let status = check_update("0.2.0", &manifest);
    assert!(matches!(status, UpdateStatus::Available(_)));

    let status_up_to_date = check_update("0.2.1", &manifest);
    assert_eq!(status_up_to_date, UpdateStatus::UpToDate);
}
