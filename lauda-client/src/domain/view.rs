pub use lauda_types::{MessageEnvelope, ViewMode};
use pulldown_cmark::{Options, Parser, html};

/// Remove pontuações terminais de frases anexadas a URLs.
fn trim_url_punctuation(raw: &str) -> (&str, &str) {
    let mut url = raw;
    let mut trailing = &raw[raw.len()..];

    while !url.is_empty() {
        let last_char = url.chars().last().unwrap();
        if last_char == '.'
            || last_char == ','
            || last_char == ';'
            || last_char == '!'
            || last_char == '?'
            || last_char == ':'
            || last_char == '\''
            || last_char == '"'
            || last_char == '>'
        {
            let prev_len = url.len() - last_char.len_utf8();
            url = &url[..prev_len];
            trailing = &raw[prev_len..];
        } else if last_char == ')' {
            let opens = url.chars().filter(|&c| c == '(').count();
            let closes = url.chars().filter(|&c| c == ')').count();
            if closes > opens {
                let prev_len = url.len() - 1;
                url = &url[..prev_len];
                trailing = &raw[prev_len..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (url, trailing)
}

/// Remove pontuações terminais anexadas a endereços de e-mail.
fn trim_email_punctuation(raw: &str) -> (&str, &str) {
    let mut email = raw;
    let mut trailing = &raw[raw.len()..];

    while !email.is_empty() {
        let last_char = email.chars().last().unwrap();
        if last_char == '.'
            || last_char == ','
            || last_char == ';'
            || last_char == '!'
            || last_char == '?'
            || last_char == ':'
            || last_char == '\''
            || last_char == '"'
            || last_char == ')'
            || last_char == ']'
            || last_char == '>'
        {
            let prev_len = email.len() - last_char.len_utf8();
            email = &email[..prev_len];
            trailing = &raw[prev_len..];
        } else {
            break;
        }
    }
    (email, trailing)
}

/// Processa uma linha de texto identificando URLs puras e e-mails avulsos para autolink CommonMark.
fn autolink_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 16);
    let chars: Vec<char> = line.chars().collect();
    let char_count = chars.len();
    let mut i = 0;

    while i < char_count {
        // 1. Inline code: `...`
        if chars[i] == '`' {
            let mut tick_count = 0;
            while i + tick_count < char_count && chars[i + tick_count] == '`' {
                tick_count += 1;
            }
            for _ in 0..tick_count {
                out.push('`');
            }
            i += tick_count;
            let mut closed = false;
            let mut j = i;
            while j < char_count {
                if chars[j] == '`' {
                    let mut close_count = 0;
                    while j + close_count < char_count && chars[j + close_count] == '`' {
                        close_count += 1;
                    }
                    if close_count == tick_count {
                        out.extend(&chars[i..j]);
                        for _ in 0..tick_count {
                            out.push('`');
                        }
                        i = j + tick_count;
                        closed = true;
                        break;
                    }
                    j += close_count;
                } else {
                    j += 1;
                }
            }
            if !closed {
                out.extend(&chars[i..char_count]);
                i = char_count;
            }
            continue;
        }

        // 2. Autolink existente ou tag HTML: <...>
        if chars[i] == '<'
            && let Some(close_idx) = chars[i + 1..].iter().position(|&c| c == '>')
        {
            let end = i + 1 + close_idx;
            out.extend(&chars[i..=end]);
            i = end + 1;
            continue;
        }

        // 3. Destino de link Markdown: ](...)
        if chars[i] == ']'
            && i + 1 < char_count
            && chars[i + 1] == '('
            && let Some(close_idx) = chars[i + 2..].iter().position(|&c| c == ')')
        {
            let end = i + 2 + close_idx;
            out.extend(&chars[i..=end]);
            i = end + 1;
            continue;
        }

        // 4. URL pura candidata: http:// ou https://
        let remaining_str: String = chars[i..].iter().collect();
        if (remaining_str.starts_with("http://") || remaining_str.starts_with("https://"))
            && (i == 0
                || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '@' && chars[i - 1] != '/')
        {
            let mut end = i;
            while end < char_count
                && !chars[end].is_whitespace()
                && chars[end] != '<'
                && chars[end] != '>'
            {
                end += 1;
            }
            let raw_url: String = chars[i..end].iter().collect();
            let (clean_url, trailing) = trim_url_punctuation(&raw_url);
            if clean_url.starts_with("http://") && clean_url.len() > 7
                || clean_url.starts_with("https://") && clean_url.len() > 8
            {
                out.push('<');
                out.push_str(clean_url);
                out.push('>');
                out.push_str(trailing);
                i = end;
                continue;
            }
        }

        // 5. E-mail avulso candidato: usuario@dominio.tld
        if (i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '@' && chars[i - 1] != '.')
            && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '+' || chars[i] == '-')
        {
            let mut end = i;
            while end < char_count
                && !chars[end].is_whitespace()
                && chars[end] != '<'
                && chars[end] != '>'
                && chars[end] != '('
            {
                end += 1;
            }
            let word: String = chars[i..end].iter().collect();
            if let Some(at_idx) = word.find('@')
                && at_idx > 0
            {
                let local = &word[..at_idx];
                let raw_domain = &word[at_idx + 1..];
                let (clean_domain, trailing) = trim_email_punctuation(raw_domain);
                if local.chars().all(|c| {
                    c.is_alphanumeric() || c == '.' || c == '_' || c == '%' || c == '+' || c == '-'
                }) && !local.starts_with('.')
                    && !local.ends_with('.')
                    && clean_domain.contains('.')
                    && !clean_domain.starts_with('.')
                    && !clean_domain.ends_with('.')
                    && let Some(dot_idx) = clean_domain.rfind('.')
                {
                    let tld = &clean_domain[dot_idx + 1..];
                    if tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()) {
                        out.push('<');
                        out.push_str(local);
                        out.push('@');
                        out.push_str(clean_domain);
                        out.push('>');
                        out.push_str(trailing);
                        i = end;
                        continue;
                    }
                }
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Pre-processa o Markdown inserindo delimitadores de autolink (<...>) em URLs puras e e-mails avulsos,
/// protegendo blocos de código e links já existentes.
pub fn autolink_raw_markdown(markdown: &str) -> String {
    let mut result = String::with_capacity(markdown.len() + 64);
    let mut in_fenced_code = false;
    let mut fence_char = ' ';

    for line in markdown.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let f = trimmed.chars().next().unwrap();
            if in_fenced_code {
                if f == fence_char {
                    in_fenced_code = false;
                }
            } else {
                in_fenced_code = true;
                fence_char = f;
            }
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_fenced_code {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        let processed_line = autolink_line(line);
        result.push_str(&processed_line);
        result.push('\n');
    }

    if !markdown.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Converte Markdown ("txt auto") em representação HTML higienizada para visualização gráfica,
/// habilitando autolink de URLs puras e e-mails avulsos de forma não-intrusiva.
pub fn parse_markdown_to_html(markdown: &str) -> String {
    // Normaliza quebras de linha CRLF e espaços não-separáveis (\u{00A0}) isolados
    let normalized = markdown.replace("\r\n", "\n").replace('\u{00A0}', " ");
    let autolinked = autolink_raw_markdown(&normalized);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&autolinked, options).map(|event| match event {
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

/// Limiar mínimo de caracteres para ativar a busca semântica/local no cliente.
pub const MIN_SEARCH_QUERY_LEN: usize = 3;

/// Informa se a busca deve ser ativada para o termo fornecido.
pub fn is_search_active(query: &str) -> bool {
    query.trim().chars().count() >= MIN_SEARCH_QUERY_LEN
}

/// Informa se o aviso explicativo de digitação mínima deve ser exibido (1 ou 2 caracteres).
pub fn should_show_search_hint(query: &str) -> bool {
    let count = query.trim().chars().count();
    count > 0 && count < MIN_SEARCH_QUERY_LEN
}

/// Formata a mensagem dinâmica de caracteres restantes para a busca rápida.
/// Retorna `None` se o termo estiver vazio ou já possuir caracteres suficientes (>= MIN_SEARCH_QUERY_LEN).
pub fn format_search_hint(query: &str) -> Option<String> {
    let count = query.trim().chars().count();
    if count == 0 || count >= MIN_SEARCH_QUERY_LEN {
        None
    } else {
        let remaining = MIN_SEARCH_QUERY_LEN - count;
        if remaining == 1 {
            Some("FALTA 1 CARACTERE PARA PESQUISAR...".to_string())
        } else {
            Some(format!("FALTAM {remaining} CARACTERES PARA PESQUISAR..."))
        }
    }
}

/// Filtra uma lista de envelopes locais a partir de uma busca direta e instantânea no cliente.
/// Caso o termo possua menos de `MIN_SEARCH_QUERY_LEN` caracteres, a lista original é preservada
/// para evitar falsos negativos na interface enquanto o usuário digita.
pub fn fast_filter_envelopes(
    envelopes: &[lauda_types::MessageEnvelope],
    query: &str,
) -> Vec<lauda_types::MessageEnvelope> {
    let q = query.trim().to_lowercase();
    if q.chars().count() < MIN_SEARCH_QUERY_LEN {
        return envelopes.to_vec();
    }

    envelopes
        .iter()
        .filter(|m| m.subject.to_lowercase().contains(&q) || m.sender.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

/// Estado do fluxo de download e gravação de anexo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentSaveState {
    Downloading,
    Saving,
    Saved(String),
    Failed(String),
}

/// Formata a mensagem de status para exibição ao usuário durante o download/salvamento de anexo.
pub fn format_attachment_status(filename: &str, state: AttachmentSaveState) -> String {
    match state {
        AttachmentSaveState::Downloading => format!("⌛ BAIXANDO ANEXO: {filename}..."),
        AttachmentSaveState::Saving => format!("💾 SALVANDO EM DISCO: {filename}..."),
        AttachmentSaveState::Saved(path) => format!("✓ Salvo em {path}"),
        AttachmentSaveState::Failed(err) => format!("Falha no salvamento: {err}"),
    }
}

/// Executa uma closure bloqueante fora da thread da UI (via worker thread e oneshot channel em plataformas nativas).
#[cfg(not(target_arch = "wasm32"))]
pub async fn run_blocking_task<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let res = f();
        let _ = tx.send(res);
    });
    rx.await
        .map_err(|_| "Falha ao executar tarefa em background".to_string())
}

/// Execução direta para alvos WebAssembly (single-threaded).
#[cfg(target_arch = "wasm32")]
pub async fn run_blocking_task<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce() -> R + 'static,
    R: 'static,
{
    Ok(f())
}

/// Grava bytes de um anexo em disco de forma assíncrona desacoplada da thread da interface.
pub async fn save_attachment_bytes(
    target: &std::path::Path,
    bytes: &[u8],
) -> Result<(), std::io::Error> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let target = target.to_path_buf();
        let bytes = bytes.to_vec();
        run_blocking_task(move || {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&target, bytes)
        })
        .await
        .map_err(std::io::Error::other)?
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (target, bytes);
        Ok(())
    }
}

/// Busca contatos de forma não-bloqueante para autocomplete na interface.
pub async fn search_contacts_async(
    cache_dir: std::path::PathBuf,
    query: String,
    limit: usize,
) -> Vec<lauda_types::contact::ContactEntry> {
    run_blocking_task(move || {
        if let Ok(cache) = crate::domain::contacts_cache::LocalContactsCache::open(&cache_dir) {
            cache.search_contacts(&query, limit).unwrap_or_default()
        } else {
            Vec::new()
        }
    })
    .await
    .unwrap_or_default()
}

/// Busca difusa (fuzzy) de contatos de forma não-bloqueante para autocomplete na agenda.
pub async fn search_contacts_fuzzy_async(
    cache_dir: std::path::PathBuf,
    query: String,
    limit: usize,
) -> Vec<lauda_types::contact::ContactEntry> {
    run_blocking_task(move || {
        if let Ok(cache) = crate::domain::contacts_cache::LocalContactsCache::open(&cache_dir) {
            cache
                .search_contacts_fuzzy(&query, limit)
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    })
    .await
    .unwrap_or_default()
}

/// Estilo base do container raiz do MessageView com contenção estrita de flexbox e viewport.
pub const MESSAGE_VIEW_ROOT_STYLE: &str = "display: flex; flex-direction: column; gap: 1rem; width: 100%; max-width: 100%; min-width: 0; box-sizing: border-box;";

/// Estilo base do painel do corpo da mensagem com contenção de overflow e quebra de palavras.
pub const MESSAGE_BODY_CONTAINER_STYLE: &str = "width: 100%; max-width: 100%; min-width: 0; box-sizing: border-box; overflow-x: auto; overflow-wrap: break-word; word-break: break-word;";

/// Envelopa o HTML bruto de um e-mail com regras CSS de contenção para impedir transbordamento de layout.
pub fn format_html_body_contained(html: &str) -> String {
    format!(
        r#"<style>
.lauda-html-body {{
    max-width: 100% !important;
    overflow-x: auto !important;
    overflow-wrap: break-word !important;
    word-break: break-word !important;
    box-sizing: border-box !important;
}}
.lauda-html-body table {{
    max-width: 100% !important;
    word-break: break-word !important;
}}
.lauda-html-body img {{
    max-width: 100% !important;
    height: auto !important;
}}
.lauda-html-body pre {{
    white-space: pre-wrap !important;
    word-break: break-word !important;
}}
.lauda-html-body .paper-sheet > table {{
    min-height: 848px !important;
    height: 100% !important;
}}
.lauda-html-body .paper-sheet > table > tbody > tr > td {{
    height: 100% !important;
}}
</style><div class="lauda-html-body">{html}</div>"#
    )
}

/// Envelopa o HTML gerado a partir de Markdown (txtauto) com regras CSS de contenção para tabelas, código e imagens.
pub fn format_txtauto_body_contained(markdown_html: &str) -> String {
    format!(
        r#"<style>
.lauda-txtauto-body {{
    max-width: 100%;
    overflow-x: auto;
    overflow-wrap: break-word;
    word-break: break-word;
    line-height: 1.6;
    font-size: 1rem;
    color: #0a0a0a;
    box-sizing: border-box;
}}
.lauda-txtauto-body pre, .lauda-txtauto-body code {{
    white-space: pre-wrap;
    word-break: break-all;
    max-width: 100%;
    overflow-x: auto;
}}
.lauda-txtauto-body table {{
    max-width: 100%;
    display: block;
    overflow-x: auto;
    border-collapse: collapse;
    word-break: break-word;
}}
.lauda-txtauto-body img {{
    max-width: 100%;
    height: auto;
}}
</style><div class="lauda-txtauto-body">{markdown_html}</div>"#
    )
}

/// Mescla resultados ranqueados de busca local e remota aplicando Reciprocal Rank Fusion (RRF).
/// Garante boost de relevância para mensagens confirmadas em ambos os fluxos e deduplicação estrita.
pub fn merge_search_results(
    local: &[MessageEnvelope],
    remote: &[MessageEnvelope],
) -> Vec<MessageEnvelope> {
    let mut scores: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
    let mut envelopes: std::collections::HashMap<u32, MessageEnvelope> =
        std::collections::HashMap::new();

    for (rank, env) in local.iter().enumerate() {
        let rrf = 1.0 / (60.0 + rank as f64);
        *scores.entry(env.uid).or_insert(0.0) += rrf;
        envelopes.entry(env.uid).or_insert_with(|| env.clone());
    }

    for (rank, env) in remote.iter().enumerate() {
        let rrf = 1.0 / (60.0 + rank as f64);
        *scores.entry(env.uid).or_insert(0.0) += rrf;
        envelopes.entry(env.uid).or_insert_with(|| env.clone());
    }

    let mut ranked_uids: Vec<(u32, f64)> = scores.into_iter().collect();
    ranked_uids.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.0.cmp(&a.0))
    });

    ranked_uids
        .into_iter()
        .filter_map(|(uid, _)| envelopes.remove(&uid))
        .collect()
}
