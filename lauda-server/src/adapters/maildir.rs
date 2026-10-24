use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use async_trait::async_trait;
use lauda_types::{
    ComposeForm, MailboxInfo, MailboxSyncStatus, Message, MessageEnvelope, SearchResult,
    message::{
        CryptoMeta, SignatureStatus, clean_ishcondidim_preamble_html,
        clean_ishcondidim_preamble_text,
    },
};
use mail_parser::{Address, MessageParser, MimeHeaders};

use crate::adapters::sqlite_index::SqliteIndexStore;
use crate::domain::ports::{BackendError, MailBackend};

/// Backend de correio baseado em armazenamento Maildir real (padrão Dovecot).
pub struct MaildirBackend {
    base_dir: PathBuf,
    users: RwLock<HashMap<String, String>>, // username -> password
    sqlite_index: Option<SqliteIndexStore>,
}

/// Verifica se o ambiente atual é de testes ou CI para suprimir envio real de e-mails
pub fn is_sendmail_suppressed_for_tests() -> bool {
    MaildirBackend::is_sendmail_suppressed_for_tests()
}

/// Extrai o endereço de e-mail limpo de uma string que pode conter nome de exibição (ex: "Lumen <lumen@2lp.in>").
pub fn extract_clean_email(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find('<')
        && let Some(end) = trimmed[start + 1..].find('>')
    {
        return trimmed[start + 1..start + 1 + end].trim().to_string();
    }
    trimmed.to_string()
}

