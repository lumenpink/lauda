use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use async_trait::async_trait;
use lauda_types::{ComposeForm, MailboxInfo, Message, MessageEnvelope};
use mail_parser::{Address, MessageParser};

use crate::domain::ports::{BackendError, MailBackend};

/// Backend de correio baseado em armazenamento Maildir real (padrão Dovecot).
pub struct MaildirBackend {
    base_dir: PathBuf,
    users: RwLock<HashMap<String, String>>, // username -> password
}

impl MaildirBackend {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            users: RwLock::new(HashMap::new()),
        }
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
            "ALL" => self.base_dir.join(".All"),
            other => {
                let custom = self.base_dir.join(format!(".{}", other));
                if custom.exists() {
                    custom
                } else {
                    return Err(BackendError::NotFound(format!("Caixa postal '{}' não encontrada", mailbox)));
                }
            }
        };

        if !dir.exists() {
            return Err(BackendError::NotFound(format!("Diretório da caixa postal não existe: {:?}", dir)));
        }

        Ok(dir)
    }

    fn load_uid_maps(&self, dir: &Path) -> (HashMap<String, u32>, HashMap<u32, String>) {
        let mut prefix_to_uid = HashMap::new();
        let mut uid_to_prefix = HashMap::new();
        let uidlist_file = dir.join("dovecot-uidlist");

        if let Ok(content) = std::fs::read_to_string(uidlist_file) {
            for line in content.lines() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(uid) = parts[0].parse::<u32>() {
                        let prefix = parts[1].trim_start_matches(':');
                        if !prefix.is_empty() {
                            prefix_to_uid.insert(prefix.to_string(), uid);
                            uid_to_prefix.insert(uid, prefix.to_string());
                        }
                    }
                }
            }
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
}

#[async_trait]
impl MailBackend for MaildirBackend {
    async fn authenticate(&self, username: &str, password: &str) -> Result<(), BackendError> {
        let lock = self.users.read().map_err(|e| BackendError::Internal(e.to_string()))?;
        if let Some(expected_pass) = lock.get(username) {
            if expected_pass == password {
                return Ok(());
            }
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
                        if *sub == "new" || (!filename.contains(":2,S") && !filename.contains(",S")) {
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
    ) -> Result<Vec<MessageEnvelope>, BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (prefix_to_uid, _) = self.load_uid_maps(&dir);
        let mut envelopes = Vec::new();
        let mut fallback_uid = 1000u32;

        for sub in &["new", "cur"] {
            let sub_path = dir.join(sub);
            if let Ok(entries) = std::fs::read_dir(sub_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    let filename = entry.file_name().to_string_lossy().to_string();
                    if filename.starts_with('.') {
                        continue;
                    }

                    let prefix = filename.split(":2,").next().unwrap_or(&filename);
                    let uid = prefix_to_uid.get(prefix).copied().unwrap_or_else(|| {
                        fallback_uid += 1;
                        fallback_uid
                    });

                    let is_read = filename.contains(":2,S") || filename.contains(",S");
                    let is_flagged = filename.contains('F');

                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Some(parsed) = MessageParser::default().parse_headers(&bytes) {
                            let subject = parsed.subject().unwrap_or("(Sem assunto)").to_string();
                            let sender = parsed.from().map(Self::format_address).unwrap_or_else(|| "Desconhecido".to_string());
                            let date = parsed.date().map(|d| d.to_rfc3339()).unwrap_or_else(|| {
                                if let Ok(metadata) = entry.metadata() {
                                    if let Ok(modified) = metadata.modified() {
                                        let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                                        return datetime.to_rfc3339();
                                    }
                                }
                                "2026-10-05T00:00:00Z".to_string()
                            });

                            envelopes.push(MessageEnvelope {
                                uid,
                                subject,
                                sender,
                                date,
                                is_read,
                                is_flagged,
                            });
                        }
                    }
                }
            }
        }

        // Ordenar do mais novo para o mais antigo (UID decrescente)
        envelopes.sort_by(|a, b| b.uid.cmp(&a.uid));

        Ok(envelopes)
    }

    async fn get_message(
        &self,
        _username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, BackendError> {
        let dir = self.resolve_mailbox_dir(mailbox)?;
        let (prefix_to_uid, uid_to_prefix) = self.load_uid_maps(&dir);

        let target_prefix = uid_to_prefix.get(&uid);

        // Encontrar o arquivo correspondente ao UID ou prefixo
        let mut target_file = None;
        let mut is_read = false;
        let mut is_flagged = false;

        for sub in &["cur", "new"] {
            let sub_path = dir.join(sub);
            if let Ok(entries) = std::fs::read_dir(sub_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let prefix = filename.split(":2,").next().unwrap_or(&filename);

                    let matched = if let Some(target) = target_prefix {
                        prefix == target
                    } else if let Some(&file_uid) = prefix_to_uid.get(prefix) {
                        file_uid == uid
                    } else {
                        false
                    };

                    if matched {
                        is_read = filename.contains(":2,S") || filename.contains(",S");
                        is_flagged = filename.contains('F');
                        target_file = Some(path);
                        break;
                    }
                }
            }
            if target_file.is_some() {
                break;
            }
        }

        let file_path = target_file.ok_or_else(|| {
            BackendError::NotFound(format!("Mensagem UID {} não encontrada na caixa {}", uid, mailbox))
        })?;

        let bytes = std::fs::read(&file_path).map_err(|e| BackendError::Internal(e.to_string()))?;
        let parsed = MessageParser::default()
            .parse(&bytes)
            .ok_or_else(|| BackendError::Internal("Falha ao analisar mensagem RFC 5322".to_string()))?;

        let subject = parsed.subject().unwrap_or("(Sem assunto)").to_string();
        let sender = parsed.from().map(Self::format_address).unwrap_or_else(|| "Desconhecido".to_string());
        let date = parsed.date().map(|d| d.to_rfc3339()).unwrap_or_else(|| "2026-10-05T00:00:00Z".to_string());

        let envelope = MessageEnvelope {
            uid,
            subject,
            sender,
            date,
            is_read,
            is_flagged,
        };

        let txt_original = parsed.body_text(0).map(|s| s.to_string());
        let html_sanitized = parsed.body_html(0).map(|s| s.to_string());

        // Se houver texto puro, use-o para o txt_auto; caso contrário, faça fallback para html
        let txt_auto = txt_original.clone().or_else(|| {
            html_sanitized.as_ref().map(|html| {
                html.replace("<br>", "\n")
                    .replace("<br/>", "\n")
                    .replace("<br />", "\n")
                    .replace("<p>", "\n\n")
                    .replace("</p>", "")
            })
        });

        Ok(Message {
            envelope,
            txt_auto,
            txt_original,
            html_sanitized,
        })
    }

    async fn send_message(&self, _username: &str, form: &ComposeForm) -> Result<(), BackendError> {
        let sent_dir = self.base_dir.join(".Sent").join("cur");
        if sent_dir.exists() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = format!("{}.M_out.lauda:2,S", timestamp);
            let raw_email = format!(
                "From: {}\nTo: {}\nSubject: {}\nDate: {}\nContent-Type: text/plain; charset=utf-8\n\n{}",
                form.from, form.to, form.subject, chrono::Utc::now().to_rfc3339(), form.body
            );
            let _ = std::fs::write(sent_dir.join(filename), raw_email);
        }

        Ok(())
    }
}
