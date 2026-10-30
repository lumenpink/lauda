use lauda_client::domain::view::parse_markdown_to_html;

#[test]
fn test_autolink_pure_urls_http_and_https() {
    let input =
        "Visite https://2lp.in para conferir o projeto e http://localhost:8432/api para testar.";
    let html = parse_markdown_to_html(input);

    assert!(
        html.contains(r#"<a href="https://2lp.in">https://2lp.in</a>"#),
        "URL https pura deve ser convertida em hiperlink. HTML: {html}"
    );
    assert!(
        html.contains(r#"<a href="http://localhost:8432/api">http://localhost:8432/api</a>"#),
        "URL http pura deve ser convertida em hiperlink. HTML: {html}"
    );
}

#[test]
fn test_autolink_urls_with_trailing_punctuation_and_parentheses() {
    let input = "Confira em https://rust-lang.org, que linguagem! E veja (https://docs.rs/dioxus).";
    let html = parse_markdown_to_html(input);

    assert!(
        html.contains(r#"<a href="https://rust-lang.org">https://rust-lang.org</a>,"#),
        "A vírgula não deve fazer parte da URL. HTML: {html}"
    );
    assert!(
        html.contains(r#"(<a href="https://docs.rs/dioxus">https://docs.rs/dioxus</a>)."#),
        "O parêntese e ponto final não devem fazer parte da URL. HTML: {html}"
    );
}

#[test]
fn test_autolink_bare_emails_with_mailto() {
    let input = "Para suporte envie mensagem a lumen@2lp.in ou contato@empresa.com.br; aguardamos.";
    let html = parse_markdown_to_html(input);

    assert!(
        html.contains(r#"<a href="mailto:lumen@2lp.in">lumen@2lp.in</a>"#),
        "E-mail avulso deve virar link mailto:. HTML: {html}"
    );
    assert!(
        html.contains(r#"<a href="mailto:contato@empresa.com.br">contato@empresa.com.br</a>;"#),
        "E-mail avulso com ponto e vírgula subsequente deve manter a pontuação fora do link. HTML: {html}"
    );
}

#[test]
fn test_autolink_code_blocks_and_inline_code_are_not_converted() {
    let input = "Use o comando `curl https://example.com` ou o bloco:\n```\nhttps://example.com\nuser@example.com\n```";
    let html = parse_markdown_to_html(input);

    assert!(
        !html.contains(r#"<a href="https://example.com">"#),
        "URLs dentro de código inline ou bloco de código não devem ser convertidas em links. HTML: {html}"
    );
    assert!(
        !html.contains(r#"<a href="mailto:user@example.com">"#),
        "E-mails dentro de bloco de código não devem ser convertidos em links. HTML: {html}"
    );
    assert!(html.contains("<code>"));
}

#[test]
fn test_autolink_does_not_nest_inside_existing_markdown_links() {
    let input = "Acesse o [Portal](https://example.com) ou leia <https://example.org>.";
    let html = parse_markdown_to_html(input);

    // Deve conter exatamente uma tag <a> por link, sem <a href="..."><a href="...">
    assert!(!html.contains("<a href=\"https://example.com\"><a"));
    assert!(html.contains(r#"<a href="https://example.com">Portal</a>"#));
    assert!(html.contains(r#"<a href="https://example.org">https://example.org</a>"#));
}

#[test]
fn test_autolink_mutations_complex_query_params_and_anchors() {
    let input = "Endpoint: https://api.2lp.in/v1/search?q=teste&limit=50#results! Visite agora?";
    let html = parse_markdown_to_html(input);

    assert!(
        html.contains(r#"<a href="https://api.2lp.in/v1/search?q=teste&amp;limit=50#results">https://api.2lp.in/v1/search?q=teste&amp;limit=50#results</a>!"#),
        "Query params e âncoras devem ser preservados, mas o ponto de exclamação terminal descartado. HTML: {html}"
    );
}

#[test]
fn test_autolink_mutations_balanced_parentheses_in_url() {
    let input =
        "Leia em (https://pt.wikipedia.org/wiki/Rust_(linguagem_de_programa%C3%A7%C3%A3o)).";
    let html = parse_markdown_to_html(input);

    assert!(
        html.contains(r#"(<a href="https://pt.wikipedia.org/wiki/Rust_(linguagem_de_programa%C3%A7%C3%A3o)">https://pt.wikipedia.org/wiki/Rust_(linguagem_de_programa%C3%A7%C3%A3o)</a>)."#),
        "Parênteses internos da URL devem ser preservados e o parêntese externo descartado. HTML: {html}"
    );
}

#[test]
fn test_autolink_mutations_multiple_mixed_tokens_in_text() {
    let input =
        "Contate dev@2lp.in ou fale em https://forum.2lp.in: dúvidas com admin@empresa.com.br?";
    let html = parse_markdown_to_html(input);

    assert!(html.contains(r#"<a href="mailto:dev@2lp.in">dev@2lp.in</a>"#));
    assert!(html.contains(r#"<a href="https://forum.2lp.in">https://forum.2lp.in</a>:"#));
    assert!(html.contains(r#"<a href="mailto:admin@empresa.com.br">admin@empresa.com.br</a>?"#));
}

#[test]
fn test_autolink_mutations_false_positives_not_linked() {
    let input = "Apenas texto com @ sem email, user@.com inválido, http:// solitário e normal@tld.";
    let html = parse_markdown_to_html(input);

    assert!(!html.contains("<a href=\"mailto:@\">"));
    assert!(!html.contains("<a href=\"mailto:user@.com\">"));
    assert!(!html.contains("<a href=\"http://\">"));
    // normal@tld não tem pelo menos duas letras após o ponto do domínio
    assert!(!html.contains("<a href=\"mailto:normal@tld\">"));
}
