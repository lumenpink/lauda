use serde::{Deserialize, Serialize};

/// Modos de visualização suportados pelo smolclient Lauda.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewMode {
    /// Renderização Markdown limpa gerada pelo pipeline do Urupema.
    #[default]
    TxtAuto,
    /// Texto cru original preservando quebras de linha e formatação ASCII original.
    TxtOriginal,
    /// Conteúdo HTML higienizado em sandbox.
    Html,
}

impl ViewMode {
    /// Alterna para o próximo modo no ciclo: TxtAuto -> Html -> TxtOriginal -> TxtAuto.
    pub fn next(&self) -> Self {
        match self {
            Self::TxtAuto => Self::Html,
            Self::Html => Self::TxtOriginal,
            Self::TxtOriginal => Self::TxtAuto,
        }
    }

    /// Rótulo conciso com glifo rotativo para botões compactos Trash Polka.
    pub fn label(&self) -> &'static str {
        match self {
            Self::TxtAuto => "TXT AUTO ⟳",
            Self::Html => "HTML ⟳",
            Self::TxtOriginal => "ORIGINAL ⟳",
        }
    }
}

/// Metadados resumidos de um anexo da mensagem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AttachmentMeta {
    /// Identificador/índice do anexo na mensagem.
    pub id: usize,
    /// Nome original do arquivo.
    pub filename: String,
    /// Tipo MIME do anexo.
    pub content_type: String,
    /// Tamanho em bytes.
    pub size_bytes: usize,
}

/// Metadados resumidos de uma mensagem para listagens de caixa de entrada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MessageEnvelope {
    /// Identificador estável IMAP (UID).
    pub uid: u32,
    /// Assunto da mensagem.
    pub subject: String,
    /// Remetente formatado ("Nome <email@dominio>").
    pub sender: String,
    /// Data no formato ISO-8601 UTC.
    pub date: String,
    /// Flag booleana de mensagem lida (\Seen).
    pub is_read: bool,
    /// Flag booleana de mensagem marcada (\Flagged).
    pub is_flagged: bool,
    /// Destinatários em cópia (CC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    /// Indica se a mensagem possui anexos.
    #[serde(default)]
    pub has_attachments: bool,
    /// Metadados de segurança criptográfica (PGP / Ishcondidim).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crypto_meta: Option<CryptoMeta>,
}

/// Status da assinatura digital OpenPGP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    /// Sem assinatura digital detectada.
    #[default]
    None,
    /// Assinatura verificada e válida.
    Valid,
    /// Assinatura falhou ou chave não confiável / corrompida.
    Invalid,
}

/// Metadados criptográficos extraídos de cabeçalhos ou preâmbulo do Ishcondidim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CryptoMeta {
    /// Se o e-mail foi transmitido criptografado (PGP/MIME ou Autocrypt).
    pub is_encrypted: bool,
    /// Status de validação da assinatura digital.
    pub signature_status: SignatureStatus,
    /// ID ou Fingerprint da chave OpenPGP do remetente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer_key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_sender: Option<String>,
    /// Palavra secreta de validação de autenticidade (se configurada no servidor Ishcondidim).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_word: Option<String>,
}

/// Mensagem completa incluindo envelopes e representações de corpo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Message {
    /// Envelope e cabeçalhos principais da mensagem.
    #[serde(flatten)]
    pub envelope: MessageEnvelope,
    /// Versão Markdown limpa (txt auto).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txt_auto: Option<String>,
    /// Texto cru original preservado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txt_original: Option<String>,
    /// HTML sanitizado sem scripts ou rastreadores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_sanitized: Option<String>,
    /// Lista de anexos da mensagem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMeta>,
}

