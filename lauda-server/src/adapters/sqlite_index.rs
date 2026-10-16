use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use lauda_index::{IndexedDoc, MessageIndex};
use rusqlite::{Connection, params};
use urupema::domain::SegmentKind;

/// Resultado da comparação e reconciliação entre UIDs do SQLite e dovecot-uidlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileResult {
    pub to_index: Vec<u32>,
    pub to_remove: Vec<u32>,
}

/// Codifica vetores de segmentos em um BLOB binário compacto (1 byte tipo + 384 floats little-endian).
pub fn encode_segment_embeddings(embeddings: &[(SegmentKind, Vec<f32>)]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(embeddings.len() * (1 + 384 * 4));
    for (kind, vec) in embeddings {
        let k: u8 = match kind {
            SegmentKind::Header => 0,
            SegmentKind::Body => 1,
            SegmentKind::Signature => 2,
            SegmentKind::QuotedThread => 3,
        };
        blob.push(k);
        for &val in vec {
            blob.extend_from_slice(&val.to_le_bytes());
        }
    }
    blob
}

/// Decodifica BLOB binário de volta para a lista de segmentos e vetores.
pub fn decode_segment_embeddings(blob: &[u8]) -> Vec<(SegmentKind, Vec<f32>)> {
    let entry_size = 1 + 384 * 4;
    let mut embeddings = Vec::new();
    for chunk in blob.chunks_exact(entry_size) {
        let kind = match chunk[0] {
            0 => SegmentKind::Header,
            1 => SegmentKind::Body,
            2 => SegmentKind::Signature,
            3 => SegmentKind::QuotedThread,
            _ => continue,
        };
        let mut vec = Vec::with_capacity(384);
        for i in 0..384 {
            let start = 1 + i * 4;
            let bytes: [u8; 4] = chunk[start..start + 4].try_into().unwrap_or([0; 4]);
            vec.push(f32::from_le_bytes(bytes));
        }
        embeddings.push((kind, vec));
    }
    embeddings
}

/// Adaptador de persistência SQLite para o índice híbrido (BM25 + Vetores Urupema).
#[derive(Clone)]
pub struct SqliteIndexStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteIndexStore {
    /// Abre ou cria o banco SQLite de índice no caminho especificado.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Inicializa as tabelas no modo WAL para alto desempenho e concorrência.
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS indexed_documents (
                 mailbox TEXT NOT NULL,
                 uid INTEGER NOT NULL,
                 mtime INTEGER NOT NULL DEFAULT 0,
                 subject TEXT NOT NULL,
                 sender TEXT NOT NULL,
                 date TEXT NOT NULL,
                 is_read INTEGER NOT NULL DEFAULT 0,
                 is_flagged INTEGER NOT NULL DEFAULT 0,
                 has_attachments INTEGER NOT NULL DEFAULT 0,
                 doc_len INTEGER NOT NULL DEFAULT 0,
                 terms_json TEXT NOT NULL DEFAULT '{}',
                 embeddings_blob BLOB,
                 PRIMARY KEY (mailbox, uid)
             );

             CREATE INDEX IF NOT EXISTS idx_indexed_docs_box ON indexed_documents(mailbox);
             CREATE INDEX IF NOT EXISTS idx_indexed_docs_box_uid ON indexed_documents(mailbox, uid);

             CREATE TABLE IF NOT EXISTS inverted_index (
                 term TEXT NOT NULL,
                 mailbox TEXT NOT NULL,
                 uid INTEGER NOT NULL,
                 tf INTEGER NOT NULL,
                 PRIMARY KEY (term, mailbox, uid)
             );

