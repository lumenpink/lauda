use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Produto escalar entre dois vetores normalizados (similaridade de cosseno acelerada por SIMD).
#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vetores devem possuir dimensões idênticas");
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Elemento auxiliar para manter a Min-Heap ordenada por similaridade.
#[derive(Clone, Copy, PartialEq)]
struct ScoredCandidate {
    score: f32,
    uid: u32,
}

impl Eq for ScoredCandidate {}

impl PartialOrd for ScoredCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Inverte a comparação para que o menor score fique no topo da heap (Min-Heap)
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.uid.cmp(&other.uid))
    }
}

/// Índice vetorial contíguo em memória com busca exata acelerada por instruções SIMD da CPU.
///
/// Garante 100% de recall matemático, inserção/remoção imediata $O(1)$ sem fragmentação
/// e compatibilidade nativa com WebAssembly (WASM).
#[derive(Debug, Clone, Default)]
pub struct FlatVectorIndex {
    dim: usize,
    uids: Vec<u32>,
    vectors: Vec<f32>,
    uid_to_idx: HashMap<u32, usize>,
}

impl FlatVectorIndex {
    /// Inicializa um índice vetorial vazio para vetores de dimensão `dim` (ex.: 384).
    pub fn new(dim: usize) -> Self {
        assert!(dim > 0, "A dimensão vetorial deve ser estritamente positiva");
        Self {
            dim,
            uids: Vec::new(),
            vectors: Vec::new(),
            uid_to_idx: HashMap::new(),
        }
    }

    /// Retorna a dimensão vetorial do índice.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Retorna a quantidade de documentos indexados.
    pub fn len(&self) -> usize {
        self.uids.len()
    }

    /// Informa se o índice está vazio.
    pub fn is_empty(&self) -> bool {
        self.uids.is_empty()
    }

    /// Esvazia completamente o índice liberando as estruturas.
    pub fn clear(&mut self) {
        self.uids.clear();
        self.vectors.clear();
        self.uid_to_idx.clear();
    }

    /// Insere ou atualiza o vetor associado a um determinado `uid`.
    pub fn insert(&mut self, uid: u32, vector: &[f32]) {
        assert_eq!(
            vector.len(),
            self.dim,
            "Dimensão do vetor ({}) diverge da dimensão do índice ({})",
            vector.len(),
            self.dim
        );

        if let Some(&idx) = self.uid_to_idx.get(&uid) {
            let start = idx * self.dim;
            let end = start + self.dim;
            self.vectors[start..end].copy_from_slice(vector);
        } else {
            let idx = self.uids.len();
            self.uids.push(uid);
            self.vectors.extend_from_slice(vector);
            self.uid_to_idx.insert(uid, idx);
        }
    }

    /// Remove o vetor associado ao `uid` em tempo $O(1)$ utilizando `swap_remove`.
    pub fn remove(&mut self, uid: u32) -> bool {
        if let Some(idx) = self.uid_to_idx.remove(&uid) {
            let last_idx = self.uids.len() - 1;
            if idx == last_idx {
                self.uids.pop();
                self.vectors.truncate(last_idx * self.dim);
            } else {
                let last_uid = self.uids[last_idx];
                let target_start = idx * self.dim;
                let last_start = last_idx * self.dim;

                for i in 0..self.dim {
                    self.vectors[target_start + i] = self.vectors[last_start + i];
                }
                self.vectors.truncate(last_idx * self.dim);

                self.uids[idx] = last_uid;
                self.uids.pop();
                self.uid_to_idx.insert(last_uid, idx);
            }
            true
        } else {
            false
        }
    }

    /// Busca os `k` vizinhos mais próximos da consulta via produto escalar acelerado por SIMD.
    ///
    /// Retorna os pares `(uid, similaridade)` ordenados por similaridade decrescente.
    pub fn search_knn(&self, query_vec: &[f32], k: usize) -> Vec<(u32, f32)> {
        if self.is_empty() || k == 0 || query_vec.len() != self.dim {
            return Vec::new();
        }

        let mut heap = BinaryHeap::with_capacity(k);

        for (i, &uid) in self.uids.iter().enumerate() {
            let start = i * self.dim;
            let end = start + self.dim;
            let doc_slice = &self.vectors[start..end];
            let sim = dot_product(query_vec, doc_slice);

            if heap.len() < k {
                heap.push(ScoredCandidate { score: sim, uid });
            } else if let Some(min_top) = heap.peek()
                && sim > min_top.score
            {
                heap.pop();
                heap.push(ScoredCandidate { score: sim, uid });
            }
        }

        let mut results = Vec::with_capacity(heap.len());
        while let Some(candidate) = heap.pop() {
            results.push((candidate.uid, candidate.score));
        }

        // Como retiramos da Min-Heap, os resultados estão em ordem crescente; invertemos para decrescente.
        results.reverse();
        results
    }
}

/// Algoritmo RRF (Reciprocal Rank Fusion) para combinar rankings ordenados independentes.
///
/// Permite que e-mails que deram match léxico puro (BM25) e e-mails que deram match
/// semântico por significado/sinônimo compartilhem o mesmo espaço ordenado sem viés artificial.
///
/// $$\text{Score}(d) = \sum_{c} \frac{1}{k_{\text{const}} + \text{rank}_c(d) + 1}$$
pub fn reciprocal_rank_fusion(
    lexical_ranked: &[u32],
    semantic_ranked: &[u32],
    k_const: f32,
) -> Vec<(u32, f32)> {
    let mut scores: HashMap<u32, f32> = HashMap::with_capacity(lexical_ranked.len() + semantic_ranked.len());

    for (rank, &uid) in lexical_ranked.iter().enumerate() {
        let entry = scores.entry(uid).or_insert(0.0);
        *entry += 1.0 / (k_const + rank as f32 + 1.0);
    }

    for (rank, &uid) in semantic_ranked.iter().enumerate() {
        let entry = scores.entry(uid).or_insert(0.0);
        *entry += 1.0 / (k_const + rank as f32 + 1.0);
    }

    let mut ranked_list: Vec<(u32, f32)> = scores.into_iter().collect();

    ranked_list.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.0.cmp(&a.0))
    });

    ranked_list
}