/// Higieniza preâmbulos de segurança do Ishcondidim em texto puro (`text/plain`).
/// Retorna o corpo limpo e, se encontrado o banner, os metadados criptográficos extraídos.
pub fn clean_ishcondidim_preamble_text(text: &str) -> (String, Option<CryptoMeta>) {
    let start_needle = "┌── 🔒 [";
    let is_ishcondidim =
        text.contains("┌── 🔒 [Ixcondidim:") || text.contains("┌── 🔒 [Ishcondidim:");

    if !is_ishcondidim {
        return (text.to_string(), None);
    }

    if let Some(start_idx) = text.find(start_needle) {
        let after_start = &text[start_idx..];
        let end_needle = "└─";
        if let Some(end_rel) = after_start.find(end_needle) {
            let banner_sub = &after_start[..end_rel];
            // Encontrar o fim da linha do encerramento do banner
            let remainder_after_banner = &after_start[end_rel..];
            let end_line_offset = remainder_after_banner
                .find('\n')
                .map(|idx| idx + 1)
                .unwrap_or(remainder_after_banner.len());

            let mut verified_sender = None;
            let mut signer_key_id = None;
            let mut security_word = None;
            let mut signature_status = SignatureStatus::None;

            for line in banner_sub.lines() {
                let trimmed = line.trim();
                if let Some(idx) = trimmed.find("Remetente:") {
                    let val = trimmed[idx + "Remetente:".len()..].trim();
                    if !val.is_empty() {
                        verified_sender = Some(val.to_string());
                    }
                }
                if let Some(idx) = trimmed.find("ID da Chave:") {
                    let val = trimmed[idx + "ID da Chave:".len()..].trim();
                    if !val.is_empty() && val != "N/A" {
                        signer_key_id = Some(val.to_string());
                    }
                }
                if let Some(idx) = trimmed.find("Palavra Secreta de Validação:") {
                    let val = trimmed[idx + "Palavra Secreta de Validação:".len()..].trim();
                    if let Some(w) = val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                        security_word = Some(w.trim().to_string());
                    }
                }
                if let Some(idx) = trimmed.find("Assinatura Digital:") {
                    let val = trimmed[idx + "Assinatura Digital:".len()..].trim();
                    if val.contains("INVÁLIDA") {
                        signature_status = SignatureStatus::Invalid;
                    } else if val.contains("VÁLIDA") {
                        signature_status = SignatureStatus::Valid;
                    }
                }
            }

            let meta = CryptoMeta {
                is_encrypted: true,
                signature_status,
                signer_key_id,
                verified_sender,
                security_word,
            };

            let before = &text[..start_idx];
            let after = &remainder_after_banner[end_line_offset..];
            let cleaned = format!("{}{}", before, after)
                .trim_start_matches(['\r', '\n'])
                .to_string();

            return (cleaned, Some(meta));
        }
    }

    (text.to_string(), None)
}