             CREATE INDEX IF NOT EXISTS idx_inverted_term_box ON inverted_index(term, mailbox);",
        )?;
        Ok(())
    }

    /// Salva ou atualiza um documento indexado no SQLite.
    pub fn save_document(
        &self,
        mailbox: &str,
        doc: &IndexedDoc,
        mtime: i64,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let terms_json =
            serde_json::to_string(&doc.term_frequencies).unwrap_or_else(|_| "{}".to_string());
        let embeddings_blob = encode_segment_embeddings(&doc.segment_embeddings);

        conn.execute(
            "INSERT OR REPLACE INTO indexed_documents (
                mailbox, uid, mtime, subject, sender, date,
                is_read, is_flagged, has_attachments, doc_len,
                terms_json, embeddings_blob
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                mailbox,
                doc.uid,
                mtime,
                doc.subject,
                doc.sender,
                doc.date,
                if doc.is_read { 1 } else { 0 },
                if doc.is_flagged { 1 } else { 0 },
                if doc.has_attachments { 1 } else { 0 },
                doc.doc_len as i64,
                terms_json,
                embeddings_blob,
            ],
        )?;

        // Atualiza a tabela de índice invertido
        conn.execute(
            "DELETE FROM inverted_index WHERE mailbox = ?1 AND uid = ?2",
            params![mailbox, doc.uid],
        )?;

        let mut stmt = conn.prepare(
            "INSERT OR REPLACE INTO inverted_index (term, mailbox, uid, tf) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (term, &tf) in &doc.term_frequencies {
            stmt.execute(params![term, mailbox, doc.uid, tf])?;
        }

        Ok(())
    }

    /// Remove um documento indexado do SQLite.
    pub fn remove_document(&self, mailbox: &str, uid: u32) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM indexed_documents WHERE mailbox = ?1 AND uid = ?2",
            params![mailbox, uid],
        )?;
        conn.execute(
            "DELETE FROM inverted_index WHERE mailbox = ?1 AND uid = ?2",
            params![mailbox, uid],
        )?;
        Ok(())
    }

    /// Retorna todos os UIDs atualmente presentes no banco para a caixa postal informada.
    pub fn get_indexed_uids(&self, mailbox: &str) -> Result<HashSet<u32>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT uid FROM indexed_documents WHERE mailbox = ?1")?;
        let rows = stmt.query_map(params![mailbox], |row| row.get::<_, u32>(0))?;
        let mut uids = HashSet::new();
        for r in rows {
            uids.insert(r?);
        }
        Ok(uids)
    }

    /// Reconcilia o índice com a lista ativa de UIDs (obtida do dovecot-uidlist ou Maildir).
    pub fn reconcile_mailbox(
        &self,
        mailbox: &str,
        active_uids: &HashSet<u32>,
    ) -> Result<ReconcileResult, rusqlite::Error> {
        let current_indexed = self.get_indexed_uids(mailbox)?;

        let mut to_index = Vec::new();
        for &uid in active_uids {
            if !current_indexed.contains(&uid) {
                to_index.push(uid);
            }
        }
        to_index.sort();

        let mut to_remove = Vec::new();
        for &uid in &current_indexed {
            if !active_uids.contains(&uid) {
                to_remove.push(uid);
            }
        }
        to_remove.sort();

        Ok(ReconcileResult {
            to_index,
            to_remove,
        })
    }

    /// Carrega o índice em memória estruturado a partir do banco SQLite para busca veloz.
    pub fn load_mailbox_index(&self, mailbox: &str) -> Result<MessageIndex, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT uid, subject, sender, date, is_read, is_flagged, has_attachments,
                    doc_len, terms_json, embeddings_blob
             FROM indexed_documents WHERE mailbox = ?1",
        )?;

        let mut index = MessageIndex::new();

        let rows = stmt.query_map(params![mailbox], |row| {
            let uid: u32 = row.get(0)?;
            let subject: String = row.get(1)?;
            let sender: String = row.get(2)?;
            let date: String = row.get(3)?;
            let is_read: bool = row.get::<_, i64>(4)? != 0;
            let is_flagged: bool = row.get::<_, i64>(5)? != 0;
            let has_attachments: bool = row.get::<_, i64>(6)? != 0;
            let doc_len: usize = row.get::<_, i64>(7)? as usize;
            let terms_json: String = row.get(8)?;
            let embeddings_blob: Vec<u8> = row.get(9)?;
            Ok((
                uid,
                subject,
                sender,
                date,
                is_read,
                is_flagged,
                has_attachments,
                doc_len,
                terms_json,
                embeddings_blob,
            ))
        })?;

        for r in rows {
            let (
                uid,
                subject,
                sender,
                date,
                is_read,
                is_flagged,
                has_attachments,
                doc_len,
                terms_json,
                embeddings_blob,
            ) = r?;

            let term_frequencies: HashMap<String, u32> =
                serde_json::from_str(&terms_json).unwrap_or_default();
            let segment_embeddings = decode_segment_embeddings(&embeddings_blob);

            let subject_terms = lauda_index::tokenize(&lauda_index::normalize(&subject))
                .into_iter()
                .collect();
            let sender_terms = lauda_index::tokenize(&lauda_index::normalize(&sender))
                .into_iter()
                .collect();

            let doc = IndexedDoc {
                uid,
                mailbox: mailbox.to_string(),
                sender,
                subject,
                date,
                is_read,
                is_flagged,
                has_attachments,
                subject_terms,
                sender_terms,
                term_frequencies,
                doc_len,
                segment_embeddings,
            };

            index.insert_doc(doc);
        }

        Ok(index)
    }

    /// Retorna a fatia de mensagens indexadas dos últimos N dias para sincronização offline dos clientes.
    pub fn get_recent_slice(
        &self,
        mailbox: &str,
        limit_days: u32,
    ) -> Result<Vec<IndexedDoc>, rusqlite::Error> {
        let cutoff_date = if limit_days > 0 {
            Utc::now() - chrono::Duration::days(limit_days as i64)
        } else {
            DateTime::<Utc>::MIN_UTC
        };

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT uid, subject, sender, date, is_read, is_flagged, has_attachments,
                    doc_len, terms_json, embeddings_blob
             FROM indexed_documents WHERE mailbox = ?1 ORDER BY uid DESC",
        )?;

        let mut list = Vec::new();
        let rows = stmt.query_map(params![mailbox], |row| {
            let uid: u32 = row.get(0)?;
            let subject: String = row.get(1)?;
            let sender: String = row.get(2)?;
            let date: String = row.get(3)?;
            let is_read: bool = row.get::<_, i64>(4)? != 0;
            let is_flagged: bool = row.get::<_, i64>(5)? != 0;
            let has_attachments: bool = row.get::<_, i64>(6)? != 0;
            let doc_len: usize = row.get::<_, i64>(7)? as usize;
            let terms_json: String = row.get(8)?;
            let embeddings_blob: Vec<u8> = row.get(9)?;
            Ok((
                uid,
                subject,
                sender,
                date,
                is_read,
                is_flagged,
                has_attachments,
                doc_len,
                terms_json,
                embeddings_blob,
            ))
        })?;

        for r in rows {
            let (
                uid,
                subject,
                sender,
                date,
                is_read,
                is_flagged,
                has_attachments,
                doc_len,
                terms_json,
                embeddings_blob,
            ) = r?;

            if let Ok(parsed_date) = DateTime::parse_from_rfc3339(&date)
                && parsed_date.with_timezone(&Utc) < cutoff_date
            {
                continue;
            } else if let Ok(parsed_date) = DateTime::parse_from_rfc2822(&date)
                && parsed_date.with_timezone(&Utc) < cutoff_date
            {
                continue;
            }

            let term_frequencies: HashMap<String, u32> =
                serde_json::from_str(&terms_json).unwrap_or_default();
            let segment_embeddings = decode_segment_embeddings(&embeddings_blob);

            let subject_terms = lauda_index::tokenize(&lauda_index::normalize(&subject))
                .into_iter()
                .collect();
            let sender_terms = lauda_index::tokenize(&lauda_index::normalize(&sender))
                .into_iter()
                .collect();

            let doc = IndexedDoc {
                uid,
                mailbox: mailbox.to_string(),
                sender,
                subject,
                date,
                is_read,
                is_flagged,
                has_attachments,
                subject_terms,
                sender_terms,
                term_frequencies,
                doc_len,
                segment_embeddings,
            };

            list.push(doc);
        }

        Ok(list)
    }
}
