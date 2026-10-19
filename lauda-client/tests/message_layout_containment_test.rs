use lauda_client::domain::view::{
    MESSAGE_BODY_CONTAINER_STYLE, MESSAGE_VIEW_ROOT_STYLE, format_html_body_contained,
    format_txtauto_body_contained,
};

#[test]
fn test_message_view_root_and_container_containment() {
    // 1. O container raiz do MessageView deve impedir crescimento infinito em layouts flex
    assert!(
        MESSAGE_VIEW_ROOT_STYLE.contains("min-width: 0"),
        "Container raiz deve conter min-width: 0 para flex item shrinking"
    );
    assert!(
        MESSAGE_VIEW_ROOT_STYLE.contains("max-width: 100%"),
        "Container raiz deve conter max-width: 100%"
    );
    assert!(
        MESSAGE_VIEW_ROOT_STYLE.contains("width: 100%"),
        "Container raiz deve conter width: 100%"
    );
    assert!(
        MESSAGE_VIEW_ROOT_STYLE.contains("box-sizing: border-box"),
        "Container raiz deve conter box-sizing: border-box"
    );

    // 2. O container do corpo da mensagem deve prevenir transbordamento horizontal
    assert!(
        MESSAGE_BODY_CONTAINER_STYLE.contains("max-width: 100%"),
        "Container do corpo da mensagem deve conter max-width: 100%"
    );
    assert!(
        MESSAGE_BODY_CONTAINER_STYLE.contains("overflow-x: auto"),
        "Container do corpo da mensagem deve conter overflow-x: auto"
    );
    assert!(
        MESSAGE_BODY_CONTAINER_STYLE.contains("overflow-wrap: break-word"),
        "Container do corpo da mensagem deve conter overflow-wrap: break-word"
    );
    assert!(
        MESSAGE_BODY_CONTAINER_STYLE.contains("word-break: break-word"),
        "Container do corpo da mensagem deve conter word-break: break-word"
    );
    assert!(
        MESSAGE_BODY_CONTAINER_STYLE.contains("min-width: 0"),
        "Container do corpo da mensagem deve conter min-width: 0"
    );
}

#[test]
fn test_html_body_containment_wrapper() {
    let raw_html = "<table width='2000'><tr><td>Conteudo Extenso</td></tr></table>";
    let wrapped = format_html_body_contained(raw_html);

    assert!(wrapped.contains("class=\"lauda-html-body\""));
    assert!(wrapped.contains("max-width: 100% !important"));
    assert!(wrapped.contains("overflow-x: auto"));
    assert!(wrapped.contains("table"));
    assert!(wrapped.contains("word-break: break-word"));
    assert!(wrapped.contains(raw_html));
}

#[test]
fn test_txtauto_body_containment_wrapper() {
    let markdown_html = "<p>Paragrafo normal</p><pre><code>codigo_muito_longo_sem_espacos_1234567890_abcdefghijklmnopqrstuvwxyz</code></pre>";
    let wrapped = format_txtauto_body_contained(markdown_html);

    assert!(wrapped.contains("class=\"lauda-txtauto-body\""));
    assert!(wrapped.contains("max-width: 100%"));
    assert!(wrapped.contains("overflow-x: auto"));
    assert!(wrapped.contains("white-space: pre-wrap"));
    assert!(wrapped.contains("word-break: break-word"));
    assert!(wrapped.contains(markdown_html));
}