/// Extrai múltiplos e-mails separados por vírgula.
pub fn extract_recipient_emails(to_or_cc: &str) -> Vec<String> {
    to_or_cc
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(extract_clean_email)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Normaliza os dados brutos de e-mail de acordo com a RFC 5321 (dot-stuffing e terminação CRLF).
pub fn normalize_smtp_data(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 64);
    let normalized = raw.replace("\r\n", "\n");
    for line in normalized.lines() {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out
}

/// Envia e-mail diretamente via conexão TCP SMTP pura (RFC 5321).
pub async fn dispatch_smtp_raw(
    host: &str,
    port: u16,
    sender: &str,
    recipients: &[String],
    raw_email: &str,
) -> Result<(), String> {
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    let clean_sender = extract_clean_email(sender);
    if clean_sender.is_empty() {
        return Err("Remetente (MAIL FROM) inválido ou vazio".to_string());
    }
    if recipients.is_empty() {
        return Err("Lista de destinatários vazia".to_string());
    }

    let addr = format!("{}:{}", host, port);
    let connect_fut = TcpStream::connect(&addr);
    let stream = tokio::time::timeout(Duration::from_secs(10), connect_fut)
        .await
        .map_err(|_| format!("Timeout ao conectar ao servidor SMTP em {}", addr))?
        .map_err(|e| format!("Falha de conexão ao servidor SMTP em {}: {}", addr, e))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    async fn read_reply<R: AsyncBufReadExt + Unpin>(
        reader: &mut R,
        expected_prefix: &[&str],
    ) -> Result<String, String> {
        let mut full_reply = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            let n = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
                .await
                .map_err(|_| "Timeout lendo resposta SMTP".to_string())?
                .map_err(|e| format!("Erro lendo resposta SMTP: {}", e))?;

            if n == 0 {
                return Err("Conexão fechada prematuramente pelo servidor SMTP".to_string());
            }

            full_reply.push_str(&line);
            if line.len() >= 4 && line.as_bytes()[3] == b' ' {
                break;
            }
            if line.len() == 3 {
                break;
            }
        }

        let code = if full_reply.len() >= 3 {
            &full_reply[..3]
        } else {
            ""
        };

        if !expected_prefix.iter().any(|&p| code.starts_with(p)) {
            return Err(format!(
                "Erro SMTP (esperado {:?}): {}",
                expected_prefix,
                full_reply.trim()
            ));
        }

        Ok(full_reply)
    }

    // 1. Banner
    read_reply(&mut reader, &["220"]).await?;

    // 2. EHLO
    writer
        .write_all(b"EHLO localhost\r\n")
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    read_reply(&mut reader, &["250"]).await?;

    // 3. MAIL FROM
    let mail_from = format!("MAIL FROM:<{}>\r\n", clean_sender);
    writer
        .write_all(mail_from.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    read_reply(&mut reader, &["250"]).await?;

    // 4. RCPT TO
    for rcpt in recipients {
        let clean_rcpt = extract_clean_email(rcpt);
        if clean_rcpt.is_empty() {
            continue;
        }
        let rcpt_to = format!("RCPT TO:<{}>\r\n", clean_rcpt);
        writer
            .write_all(rcpt_to.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        writer.flush().await.map_err(|e| e.to_string())?;
        read_reply(&mut reader, &["250", "251"]).await?;
    }

    // 5. DATA
    writer
        .write_all(b"DATA\r\n")
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    read_reply(&mut reader, &["354"]).await?;

    // 6. Envio dos dados normalizados + finalizador . CRLF
    let normalized = normalize_smtp_data(raw_email);
    writer
        .write_all(normalized.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer
        .write_all(b".\r\n")
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;
    read_reply(&mut reader, &["250"]).await?;

    // 7. QUIT
    let _ = writer.write_all(b"QUIT\r\n").await;
    let _ = writer.flush().await;

    Ok(())
}

impl MaildirBackend {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let dir: PathBuf = base_dir.into();
        let db_path = dir.join(".lauda_index.db");
        let sqlite_index = SqliteIndexStore::open(&db_path).ok();
        Self {
            base_dir: dir,
            users: RwLock::new(HashMap::new()),
            sqlite_index,
        }
    }

    pub fn sqlite_index(&self) -> Option<&SqliteIndexStore> {
        self.sqlite_index.as_ref()
    }

    pub fn add_user(&self, username: impl Into<String>, password: impl Into<String>) {
        if let Ok(mut lock) = self.users.write() {
            lock.insert(username.into(), password.into());
        }
    }

    fn resolve_mailbox_dir(&self, mailbox: &str) -> Result<PathBuf, BackendError> {
        let dir = match mailbox.to_uppercase().as_str() {
            "INBOX" => self.base_dir.clone(),
            "SENT" => self.base_dir.join(".Sent"),
            "DRAFTS" => self.base_dir.join(".Drafts"),
            "TRASH" => self.base_dir.join(".Trash"),
            "JUNK" | "SPAM" => self.base_dir.join(".Junk"),
            "ARCHIVE" => self.base_dir.join(".Archive"),
            "ALL" => self.base_dir.join(".All"),
            other => {
                let custom = self.base_dir.join(format!(".{}", other));
                if custom.exists() {
                    custom
                } else {
                    return Err(BackendError::NotFound(format!(
                        "Caixa postal '{}' não encontrada",
                        mailbox
                    )));
                }
            }
        };

        if !dir.exists() {
            return Err(BackendError::NotFound(format!(
                "Diretório da caixa postal não existe: {:?}",
                dir
            )));
        }

        Ok(dir)
    }

    fn load_uid_maps(&self, dir: &Path) -> (HashMap<String, u32>, HashMap<u32, String>) {
        let mut prefix_to_uid = HashMap::new();
        let mut uid_to_prefix = HashMap::new();
        let uidlist_file = dir.join("dovecot-uidlist");

        let mut max_uid = 1000u32;
        if let Ok(content) = std::fs::read_to_string(uidlist_file) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2
                    && let Ok(uid) = parts[0].parse::<u32>()
                {
                    let raw_prefix = parts[1].trim_start_matches(':');
                    let prefix = raw_prefix.split(',').next().unwrap_or(raw_prefix);
                    if !prefix.is_empty() {
                        prefix_to_uid.insert(prefix.to_string(), uid);
                        uid_to_prefix.insert(uid, prefix.to_string());
                        if uid > max_uid {
                            max_uid = uid;
                        }
                    }
                }
            }
        }

        // Para arquivos sem entrada no dovecot-uidlist, garanta mapeamento consistente em O(1) com HashSet
        let mut unmapped_set = HashSet::new();
        let mut unmapped = Vec::new();
        for sub in &["cur", "new"] {
            if let Ok(entries) = std::fs::read_dir(dir.join(sub)) {
                for entry in entries.flatten() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with('.') {
                        continue;
                    }
                    let prefix = Self::extract_prefix(&filename);
                    if !prefix_to_uid.contains_key(prefix)
                        && unmapped_set.insert(prefix.to_string())
                    {
                        unmapped.push(prefix.to_string());
                    }
                }
            }
        }
        unmapped.sort();
        for prefix in unmapped {
            max_uid += 1;
            prefix_to_uid.insert(prefix.clone(), max_uid);
            uid_to_prefix.insert(max_uid, prefix);
        }

        (prefix_to_uid, uid_to_prefix)
    }

    fn format_address(addr: &Address) -> String {
        match addr {
            Address::List(list) => {
                if let Some(first) = list.first() {
                    match (&first.name, &first.address) {
                        (Some(name), Some(email)) => format!("{} <{}>", name, email),
                        (None, Some(email)) => email.to_string(),
                        (Some(name), None) => name.to_string(),
                        _ => "Desconhecido".to_string(),
                    }
                } else {
                    "Desconhecido".to_string()
                }
            }
            Address::Group(groups) => {
                if let Some(first_grp) = groups.first() {
                    if let Some(first) = first_grp.addresses.first() {
                        match (&first.name, &first.address) {
                            (Some(name), Some(email)) => format!("{} <{}>", name, email),
                            (None, Some(email)) => email.to_string(),
                            (Some(name), None) => name.to_string(),
                            _ => "Desconhecido".to_string(),
                        }
                    } else {
                        "Desconhecido".to_string()
                    }
                } else {
                    "Desconhecido".to_string()
                }
            }
        }
    }

    fn extract_prefix(filename: &str) -> &str {
        let raw = filename.split(":2,").next().unwrap_or(filename);
        raw.split(',').next().unwrap_or(raw)
    }

    fn find_message_file(&self, dir: &Path, uid: u32) -> Option<(PathBuf, String, bool, bool)> {
        let (prefix_to_uid, uid_to_prefix) = self.load_uid_maps(dir);
        let target_prefix = uid_to_prefix.get(&uid);

        for sub in &["cur", "new"] {
            let sub_path = dir.join(sub);
            if let Ok(entries) = std::fs::read_dir(sub_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let prefix = Self::extract_prefix(&filename);

                    let matched = if let Some(target) = target_prefix {
                        prefix == target
                    } else if let Some(&file_uid) = prefix_to_uid.get(prefix) {
                        file_uid == uid
                    } else {
                        false
                    };

                    if matched {
                        let (is_read, is_flagged) = Self::parse_maildir_flags(sub, &filename);
                        return Some((path, filename, is_read, is_flagged));
                    }
                }
            }
        }
        None
    }

    fn parse_maildir_flags(sub: &str, filename: &str) -> (bool, bool) {
        let (is_read, is_flagged) = if let Some(flags) = filename.split(":2,").nth(1) {
            (flags.contains('S'), flags.contains('F'))
        } else {
            (false, false)
        };

        let is_read = if sub == "new" { false } else { is_read };
        (is_read, is_flagged)
    }

    fn clean_comment_tags(text: &str) -> String {
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

    fn extract_crypto_meta_from_headers(parsed: &mail_parser::Message) -> Option<CryptoMeta> {
        let mut is_encrypted = false;
        let mut signature_status = SignatureStatus::None;
        let signer_key_id = None;
        let mut has_crypto_headers = false;

        for header in parsed.headers() {
            let name = header.name();
            let val = header.value().as_text().unwrap_or_default();
            if name.eq_ignore_ascii_case("X-Ixcondidim-Decrypted")
                || name.eq_ignore_ascii_case("X-Ishcondidim-Decrypted")
            {
                has_crypto_headers = true;
                is_encrypted = true;
                if val.contains("verified=pass") || val.contains("verified=true") {
                    signature_status = SignatureStatus::Valid;
                } else if val.contains("verified=fail") || val.contains("verified=invalid") {
                    signature_status = SignatureStatus::Invalid;
                }
            } else if name.eq_ignore_ascii_case("X-Ixcondidim-Status")
                || name.eq_ignore_ascii_case("X-Ishcondidim-Status")
            {
                has_crypto_headers = true;
                if val.contains("decrypted-and-verified") {
                    is_encrypted = true;
                    signature_status = SignatureStatus::Valid;
                } else if val.contains("decrypted") {
                    is_encrypted = true;
                }
            } else if name.eq_ignore_ascii_case("Autocrypt") {
                has_crypto_headers = true;
            } else if name.eq_ignore_ascii_case("X-GPG-Status") {
                has_crypto_headers = true;
                if val.contains("GOODSIG") || val.contains("VALIDSIG") {
                    signature_status = SignatureStatus::Valid;
                } else if val.contains("BADSIG") || val.contains("ERRSIG") {
                    signature_status = SignatureStatus::Invalid;
                }
            }
        }

        if has_crypto_headers {
            Some(CryptoMeta {
                is_encrypted,
                signature_status,
                signer_key_id,
                verified_sender: None,
                security_word: None,
            })
        } else {
            None
        }
    }

    /// Verifica se o ambiente atual é de testes ou CI para suprimir envio real de e-mails
    pub fn is_sendmail_suppressed_for_tests() -> bool {
        if std::env::var("LAUDA_FORCE_DISPATCH").is_ok() {
            return false;
        }
        if cfg!(test) {
            return true;
        }
        if std::env::var("LAUDA_TEST_MODE").is_ok()
            || std::env::var("CI").is_ok()
            || std::env::var("RUST_TEST").is_ok()
        {
            return true;
        }
        if let Ok(exe) = std::env::current_exe() {
            let s = exe.to_string_lossy();
            if s.contains("/deps/") || s.contains("test") {
                return true;
            }
        }
        false
    }

    pub async fn dispatch_email(
        sender: &str,
        recipients: &[String],
        raw_email: &str,
    ) -> Result<(), String> {
        if Self::is_sendmail_suppressed_for_tests() {
            tracing::info!(
                "lauda-server: modo de teste ativo, despacho real de e-mail suprimido com sucesso"
            );
            return Ok(());
        }

        if let Ok(custom_cmd) = std::env::var("LAUDA_SENDMAIL_CMD") {
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(&custom_cmd)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Erro ao iniciar comando configurado: {}", e))?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(raw_email.as_bytes())
                    .map_err(|e| e.to_string())?;
            }
            let status = child.wait().map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Comando retornou erro: {:?}", status.code()));
            }
            return Ok(());
        }

        let host = std::env::var("LAUDA_SMTP_HOST").unwrap_or_else(|_| "mailserver".to_string());
        let port: u16 = std::env::var("LAUDA_SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(25);

        dispatch_smtp_raw(&host, port, sender, recipients, raw_email).await
    }

    pub fn dispatch_sendmail(raw_email: &str) -> Result<(), String> {
        if Self::is_sendmail_suppressed_for_tests() {
            return Ok(());
        }
        if let Ok(custom_cmd) = std::env::var("LAUDA_SENDMAIL_CMD") {
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(&custom_cmd)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Erro ao iniciar comando configurado: {}", e))?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin
                    .write_all(raw_email.as_bytes())
                    .map_err(|e| e.to_string())?;
            }
            let status = child.wait().map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Comando retornou erro: {:?}", status.code()));
            }
            return Ok(());
        }
        Err("dispatch_sendmail síncrono legado indisponível".to_string())
    }
}

