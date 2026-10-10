use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::parser::{Query, QueryClause, parse_query};
use crate::scorer::{levenshtein_distance, score_bm25};
use crate::tokenizer::{normalize, tokenize};
use lauda_types::Message;

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
        };

        self.documents.insert(uid, doc);
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
