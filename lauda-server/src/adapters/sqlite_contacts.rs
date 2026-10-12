use std::path::Path;
use std::sync::{Arc, Mutex};

use lauda_types::contact::{ContactEntry, ContactSyncRequest, ContactSyncResponse};
use rusqlite::{Connection, params};

/// Adaptador de persistência SQLite para contatos com suporte a WAL mode e sincronização incremental.
#[derive(Clone)]
pub struct SqliteContactsStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteContactsStore {
    /// Inicializa ou abre um banco SQLite de contatos no caminho fornecido.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Inicializa as tabelas e índices necessários no SQLite.
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS contacts (
                 id TEXT PRIMARY KEY,
                 username TEXT NOT NULL,
                 display_name TEXT,
                 primary_email TEXT NOT NULL,
                 secondary_emails TEXT NOT NULL DEFAULT '[]',
                 phone_numbers TEXT NOT NULL DEFAULT '[]',
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

             CREATE INDEX IF NOT EXISTS idx_contacts_user_email 
                 ON contacts(username, primary_email);

             CREATE INDEX IF NOT EXISTS idx_contacts_user_updated 
                 ON contacts(username, updated_at);

             CREATE INDEX IF NOT EXISTS idx_contacts_user_deleted 
                 ON contacts(username, deleted_at);",
        )?;
        Ok(())
    }

    /// Insere ou atualiza um contato no banco.
    pub fn upsert_contact(&self, contact: &ContactEntry) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let sec_emails_json =
            serde_json::to_string(&contact.secondary_emails).unwrap_or_else(|_| "[]".to_string());
        let phones_json =
            serde_json::to_string(&contact.phone_numbers).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
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
                version = contacts.version + 1,
                updated_at = excluded.updated_at,
                deleted_at = excluded.deleted_at",
            params![
                contact.id,
                contact.username,
                contact.display_name,
                contact.primary_email.to_lowercase(),
                sec_emails_json,
                phones_json,
                contact.notes,
                contact.vcard_raw,
                if contact.has_autocrypt { 1 } else { 0 },
                contact.autocrypt_fingerprint,
                contact.etag,
                contact.version,
                contact.created_at as i64,
                contact.updated_at as i64,
                contact.deleted_at.map(|d| d as i64),
            ],
        )?;
        Ok(())
    }

    /// Busca um contato ativo pelo e-mail do usuário proprietário.
    pub fn get_contact_by_email(
        &self,
        username: &str,
        email: &str,
    ) -> Result<Option<ContactEntry>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let email_normalized = email.to_lowercase();
        let mut stmt = conn.prepare(
            "SELECT id, username, display_name, primary_email, secondary_emails, phone_numbers,
                    notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                    created_at, updated_at, deleted_at
             FROM contacts
             WHERE username = ?1 AND primary_email = ?2 AND deleted_at IS NULL",
        )?;

        let mut rows = stmt.query(params![username, email_normalized])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Busca um contato ativo pelo ID e usuário proprietário.
    pub fn get_contact_by_id(
        &self,
        username: &str,
        id: &str,
    ) -> Result<Option<ContactEntry>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, display_name, primary_email, secondary_emails, phone_numbers,
                    notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                    created_at, updated_at, deleted_at
             FROM contacts
             WHERE username = ?1 AND id = ?2 AND deleted_at IS NULL",
        )?;

        let mut rows = stmt.query(params![username, id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Realiza a sincronização incremental a partir de um sync_token anterior.
    pub fn sync_contacts(
        &self,
        username: &str,
        req: &ContactSyncRequest,
    ) -> Result<ContactSyncResponse, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let limit = req.limit.unwrap_or(200);

        // Interpretar sync_token como timestamp base
        let since_ts: i64 = req
            .sync_token
            .as_deref()
            .and_then(|t| t.strip_prefix("ts:").or(Some(t)))
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0);

        // 1. Buscar registros atualizados/criados após since_ts que não foram deletados
        let mut updated = Vec::new();
        let mut max_ts = since_ts;

        let mut stmt_updated = conn.prepare(
            "SELECT id, username, display_name, primary_email, secondary_emails, phone_numbers,
                    notes, vcard_raw, has_autocrypt, autocrypt_fingerprint, etag, version,
                    created_at, updated_at, deleted_at
             FROM contacts
             WHERE username = ?1 AND updated_at > ?2 AND deleted_at IS NULL
             ORDER BY updated_at ASC
             LIMIT ?3",
        )?;

        let mut rows = stmt_updated.query(params![username, since_ts, (limit + 1) as i64])?;
        while let Some(row) = rows.next()? {
            let entry = Self::map_row(row)?;
            if (entry.updated_at as i64) > max_ts {
                max_ts = entry.updated_at as i64;
            }
            updated.push(entry);
        }

        // 2. Buscar tombstones (deleted_at > since_ts)
        let mut deleted_ids = Vec::new();
        let mut stmt_deleted = conn.prepare(
            "SELECT id, updated_at
             FROM contacts
             WHERE username = ?1 AND deleted_at IS NOT NULL AND updated_at > ?2
             ORDER BY updated_at ASC
             LIMIT ?3",
        )?;

        let mut del_rows = stmt_deleted.query(params![username, since_ts, limit as i64])?;
        while let Some(row) = del_rows.next()? {
            let id: String = row.get(0)?;
            let ts: i64 = row.get(1)?;
            if ts > max_ts {
                max_ts = ts;
            }
            deleted_ids.push(id);
        }

        let has_more = updated.len() > limit;
        if has_more {
            updated.truncate(limit);
        }

        let new_token = format!("ts:{}", max_ts);
        Ok(ContactSyncResponse {
            sync_token: new_token,
            updated,
            deleted_ids,
            has_more,
        })
    }

    /// Realiza soft-delete de um contato para registrar tombstone.
    pub fn delete_contact(
        &self,
        username: &str,
        id: &str,
        now_ts: u64,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "UPDATE contacts
             SET deleted_at = ?1, updated_at = ?1, version = version + 1
             WHERE username = ?2 AND id = ?3 AND deleted_at IS NULL",
            params![now_ts as i64, username, id],
        )?;
        Ok(rows_affected > 0)
    }

    /// Mapeia uma linha do SQLite para a struct ContactEntry.
    fn map_row(row: &rusqlite::Row) -> Result<ContactEntry, rusqlite::Error> {
        let sec_emails_str: String = row.get(4)?;
        let phones_str: String = row.get(5)?;
        let has_auto_int: i32 = row.get(8)?;
        let created_i64: i64 = row.get(12)?;
        let updated_i64: i64 = row.get(13)?;
        let deleted_i64: Option<i64> = row.get(14)?;

        let secondary_emails = serde_json::from_str(&sec_emails_str).unwrap_or_default();
        let phone_numbers = serde_json::from_str(&phones_str).unwrap_or_default();

        Ok(ContactEntry {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            primary_email: row.get(3)?,
            secondary_emails,
            phone_numbers,
            notes: row.get(6)?,
            vcard_raw: row.get(7)?,
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