#[async_trait]
impl MailBackend for MaildirBackend {
    async fn authenticate(&self, username: &str, password: &str) -> Result<(), BackendError> {
        let lock = self
            .users
            .read()
            .map_err(|e| BackendError::Internal(e.to_string()))?;
        if let Some(expected_pass) = lock.get(username)
            && expected_pass == password
        {
            return Ok(());
        }
        Err(BackendError::AuthFailed)
    }

    async fn list_mailboxes(&self, _username: &str) -> Result<Vec<MailboxInfo>, BackendError> {
        let mut mailboxes = Vec::new();
        let candidates = vec![
            ("INBOX", self.base_dir.clone()),
            ("Sent", self.base_dir.join(".Sent")),
            ("Drafts", self.base_dir.join(".Drafts")),
            ("Trash", self.base_dir.join(".Trash")),
            ("Junk", self.base_dir.join(".Junk")),
            ("Archive", self.base_dir.join(".Archive")),
        ];

        for (name, path) in candidates {
            if !path.exists() {
                continue;
            }

            let mut total = 0u32;
            let mut unread = 0u32;

            for sub in &["cur", "new"] {
                let sub_path = path.join(sub);
                if let Ok(entries) = std::fs::read_dir(sub_path) {
                    for entry in entries.flatten() {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        if filename.starts_with('.') {
                            continue;
                        }
                        total += 1;
                        let (is_read, _) = Self::parse_maildir_flags(sub, &filename);
                        if !is_read {
                            unread += 1;
                        }
                    }
                }
            }

            mailboxes.push(MailboxInfo {
                name: name.to_string(),
                total_messages: total,
                unread_messages: unread,
            });
        }

        Ok(mailboxes)
    }

