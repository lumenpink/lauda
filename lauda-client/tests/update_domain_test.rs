use lauda_client::domain::update::{
    UpdateManifest, UpdateStatus, check_update, should_show_update_banner,
};

fn mock_manifest(version: &str, min_compatible_version: &str) -> UpdateManifest {
    UpdateManifest {
        version: version.to_string(),
        min_compatible_version: min_compatible_version.to_string(),
        apk_url: "https://bucket.2lp.in/pratilera/lauda.apk".to_string(),
        release_notes: "Release notes de teste".to_string(),
        published_at: "2026-11-18T00:00:00Z".to_string(),
    }
}

#[test]
fn test_check_update_identical_version_must_be_up_to_date() {
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
fn test_check_update_identical_version_with_spaces() {
    let manifest = mock_manifest(" 0.5.0 ", "0.6.0");
    let status = check_update("  v0.5.0  ", &manifest);

    assert_eq!(
        status,
        UpdateStatus::UpToDate,
        "Versão com espaços e prefixo 'v' deve ser UpToDate"
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

#[test]
fn test_check_update_mutation_garbage_min_compatible() {
    // Se min_compatible_version estiver vazia ou for lixo textual,
    // triplet de min avalia para (0, 0, 0), portanto a atualização é opcional (Available)
    let manifest = mock_manifest("0.6.0", "invalid-semver");
    let status = check_update("0.5.0", &manifest);

    assert_eq!(
        status,
        UpdateStatus::Available(manifest),
        "Min compatível inválido não deve quebrar nem forçar Required indevido"
    );
}

#[test]
fn test_should_show_update_banner_logic() {
    let manifest = mock_manifest("0.6.0", "0.5.0");

    // Banner exibido se não foi dispensado
    assert!(should_show_update_banner(Some(&manifest), None));

    // Banner ocultado se a versão dispensada for a mesma
    assert!(!should_show_update_banner(Some(&manifest), Some("0.6.0")));

    // Banner reexibido se uma versão mais nova chegar após dispensa anterior
    assert!(should_show_update_banner(Some(&manifest), Some("0.5.0")));

    // Banner não exibido se manifest for None
    assert!(!should_show_update_banner(None, None));
}
