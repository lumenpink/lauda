use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Utc};
use lauda_index::{EmbeddingProvider, FlatVectorIndex, IndexedDoc, MessageIndex};
use lauda_types::{MailboxSyncStatus, MessageEnvelope};
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

/// Extrai o vetor representativo dos segmentos utilizando Max-Pooling ponderado
/// (Header e Body possuem peso pleno; Signature e QuotedThread são atenuados).
pub fn extract_representative_vector(embeddings: &[(SegmentKind, Vec<f32>)]) -> Option<Vec<f32>> {
    if embeddings.is_empty() {
        return None;
    }
    let dim = embeddings[0].1.len();
    let mut pooled = vec![0.0f32; dim];

    for (kind, vec) in embeddings {
        if vec.len() != dim {
            continue;
        }
        let weight = match kind {
            SegmentKind::Header => 1.0f32,
            SegmentKind::Body => 1.0f32,
            SegmentKind::Signature => 0.2f32,
            SegmentKind::QuotedThread => 0.1f32,
        };
        for i in 0..dim {
            let val = vec[i] * weight;
            if val > pooled[i] {
                pooled[i] = val;
            }
        }
    }

    // Normalização Euclidiana (L2)
    let norm: f32 = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut pooled {
            *v /= norm;
        }
    }

    Some(pooled)
}

/// Adaptador de persistência SQLite para o índice híbrido (BM25 + Vetores Urupema com FlatVectorIndex em RAM).
#[derive(Clone)]
pub struct SqliteIndexStore {
    conn: Arc<Mutex<Connection>>,
    vector_indexes: Arc<RwLock<HashMap<String, FlatVectorIndex>>>,
}

