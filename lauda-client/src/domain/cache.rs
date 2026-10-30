use lauda_types::{MailboxInfo, Message, MessageEnvelope};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

/// Motor de persistência de cache local de mensagens.
///
/// Suporta:
/// - Desktop & Android (Native): Banco de dados SQLite local (`messages.db`) de alta performance com modo WAL.
/// - Web (WASM): Persistência assíncrona/estruturada via localStorage/IndexedDB.
#[derive(Clone)]
pub struct LocalMessageCache {
    base_dir: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for LocalMessageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalMessageCache")
            .field("base_dir", &self.base_dir)
            .finish()
    }
}

impl LocalMessageCache {
    /// Abre ou cria o banco de cache SQLite no diretório base especificado.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open<P: AsRef<Path>>(base_dir: P) -> std::io::Result<Self> {
        let base_path = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_path)?;

        let db_path = base_path.join("messages.db");
        let conn = Connection::open(&db_path).map_err(|e| std::io::Error::other(e.to_string()))?;

        // Configuração de alto desempenho para leituras e escritas seguras
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        );

        // Criação de tabelas
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_mailboxes (
                name TEXT PRIMARY KEY,
                total_messages INTEGER NOT NULL,
                unread_messages INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cache_envelopes (
                mailbox TEXT NOT NULL,
                uid INTEGER NOT NULL,
                envelope_json TEXT NOT NULL,
                date_unix INTEGER NOT NULL,
                sort_order INTEGER NOT NULL,
                PRIMARY KEY (mailbox, uid)
            );

            CREATE INDEX IF NOT EXISTS idx_envelopes_box_sort ON cache_envelopes (mailbox, sort_order ASC);
            CREATE INDEX IF NOT EXISTS idx_envelopes_box_uid ON cache_envelopes (mailbox, uid DESC);
            CREATE INDEX IF NOT EXISTS idx_envelopes_box_date ON cache_envelopes (mailbox, date_unix DESC);

            CREATE TABLE IF NOT EXISTS cache_messages (
                mailbox TEXT NOT NULL,
                uid INTEGER NOT NULL,
                message_json TEXT NOT NULL,
                date_unix INTEGER NOT NULL,
                cached_at INTEGER NOT NULL,
                PRIMARY KEY (mailbox, uid)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_date ON cache_messages (date_unix);

            CREATE TABLE IF NOT EXISTS cache_indexed_documents (
                mailbox TEXT NOT NULL,
                uid INTEGER NOT NULL,
                doc_json TEXT NOT NULL,
                PRIMARY KEY (mailbox, uid)
            );

            CREATE INDEX IF NOT EXISTS idx_indexed_docs_box ON cache_indexed_documents (mailbox);",
        )
        .map_err(|e| std::io::Error::other(e.to_string()))?;

        Ok(Self {
            base_dir: base_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open<P: AsRef<Path>>(base_dir: P) -> std::io::Result<Self> {
        Ok(Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        })
    }

    /// Cria uma nova instância de cache apontando para o diretório informado.
    /// No ambiente nativo, inicializa o banco SQLite via `open`.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::open(&base_dir).unwrap_or_else(|_| {
                // Fallback em memória caso falhe o acesso ao diretório
                let conn = Connection::open_in_memory().expect("sqlite in-memory fallback");
                Self {
                    base_dir: base_dir.as_ref().to_path_buf(),
                    conn: Arc::new(Mutex::new(conn)),
                }
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                base_dir: base_dir.as_ref().to_path_buf(),
            }
        }
    }

    /// Retorna o caminho base do cache.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Sanitiza o nome da caixa de correio para uso seguro como chave.
    #[allow(dead_code)]
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

    /// Grava a lista de caixas de correio de forma atômica no SQLite ou storage Web.
    pub fn save_mailboxes(&self, mailboxes: &[MailboxInfo]) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(json) = serde_json::to_string(mailboxes)
            {
                let _ = storage.set_item("lauda_cache_mailboxes", &json);
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let tx = conn
                .transaction()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let now = crate::domain::time::current_unix_timestamp() as i64;
            tx.execute("DELETE FROM cache_mailboxes", [])
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO cache_mailboxes (name, total_messages, unread_messages, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                ).map_err(|e| std::io::Error::other(e.to_string()))?;

                for mb in mailboxes {
                    stmt.execute(params![
                        mb.name,
                        mb.total_messages as i64,
                        mb.unread_messages as i64,
                        now
                    ])
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                }
            }

            tx.commit()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            Ok(())
        }
    }

    /// Lê a lista de caixas de correio em cache. Retorna None se não existir.
    pub fn load_mailboxes(&self) -> std::io::Result<Option<Vec<MailboxInfo>>> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(Some(content)) = storage.get_item("lauda_cache_mailboxes")
            {
                let mailboxes = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(mailboxes))
            } else {
                Ok(None)
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT name, total_messages, unread_messages FROM cache_mailboxes ORDER BY name ASC",
            ).map_err(|e| std::io::Error::other(e.to_string()))?;

            let rows = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let total: i64 = row.get(1)?;
                    let unread: i64 = row.get(2)?;
                    Ok(MailboxInfo {
                        name,
                        total_messages: total as u32,
                        unread_messages: unread as u32,
                    })
                })
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let mut list = Vec::new();
            for r in rows {
                list.push(r.map_err(|e| std::io::Error::other(e.to_string()))?);
            }

            if list.is_empty() {
                Ok(None)
            } else {
                Ok(Some(list))
            }
        }
    }

    /// Grava a lista de envelopes de uma caixa de correio de forma atômica.
    pub fn save_envelopes(
        &self,
        mailbox: &str,
        envelopes: &[MessageEnvelope],
    ) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!("lauda_cache_envelopes_{}", Self::sanitize_slug(mailbox));
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(json) = serde_json::to_string(envelopes)
            {
                let _ = storage.set_item(&key, &json);
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let tx = conn
                .transaction()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            tx.execute(
                "DELETE FROM cache_envelopes WHERE mailbox = ?1",
                params![mailbox],
            )
            .map_err(|e| std::io::Error::other(e.to_string()))?;

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO cache_envelopes (mailbox, uid, envelope_json, date_unix, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                ).map_err(|e| std::io::Error::other(e.to_string()))?;

                for (idx, env) in envelopes.iter().enumerate() {
                    let json = serde_json::to_string(env)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                    let date_unix = Self::parse_iso_date_to_unix(&env.date).unwrap_or(0) as i64;

                    stmt.execute(params![
                        mailbox,
                        env.uid as i64,
                        json,
                        date_unix,
                        idx as i64,
                    ])
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                }
            }

            tx.commit()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            Ok(())
        }
    }

    /// Lê a lista de envelopes de uma caixa de correio em cache.
    pub fn load_envelopes(&self, mailbox: &str) -> std::io::Result<Option<Vec<MessageEnvelope>>> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!("lauda_cache_envelopes_{}", Self::sanitize_slug(mailbox));
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(Some(content)) = storage.get_item(&key)
            {
                let envelopes = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(envelopes))
            } else {
                Ok(None)
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT envelope_json FROM cache_envelopes WHERE mailbox = ?1 ORDER BY sort_order ASC",
            ).map_err(|e| std::io::Error::other(e.to_string()))?;

            let rows = stmt
                .query_map(params![mailbox], |row| {
                    let json: String = row.get(0)?;
                    Ok(json)
                })
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let mut list = Vec::new();
            for r in rows {
                let json = r.map_err(|e| std::io::Error::other(e.to_string()))?;
                let env: MessageEnvelope = serde_json::from_str(&json)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                list.push(env);
            }

            if list.is_empty() {
                Ok(None)
            } else {
                Ok(Some(list))
            }
        }
    }

    /// Atualiza o status de leitura do envelope (e mensagem completa correspondente, se em cache)
    /// no SQLite e no Storage Web de forma atômica.
    pub fn mark_envelope_read(&self, mailbox: &str, uid: u32, read: bool) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!("lauda_cache_envelopes_{}", Self::sanitize_slug(mailbox));
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
            {
                if let Ok(Some(content)) = storage.get_item(&key)
                    && let Ok(mut envelopes) =
                        serde_json::from_str::<Vec<MessageEnvelope>>(&content)
                {
                    if let Some(env) = envelopes.iter_mut().find(|e| e.uid == uid) {
                        env.is_read = read;
                    }
                    if let Ok(json) = serde_json::to_string(&envelopes) {
                        let _ = storage.set_item(&key, &json);
                    }
                }

                let msg_key = format!("lauda_cache_msg_{}_{}", Self::sanitize_slug(mailbox), uid);
                if let Ok(Some(msg_content)) = storage.get_item(&msg_key)
                    && let Ok(mut msg) = serde_json::from_str::<Message>(&msg_content)
                {
                    msg.envelope.is_read = read;
                    if let Ok(msg_json) = serde_json::to_string(&msg) {
                        let _ = storage.set_item(&msg_key, &msg_json);
                    }
                }
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            let tx = conn
                .transaction()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            // 1. Atualizar envelope em cache_envelopes
            let env_opt: Option<String> = tx
                .query_row(
                    "SELECT envelope_json FROM cache_envelopes WHERE mailbox = ?1 AND uid = ?2",
                    params![mailbox, uid as i64],
                    |row| row.get(0),
                )
                .ok();

            if let Some(json) = env_opt
                && let Ok(mut env) = serde_json::from_str::<MessageEnvelope>(&json)
            {
                env.is_read = read;
                if let Ok(new_json) = serde_json::to_string(&env) {
                    let _ = tx.execute(
                        "UPDATE cache_envelopes SET envelope_json = ?1 WHERE mailbox = ?2 AND uid = ?3",
                        params![new_json, mailbox, uid as i64],
                    );
                }
            }

            // 2. Atualizar mensagem completa em cache_messages se existir
            let msg_opt: Option<String> = tx
                .query_row(
                    "SELECT message_json FROM cache_messages WHERE mailbox = ?1 AND uid = ?2",
                    params![mailbox, uid as i64],
                    |row| row.get(0),
                )
                .ok();

            if let Some(json) = msg_opt
                && let Ok(mut msg) = serde_json::from_str::<Message>(&json)
            {
                msg.envelope.is_read = read;
                if let Ok(new_json) = serde_json::to_string(&msg) {
                    let _ = tx.execute(
                        "UPDATE cache_messages SET message_json = ?1 WHERE mailbox = ?2 AND uid = ?3",
                        params![new_json, mailbox, uid as i64],
                    );
                }
            }

            tx.commit()
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            Ok(())
        }
    }

    /// Grava o corpo completo de uma mensagem individual de forma atômica no SQLite ou storage Web.
    pub fn save_message(&self, mailbox: &str, msg: &Message) -> std::io::Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!(
                "lauda_cache_msg_{}_{}",
                Self::sanitize_slug(mailbox),
                msg.envelope.uid
            );
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(json) = serde_json::to_string(msg)
            {
                let _ = storage.set_item(&key, &json);
            }
            Ok(())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let json = serde_json::to_string(msg)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let date_unix = Self::parse_iso_date_to_unix(&msg.envelope.date).unwrap_or(0) as i64;
            let now = crate::domain::time::current_unix_timestamp() as i64;

            conn.execute(
                "INSERT INTO cache_messages (mailbox, uid, message_json, date_unix, cached_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(mailbox, uid) DO UPDATE SET
                     message_json = excluded.message_json,
                     date_unix = excluded.date_unix,
                     cached_at = excluded.cached_at",
                params![mailbox, msg.envelope.uid as i64, json, date_unix, now],
            )
            .map_err(|e| std::io::Error::other(e.to_string()))?;

            Ok(())
        }
    }

    /// Carrega o corpo completo de uma mensagem individual em cache.
    pub fn load_message(&self, mailbox: &str, uid: u32) -> std::io::Result<Option<Message>> {
        #[cfg(target_arch = "wasm32")]
        {
            let key = format!("lauda_cache_msg_{}_{}", Self::sanitize_slug(mailbox), uid);
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
                && let Ok(Some(content)) = storage.get_item(&key)
            {
                let msg = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(msg))
            } else {
                Ok(None)
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let mut stmt = conn
                .prepare("SELECT message_json FROM cache_messages WHERE mailbox = ?1 AND uid = ?2")
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let mut rows = stmt
                .query(params![mailbox, uid as i64])
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            if let Some(row) = rows
                .next()
                .map_err(|e| std::io::Error::other(e.to_string()))?
            {
                let json: String = row
                    .get(0)
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                let msg: Message = serde_json::from_str(&json)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(msg))
            } else {
                Ok(None)
            }
        }
    }

    /// Parser robusto para timestamps de mensagens (ISO-8601/RFC-3339 e RFC-2822).
    /// Suporta offsets de fuso horário (+00:00, -03:00, +0000, -0300) e converte para segundos Unix UTC.
    pub fn parse_iso_date_to_unix(date_str: &str) -> Option<u64> {
        let trimmed = date_str.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Se contiver vírgula (ex: "Fri, 30 Oct 2026 13:00:00 -0300"), descarta o dia da semana
        let s = if let Some(idx) = trimmed.find(',') {
            trimmed[idx + 1..].trim()
        } else {
            trimmed
        };

        // Se o primeiro segmento contiver '-', trata como RFC-3339 / ISO-8601 (ex: YYYY-MM-DD).
        // Caso contrário, trata como RFC-2822 (ex: DD Mon YYYY).
        let first_segment = s.split(&[' ', 'T'][..]).next().unwrap_or("");
        if first_segment.contains('-') {
            Self::parse_rfc3339_to_unix(s)
        } else {
            Self::parse_rfc2822_to_unix(s)
        }
    }

    fn parse_rfc3339_to_unix(s: &str) -> Option<u64> {
        let (date_part, time_and_tz) = if let Some(idx) = s.find(['T', ' ']) {
            (&s[..idx], &s[idx + 1..])
        } else {
            (s, "00:00:00")
        };

        let date_parts: Vec<&str> = date_part.split('-').collect();
        if date_parts.len() != 3 {
            return None;
        }

        let year: i64 = date_parts[0].parse().ok()?;
        let month: u64 = date_parts[1].parse().ok()?;
        let day: u64 = date_parts[2].parse().ok()?;

        let (time_part, offset_secs) = Self::extract_time_and_offset(time_and_tz);

        let time_parts: Vec<&str> = time_part.split(':').collect();
        let hour: u64 = time_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let min: u64 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let sec: u64 = time_parts
            .get(2)
            .and_then(|s| s.split('.').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let local_secs = Self::ymd_hms_to_unix(year, month, day, hour, min, sec)?;
        let utc_secs = (local_secs as i64) - offset_secs;
        if utc_secs < 0 {
            Some(0)
        } else {
            Some(utc_secs as u64)
        }
    }

    fn extract_time_and_offset(s: &str) -> (&str, i64) {
        let trimmed = s.trim();
        if let Some(t) = trimmed.strip_suffix('Z') {
            return (t.trim(), 0);
        }

        // Procura '+' ou '-' após pelo menos 2 caracteres de hora
        if trimmed.len() > 2
            && let Some(idx) = trimmed[2..].find(['+', '-'])
        {
            let actual_idx = 2 + idx;
            let time_part = trimmed[..actual_idx].trim();
            let tz_part = trimmed[actual_idx..].trim();
            let sign = if tz_part.starts_with('+') { 1 } else { -1 };
            let raw_tz = tz_part.trim_start_matches(['+', '-']);
            let offset_secs = Self::parse_tz_digits(raw_tz) * sign;
            return (time_part, offset_secs);
        }

        (trimmed, 0)
    }

    fn parse_tz_digits(raw: &str) -> i64 {
        if let Some((h_str, m_str)) = raw.split_once(':') {
            let h: i64 = h_str.parse().unwrap_or(0);
            let m: i64 = m_str.parse().unwrap_or(0);
            h * 3600 + m * 60
        } else if raw.len() == 4 {
            let h: i64 = raw[..2].parse().unwrap_or(0);
            let m: i64 = raw[2..].parse().unwrap_or(0);
            h * 3600 + m * 60
        } else if raw.len() == 2 {
            let h: i64 = raw.parse().unwrap_or(0);
            h * 3600
        } else {
            0
        }
    }

    fn parse_rfc2822_to_unix(s: &str) -> Option<u64> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 4 {
            return None;
        }

        let day: u64 = parts[0].parse().ok()?;
        let month = match parts[1].to_ascii_lowercase().as_str() {
            "jan" => 1,
            "feb" => 2,
            "mar" => 3,
            "apr" => 4,
            "may" => 5,
            "jun" => 6,
            "jul" => 7,
            "aug" => 8,
            "sep" => 9,
            "oct" => 10,
            "nov" => 11,
            "dec" => 12,
            _ => return None,
        };
        let year: i64 = parts[2].parse().ok()?;

        let time_parts: Vec<&str> = parts[3].split(':').collect();
        let hour: u64 = time_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let min: u64 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let sec: u64 = time_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        let offset_secs = if parts.len() > 4 {
            let tz_part = parts[4];
            let sign = if tz_part.starts_with('+') {
                1
            } else if tz_part.starts_with('-') {
                -1
            } else {
                0
            };
            if sign != 0 {
                let raw = tz_part.trim_start_matches(['+', '-']);
                Self::parse_tz_digits(raw) * sign
            } else {
                0
            }
        } else {
            0
        };

        let local_secs = Self::ymd_hms_to_unix(year, month, day, hour, min, sec)?;
        let utc_secs = (local_secs as i64) - offset_secs;
        if utc_secs < 0 {
            Some(0)
        } else {
            Some(utc_secs as u64)
        }
    }

    fn ymd_hms_to_unix(
        year: i64,
        month: u64,
        day: u64,
        hour: u64,
        min: u64,
        sec: u64,
    ) -> Option<u64> {
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }

        let mut days: i64 = 0;
        for y in 1970..year {
            let is_leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            days += if is_leap { 366 } else { 365 };
        }

        let is_current_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let month_days = [
            31,
            if is_current_leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];

        for m in 1..month {
            days += month_days[(m - 1) as usize];
        }
        days += (day as i64) - 1;

        if days < 0 {
            return None;
        }

        Some((days as u64) * 86400 + hour * 3600 + min * 60 + sec)
    }

    /// Executa purga de mensagens anteriores a uma determinada duração (ex: 90 dias)
    /// com base na data do sistema atual.
    pub fn purge_older_than(&self, max_age: Duration) -> std::io::Result<usize> {
        let now_unix = crate::domain::time::current_unix_timestamp();
        self.purge_expired_messages(max_age, now_unix)
    }

    /// Executa a purga de retenção: remove mensagens completas
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
        let cutoff_unix = (now_unix - max_age_secs) as i64;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let purged = conn
                .execute(
                    "DELETE FROM cache_messages WHERE date_unix < ?1",
                    params![cutoff_unix],
                )
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            Ok(purged)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = cutoff_unix;
            Ok(0)
        }
    }

    /// Salva a fatia de documentos pré-indexados (ex: últimos 90 dias) no cache local.
    pub fn save_index_slice(
        &self,
        mailbox: &str,
        docs: &[lauda_index::IndexedDoc],
    ) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare(
                    "INSERT OR REPLACE INTO cache_indexed_documents (mailbox, uid, doc_json) VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            for doc in docs {
                if let Ok(json) = serde_json::to_string(doc) {
                    let _ = stmt.execute(params![mailbox, doc.uid, json]);
                }
            }
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (mailbox, docs);
            Ok(())
        }
    }

    /// Carrega o MessageIndex reconstruído a partir da fatia salva no cache local para busca offline.
    pub fn load_index_slice(
        &self,
        mailbox: &str,
    ) -> std::io::Result<Option<lauda_index::MessageIndex>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT doc_json FROM cache_indexed_documents WHERE mailbox = ?1")
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let rows = stmt
                .query_map(params![mailbox], |row| row.get::<_, String>(0))
                .map_err(|e| std::io::Error::other(e.to_string()))?;

            let mut index = lauda_index::MessageIndex::new();
            let mut count = 0;

            for r in rows {
                if let Ok(json) = r
                    && let Ok(doc) = serde_json::from_str::<lauda_index::IndexedDoc>(&json)
                {
                    index.insert_doc(doc);
                    count += 1;
                }
            }

            if count > 0 { Ok(Some(index)) } else { Ok(None) }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = mailbox;
            Ok(None)
        }
    }
}

/// Obtém o caminho padrão de diretório de cache conforme a plataforma.
pub fn get_default_cache_dir() -> PathBuf {
    #[cfg(target_arch = "wasm32")]
    {
        PathBuf::from("web_cache")
    }
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
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
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
