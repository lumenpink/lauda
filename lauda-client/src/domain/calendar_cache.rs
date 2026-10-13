use crate::domain::ports::ClientError;
use lauda_types::calendar::CalendarEvent;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Connection, params};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

/// Gerenciador de cache local offline de eventos do lauda-client.
#[derive(Clone)]
pub struct LocalCalendarCache {
    base_dir: PathBuf,
    #[cfg(not(target_arch = "wasm32"))]
    conn: Arc<Mutex<Connection>>,
}

impl LocalCalendarCache {
    /// Inicializa ou abre o banco de dados de eventos no diretório informado.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open<P: AsRef<Path>>(base_dir: P) -> Result<Self, ClientError> {
        let base_path = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_path)
            .map_err(|e| ClientError::Internal(format!("Erro ao criar diretório de cache: {e}")))?;

        let db_path = base_path.join("calendar.db");
        let conn = Connection::open(&db_path).map_err(|e| {
            ClientError::Internal(format!("Erro ao abrir SQLite de calendário: {e}"))
        })?;

        let _ = conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        );

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS client_events (
                id TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                location TEXT,
                start_ts INTEGER NOT NULL,
                end_ts INTEGER NOT NULL,
                is_all_day INTEGER NOT NULL DEFAULT 0,
                rrule TEXT,
                ics_raw TEXT NOT NULL,
                etag TEXT NOT NULL,
                status TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                deleted_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_client_events_time ON client_events (start_ts, end_ts);
            CREATE INDEX IF NOT EXISTS idx_client_events_summary ON client_events (summary);

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
                .prepare("SELECT value FROM metadata WHERE key = 'calendar_sync_token'")
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
                let token = storage.get_item("lauda_calendar_sync_token").ok().flatten();
                Ok(token)
            } else {
                Ok(None)
            }
        }
    }

    /// Retorna o total de eventos ativos no cache local.
    pub fn count_events(&self) -> Result<usize, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM client_events WHERE deleted_at IS NULL",
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

    /// Carrega todos os eventos ativos ordenados por data de início.
    pub fn load_events(&self) -> Result<Vec<CalendarEvent>, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn
                .prepare(
                    "SELECT ics_raw FROM client_events 
                     WHERE deleted_at IS NULL 
                     ORDER BY start_ts ASC",
                )
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            let mut events = Vec::new();
            for item in rows {
                if let Ok(ics) = item
                    && let Ok(ev) = CalendarEvent::from_ical(&ics)
                {
                    events.push(ev);
                }
            }
            Ok(events)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Vec::new())
        }
    }

    /// Insere ou atualiza um evento no cache local.
    pub fn upsert_event(&self, event: &CalendarEvent) -> Result<(), ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let ics_raw = event.to_ical();
            let start_ts = event.start.to_timestamp_utc();
            let end_ts = event.end.to_timestamp_utc();
            let is_all_day = if event.is_all_day { 1 } else { 0 };
            let rrule = event.rrule.as_ref().map(|r| r.to_rrule_string());
            let etag = format!("{:016x}", ics_raw.len());

            conn.execute(
                "INSERT INTO client_events (
                    id, summary, location, start_ts, end_ts, is_all_day,
                    rrule, ics_raw, etag, status, version, created_at, updated_at, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?11, NULL)
                ON CONFLICT(id) DO UPDATE SET
                    summary = excluded.summary,
                    location = excluded.location,
                    start_ts = excluded.start_ts,
                    end_ts = excluded.end_ts,
                    is_all_day = excluded.is_all_day,
                    rrule = excluded.rrule,
                    ics_raw = excluded.ics_raw,
                    etag = excluded.etag,
                    status = excluded.status,
                    version = client_events.version + 1,
                    updated_at = excluded.updated_at,
                    deleted_at = NULL",
                params![
                    event.uid,
                    event.summary,
                    event.location,
                    start_ts,
                    end_ts,
                    is_all_day,
                    rrule,
                    ics_raw,
                    etag,
                    event.status.to_ical_str(),
                    now,
                ],
            )
            .map_err(|e| ClientError::Internal(e.to_string()))?;

            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = event;
            Ok(())
        }
    }

    /// Exclui suavemente (soft-delete) um evento do cache local.
    pub fn delete_event(&self, id: &str) -> Result<bool, ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let rows = conn
                .execute(
                    "UPDATE client_events 
                     SET deleted_at = ?1, updated_at = ?1, version = version + 1 
                     WHERE id = ?2 AND deleted_at IS NULL",
                    params![now, id],
                )
                .map_err(|e| ClientError::Internal(e.to_string()))?;

            Ok(rows > 0)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = id;
            Ok(true)
        }
    }

    /// Aplica delta de sincronização recebido do servidor.
    pub fn apply_sync_delta(
        &self,
        updated: &[CalendarEvent],
        deleted_ids: &[String],
        new_token: &str,
    ) -> Result<(), ClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            for ev in updated {
                self.upsert_event(ev)?;
            }
            for del_id in deleted_ids {
                self.delete_event(del_id)?;
            }

            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO metadata (key, value) VALUES ('calendar_sync_token', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![new_token],
            )
            .map_err(|e| ClientError::Internal(e.to_string()))?;

            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = updated;
            let _ = deleted_ids;
            let _ = new_token;
            Ok(())
        }
    }

    /// Busca eventos por substring de resumo ou local.
    pub fn search_events(&self, query: &str) -> Result<Vec<CalendarEvent>, ClientError> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.load_events();
        }

        let all = self.load_events()?;
        Ok(all
            .into_iter()
            .filter(|ev| {
                ev.summary.to_lowercase().contains(&q)
                    || ev
                        .location
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .collect())
    }
}

/// Helper determinístico que detecta e extrai um convite VEVENT do corpo de um e-mail ou payload de anexo.
pub fn detect_ics_invite(text_or_body: &str) -> Option<CalendarEvent> {
    let start_tag = "BEGIN:VCALENDAR";
    let end_tag = "END:VCALENDAR";

    let start_pos = text_or_body.find(start_tag)?;
    let remainder = &text_or_body[start_pos..];
    let end_rel = remainder.find(end_tag)?;
    let ics_payload = &remainder[..end_rel + end_tag.len()];

    CalendarEvent::from_ical(ics_payload).ok()
}
