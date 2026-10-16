pub use lauda_types::ViewMode;
use pulldown_cmark::{Options, Parser, html};

/// Converte Markdown ("txt auto") em representação HTML higienizada para visualização gráfica.
pub fn parse_markdown_to_html(markdown: &str) -> String {
    // Normaliza quebras de linha CRLF e espaços não-separáveis (\u{00A0}) isolados
    let normalized = markdown.replace("\r\n", "\n").replace('\u{00A0}', " ");

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&normalized, options).map(|event| match event {
        pulldown_cmark::Event::SoftBreak => pulldown_cmark::Event::HardBreak,
        other => other,
    });
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Filtra uma lista de aliases autorizados a partir de uma busca do usuário.
pub fn filter_aliases(aliases: &[String], query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return aliases.to_vec();
    }

    aliases
        .iter()
        .filter(|alias| alias.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

/// Remove marcadores de comentários HTML (ex: <!--papeli:sig:...--> ou <!-- ... -->)
/// presentes no texto puro de e-mails, preservando o conteúdo textual e quebras de linha.
pub fn clean_comment_tags(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(start_idx) = remaining.find("<!--") {
        result.push_str(&remaining[..start_idx]);
        if let Some(end_rel) = remaining[start_idx..].find("-->") {
            let after_comment = start_idx + end_rel + 3;
            remaining = &remaining[after_comment..];
            if remaining.starts_with("\r\n") {
                remaining = &remaining[2..];
            } else if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            }
        } else if let Some(end_rel) = remaining[start_idx..].find('>') {
            let after_comment = start_idx + end_rel + 1;
            remaining = &remaining[after_comment..];
            if remaining.starts_with("\r\n") {
                remaining = &remaining[2..];
            } else if remaining.starts_with('\n') {
                remaining = &remaining[1..];
            }
        } else {
            result.push_str(&remaining[start_idx..]);
            remaining = "";
            break;
        }
    }
    result.push_str(remaining);
    result
}

/// Filtra uma lista de envelopes locais a partir de uma busca direta e instantânea no cliente.
pub fn fast_filter_envelopes(
    envelopes: &[lauda_types::MessageEnvelope],
    query: &str,
) -> Vec<lauda_types::MessageEnvelope> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return envelopes.to_vec();
    }

    envelopes
        .iter()
        .filter(|m| m.subject.to_lowercase().contains(&q) || m.sender.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

