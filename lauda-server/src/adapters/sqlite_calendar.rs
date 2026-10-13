use std::path::Path;
use std::sync::{Arc, Mutex};

use lauda_types::calendar::CalendarEvent;
use rusqlite::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSyncResult {
    pub updated: Vec<(String, String, String)>,
    pub deleted_ids: Vec<String>,
    pub sync_token: String,
}

/// Adaptador de persistência SQLite para calendário com suporte a WAL mode e consultas temporais.
#[derive(Clone)]
pub struct SqliteCalendarStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCalendarStore {
    /// Inicializa ou abre um banco SQLite de calendário no caminho fornecido.
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

             CREATE TABLE IF NOT EXISTS calendar_events (
                 id TEXT PRIMARY KEY,
                 username TEXT NOT NULL,
                 summary TEXT NOT NULL,
                 description TEXT,
                 location TEXT,
                 start_ts INTEGER NOT NULL,
                 end_ts INTEGER NOT NULL,
                 is_all_day INTEGER NOT NULL DEFAULT 0,
                 rrule TEXT,
                 ics_raw TEXT NOT NULL,
                 etag TEXT NOT NULL,
                 version INTEGER NOT NULL DEFAULT 1,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 deleted_at INTEGER
             );

             CREATE INDEX IF NOT EXISTS idx_calendar_user_time 
                 ON calendar_events(username, start_ts, end_ts);

             CREATE INDEX IF NOT EXISTS idx_calendar_user_updated 
                 ON calendar_events(username, updated_at);

             CREATE INDEX IF NOT EXISTS idx_calendar_user_deleted 
                 ON calendar_events(username, deleted_at);",
        )?;
        Ok(())
    }

    /// Calcula ETag determinístico FNV-1a para um payload.
    pub fn compute_etag(content: &[u8]) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in content {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    /// Insere ou atualiza um evento de calendário no banco.
    pub fn upsert_event(
        &self,
        username: &str,
        event: &CalendarEvent,
    ) -> Result<String, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let ics_raw = event.to_ical();
        let etag = Self::compute_etag(ics_raw.as_bytes());
        let start_ts = event.start.to_timestamp_utc();
        let end_ts = event.end.to_timestamp_utc();
        let is_all_day = if event.is_all_day { 1 } else { 0 };
        let rrule_str = event.rrule.as_ref().map(|r| r.to_rrule_string());

        conn.execute(
            "INSERT INTO calendar_events (
                 id, username, summary, description, location,
                 start_ts, end_ts, is_all_day, rrule, ics_raw,
                 etag, version, created_at, updated_at, deleted_at
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5,
                 ?6, ?7, ?8, ?9, ?10,
                 ?11, 1, ?12, ?12, NULL
             )
             ON CONFLICT(id) DO UPDATE SET
                 summary = excluded.summary,
                 description = excluded.description,
                 location = excluded.location,
                 start_ts = excluded.start_ts,
                 end_ts = excluded.end_ts,
                 is_all_day = excluded.is_all_day,
                 rrule = excluded.rrule,
                 ics_raw = excluded.ics_raw,
                 etag = excluded.etag,
                 version = calendar_events.version + 1,
                 updated_at = MAX(excluded.updated_at, calendar_events.updated_at + 1),
                 deleted_at = NULL",
            params![
                event.uid,
                username,
                event.summary,
                event.description,
                event.location,
                start_ts,
                end_ts,
                is_all_day,
                rrule_str,
                ics_raw,
                etag,
                now,
            ],
        )?;

        Ok(etag)
    }

    /// Retorna um evento cru (ics_raw, etag) pelo UID.
    pub fn get_event_raw(
        &self,
        username: &str,
        id: &str,
    ) -> Result<Option<(String, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ics_raw, etag FROM calendar_events 
             WHERE username = ?1 AND id = ?2 AND deleted_at IS NULL",
        )?;

        let mut rows = stmt.query(params![username, id])?;
        if let Some(row) = rows.next()? {
            let ics_raw: String = row.get(0)?;
            let etag: String = row.get(1)?;
            Ok(Some((ics_raw, etag)))
        } else {
            Ok(None)
        }
    }

    /// Marca um evento como removido (soft-delete tombstone com incremento estrito de timestamp).
    pub fn delete_event(&self, username: &str, id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let rows_affected = conn.execute(
            "UPDATE calendar_events 
             SET deleted_at = MAX(?1, updated_at + 1), 
                 updated_at = MAX(?1, updated_at + 1), 
                 version = version + 1 
             WHERE username = ?2 AND id = ?3 AND deleted_at IS NULL",
            params![now, username, id],
        )?;

        Ok(rows_affected > 0)
    }

    /// Consulta eventos em uma faixa temporal [start_ts, end_ts].
    pub fn query_events_by_range(
        &self,
        username: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<(String, String, String)>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, ics_raw, etag FROM calendar_events 
             WHERE username = ?1 
               AND deleted_at IS NULL
               AND ((start_ts <= ?3 AND end_ts >= ?2) OR (start_ts >= ?2 AND start_ts <= ?3))
             ORDER BY start_ts ASC",
        )?;

        let rows = stmt.query_map(params![username, start_ts, end_ts], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let mut events = Vec::new();
        for ev in rows {
            events.push(ev?);
        }
        Ok(events)
    }

    /// Sincronização incremental baseada em timestamp (RFC 6578 sync-collection).
    pub fn sync_events(
        &self,
        username: &str,
        since_token: Option<&str>,
    ) -> Result<CalendarSyncResult, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();

        let since_ts: i64 = since_token
            .and_then(|tok| tok.strip_prefix("ts:"))
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or(0);

        let mut stmt_updated = conn.prepare(
            "SELECT id, ics_raw, etag FROM calendar_events 
             WHERE username = ?1 AND updated_at > ?2 AND deleted_at IS NULL
             ORDER BY updated_at ASC",
        )?;

        let updated_rows = stmt_updated.query_map(params![username, since_ts], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

        let mut updated = Vec::new();
        for item in updated_rows {
            updated.push(item?);
        }

        let mut stmt_deleted = conn.prepare(
            "SELECT id FROM calendar_events 
             WHERE username = ?1 AND deleted_at IS NOT NULL AND deleted_at > ?2
             ORDER BY deleted_at ASC",
        )?;

        let deleted_rows = stmt_deleted.query_map(params![username, since_ts], |row| {
            row.get::<_, String>(0)
        })?;

        let mut deleted_ids = Vec::new();
        for item in deleted_rows {
            deleted_ids.push(item?);
        }

        let max_updated: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(updated_at), 0) FROM calendar_events WHERE username = ?1",
                params![username],
                |r| r.get(0),
            )
            .unwrap_or(0);

        let sync_token = format!("ts:{max_updated}");

        Ok(CalendarSyncResult {
            updated,
            deleted_ids,
            sync_token,
        })
    }
}
