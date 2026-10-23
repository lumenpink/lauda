#[test]
fn test_landing_page_e_readme_copy_aconchegante() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let site_html = std::fs::read_to_string(root.join("site/index.html")).expect("ler site/index.html");
    let readme = std::fs::read_to_string(root.join("README.md")).expect("ler README.md");

    let site_lower = site_html.to_lowercase();
    let readme_lower = readme.to_lowercase();

    // Termos proibidos (jargões de infraestrutura desnecessários)
    let proibidos = [
        "hospedagem soberana",
        "http range",
        "correio soberano minimalista",
        "rust puro",
    ];

    for termo in proibidos {
        assert!(
            !site_lower.contains(termo),
            "site/index.html ainda contém termo proibido: '{termo}'"
        );
        assert!(
            !readme_lower.contains(termo),
            "README.md ainda contém termo proibido: '{termo}'"
        );
    }

    // Termos que devem estar presentes
    assert!(
        site_lower.contains("aconchegante") || site_lower.contains("aconchego"),
        "site/index.html deve transmitir experiência aconchegante"
    );
    assert!(
        site_lower.contains("tranquilidade") || site_lower.contains("com calma"),
        "site/index.html deve focar em leitura com calma e tranquilidade"
    );
}
