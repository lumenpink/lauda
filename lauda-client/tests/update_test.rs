use lauda_client::update::{
    get_distribution_endpoints, get_download_url, resolve_channel, save_default_channel,
    semver_is_newer,
};
use tempfile::tempdir;

#[test]
fn test_distribution_endpoints_banca() {
    let (stable, nightly, ver) = get_distribution_endpoints("stable");
    assert_eq!(
        stable,
        "https://banca.2lp.in/downloads/lauda/stable-latest/lauda"
    );
    assert_eq!(
        nightly,
        "https://banca.2lp.in/downloads/lauda/nightly-latest/lauda"
    );
    assert_eq!(ver, "https://banca.2lp.in/downloads/lauda/version");

    assert!(!stable.contains("codeberg.org"));
    assert!(!nightly.contains("codeberg.org"));
    assert!(!ver.contains("codeberg.org"));

    let (_, _, nightly_ver) = get_distribution_endpoints("nightly");
    assert_eq!(
        nightly_ver,
        "https://banca.2lp.in/downloads/lauda/nightly-version"
    );
}

#[test]
fn test_semver_precedence() {
    assert!(semver_is_newer("0.5.0", "0.4.1"));
    assert!(!semver_is_newer("0.4.1", "0.4.1"));
    assert!(!semver_is_newer("0.4.1", "0.5.0"));
    assert!(semver_is_newer("0.4.2", "0.4.1"));
    assert!(semver_is_newer("1.0.0", "0.9.9"));
    assert!(semver_is_newer(
        "0.5.0-dev.2+20261019",
        "0.5.0-dev.1+20261019"
    ));
}

#[test]
fn test_resolve_channel_precedence() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("config.toml");

    // 1. Default without flag, env or config -> "stable"
    let ch = resolve_channel(None, None, Some(&config_file));
    assert_eq!(ch, "stable");

    // 2. Persisted config -> "nightly"
    save_default_channel("nightly", Some(&config_file)).unwrap();
    let ch = resolve_channel(None, None, Some(&config_file));
    assert_eq!(ch, "nightly");

    // 3. Env var overrides config
    let ch = resolve_channel(None, Some("stable"), Some(&config_file));
    assert_eq!(ch, "stable");

    // 4. CLI flag overrides everything
    let ch = resolve_channel(Some("nightly"), Some("stable"), Some(&config_file));
    assert_eq!(ch, "nightly");
}

#[test]
fn test_get_download_url_and_pinning() {
    assert_eq!(
        get_download_url("stable", None),
        "https://banca.2lp.in/downloads/lauda/stable-latest/lauda"
    );
    assert_eq!(
        get_download_url("nightly", None),
        "https://banca.2lp.in/downloads/lauda/nightly-latest/lauda"
    );
    assert_eq!(
        get_download_url("stable", Some("0.5.0")),
        "https://banca.2lp.in/downloads/lauda/releases/v0.5.0/lauda"
    );
    assert_eq!(
        get_download_url("stable", Some("v0.6.0")),
        "https://banca.2lp.in/downloads/lauda/releases/v0.6.0/lauda"
    );
}

#[test]
fn test_semver_edge_cases_with_prefixes() {
    assert!(semver_is_newer("v0.5.0", "0.4.1"));
    assert!(semver_is_newer("0.5.0", "v0.4.1"));
    assert!(!semver_is_newer("v0.4.1", "0.4.1"));
    assert!(!semver_is_newer("0.4.1", "v0.4.1"));
    assert!(semver_is_newer("0.5.0", "0.4.99"));
}

#[test]
fn test_resolve_channel_env_empty() {
    let tmp = tempdir().unwrap();
    let config_file = tmp.path().join("config.toml");

    let ch = resolve_channel(None, Some("   "), Some(&config_file));
    assert_eq!(ch, "stable");
}

#[test]
fn test_discontinued_autoupdate_banner_in_source() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let update_rs = manifest_dir.join("src").join("update.rs");
    let content = std::fs::read_to_string(&update_rs).expect("update.rs deve existir");
    assert!(
        !content.contains("⚡ Lauda Auto-Update"),
        "A string legada '⚡ Lauda Auto-Update' deve ser removida em favor de '⚡ Lauda Updater // Atualização Manual' (#55)"
    );
    assert!(
        content.contains("⚡ Lauda Updater"),
        "O cabeçalho do atualizador manual deve conter '⚡ Lauda Updater'"
    );
}