    async fn list_messages(
        &self,
        _username: &str,
        mailbox: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<MessageEnvelope>, BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (prefix_to_uid, _) = self.load_uid_maps(&dir);

        struct FileEntry {
            path: std::path::PathBuf,
            filename: String,
            sub: &'static str,
            mtime: std::time::SystemTime,
        }

        let mut entries = Vec::new();

        for sub in &["new", "cur"] {
            let sub_path = dir.join(sub);
            if let Ok(dir_entries) = std::fs::read_dir(sub_path) {
                for entry in dir_entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with('.') {
                        continue;
                    }

                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                    entries.push(FileEntry {
                        path,
                        filename,
                        sub,
                        mtime,
                    });
                }
            }
        }

        // Ordenar do mais novo para o mais antigo (mtime decrescente, secundário por filename)
        entries.sort_by(|a, b| {
            b.mtime
                .cmp(&a.mtime)
                .then_with(|| b.filename.cmp(&a.filename))
        });

        let offset = offset.unwrap_or(0);
        let selected_entries: Vec<FileEntry> = if let Some(limit) = limit {
            entries.into_iter().skip(offset).take(limit).collect()
        } else {
            entries.into_iter().skip(offset).collect()
        };

        let mut envelopes = Vec::with_capacity(selected_entries.len());
        let mut fallback_uid = 1000u32;

