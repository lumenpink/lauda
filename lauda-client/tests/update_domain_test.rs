use lauda_client::domain::update::{check_update, UpdateManifest, UpdateStatus};

fn mock_manifest(version: &str, min_compatible_version: &str) -> UpdateManifest {
    UpdateManifest {
        version: version.to_string(),
        min_compatible_version: min_compatible_version.to_string(),
        apk_url: "https://bucket.2lp.in/lauda.apk".to_string(),
        release_notes: "Release notes de teste".to_string(),
        published_at: "2026-11-18T00:00:00Z".to_string(),
    }
}

#[test]
fn test_check_update_identical_version_must_be_up_to_date() {
    // Cenário real do bug do ticket #111:
    // A versão instalada é idêntica à versão do manifesto, mas min_compatible_version
    // é superior ("0.1.0" vs "0.0.2-...").
    let manifest = mock_manifest("0.0.2-dev.1+20261113.5b5c4c4", "0.1.0");
    let status = check_update("0.0.2-dev.1+20261113.5b5c4c4", &manifest);

    assert_eq!(
        status,
        UpdateStatus::UpToDate,
        "Versão idêntica à remota deve ser UpToDate mesmo com min_compatible_version divergente"
    );
}

#[test]
fn test_check_update_identical_version_with_v_prefix() {
    let manifest = mock_manifest("0.5.0", "0.6.0");
    let status = check_update("v0.5.0", &manifest);

    assert_eq!(
        status,
        UpdateStatus::UpToDate,
        "Versão com prefixo 'v' idêntica à remota deve ser UpToDate"
    );
}

#[test]
fn test_check_update_local_ahead_of_remote() {
    let manifest = mock_manifest("0.5.0", "0.6.0");
    let status = check_update("0.6.0", &manifest);

    assert_eq!(
        status,
        UpdateStatus::UpToDate,
        "Versão local superior à remota deve ser UpToDate"
    );
}

#[test]
fn test_check_update_legitimate_available() {
    let manifest = mock_manifest("0.5.1", "0.4.0");
    let status = check_update("0.5.0", &manifest);

    assert_eq!(
        status,
        UpdateStatus::Available(manifest),
        "Versão remota mais recente e dentro do mínimo deve ser Available"
    );
}

#[test]
fn test_check_update_legitimate_required() {
    let manifest = mock_manifest("0.6.0", "0.5.0");
    let status = check_update("0.4.0", &manifest);

    assert_eq!(
        status,
        UpdateStatus::Required(manifest),
        "Versão local abaixo do mínimo compatível com remota mais nova deve ser Required"
    );
}
