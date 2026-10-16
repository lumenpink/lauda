use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::parser::{Query, QueryClause, parse_query};
use crate::scorer::{cosine_similarity, levenshtein_distance, score_bm25};
use crate::tokenizer::{normalize, tokenize};
use lauda_types::Message;
use urupema::domain::{HeaderMeta, SegmentKind, SegmentationConfig, segment_email};

/// Provedor de embeddings textuais (ex: modelo CPU ONNX ou mock para testes).
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Provedor determinístico em CPU de 384 dimensões baseado em hashing FNV-1a de n-gramas e normalização L2.
/// Garante 0 MB de VRAM e busca semântica preliminar sem necessidade de modelos pesados em disco.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleCpuEmbeddingProvider;

impl EmbeddingProvider for SimpleCpuEmbeddingProvider {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; 384];
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();
        for word in &words {
            let bytes = word.as_bytes();
            if bytes.len() >= 3 {
                for window in bytes.windows(3) {
                    let mut hash: u64 = 0xcbf29ce484222325;
                    for &b in window {
                        hash ^= b as u64;
                        hash = hash.wrapping_mul(0x100000001b3);
                    }
                    let idx = (hash % 384) as usize;
                    vec[idx] += 1.0;
                }
            } else {
                let mut hash: u64 = 0xcbf29ce484222325;
                for &b in bytes {
                    hash ^= b as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                let idx = (hash % 384) as usize;
                vec[idx] += 1.0;
            }
        }
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        vec
    }
}


mod segment_kind_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(kind: &Option<SegmentKind>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match kind {
            Some(k) => {
                let s = match k {
                    SegmentKind::Header => "Header",
                    SegmentKind::Body => "Body",
                    SegmentKind::Signature => "Signature",
                    SegmentKind::QuotedThread => "QuotedThread",
                };
                serializer.serialize_some(s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SegmentKind>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<String> = Option::deserialize(deserializer)?;
        match opt.as_deref() {
            Some("Header") => Ok(Some(SegmentKind::Header)),
            Some("Body") => Ok(Some(SegmentKind::Body)),
            Some("Signature") => Ok(Some(SegmentKind::Signature)),
            Some("QuotedThread") => Ok(Some(SegmentKind::QuotedThread)),
            _ => Ok(None),
        }
    }
}

mod segment_embeddings_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Entry {
        kind: String,
        vector: Vec<f32>,
    }

    pub fn serialize<S>(
        entries: &[(SegmentKind, Vec<f32>)],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let list: Vec<Entry> = entries
            .iter()
            .map(|(k, v)| {
                let kind = match k {
                    SegmentKind::Header => "Header",
                    SegmentKind::Body => "Body",
                    SegmentKind::Signature => "Signature",
                    SegmentKind::QuotedThread => "QuotedThread",
                };
                Entry {
                    kind: kind.to_string(),
                    vector: v.clone(),
                }
            })
            .collect();
        list.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<(SegmentKind, Vec<f32>)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let list: Vec<Entry> = Vec::deserialize(deserializer)?;
        Ok(list
            .into_iter()
            .filter_map(|e| {
                let kind = match e.kind.as_str() {
                    "Header" => SegmentKind::Header,
                    "Body" => SegmentKind::Body,
                    "Signature" => SegmentKind::Signature,
                    "QuotedThread" => SegmentKind::QuotedThread,
                    _ => return None,
                };
                Some((kind, e.vector))
            })
            .collect())
    }
}

/// Resultado de busca ranqueada contendo pontuações combinadas e segmento de maior relevância.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub uid: u32,
    pub score: f64,
    pub bm25_score: f64,
    pub semantic_score: f64,
    #[serde(default, with = "segment_kind_serde")]
    pub matched_segment: Option<SegmentKind>,
}

/// Documento indexado com metadados para busca e ranqueamento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDoc {
    pub uid: u32,
    pub mailbox: String,
    pub sender: String,
    pub subject: String,
    pub date: String,
    pub is_read: bool,
    pub is_flagged: bool,
    pub has_attachments: bool,
    /// Termos normalizados do assunto
    pub subject_terms: HashSet<String>,
    /// Termos normalizados do remetente
    pub sender_terms: HashSet<String>,
    /// Contagem de frequência de todos os termos (assunto + corpo)
    pub term_frequencies: HashMap<String, u32>,
    pub doc_len: usize,
    #[serde(default, with = "segment_embeddings_serde")]
    pub segment_embeddings: Vec<(SegmentKind, Vec<f32>)>,
}