/// Higieniza preâmbulos de segurança do Ishcondidim em conteúdo HTML (`text/html`).
/// Retorna o HTML limpo e, se encontrado o banner, os metadados criptográficos extraídos.
pub fn clean_ishcondidim_preamble_html(html: &str) -> (String, Option<CryptoMeta>) {
    let needle_ix = "🔒 [Ixcondidim:";
    let needle_ish = "🔒 [Ishcondidim:";

    if !html.contains(needle_ix) && !html.contains(needle_ish) {
        return (html.to_string(), None);
    }

    // Procura o <div mais externo que encapsula o banner
    let marker_idx = html
        .find(needle_ix)
        .or_else(|| html.find(needle_ish))
        .unwrap_or(0);
    // Procuramos o primeiro <div antes do marker que tenha border-left ou background correspondente ao banner do Ishcondidim
    let mut div_start = None;
    let mut search_slice = &html[..marker_idx];
    while let Some(pos) = search_slice.rfind("<div") {
        let snippet = &html[pos..marker_idx];
        if snippet.contains("border-left")
            || snippet.contains("#10b981")
            || snippet.contains("#f0fdf4")
        {
            div_start = Some(pos);
            break;
        }
        if pos == 0 {
            break;
        }
        search_slice = &html[..pos];
    }
    // Fallback: se não achou div estilizado, pega o mais próximo
    let start_idx = match div_start.or_else(|| html[..marker_idx].rfind("<div")) {
        Some(idx) => idx,
        None => return (html.to_string(), None),
    };

    {
        // Encontrar o fechamento correspondente </div> após o marker
        // O banner possui uma tabela e divs internas, terminando com </div>\n
        let sub = &html[start_idx..];
        // O banner do Ishcondidim termina após a </table> com um </div>
        if let Some(table_end) = sub.find("</table>") {
            let after_table = &sub[table_end..];
            if let Some(div_close_rel) = after_table.find("</div>") {
                let banner_end = start_idx + table_end + div_close_rel + "</div>".len();
                let banner_chunk = &html[start_idx..banner_end];

                let mut verified_sender = None;
                let mut signer_key_id = None;
                let mut security_word = None;
                let mut signature_status = SignatureStatus::None;

                if let Some(sender_pos) = banner_chunk.find("Remetente:") {
                    let after = &banner_chunk[sender_pos..];
                    if let Some(code_s) = after.find("<code>")
                        && let Some(code_e) = after.find("</code>")
                    {
                        let s = &after[code_s + 6..code_e];
                        verified_sender = Some(s.trim().to_string());
                    }
                }

                if let Some(key_pos) = banner_chunk.find("ID da Chave:") {
                    let after = &banner_chunk[key_pos..];
                    if let Some(code_s) = after.find("<code>")
                        && let Some(code_e) = after.find("</code>")
                    {
                        let s = &after[code_s + 6..code_e];
                        signer_key_id = Some(s.trim().to_string());
                    }
                }

                if let Some(word_pos) = banner_chunk.find("Palavra Secreta:") {
                    let after = &banner_chunk[word_pos..];
                    if let Some(b_s) = after.find('[')
                        && let Some(b_e) = after.find(']')
                        && b_e > b_s
                    {
                        let s = &after[b_s + 1..b_e];
                        security_word = Some(s.trim().to_string());
                    }
                }

                if let Some(sig_pos) = banner_chunk.find("Assinatura:") {
                    let after = &banner_chunk[sig_pos..];
                    if after.contains("INVÁLIDA") {
                        signature_status = SignatureStatus::Invalid;
                    } else if after.contains("VÁLIDA") {
                        signature_status = SignatureStatus::Valid;
                    }
                }

                let meta = CryptoMeta {
                    is_encrypted: true,
                    signature_status,
                    signer_key_id,
                    verified_sender,
                    security_word,
                };

                let before = &html[..start_idx];
                let after = &html[banner_end..];
                let cleaned = format!("{}{}", before, after)
                    .trim_start_matches(['\r', '\n'])
                    .to_string();

                return (cleaned, Some(meta));
            }
        }
    }

    (html.to_string(), None)
}

/// Status de sincronização da caixa postal com o índice de busca.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MailboxSyncStatus {
    /// Indica se 100% dos e-mails da caixa postal já foram indexados.
    pub is_synced: bool,
    /// Quantidade de mensagens indexadas no banco local/remoto.
    pub indexed_count: u64,
    /// Quantidade total de mensagens na caixa postal física/remota.
    pub total_count: u64,
    /// Quantidade de mensagens pendentes de indexação.
    pub remaining_count: u64,
}

impl MailboxSyncStatus {
    /// Construtor calculando o estado de sincronização e itens pendentes.
    pub fn new(indexed_count: u64, total_count: u64) -> Self {
        let is_synced = indexed_count >= total_count;
        let remaining_count = total_count.saturating_sub(indexed_count);
        Self {
            is_synced,
            indexed_count,
            total_count,
            remaining_count,
        }
    }
}

/// Resultado estruturado de busca de mensagens com transparência de sincronização.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SearchResult {
    /// Lista de mensagens encontradas ranqueadas por relevância.
    pub messages: Vec<MessageEnvelope>,
    /// Metadados de sincronização do índice da caixa consultada.
    pub sync_status: MailboxSyncStatus,
}

