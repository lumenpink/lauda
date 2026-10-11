fn main() {
    let version_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("VERSION"))
        .unwrap_or_else(|| std::path::PathBuf::from("../VERSION"));

    let version = if version_path.exists() {
        std::fs::read_to_string(&version_path)
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    let display_version = if version.is_empty() {
        env!("CARGO_PKG_VERSION").to_string()
    } else {
        version
    };

    println!("cargo:rustc-env=LAUDA_APP_VERSION={}", display_version);
    println!("cargo:rerun-if-changed={}", version_path.display());
}
