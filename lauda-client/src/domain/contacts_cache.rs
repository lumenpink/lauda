use crate::domain::ports::ClientError;
use lauda_types::contact::ContactEntry;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

/// Gerenciador de cache local de contatos offline do lauda-client.
#[derive(Clone)]
pub struct LocalContactsCache {
    base_dir: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    conn: Arc<Mutex<Connection>>,
}

impl LocalContactsCache {
    /// Inicializa ou abre o banco de dados de contatos no diretório informado.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open<P: AsRef<Path>>(base_dir: P) -> Result<Self, ClientError> {
        let base_path = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_path)
            .map_err(|e| ClientError::Internal(format!("Erro ao criar diretório de cache: {e}")))?;

        let db_path = base_path.join("contacts.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| ClientError::Internal(format!("Erro ao abrir SQLite de contatos: {e}")))?;

        // Configuração de alto desempenho para leituras e escritas seguras
        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        );

        // Criação de tabelas
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                display_name TEXT,
                primary_email TEXT NOT NULL,
                secondary_emails TEXT NOT NULL,
                phone_numbers TEXT NOT NULL,
                notes TEXT,
                vcard_raw TEXT NOT NULL,
                has_autocrypt INTEGER NOT NULL DEFAULT 0,
                autocrypt_fingerprint TEXT,
                etag TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                deleted_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_client_contacts_email ON contacts (primary_email);
            CREATE INDEX IF NOT EXISTS idx_client_contacts_name ON contacts (display_name);

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| ClientError::Internal(format!("Erro ao criar schema SQLite: {e}")))?;

        Ok(Self {
            base_dir: base_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open<P: AsRef<Path>>(base_dir: P) -> Result<Self, ClientError> {
        Ok(Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        })
    }

    /// Retorna o caminho base do cache.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Retorna o último sync_token armazenado.
    pub fn get_sync_token(&self) -> Result<Option<String>, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT value FROM metadata WHERE key = 'sync_token'")
                .map_err(|e| ClientError::Internal(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| ClientError::Internal(e.to_string()))?;
            if let Some(row) = rows
                .next()
                .map_err(|e| ClientError::Internal(e.to_string()))?
            {
                let token: String = row
                    .get(0)
                    .map_err(|e| ClientError::Internal(e.to_string()))?;
                Ok(Some(token))
            } else {
                Ok(None)
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window()
                && let Ok(Some(storage)) = window.local_storage()
            {
                let token = storage.get_item("lauda_contacts_sync_token").ok().flatten();
                Ok(token)
            } else {
                Ok(None)
            }
        }
    }

    /// Retorna o total de contatos ativos no cache local.
    pub fn count_contacts(&self) -> Result<usize, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM contacts WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| ClientError::Internal(e.to_string()))?;
            Ok(count as usize)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(0)
        }
    }

    /// Carrega todos os contatos ativos ordenados por nome de exibição e e-mail.
    pub fn load_contacts(&self) -> Result<Vec<ContactEntry>, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare(
                    "SELECT id, username, display_name, primary_email, secondary_emails, phone_numbers,
                            notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                            created_at, updated_at, deleted_at
                     FROM contacts
                     WHERE deleted_at IS NULL
                     ORDER BY display_name COLLATE NOCASE ASC, primary_email ASC",
                )
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let rows = stmt
                .query_map([], Self::map_row)
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let mut list = Vec::new();
            for r in rows {
                list.push(r.map_err(|e| ClientError::Internal(e.to_string()))?);
            }
            Ok(list)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Vec::new())
        }
    }

    /// Busca contatos ativos por correspondência no nome de exibição ou e-mail.
    pub fn search_contacts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContactEntry>, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let pattern = format!("%{}%", query.trim().to_lowercase());
            let mut stmt = conn
                .prepare(
                    "SELECT id, username, display_name, primary_email, secondary_emails, phone_numbers,
                            notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                            created_at, updated_at, deleted_at
                     FROM contacts
                     WHERE deleted_at IS NULL
                       AND (LOWER(primary_email) LIKE ?1 OR LOWER(display_name) LIKE ?1)
                     ORDER BY display_name COLLATE NOCASE ASC
                     LIMIT ?2",
                )
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let rows = stmt
                .query_map(params![pattern, limit as i64], Self::map_row)
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let mut list = Vec::new();
            for r in rows {
                list.push(r.map_err(|e| ClientError::Internal(e.to_string()))?);
            }
            Ok(list)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (query, limit);
            Ok(Vec::new())
        }
    }

    /// Executa busca heurística ponderada (fuzzy matching, tolerância a erros e acentos)
    /// sobre a coleção de contatos ativos.
    pub fn search_contacts_fuzzy(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ContactEntry>, ClientError> {
        let all_contacts = self.load_contacts()?;
        let mut scored: Vec<(u32, ContactEntry)> = all_contacts
            .into_iter()
            .filter_map(|contact| {
                let name = contact.display_name.as_deref().unwrap_or("");
                let score = crate::domain::fuzzy::calculate_fuzzy_match_score(
                    query,
                    name,
                    &contact.primary_email,
                );
                if score > 0 {
                    Some((score, contact))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|(s1, c1), (s2, c2)| {
            s2.cmp(s1).then_with(|| {
                let n1 = c1.display_name.as_deref().unwrap_or(&c1.primary_email);
                let n2 = c2.display_name.as_deref().unwrap_or(&c2.primary_email);
                n1.to_lowercase().cmp(&n2.to_lowercase())
            })
        });

        Ok(scored.into_iter().take(limit).map(|(_, c)| c).collect())
    }

    /// Aplica lote de atualizações e remoções (tombstones) e atualiza o sync_token de forma atômica.
    pub fn apply_sync_delta(
        &self,
        updated: &[ContactEntry],
        deleted_ids: &[String],
        new_token: &str,
    ) -> Result<(), ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut conn = self.conn.lock().unwrap();
            let tx = conn
                .transaction()
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            // 1. Inserir ou atualizar contatos recebidos
            {
                let mut stmt_upsert = tx
                    .prepare(
                        "INSERT INTO contacts (
                            id, username, display_name, primary_email, secondary_emails, phone_numbers,
                            notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                            created_at, updated_at, deleted_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                        ON CONFLICT(id) DO UPDATE SET
                            display_name = excluded.display_name,
                            primary_email = excluded.primary_email,
                            secondary_emails = excluded.secondary_emails,
                            phone_numbers = excluded.phone_numbers,
                            notes = excluded.notes,
                            vcard_raw = excluded.vcard_raw,
                            has_autocrypt = excluded.has_autocrypt,
                            autocrypt_fingerprint = excluded.autocrypt_fingerprint,
                            etag = excluded.etag,
                            version = excluded.version,
                            updated_at = excluded.updated_at,
                            deleted_at = excluded.deleted_at",
                    )
                    .map_err(|e| ClientError::Internal(e.to_string()))?;

                for c in updated {
                    let sec_emails = serde_json::to_string(&c.secondary_emails)
                        .unwrap_or_else(|_| "[]".to_string());
                    let phones = serde_json::to_string(&c.phone_numbers)
                        .unwrap_or_else(|_| "[]".to_string());

                    stmt_upsert
                        .execute(params![
                            c.id,
                            c.username,
                            c.display_name,
                            c.primary_email.to_lowercase(),
                            sec_emails,
                            phones,
                            c.notes,
                            c.vcard_raw,
                            if c.has_autocrypt { 1 } else { 0 },
                            c.autocrypt_fingerprint,
                            c.etag,
                            c.version,
                            c.created_at as i64,
                            c.updated_at as i64,
                            c.deleted_at.map(|d| d as i64),
                        ])
                        .map_err(|e| ClientError::Internal(e.to_string()))?;
                }
            }

            // 2. Remover contatos que foram excluídos no servidor
            {
                let mut stmt_del = tx
                    .prepare("DELETE FROM contacts WHERE id = ?1")
                    .map_err(|e| ClientError::Internal(e.to_string()))?;

                for del_id in deleted_ids {
                    stmt_del
                        .execute(params![del_id])
                        .map_err(|e| ClientError::Internal(e.to_string()))?;
                }
            }

            // 3. Persistir novo sync_token
            tx.execute(
                "INSERT INTO metadata (key, value) VALUES ('sync_token', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![new_token],
            )
            .map_err(|e| ClientError::Internal(e.to_string()))?;

            tx.commit()
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (updated, deleted_ids, new_token);
            Ok(())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn map_row(row: &rusqlite::Row) -> Result<ContactEntry, rusqlite::Error> {
        let sec_emails_str: String = row.get(4)?;
        let phones_str: String = row.get(5)?;
        let has_auto_int: i32 = row.get(8)?;
        let created_i64: i64 = row.get(12)?;
        let updated_i64: i64 = row.get(13)?;
        let deleted_i64: Option<i64> = row.get(14)?;

        let secondary_emails = serde_json::from_str(&sec_emails_str).unwrap_or_default();
        let phone_numbers = serde_json::from_str(&phones_str).unwrap_or_default();

        let vcard_raw: String = row.get(7)?;
        let birthday = vcard_raw
            .lines()
            .find_map(|l| l.strip_prefix("BDAY:").map(|s| s.trim().to_string()));

        Ok(ContactEntry {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            primary_email: row.get(3)?,
            secondary_emails,
            phone_numbers,
            birthday,
            notes: row.get(6)?,
            vcard_raw,
            has_autocrypt: has_auto_int != 0,
            autocrypt_fingerprint: row.get(9)?,
            etag: row.get(10)?,
            version: row.get(11)?,
            created_at: created_i64 as u64,
            updated_at: updated_i64 as u64,
            deleted_at: deleted_i64.map(|d| d as u64),
        })
    }
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Cria um novo registro ContactEntry com vCard 4.0 a partir dos dados do formulário.
pub fn create_local_contact(
    username: &str,
    name: Option<&str>,
    email: &str,
    notes: Option<&str>,
) -> ContactEntry {
    let now = crate::domain::time::current_unix_timestamp();
    let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let id = format!("c-{now:x}-{seq:x}");
    let clean_email = email.trim().to_lowercase();
    let clean_name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
    let clean_notes = notes
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());

    let mut vcard = String::new();
    vcard.push_str("BEGIN:VCARD\r\nVERSION:4.0\r\n");
    if let Some(ref n) = clean_name {
        vcard.push_str(&format!("FN:{n}\r\n"));
    } else {
        vcard.push_str(&format!("FN:{clean_email}\r\n"));
    }
    vcard.push_str(&format!("EMAIL:{clean_email}\r\n"));
    if let Some(ref note) = clean_notes {
        vcard.push_str(&format!("NOTE:{note}\r\n"));
    }
    vcard.push_str("END:VCARD\r\n");

    let etag = format!("etag-{now:x}-{seq:x}");

    ContactEntry {
        id,
        username: username.to_string(),
        display_name: clean_name,
        primary_email: clean_email,
        secondary_emails: vec![],
        phone_numbers: vec![],
        birthday: None,
        notes: clean_notes,
        vcard_raw: vcard,
        has_autocrypt: false,
        autocrypt_fingerprint: None,
        etag,
        version: 1,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Formata o contato como `Nome <email>` se houver nome, ou apenas `email`.
pub fn format_contact_recipient(display_name: Option<&str>, email: &str) -> String {
    let email_clean = email.trim();
    if let Some(name) = display_name {
        let name_clean = name.trim();
        if !name_clean.is_empty() {
            return format!("{name_clean} <{email_clean}>");
        }
    }
    email_clean.to_string()
}