impl SqliteIndexStore {
    /// Abre ou cria o banco SQLite de índice no caminho especificado e carrega os vetores em RAM.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let vector_indexes = Arc::new(RwLock::new(HashMap::new()));
        Self::preload_vector_indexes(&conn, &vector_indexes);

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            vector_indexes,
        };
        store.init_schema()?;
        Ok(store)
    }

    /// Carrega previamente todos os vetores indexados em lote para a matriz contígua em RAM.
    fn preload_vector_indexes(
        conn: &Connection,
        vector_indexes: &Arc<RwLock<HashMap<String, FlatVectorIndex>>>,
    ) {
        let mut stmt = match conn.prepare(
            "SELECT mailbox, uid, embeddings_blob FROM indexed_documents WHERE embeddings_blob IS NOT NULL",
        ) {
            Ok(s) => s,
            Err(_) => return,
        };

        let rows = match stmt.query_map([], |row| {
            let mailbox: String = row.get(0)?;
            let uid: u32 = row.get(1)?;
            let blob: Vec<u8> = row.get(2)?;
            Ok((mailbox, uid, blob))
        }) {
            Ok(r) => r,
            Err(_) => return,
        };

        let mut lock = vector_indexes.write().unwrap();
        for r in rows.flatten() {
            let (mailbox, uid, blob) = r;
            if blob.is_empty() {
                continue;
            }
            let segments = decode_segment_embeddings(&blob);
            if let Some(rep_vec) = extract_representative_vector(&segments)
                && rep_vec.len() == 384
            {
                lock.entry(mailbox)
                    .or_insert_with(|| FlatVectorIndex::new(384))
                    .insert(uid, &rep_vec);
            }
        }
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

        // Atualiza o índice vetorial em RAM
        if let Some(rep_vec) = extract_representative_vector(&doc.segment_embeddings)
            && rep_vec.len() == 384
        {
            let mut indexes = self.vector_indexes.write().unwrap();
            indexes
                .entry(mailbox.to_string())
                .or_insert_with(|| FlatVectorIndex::new(384))
                .insert(doc.uid, &rep_vec);
        }

        Ok(())
    }

    /// Remove um documento indexado do SQLite e do índice vetorial em RAM.
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

        let mut indexes = self.vector_indexes.write().unwrap();
        if let Some(idx) = indexes.get_mut(mailbox) {
            idx.remove(uid);
        }

        Ok(())
    }

    /// Limpa todos os documentos indexados e termos invertidos da caixa postal informada,
    /// permitindo uma reindexação completa do zero.
    pub fn clear_mailbox_index(&self, mailbox: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM indexed_documents WHERE mailbox = ?1",
            params![mailbox],
        )?;
        conn.execute(
            "DELETE FROM inverted_index WHERE mailbox = ?1",
            params![mailbox],
        )?;

        let mut indexes = self.vector_indexes.write().unwrap();
        if let Some(idx) = indexes.get_mut(mailbox) {
            idx.clear();
        }

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
        to_index.sort_by(|a, b| b.cmp(a));

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

    /// Obtém o status de sincronização da caixa postal comparando com o total físico de mensagens.
    pub fn get_sync_status(
        &self,
        mailbox: &str,
        total_count: u64,
    ) -> Result<MailboxSyncStatus, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let indexed_count: i64 = conn.query_row(
            "SELECT count(*) FROM indexed_documents WHERE mailbox = ?1",
            params![mailbox],
            |row| row.get(0),
        )?;
        let indexed_count = indexed_count as u64;
        Ok(MailboxSyncStatus::new(indexed_count, total_count))
    }

    /// Realiza busca BM25 direta no banco SQLite via índice invertido relacional em sub-milissegundos.
    pub fn search_bm25(
        &self,
        mailbox: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MessageEnvelope>, rusqlite::Error> {
        let tokens = lauda_index::tokenize(query);
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn.lock().unwrap();
        let placeholders = tokens
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "SELECT d.uid, d.subject, d.sender, d.date, d.is_read, d.is_flagged, d.has_attachments,
                    SUM(i.tf) AS score
             FROM inverted_index i
             JOIN indexed_documents d ON d.mailbox = i.mailbox AND d.uid = i.uid
             WHERE i.mailbox = ?1 AND i.term IN ({})
             GROUP BY d.uid
             ORDER BY score DESC, d.uid DESC
             LIMIT ?{}",
            placeholders,
            tokens.len() + 2
        );

        let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(tokens.len() + 2);
        params.push(mailbox.to_string().into());
        for t in tokens {
            params.push(t.into());
        }
        params.push((limit as i64).into());

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            let uid: u32 = row.get(0)?;
            let subject: String = row.get(1)?;
            let sender: String = row.get(2)?;
            let date: String = row.get(3)?;
            let is_read: bool = row.get::<_, i64>(4)? != 0;
            let is_flagged: bool = row.get::<_, i64>(5)? != 0;
            let has_attachments: bool = row.get::<_, i64>(6)? != 0;
            Ok(MessageEnvelope {
                uid,
                subject,
                sender,
                date,
                cc: None,
                has_attachments,
                is_read,
                is_flagged,
                crypto_meta: None,
            })
        })?;

        let mut envelopes = Vec::new();
        for r in rows {
            envelopes.push(r?);
        }
        Ok(envelopes)
    }

    /// Executa busca híbrida unificada combinando BM25 textual relacional e proximidade
    /// semântica via FlatVectorIndex em RAM através de fusão harmônica RRF (Reciprocal Rank Fusion).
    pub fn search_hybrid(
        &self,
        mailbox: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MessageEnvelope>, rusqlite::Error> {
        let parsed = lauda_index::parse_query(query);
        let provider = lauda_index::SimpleCpuEmbeddingProvider;
        let query_vec = provider.embed_query(query);

        let conn = self.conn.lock().unwrap();

        // 1. Canal Léxico: Top 50 UIDs via índice invertido relacional (BM25)
        // Filtra stop words comuns para evitar ruído falso de preposições no índice invertido
        let raw_tokens = lauda_index::tokenize(query);
        let meaningful_tokens: Vec<String> = raw_tokens
            .iter()
            .filter(|t| !lauda_index::is_stop_word(t))
            .cloned()
            .collect();
        let tokens = if meaningful_tokens.is_empty() {
            raw_tokens
        } else {
            meaningful_tokens
        };
        let mut lexical_uids: Vec<u32> = Vec::new();

        if !tokens.is_empty() {
            let placeholders = tokens
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 2))
                .collect::<Vec<_>>()
                .join(", ");

            let sql_bm25 = format!(
                "SELECT d.uid
                 FROM inverted_index i
                 JOIN indexed_documents d ON d.mailbox = i.mailbox AND d.uid = i.uid
                 WHERE i.mailbox = ?1 AND i.term IN ({})
                 GROUP BY d.uid
                 ORDER BY SUM(i.tf) DESC, d.uid DESC
                 LIMIT 50",
                placeholders
            );

            let mut params: Vec<rusqlite::types::Value> = Vec::with_capacity(tokens.len() + 1);
            params.push(mailbox.to_string().into());
            for t in &tokens {
                params.push(t.clone().into());
            }

            if let Ok(mut stmt) = conn.prepare(&sql_bm25)
                && let Ok(rows) = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    row.get::<_, u32>(0)
                })
            {
                lexical_uids = rows.flatten().collect();
            }
        }

        // 2. Canal Semântico: Top 50 UIDs via FlatVectorIndex contíguo em RAM (SIMD)
        let semantic_uids: Vec<u32> = {
            let indexes = self.vector_indexes.read().unwrap();
            if let Some(idx) = indexes.get(mailbox) {
                idx.search_knn(&query_vec, 50)
                    .into_iter()
                    .map(|(uid, _score)| uid)
                    .collect()
            } else {
                Vec::new()
            }
        };

        // 3. Fusão RRF harmônica dos dois canais independentes
        let fused = lauda_index::reciprocal_rank_fusion(&lexical_uids, &semantic_uids, 60.0);

        if fused.is_empty() {
            if tokens.is_empty() && parsed.clauses.is_empty() {
                // Query em branco: retorna mensagens mais recentes
                let mut stmt = conn.prepare(
                    "SELECT uid, subject, sender, date, is_read, is_flagged, has_attachments
                     FROM indexed_documents WHERE mailbox = ?1 ORDER BY uid DESC LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![mailbox, limit as i64], |row| {
                    let uid: u32 = row.get(0)?;
                    let subject: String = row.get(1)?;
                    let sender: String = row.get(2)?;
                    let date: String = row.get(3)?;
                    let is_read: bool = row.get::<_, i64>(4)? != 0;
                    let is_flagged: bool = row.get::<_, i64>(5)? != 0;
                    let has_attachments: bool = row.get::<_, i64>(6)? != 0;
                    Ok(MessageEnvelope {
                        uid,
                        subject,
                        sender,
                        date,
                        cc: None,
                        has_attachments,
                        is_read,
                        is_flagged,
                        crypto_meta: None,
                    })
                })?;
                return Ok(rows.flatten().collect());
            }
            return Ok(Vec::new());
        }

        // Seleciona os UIDs finalistas para carregar metadados pontuais
        let candidate_uids: Vec<u32> = fused
            .iter()
            .map(|(uid, _)| *uid)
            .take(std::cmp::max(limit, 50))
            .collect();

        // 4. Carrega metadados pontuais apenas dos finalistas sem scan de tabela
        let uid_placeholders = candidate_uids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");

        let sql_docs = format!(
            "SELECT uid, subject, sender, date, is_read, is_flagged, has_attachments
             FROM indexed_documents
             WHERE mailbox = ?1 AND uid IN ({})",
            uid_placeholders
        );

        let mut doc_params: Vec<rusqlite::types::Value> =
            Vec::with_capacity(candidate_uids.len() + 1);
        doc_params.push(mailbox.to_string().into());
        for u in &candidate_uids {
            doc_params.push((*u as i64).into());
        }

        let mut envelopes_by_uid: HashMap<u32, MessageEnvelope> = HashMap::new();
        let mut stmt = conn.prepare(&sql_docs)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(doc_params.iter()), |row| {
            let uid: u32 = row.get(0)?;
            let subject: String = row.get(1)?;
            let sender: String = row.get(2)?;
            let date: String = row.get(3)?;
            let is_read: bool = row.get::<_, i64>(4)? != 0;
            let is_flagged: bool = row.get::<_, i64>(5)? != 0;
            let has_attachments: bool = row.get::<_, i64>(6)? != 0;
            Ok((
                uid,
                MessageEnvelope {
                    uid,
                    subject,
                    sender,
                    date,
                    cc: None,
                    has_attachments,
                    is_read,
                    is_flagged,
                    crypto_meta: None,
                },
            ))
        })?;

        for r in rows.flatten() {
            envelopes_by_uid.insert(r.0, r.1);
        }

        // 5. Aplica filtros estruturais de Query e preserva a ordem exata do RRF
        let mut final_results = Vec::new();
        for (uid, _) in fused {
            if let Some(env) = envelopes_by_uid.remove(&uid) {
                let mut matches_filters = true;
                for clause in &parsed.clauses {
                    match clause {
                        lauda_index::QueryClause::From(author) => {
                            let author_norm = lauda_index::normalize(author);
                            let sender_norm = lauda_index::normalize(&env.sender);
                            if !sender_norm.contains(&author_norm) {
                                matches_filters = false;
                                break;
                            }
                        }
                        lauda_index::QueryClause::HasAttachment if !env.has_attachments => {
                            matches_filters = false;
                            break;
                        }
                        lauda_index::QueryClause::IsRead(read) if env.is_read != *read => {
                            matches_filters = false;
                            break;
                        }
                        lauda_index::QueryClause::IsFlagged(flagged)
                            if env.is_flagged != *flagged =>
                        {
                            matches_filters = false;
                            break;
                        }
                        _ => {}
                    }
                }
                if matches_filters {
                    final_results.push(env);
                    if final_results.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(final_results)
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
