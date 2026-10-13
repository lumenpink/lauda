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

            CREATE INDEX IF NOT EXISTS idx_messages_date ON cache_messages (date_unix);",
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

    /// Parser minimalista para timestamps ISO-8601 UTC (ex: "2026-10-10T12:00:00Z" ou "2026-10-10 12:00:00").
    fn parse_iso_date_to_unix(date_str: &str) -> Option<u64> {
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

        let time_part = if parts.len() > 1 {
            parts[1].trim_end_matches('Z')
        } else {
            "00:00:00"
        };
        let time_parts: Vec<&str> = time_part.split(':').collect();
        let hour: u64 = time_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let min: u64 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let sec: u64 = time_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

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

        let secs = (days as u64) * 86400 + hour * 3600 + min * 60 + sec;
        Some(secs)
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
