use lauda_types::{MailboxInfo, Message, MessageEnvelope};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Motor de persistência de cache local no disco.
///
/// Estrutura no disco:
/// ```text
/// <cache_dir>/
/// ├── mailboxes.json
/// └── mailboxes/
///     └── <mailbox_slug>/
///         ├── envelopes.json
///         └── messages/
///             ├── <uid>.json
///             └── ...
/// ```
#[derive(Clone, Debug)]
pub struct LocalMessageCache {
    base_dir: PathBuf,
}

impl LocalMessageCache {
    /// Cria uma nova instância de cache apontando para o diretório informado.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Retorna o caminho base do cache.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Sanitiza o nome da caixa de correio para uso seguro no sistema de arquivos.
    fn sanitize_slug(mailbox: &str) -> String {
        let mut slug = String::with_capacity(mailbox.len());
        for c in mailbox.chars() {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                slug.push(c);
            } else {
                slug.push('_');
            }
        }
        if slug.is_empty() {
            "default".to_string()
        } else {
            slug
        }
    }

    /// Escreve um arquivo de forma atômica utilizando escrita em arquivo temporário (.tmp)
    /// seguida de renomeação atômica pelo sistema operacional.
    fn write_atomic(dest: &Path, content: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let tmp_file = dest.with_extension(format!(
            "tmp.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        std::fs::write(&tmp_file, content)?;
        std::fs::rename(&tmp_file, dest)?;
        Ok(())
    }

    /// Grava a lista de caixas de correio de forma atômica no disco.
    pub fn save_mailboxes(&self, mailboxes: &[MailboxInfo]) -> std::io::Result<()> {
        let path = self.base_dir.join("mailboxes.json");
        let json = serde_json::to_vec_pretty(mailboxes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::write_atomic(&path, &json)
    }

    /// Lê a lista de caixas de correio em cache. Retorna None se não existir.
    pub fn load_mailboxes(&self) -> std::io::Result<Option<Vec<MailboxInfo>>> {
        let path = self.base_dir.join("mailboxes.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read(&path)?;
        let mailboxes = serde_json::from_slice(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(mailboxes))
    }

    /// Retorna o diretório raiz de uma caixa específica.
    fn mailbox_dir(&self, mailbox: &str) -> PathBuf {
        let slug = Self::sanitize_slug(mailbox);
        self.base_dir.join("mailboxes").join(slug)
    }

    /// Grava a lista de envelopes de uma caixa de correio de forma atômica.
    pub fn save_envelopes(
        &self,
        mailbox: &str,
        envelopes: &[MessageEnvelope],
    ) -> std::io::Result<()> {
        let path = self.mailbox_dir(mailbox).join("envelopes.json");
        let json = serde_json::to_vec_pretty(envelopes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::write_atomic(&path, &json)
    }

    /// Lê a lista de envelopes de uma caixa de correio em cache.
    pub fn load_envelopes(&self, mailbox: &str) -> std::io::Result<Option<Vec<MessageEnvelope>>> {
        let path = self.mailbox_dir(mailbox).join("envelopes.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read(&path)?;
        let envelopes = serde_json::from_slice(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(envelopes))
    }

    /// Caminho do arquivo de corpo de uma mensagem individual.
    fn message_path(&self, mailbox: &str, uid: u32) -> PathBuf {
        self.mailbox_dir(mailbox)
            .join("messages")
            .join(format!("{}.json", uid))
    }

    /// Grava o corpo completo de uma mensagem individual de forma atômica.
    pub fn save_message(&self, mailbox: &str, msg: &Message) -> std::io::Result<()> {
        let path = self.message_path(mailbox, msg.envelope.uid);
        let json = serde_json::to_vec_pretty(msg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Self::write_atomic(&path, &json)
    }

    /// Carrega o corpo completo de uma mensagem individual em cache.
    pub fn load_message(&self, mailbox: &str, uid: u32) -> std::io::Result<Option<Message>> {
        let path = self.message_path(mailbox, uid);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read(&path)?;
        let msg = serde_json::from_slice(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(msg))
    }

    /// Parser minimalista para timestamps ISO-8601 UTC (ex: "2026-10-10T12:00:00Z" ou "2026-10-10 12:00:00").
    /// Não depende de crates externas de data/hora (hermético em Rust std).
    fn parse_iso_date_to_unix(date_str: &str) -> Option<u64> {
        // Formato esperado: YYYY-MM-DD[T| ]HH:MM:SS
        let parts: Vec<&str> = date_str.split(&['T', ' '][..]).collect();
        if parts.is_empty() {
            return None;
        }

        let date_parts: Vec<&str> = parts[0].split('-').collect();
        if date_parts.len() != 3 {
            return None;
        }

        let year: i64 = date_parts[0].parse().ok()?;
        let month: u64 = date_parts[1].parse().ok()?;
        let day: u64 = date_parts[2].parse().ok()?;

        let (hour, min, sec) = if parts.len() > 1 {
            let time_clean = parts[1].trim_end_matches('Z');
            let time_parts: Vec<&str> = time_clean.split(':').collect();
            if time_parts.len() >= 3 {
                (
                    time_parts[0].parse().unwrap_or(0),
                    time_parts[1].parse().unwrap_or(0),
                    time_parts[2]
                        .split('.')
                        .next()
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0),
                )
            } else {
                (0, 0, 0)
            }
        } else {
            (0, 0, 0)
        };

        // Algoritmo de dias julianos simplificado para conversão Unix Timestamp UTC
        // Dias desde 1970-01-01
        let mut y = year;
        let mut m = month as i64;
        if m <= 2 {
            y -= 1;
            m += 12;
        }
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let doy = (153 * (m - 3) + 2) / 5 + (day as i64) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146097 + doe - 719468;

        if days < 0 {
            return None;
        }

        let secs = (days as u64) * 86400 + hour * 3600 + min * 60 + sec;
        Some(secs)
    }

    /// Executa a purga de retenção: remove arquivos de corpo completo de mensagens
    /// cuja data no envelope seja anterior ao limite (ex: 90 dias).
    /// Retorna o número de mensagens expurgadas.
    pub fn purge_expired_messages(
        &self,
        max_age: Duration,
        now_unix: u64,
    ) -> std::io::Result<usize> {
        let max_age_secs = max_age.as_secs();
        if now_unix < max_age_secs {
            return Ok(0);
        }
        let cutoff_unix = now_unix - max_age_secs;

        let mailboxes_dir = self.base_dir.join("mailboxes");
        if !mailboxes_dir.exists() {
            return Ok(0);
        }

        let mut purged_count = 0;

        for entry in std::fs::read_dir(mailboxes_dir)? {
            let entry = entry?;
            let box_path = entry.path();
            if !box_path.is_dir() {
                continue;
            }

            // Ler envelopes da caixa
            let envelopes_file = box_path.join("envelopes.json");
            if !envelopes_file.exists() {
                continue;
            }

            let content = match std::fs::read(&envelopes_file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let envelopes: Vec<MessageEnvelope> = match serde_json::from_slice(&content) {
                Ok(envs) => envs,
                Err(_) => continue,
            };

            let messages_dir = box_path.join("messages");
            if !messages_dir.exists() {
                continue;
            }

            for env in envelopes {
                if let Some(msg_unix) = Self::parse_iso_date_to_unix(&env.date)
                    && msg_unix < cutoff_unix
                {
                    let msg_file = messages_dir.join(format!("{}.json", env.uid));
                    if msg_file.exists() && std::fs::remove_file(msg_file).is_ok() {
                        purged_count += 1;
                    }
                }
            }
        }

        Ok(purged_count)
    }
}

/// Obtém o caminho padrão de diretório de cache conforme a plataforma.
pub fn get_default_cache_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home).join("cache");
        }
        if Path::new("/data/data/in.twolp.lauda/cache").exists() {
            return PathBuf::from("/data/data/in.twolp.lauda/cache");
        }
        PathBuf::from("/data/data/in.twolp.lauda/cache")
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(path) = std::env::var("LAUDA_CACHE_PATH")
            && !path.is_empty()
        {
            return PathBuf::from(path);
        }
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("lauda");
        }
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home).join(".cache/lauda");
        }
        PathBuf::from("cache")
    }
}