        for item in selected_entries {
            let prefix = Self::extract_prefix(&item.filename);
            let uid = prefix_to_uid.get(prefix).copied().unwrap_or_else(|| {
                fallback_uid += 1;
                fallback_uid
            });

            let (is_read, is_flagged) = Self::parse_maildir_flags(item.sub, &item.filename);

            if let Ok(bytes) = std::fs::read(&item.path)
                && let Some(parsed) = MessageParser::default().parse_headers(&bytes)
            {
                let subject = parsed.subject().unwrap_or("(Sem assunto)").to_string();
                let sender = parsed
                    .from()
                    .map(Self::format_address)
                    .unwrap_or_else(|| "Desconhecido".to_string());
                let date = parsed.date().map(|d| d.to_rfc3339()).unwrap_or_else(|| {
                    let datetime: chrono::DateTime<chrono::Utc> = item.mtime.into();
                    datetime.to_rfc3339()
                });

                let cc = parsed.cc().map(Self::format_address);
                let has_attachments = parsed
                    .content_type()
                    .map(|ct| {
                        ct.c_type.eq_ignore_ascii_case("multipart")
                            && !ct
                                .subtype()
                                .map(|s| s.eq_ignore_ascii_case("alternative"))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

                let crypto_meta = Self::extract_crypto_meta_from_headers(&parsed);

                envelopes.push(MessageEnvelope {
                    uid,
                    subject,
                    sender,
                    date,
                    is_read,
                    is_flagged,
                    cc,
                    has_attachments,
                    crypto_meta,
                });
            }
        }

        Ok(envelopes)
    }

    async fn get_message(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (file_path, _, is_read, is_flagged) =
            self.find_message_file(&dir, uid).ok_or_else(|| {
                BackendError::NotFound(format!(
                    "Mensagem UID {} não encontrada na caixa {}",
                    uid, mailbox
                ))
            })?;

        let bytes = std::fs::read(&file_path).map_err(|e| BackendError::Internal(e.to_string()))?;
        let parsed = MessageParser::default().parse(&bytes).ok_or_else(|| {
            BackendError::Internal("Falha ao analisar mensagem RFC 5322".to_string())
        })?;

        let subject = parsed.subject().unwrap_or("(Sem assunto)").to_string();
        let sender = parsed
            .from()
            .map(Self::format_address)
            .unwrap_or_else(|| "Desconhecido".to_string());
        let date = parsed
            .date()
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| "2026-10-05T00:00:00Z".to_string());
        let cc = parsed.cc().map(Self::format_address);

        let mut crypto_meta = Self::extract_crypto_meta_from_headers(&parsed);

        let (txt_original, meta_from_txt) = match parsed.body_text(0) {
            Some(txt) => {
                let (cleaned, meta) = clean_ishcondidim_preamble_text(&txt);
                (Some(cleaned), meta)
            }
            None => (None, None),
        };

        let (html_sanitized, meta_from_html) = match parsed.body_html(0) {
            Some(html) => {
                let (cleaned, meta) = clean_ishcondidim_preamble_html(&html);
                (Some(cleaned), meta)
            }
            None => (None, None),
        };

        // Mesclar metadados: prioriza o mais completo (preâmbulo costuma ter key id e sender)
        if let Some(m) = meta_from_txt.or(meta_from_html) {
            if let Some(existing) = crypto_meta.as_mut() {
                existing.is_encrypted = existing.is_encrypted || m.is_encrypted;
                if m.signature_status != SignatureStatus::None {
                    existing.signature_status = m.signature_status;
                }
                if m.signer_key_id.is_some() {
                    existing.signer_key_id = m.signer_key_id;
                }
                if m.verified_sender.is_some() {
                    existing.verified_sender = m.verified_sender;
                }
                if m.security_word.is_some() {
                    existing.security_word = m.security_word;
                }
            } else {
                crypto_meta = Some(m);
            }
        }

        let txt_auto = if let Some(ref html) = html_sanitized {
            let clean = Self::clean_comment_tags(html);
            let md = html2text::from_read(clean.as_bytes(), 10_000);
            let trimmed = md.trim().to_string();
            if !trimmed.is_empty() {
                Some(trimmed)
            } else {
                txt_original.as_deref().map(Self::clean_comment_tags)
            }
        } else {
            txt_original.as_deref().map(Self::clean_comment_tags)
        };

        let attachments: Vec<lauda_types::AttachmentMeta> = parsed
            .attachments()
            .enumerate()
            .map(|(idx, part)| {
                let filename = part
                    .attachment_name()
                    .unwrap_or("anexo_sem_nome")
                    .to_string();
                let content_type = part
                    .content_type()
                    .map(|ct| match ct.subtype() {
                        Some(sub) => format!("{}/{}", ct.c_type, sub),
                        None => ct.c_type.to_string(),
                    })
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let size_bytes = part.contents().len();

                lauda_types::AttachmentMeta {
                    id: idx,
                    filename,
                    content_type,
                    size_bytes,
                }
            })
            .collect();

        let has_attachments = !attachments.is_empty();

        let envelope = MessageEnvelope {
            uid,
            subject,
            sender,
            date,
            is_read,
            is_flagged,
            cc,
            has_attachments,
            crypto_meta,
        };

        Ok(Message {
            envelope,
            txt_auto,
            txt_original,
            html_sanitized,
            attachments,
        })
    }