/// Motor de índice invertido com suporte a BM25 e fuzzy search.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MessageIndex {
    /// Mapeamento de termo -> UIDs que contêm o termo
    inverted_index: HashMap<String, HashSet<u32>>,
    /// Coleção de documentos por UID
    documents: HashMap<u32, IndexedDoc>,
    /// Comprimento total somado para cálculo de média BM25
    total_len: usize,
}

impl MessageIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Indexa uma nova mensagem de forma incremental.
    pub fn index_message(&mut self, mailbox: &str, msg: &Message) {
        let uid = msg.envelope.uid;

        // Se já existe, remove primeiro para evitar contagem duplicada
        self.remove_message(uid);

        let subject_norm = normalize(&msg.envelope.subject);
        let sender_norm = normalize(&msg.envelope.sender);

        let subject_tokens = tokenize(&subject_norm);
        let sender_tokens = tokenize(&sender_norm);

        let mut body_tokens = Vec::new();
        if let Some(ref text) = msg.txt_auto {
            body_tokens.extend(tokenize(text));
        } else if let Some(ref raw) = msg.txt_original {
            body_tokens.extend(tokenize(raw));
        }
        for att in &msg.attachments {
            let att_norm = normalize(&att.filename);
            body_tokens.extend(tokenize(&att_norm));
        }

        let mut term_frequencies: HashMap<String, u32> = HashMap::new();
        for t in &subject_tokens {
            *term_frequencies.entry(t.clone()).or_insert(0) += 3; // Boost x3 para termos no assunto
        }
        for t in &sender_tokens {
            *term_frequencies.entry(t.clone()).or_insert(0) += 2; // Boost x2 para remetente
        }
        for t in &body_tokens {
            *term_frequencies.entry(t.clone()).or_insert(0) += 1;
        }

        let doc_len = subject_tokens.len() + sender_tokens.len() + body_tokens.len();
        self.total_len += doc_len;

        for term in term_frequencies.keys() {
            self.inverted_index
                .entry(term.clone())
                .or_default()
                .insert(uid);
        }

        let doc = IndexedDoc {
            uid,
            mailbox: mailbox.to_string(),
            sender: msg.envelope.sender.clone(),
            subject: msg.envelope.subject.clone(),
            date: msg.envelope.date.clone(),
            is_read: msg.envelope.is_read,
            is_flagged: msg.envelope.is_flagged,
            has_attachments: msg.envelope.has_attachments,
            subject_terms: subject_tokens.into_iter().collect(),
            sender_terms: sender_tokens.into_iter().collect(),
            term_frequencies,
            doc_len,
            segment_embeddings: Vec::new(),
        };

        self.documents.insert(uid, doc);
    }

    /// Indexa uma nova mensagem com dissecação semântica Urupema e geração de embeddings por segmento.
    pub fn index_message_semantic(
        &mut self,
        mailbox: &str,
        msg: &Message,
        provider: &dyn EmbeddingProvider,
    ) {
        self.index_message(mailbox, msg);

        let headers = HeaderMeta {
            from: Some(msg.envelope.sender.clone()),
            to: None,
            subject: Some(msg.envelope.subject.clone()),
            date: Some(msg.envelope.date.clone()),
        };
        let body_text = msg
            .txt_auto
            .as_deref()
            .or(msg.txt_original.as_deref())
            .unwrap_or("");

        let mut full_body = body_text.to_string();
        for att in &msg.attachments {
            full_body.push_str("\nAnexo: ");
            full_body.push_str(&att.filename);
        }

        let config = SegmentationConfig::default();
        let segments = segment_email(Some(&headers), &full_body, &config);

        let mut segment_embeddings = Vec::new();
        for segment in segments {
            let trimmed = segment.content.trim();
            if trimmed.is_empty() {
                continue;
            }
            let vec = provider.embed(trimmed);
            segment_embeddings.push((segment.kind, vec));
        }

        if let Some(doc) = self.documents.get_mut(&msg.envelope.uid) {
            doc.segment_embeddings = segment_embeddings;
        }
    }

    /// Executa busca puramente vetorial com Max-Pooling sobre os segmentos semânticos.
    pub fn search_semantic(
        &self,
        query: &str,
        provider: &dyn EmbeddingProvider,
    ) -> Vec<SearchResult> {
        let query_vec = provider.embed(query);
        let mut results: Vec<SearchResult> = Vec::new();

        for (&uid, doc) in &self.documents {
            if doc.segment_embeddings.is_empty() {
                continue;
            }

            let mut max_sim = 0.0f32;
            let mut best_kind = None;

            for (kind, emb) in &doc.segment_embeddings {
                let sim = cosine_similarity(&query_vec, emb);
                if sim > max_sim {
                    max_sim = sim;
                    best_kind = Some(*kind);
                }
            }

            if max_sim > 0.0 {
                results.push(SearchResult {
                    uid,
                    score: max_sim as f64,
                    bm25_score: 0.0,
                    semantic_score: max_sim as f64,
                    matched_segment: best_kind,
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Executa busca híbrida linear combinando BM25 e busca vetorial Urupema.
    ///
    /// Parâmetro `alpha`:
    /// - `alpha = 1.0`: busca exclusivamente BM25
    /// - `alpha = 0.0`: busca exclusivamente vetorial
    /// - `0.0 < alpha < 1.0`: fusão ponderada
    pub fn search_hybrid(
        &self,
        query_str: &str,
        provider: &dyn EmbeddingProvider,
        alpha: f64,
    ) -> Vec<SearchResult> {
        let alpha = alpha.clamp(0.0, 1.0);
        let query = parse_query(query_str);

        let total_docs = self.documents.len();
        let avg_doc_len = if total_docs > 0 {
            self.total_len as f64 / total_docs as f64
        } else {
            1.0
        };

        // 1. BM25 scoring
        let mut bm25_map: HashMap<u32, f64> = HashMap::new();
        let mut max_bm25 = 0.0f64;

        if alpha > 0.0 {
            for (uid, doc) in &self.documents {
                let mut doc_score = 0.0;
                let mut matched_any = false;

                if query.clauses.is_empty() {
                    matched_any = true;
                    doc_score = 1.0;
                } else {
                    for clause in &query.clauses {
                        if let Some(score) =
                            self.evaluate_clause(doc, clause, total_docs, avg_doc_len)
                        {
                            doc_score += score;
                            matched_any = true;
                        }
                    }
                }

                if matched_any && doc_score > 0.0 {
                    if doc_score > max_bm25 {
                        max_bm25 = doc_score;
                    }
                    bm25_map.insert(*uid, doc_score);
                }
            }
        }

        // 2. Semantic scoring
        let mut semantic_map: HashMap<u32, (f64, Option<SegmentKind>)> = HashMap::new();

        if alpha < 1.0 {
            let sem_results = self.search_semantic(query_str, provider);
            for res in sem_results {
                semantic_map.insert(res.uid, (res.semantic_score, res.matched_segment));
            }
        }

        // 3. Fusion
        let mut all_uids: HashSet<u32> = HashSet::new();
        all_uids.extend(bm25_map.keys());
        all_uids.extend(semantic_map.keys());

        let mut results = Vec::new();
        for uid in all_uids {
            let raw_bm25 = bm25_map.get(&uid).copied().unwrap_or(0.0);
            let norm_bm25 = if max_bm25 > 0.0 {
                raw_bm25 / max_bm25
            } else {
                0.0
            };

            let (sem_score, matched_segment) =
                semantic_map.get(&uid).cloned().unwrap_or((0.0, None));

            let combined_score = alpha * norm_bm25 + (1.0 - alpha) * sem_score;

            if combined_score > 0.0 {
                results.push(SearchResult {
                    uid,
                    score: combined_score,
                    bm25_score: raw_bm25,
                    semantic_score: sem_score,
                    matched_segment,
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    /// Remove uma mensagem do índice.
    pub fn remove_message(&mut self, uid: u32) {
        if let Some(doc) = self.documents.remove(&uid) {
            self.total_len = self.total_len.saturating_sub(doc.doc_len);
            for term in doc.term_frequencies.keys() {
                if let Some(set) = self.inverted_index.get_mut(term) {
                    set.remove(&uid);
                    if set.is_empty() {
                        self.inverted_index.remove(term);
                    }
                }
            }
        }
    }

    /// Executa uma busca por string textual estilo Gmail.
    pub fn search(&self, query_str: &str) -> Vec<u32> {
        let query = parse_query(query_str);
        self.execute_query(&query)
    }

    /// Executa uma Query estruturada e retorna UIDs ordenados por pontuação BM25 decrescente.
    pub fn execute_query(&self, query: &Query) -> Vec<u32> {
        if query.clauses.is_empty() {
            let mut all: Vec<u32> = self.documents.keys().copied().collect();
            all.sort();
            return all;
        }

        let total_docs = self.documents.len();
        let avg_doc_len = if total_docs > 0 {
            self.total_len as f64 / total_docs as f64
        } else {
            1.0
        };

        let mut scored_docs: Vec<(u32, f64)> = Vec::new();

        for (uid, doc) in &self.documents {
            let mut matches_all = true;
            let mut doc_score = 0.0;

            for clause in &query.clauses {
                match self.evaluate_clause(doc, clause, total_docs, avg_doc_len) {
                    Some(score) => {
                        doc_score += score;
                    }
                    None => {
                        matches_all = false;
                        break;
                    }
                }
            }

            if matches_all {
                scored_docs.push((*uid, doc_score));
            }
        }

        // Ordenar por pontuação decrescente (mais relevante primeiro)
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_docs.into_iter().map(|(uid, _)| uid).collect()
    }

    fn evaluate_clause(
        &self,
        doc: &IndexedDoc,
        clause: &QueryClause,
        total_docs: usize,
        avg_doc_len: f64,
    ) -> Option<f64> {
        match clause {
            QueryClause::Term(term) => {
                // Correspondência exata ou fuzzy (distância <= 1)
                let mut best_tf = 0.0;
                let mut best_term = term.as_str();

                if let Some(tf) = doc.term_frequencies.get(term) {
                    best_tf = *tf as f64;
                } else if term.len() >= 4 {
                    // Fuzzy matching para typos
                    for (doc_term, tf) in &doc.term_frequencies {
                        if levenshtein_distance(term, doc_term) <= 1 {
                            best_tf = (*tf as f64) * 0.8; // Penalização leve para fuzzy
                            best_term = doc_term.as_str();
                            break;
                        }
                    }
                }

                if best_tf > 0.0 {
                    let docs_with_term = self
                        .inverted_index
                        .get(best_term)
                        .map(|s| s.len())
                        .unwrap_or(1);
                    let score = score_bm25(
                        best_tf,
                        doc.doc_len,
                        avg_doc_len,
                        total_docs,
                        docs_with_term,
                    );
                    Some(score.max(0.1))
                } else {
                    None
                }
            }
            QueryClause::From(author) => {
                let author_norm = normalize(author);
                if doc.sender_terms.iter().any(|t| t.contains(&author_norm)) {
                    Some(2.0)
                } else {
                    None
                }
            }
            QueryClause::To(_recipient) => {
                // Pode expandir com campo destinatário se presente no envelope
                Some(1.0)
            }
            QueryClause::Subject(subj) => {
                let subj_norm = normalize(subj);
                if doc.subject_terms.iter().any(|t| t.contains(&subj_norm)) {
                    Some(3.0)
                } else {
                    None
                }
            }
            QueryClause::HasAttachment => {
                if doc.has_attachments {
                    Some(1.0)
                } else {
                    None
                }
            }
            QueryClause::IsRead(read) => {
                if doc.is_read == *read {
                    Some(0.5)
                } else {
                    None
                }
            }
            QueryClause::IsFlagged(flagged) => {
                if doc.is_flagged == *flagged {
                    Some(0.5)
                } else {
                    None
                }
            }
            QueryClause::Not(inner) => {
                if self
                    .evaluate_clause(doc, inner, total_docs, avg_doc_len)
                    .is_some()
                {
                    None
                } else {
                    Some(0.0)
                }
            }
            QueryClause::Or(left, right) => {
                let score_left = self.evaluate_clause(doc, left, total_docs, avg_doc_len);
                let score_right = self.evaluate_clause(doc, right, total_docs, avg_doc_len);
                match (score_left, score_right) {
                    (Some(sl), Some(sr)) => Some(sl + sr),
                    (Some(sl), None) => Some(sl),
                    (None, Some(sr)) => Some(sr),
                    (None, None) => None,
                }
            }
        }
    }

    /// Persistência atômica em disco.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let dest = path.as_ref();
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = dest.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, dest)?;
        Ok(())
    }

    /// Carregamento rápido do índice persistido.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Option<Self>> {
        let p = path.as_ref();
        if !p.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(p)?;
        let index = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(index))
    }
}
