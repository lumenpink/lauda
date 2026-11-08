use lauda_client::domain::cache::get_default_cache_dir;
use lauda_client::domain::session::{SessionConfig, logout_and_wipe};
use std::path::Path;

#[test]
fn test_default_cache_dir_usable_for_logout_and_wipe() {
    let cache_dir = get_default_cache_dir();
    assert!(!cache_dir.as_os_str().is_empty());

    let session = SessionConfig {
        server_url: "https://lauda.example.com".to_string(),
        username: "testuser".to_string(),
        token: Some("secret-token".to_string()),
    };

    let cleaned = logout_and_wipe(&session, None, &cache_dir);
    assert_eq!(cleaned.server_url, "https://lauda.example.com");
    assert_eq!(cleaned.username, "testuser");
    assert_eq!(cleaned.token, None);
}

#[test]
fn test_ci_workflow_has_dx_offline_flag() {
    let ci_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".forgejo/workflows/ci.yml");

    let content = std::fs::read_to_string(ci_path).expect("ci.yml deve existir");
    assert!(
        content.contains("dx build -p lauda-client --platform web --release --offline"),
        "Workflow ci.yml deve conter a flag --offline para o comando dx build"
    );
}