    async fn send_message(&self, _username: &str, form: &ComposeForm) -> Result<(), BackendError> {
        let sent_dir = self.base_dir.join(".Sent").join("cur");
        let _ = std::fs::create_dir_all(&sent_dir);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let pid = std::process::id();
        let msg_id = format!("<{}.{}@2lp.in>", timestamp, pid);

        let cc_header = form
            .cc
            .as_ref()
            .map(|cc| format!("Cc: {}\n", cc))
            .unwrap_or_default();

        let raw_email = if form.attachments.is_empty() {
            format!(
                "From: {}\nTo: {}\n{}Subject: {}\nDate: {}\nMessage-ID: {}\nContent-Type: text/plain; charset=utf-8\n\n{}\n",
                form.from,
                form.to,
                cc_header,
                form.subject,
                chrono::Utc::now().to_rfc2822(),
                msg_id,
                form.body
            )
        } else {
            let boundary = format!("=_lauda_{}_{}", timestamp, pid);
            let mut parts = format!(
                "From: {}\nTo: {}\n{}Subject: {}\nDate: {}\nMessage-ID: {}\nMIME-Version: 1.0\nContent-Type: multipart/mixed; boundary=\"{}\"\n\n--{}\nContent-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: 8bit\n\n{}\n\n",
                form.from,
                form.to,
                cc_header,
                form.subject,
                chrono::Utc::now().to_rfc2822(),
                msg_id,
                boundary,
                boundary,
                form.body
            );

            for att in &form.attachments {
                parts.push_str(&format!(
                    "--{}\nContent-Type: {}; name=\"{}\"\nContent-Disposition: attachment; filename=\"{}\"\nContent-Transfer-Encoding: base64\n\n{}\n\n",
                    boundary,
                    att.content_type,
                    att.filename,
                    att.filename,
                    att.data_base64
                ));
            }

            parts.push_str(&format!("--{}--\n", boundary));
            parts
        };

        let filename = format!("{}.M{}P{}.lauda:2,S", timestamp, timestamp % 1000, pid);
        let sent_file_path = sent_dir.join(filename);
        std::fs::write(&sent_file_path, &raw_email).map_err(|e| {
            BackendError::Internal(format!("Erro ao gravar mensagem em .Sent: {}", e))
        })?;

        let mut recipients = extract_recipient_emails(&form.to);
        if let Some(ref cc) = form.cc {
            recipients.extend(extract_recipient_emails(cc));
        }

        if let Err(e) = Self::dispatch_email(&form.from, &recipients, &raw_email).await {
            let _ = std::fs::remove_file(&sent_file_path);
            return Err(BackendError::Internal(format!(
                "Falha ao despachar e-mail: {}",
                e
            )));
        }

        Ok(())
    }

