pub use lauda_types::ViewMode;
use pulldown_cmark::{Options, Parser, html};

/// Converte Markdown ("txt auto") em representação HTML higienizada para visualização gráfica.
pub fn parse_markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);
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