    async fn mark_message_read(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (file_path, filename, current_read, _) =
            self.find_message_file(&dir, uid).ok_or_else(|| {
                BackendError::NotFound(format!(
                    "Mensagem UID {} não encontrada na caixa {}",
                    uid, mailbox
                ))
            })?;

        if current_read == read {
            return Ok(());
        }

        let parts: Vec<&str> = filename.split(":2,").collect();
        let base_name = parts[0];
        let current_flags = if parts.len() > 1 { parts[1] } else { "" };

        let mut flags_chars: Vec<char> = current_flags.chars().filter(|&c| c != 'S').collect();
        if read {
            flags_chars.push('S');
        }
        flags_chars.sort();
        flags_chars.dedup();
        let new_flags: String = flags_chars.into_iter().collect();

        let new_filename = format!("{}:2,{}", base_name, new_flags);
        let target_dir = dir.join("cur");
        let new_path = target_dir.join(new_filename);

        std::fs::rename(&file_path, &new_path)
            .map_err(|e| BackendError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn delete_message(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<(), BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (file_path, filename, _, _) = self.find_message_file(&dir, uid).ok_or_else(|| {
            BackendError::NotFound(format!(
                "Mensagem UID {} não encontrada na caixa {}",
                uid, mailbox
            ))
        })?;

        if mailbox.eq_ignore_ascii_case("Trash") {
            std::fs::remove_file(&file_path).map_err(|e| BackendError::Internal(e.to_string()))?;
        } else {
            let trash_dir = self.base_dir.join(".Trash").join("cur");
            let _ = std::fs::create_dir_all(&trash_dir);
            let target_path = trash_dir.join(&filename);
            std::fs::rename(&file_path, &target_path)
                .map_err(|e| BackendError::Internal(e.to_string()))?;
        }
        Ok(())
    }

    async fn mark_message_flagged(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (file_path, filename, _, current_flagged) =
            self.find_message_file(&dir, uid).ok_or_else(|| {
                BackendError::NotFound(format!(
                    "Mensagem UID {} não encontrada na caixa {}",
                    uid, mailbox
                ))
            })?;

        if current_flagged == flagged {
            return Ok(());
        }

        let parts: Vec<&str> = filename.split(":2,").collect();
        let base_name = parts[0];
        let current_flags = if parts.len() > 1 { parts[1] } else { "" };

        let mut flags_chars: Vec<char> = current_flags.chars().filter(|&c| c != 'F').collect();
        if flagged {
            flags_chars.push('F');
        }
        flags_chars.sort();
        flags_chars.dedup();
        let new_flags: String = flags_chars.into_iter().collect();

        let new_filename = format!("{}:2,{}", base_name, new_flags);
        let target_dir = dir.join("cur");
        let new_path = target_dir.join(new_filename);

        std::fs::rename(&file_path, &new_path)
            .map_err(|e| BackendError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn move_message(
        &self,
        _username: &str,
        src_box: &str,
        dest_box: &str,
        uid: u32,
    ) -> Result<(), BackendError> {
        let src_dir = self.resolve_mailbox_dir(src_box)?;
        let (file_path, filename, _, _) =
            self.find_message_file(&src_dir, uid).ok_or_else(|| {
                BackendError::NotFound(format!(
                    "Mensagem UID {} não encontrada na caixa {}",
                    uid, src_box
                ))
            })?;

        let dest_dir = if dest_box.eq_ignore_ascii_case("INBOX") {
            self.base_dir.clone()
        } else {
            self.base_dir.join(format!(".{}", dest_box))
        };

        let target_dir = dest_dir.join("cur");
        std::fs::create_dir_all(&target_dir).map_err(|e| BackendError::Internal(e.to_string()))?;
        let target_path = target_dir.join(&filename);
        std::fs::rename(&file_path, &target_path)
            .map_err(|e| BackendError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_attachment(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (file_path, _, _, _) = self.find_message_file(&dir, uid).ok_or_else(|| {
            BackendError::NotFound(format!(
                "Mensagem UID {} não encontrada na caixa {}",
                uid, mailbox
            ))
        })?;

        let bytes = std::fs::read(&file_path).map_err(|e| BackendError::Internal(e.to_string()))?;
        let parsed = MessageParser::default().parse(&bytes).ok_or_else(|| {
            BackendError::Internal("Falha ao analisar mensagem RFC 5322".to_string())
        })?;

        let part = parsed.attachments().nth(attachment_id).ok_or_else(|| {
            BackendError::NotFound(format!(
                "Anexo {} da mensagem UID {} não encontrado",
                attachment_id, uid
            ))
        })?;

        let filename = part
            .attachment_name()
            .unwrap_or("anexo_sem_nome")
            .to_string();
        let content_type = part
            .content_type()
            .map(|ct| match ct.subtype() {
                Some(sub) => format!("{}/{}", ct.c_type, sub),
                None => ct.c_type.to_string(),
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let contents = part.contents().to_vec();

        Ok((filename, content_type, contents))
    }

    async fn list_contacts(
        &self,
        _username: &str,
    ) -> Result<Vec<lauda_types::ContactInfo>, BackendError> {
        let mut map: HashMap<String, (Option<String>, usize, bool, Option<String>)> =
            HashMap::new();

        let boxes = self.list_mailboxes(_username).await.unwrap_or_default();
        let box_names: Vec<String> = if boxes.is_empty() {
            vec![
                "INBOX".to_string(),
                "Sent".to_string(),
                "Archive".to_string(),
            ]
        } else {
            boxes.into_iter().map(|b| b.name).collect()
        };

        for box_name in &box_names {
            if let Ok(dir) = self.resolve_mailbox_dir(box_name) {
                for sub in &["cur", "new"] {
                    let sub_dir = dir.join(sub);
                    if let Ok(entries) = std::fs::read_dir(sub_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if !path.is_file() {
                                continue;
                            }
                            if let Ok(bytes) = std::fs::read(&path)
                                && let Some(parsed) = MessageParser::default().parse_headers(&bytes)
                            {
                                let addr_opt = if *box_name == "Sent" {
                                    parsed.to()
                                } else {
                                    parsed.from()
                                };

                                if let Some(addr) = addr_opt {
                                    let formatted = Self::format_address(addr);
                                    let (name, email) =
                                        if formatted.contains('<') && formatted.contains('>') {
                                            let parts: Vec<&str> = formatted.split('<').collect();
                                            let n = parts[0].trim().to_string();
                                            let e = parts[1].trim_matches('>').trim().to_string();
                                            (if n.is_empty() { None } else { Some(n) }, e)
                                        } else {
                                            (None, formatted)
                                        };

                                    if !email.is_empty() && email != "Desconhecido" {
                                        let entry =
                                            map.entry(email).or_insert((name, 0, false, None));
                                        entry.1 += 1;
                                        if let Some(header_val) = parsed.header("Autocrypt")
                                            && let Some(val_str) = header_val.as_text()
                                            && val_str.contains("type=p")
                                        {
                                            entry.2 = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut contacts: Vec<lauda_types::ContactInfo> = map
            .into_iter()
            .map(
                |(email, (name, count, has_autocrypt, fingerprint))| lauda_types::ContactInfo {
                    email,
                    name,
                    message_count: count,
                    has_autocrypt,
                    autocrypt_fingerprint: fingerprint,
                    birthday: None,
                },
            )
            .collect();

        contacts.sort_by_key(|c| std::cmp::Reverse(c.message_count));
        Ok(contacts)
    }

    async fn search_messages(
        &self,
        username: &str,
        mailbox: &str,
        query: &str,
    ) -> Result<SearchResult, BackendError> {
        if let Some(ref store) = self.sqlite_index {
            let dir = self.resolve_mailbox_dir(mailbox)?;
            let (_, uid_to_prefix) = self.load_uid_maps(&dir);
            let active_uids: HashSet<u32> = uid_to_prefix.keys().copied().collect();
            let total_count = active_uids.len() as u64;

            // 1. Reconciliação inteligente com dovecot-uidlist / active_uids
            let reconcile = store
                .reconcile_mailbox(mailbox, &active_uids)
                .map_err(|e| {
                    BackendError::Internal(format!("Erro ao reconciliar índice SQLite: {}", e))
                })?;

            // Expurgar e-mails excluídos do SQLite
            for del_uid in reconcile.to_remove {
                let _ = store.remove_document(mailbox, del_uid);
            }

            // Indexar pequenos lotes para não bloquear a resposta da requisição HTTP
            let provider = lauda_index::SimpleCpuEmbeddingProvider;
            for new_uid in reconcile.to_index.iter().take(5) {
                if let Ok(msg) = self.get_message(username, mailbox, *new_uid).await {
                    let mut temp_index = lauda_index::MessageIndex::new();
                    temp_index.index_message_semantic(mailbox, &msg, &provider);
                    if let Some(doc) = temp_index.documents().get(new_uid) {
                        let _ = store.save_document(mailbox, doc, 0);
                    }
                }
            }

            let sync_status = store
                .get_sync_status(mailbox, total_count)
                .unwrap_or_else(|_| MailboxSyncStatus::new(0, total_count));

            if query.trim().is_empty() {
                let envelopes = self.list_messages(username, mailbox, None, None).await?;
                return Ok(SearchResult {
                    messages: envelopes,
                    sync_status,
                });
            }

            // 2. Busca ultrarrápida direta do índice SQLite via BM25 SQL
            let ranked = store
                .search_bm25(mailbox, query, 50)
                .map_err(|e| BackendError::Internal(format!("Erro na busca BM25 SQLite: {}", e)))?;

            Ok(SearchResult {
                messages: ranked,
                sync_status,
            })
        } else {
            let envelopes = self.list_messages(username, mailbox, None, None).await?;
            let total_count = envelopes.len() as u64;
            let sync_status = MailboxSyncStatus::new(total_count, total_count);

            if query.trim().is_empty() {
                return Ok(SearchResult {
                    messages: envelopes,
                    sync_status,
                });
            }

            let mut index = lauda_index::MessageIndex::new();
            let provider = lauda_index::SimpleCpuEmbeddingProvider;

            for env in &envelopes {
                if let Ok(msg) = self.get_message(username, mailbox, env.uid).await {
                    index.index_message_semantic(mailbox, &msg, &provider);
                }
            }

            let results = index.search_hybrid(query, &provider, 0.5);

            let mut ranked = Vec::new();
            for res in results {
                if let Some(found) = envelopes.iter().find(|e| e.uid == res.uid) {
                    ranked.push(found.clone());
                }
            }

            Ok(SearchResult {
                messages: ranked,
                sync_status,
            })
        }
    }

    async fn get_index_slice(
        &self,
        username: &str,
        mailbox: &str,
        days: u32,
    ) -> Result<Vec<lauda_index::IndexedDoc>, BackendError> {
        let _ = username;
        if let Some(ref store) = self.sqlite_index {
            // Garante reconciliação antes de exportar o slice
            if let Ok(dir) = self.resolve_mailbox_dir(mailbox) {
                let (_, uid_to_prefix) = self.load_uid_maps(&dir);
                let active_uids: HashSet<u32> = uid_to_prefix.keys().copied().collect();
                let _ = store.reconcile_mailbox(mailbox, &active_uids);
            }
            store.get_recent_slice(mailbox, days).map_err(|e| {
                BackendError::Internal(format!("Erro ao obter fatia de índice: {}", e))
            })
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_sieve_script(&self, username: &str) -> Result<String, BackendError> {
        let _ = username;
        // Caminho do script de usuário no home do Maildir ou na raiz do Maildir
        let candidates = [
            self.base_dir
                .join("home")
                .join("sieve")
                .join("active.sieve"),
            self.base_dir.join("home").join(".dovecot.sieve"),
            self.base_dir.join(".dovecot.sieve"),
            self.base_dir.join("active.sieve"),
        ];

        for path in &candidates {
            if path.exists()
                && let Ok(content) = std::fs::read_to_string(path)
            {
                return Ok(content);
            }
        }

        Ok(String::new())
    }

    async fn put_sieve_script(&self, username: &str, script: &str) -> Result<(), BackendError> {
        let _ = username;
        let sieve_dir = self.base_dir.join("home").join("sieve");
        let _ = std::fs::create_dir_all(&sieve_dir);

        let script_path = sieve_dir.join("active.sieve");
        std::fs::write(&script_path, script)
            .map_err(|e| BackendError::Internal(format!("Falha ao salvar script Sieve: {}", e)))?;

        // Link simbólico ou cópia para ~/.dovecot.sieve
        let active_link = self.base_dir.join("home").join(".dovecot.sieve");
        let _ = std::fs::write(&active_link, script);

        Ok(())
    }
}
