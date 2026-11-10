# Especificação Técnica: Busca Híbrida Unificada com RRF e Índice Flat SIMD

**ID do Ticket:** #96  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3: Doc/Spec)  
**Responsável:** Lumen Pink / Antigravity  
**Branch:** `feat/96-busca-hibrida-unificada-com-rrf-e-indice-flat-simd`  

---

## 1. Visão Geral e Motivação

O sistema de busca atual do Lauda padece de quatro deficiências graves:
1. **Full scan de blobs no servidor:** a cada requisição de busca, o servidor faz `SELECT ... embeddings_blob FROM indexed_documents` de dezenas de milhares de mensagens, decodificando vetores do disco e calculando cosseno em força bruta sem índice vetorial real.
2. **Pseudo-semântica ruidosa:** o gerador `SimpleCpuEmbeddingProvider` usa dispersão (hash) de 3-gramas com corte baixo de similaridade (`0.15`), provocando colisões estatísticas que pontuam e poluem a busca com e-mails irrelevantes.
3. **Ponderação distorcida:** a pontuação combinada dava 60% de peso ao vetor ruidoso e apenas 40% ao match léxico exato (BM25), sepultando resultados legítimos.
4. **Descompasso e duplicação no client:** enquanto os corpos são sincronizados em background contínuo no `sync_all_cycle`, a fatia de índice vetorial dos 90 dias (`fetch_index_slice`) só atualizava na abertura manual da caixa, e a busca local executava o mesmo algoritmo ruidoso.

Esta especificação implementa a arquitetura de **Busca Híbrida Unificada** cobrindo `lauda-index`, `lauda-server` e `lauda-client`:
- Recuperação em dois canais concorrentes independentes (BM25 léxico + proximidade vetorial semântica).
- Fusão harmônica de posições via **RRF (Reciprocal Rank Fusion)** para permitir a descoberta tanto de termos exatos quanto de sinônimos/conceitos (ex.: "boleto" encontrando "fatura").
- **Índice Flat Contíguo com SIMD em Rust puro** (`FlatVectorIndex`) na memória do servidor e no cache local do cliente, garantindo 100% de recall exato, inserção/remoção imediata $O(1)$, baixíssimo uso de RAM e compatibilidade nativa com WebAssembly (WASM).
- Provedor neural unificado com `multilingual-e5-small` (INT8, 384 dimensões) via `tract` em Rust puro.
- Sincronização contínua unificada da fatia de 90 dias no ciclo de background do cliente.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Crate `lauda-index`

#### 2.1.1. `FlatVectorIndex`
Estrutura em memória para armazenar e buscar vetores de 384 dimensões:
```rust
pub struct FlatVectorIndex {
    dim: usize, // 384
    uids: Vec<u32>,
    vectors: Vec<f32>, // Alinhado, tamanho uids.len() * dim
    uid_to_idx: HashMap<u32, usize>,
}

impl FlatVectorIndex {
    pub fn new(dim: usize) -> Self;
    pub fn insert(&mut self, uid: u32, vector: &[f32]);
    pub fn remove(&mut self, uid: u32) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn search_knn(&self, query_vec: &[f32], k: usize) -> Vec<(u32, f32)>;
    pub fn clear(&mut self);
}
```

- **Similaridade:** Produto escalar com vetores normalizados ($L_2$).
- **Aceleração:** Loop vetorial com suporte a SIMD ou auto-vetorização LLVM.
- **Top-K:** Manutenção de `BinaryHeap` invertida (min-heap) para complexidade $O(N \log K)$.

#### 2.1.2. Fusão RRF (`reciprocal_rank_fusion`)
Função pura de fusão de rankings:
```rust
pub fn reciprocal_rank_fusion(
    lexical_ranked: &[u32],
    semantic_ranked: &[u32],
    k_const: f32, // padrão: 60.0
) -> Vec<(u32, f32)>;
```
- Cálculo: $\text{RRF}(d) = \sum_{c \in \{\text{lexical}, \text{semantic}\}} \frac{1}{k + \text{rank}_c(d) + 1}$
- Preserva a identidade de mensagens que pontuaram apenas em um canal (sinônimos sem termo exato, ou termos exatos sem correlação semântica ampla) e impulsiona ao topo mensagens que pontuaram em ambos.

#### 2.1.3. `TractEmbeddingProvider`
Inferência ONNX em Rust puro com `tract-onnx`:
```rust
pub struct TractEmbeddingProvider {
    // Model plan otimizado
}

impl EmbeddingProvider for TractEmbeddingProvider {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn embed_query(&self, query: &str) -> Vec<f32>;
}
```
- Normalização euclidiana ($L_2$) de 384 floats.
- Prefixos assimétricos E5: `passage: ` para documentos e `query: ` para buscas.
- Fallback automático para `SimpleCpuEmbeddingProvider` caso o arquivo ONNX não esteja disponível no ambiente.

---

### 2.2. Crate `lauda-server`

#### 2.2.1. Gerenciamento em Memória no `sqlite_index.rs`
- O `SqliteIndexStore` mantém internamente uma instância de `Arc<RwLock<FlatVectorIndex>>`.
- Na inicialização (`SqliteIndexStore::open`), lê sequencialmente os vetores gravados na base em um único passe e povoa a matriz em RAM.
- Em `reconcile_mailbox`, `index_message` ou remoção de mensagem, atualiza o `FlatVectorIndex` em memória concorrentemente.
- Em `search_hybrid`:
  1. Executa busca BM25 via SQLite FTS5 (top 50 UIDs).
  2. Gera embedding da consulta (`query: ...`).
  3. Executa `flat_index.search_knn(&query_vec, 50)`.
  4. Executa `reciprocal_rank_fusion` combinando os dois canais.
  5. Carrega metadados e monta os envelopes apenas para o conjunto podado de finalistas.

#### 2.2.2. Endpoint `/api/mail/embed`
- Rota GET/POST para receber `q` e retornar o vetor de 384 floats em JSON ou binário, permitindo que clientes conectados economizem inferência local na busca rápida.

---

### 2.3. Crate `lauda-client`

#### 2.3.1. Sincronização da Fatia de 90 Dias
- O `sync_all_cycle` em `sync.rs` passa a atualizar periodicamente a fatia de índice (`fetch_index_slice`) sincronizando o cache local `cache_indexed_documents`.
- Os vetores locais são mantidos no `FlatVectorIndex` local em memória.

#### 2.3.2. Busca em 2 Estágios e Apresentação
- Ao digitar na caixa de busca:
  - Roda busca local RRF nos 90 dias com `FlatVectorIndex` (< 1ms).
  - Emite busca remota no servidor para o acervo completo.
  - Combina no `merge_search_results` sem duplicatas.
  - Exibe etiquetas contextuais nos resultados quando a correspondência é semântica/sinônimo.

---

## 3. Critérios de Aceitação

- [ ] `FlatVectorIndex` armazena e busca vetores de 384 dimensões com 100% de recall exato.
- [ ] `FlatVectorIndex` suporta inserção e remoção $O(1)$ sem vazamento ou degradação.
- [ ] `reciprocal_rank_fusion` eleva documentos presentes em ambos os rankings e preserva candidatos com match exclusivo em um canal.
- [ ] O servidor não executa mais `SELECT ... embeddings_blob` de toda a tabela a cada busca; as buscas utilizam o índice vetorial em memória + FTS5.
- [ ] O cliente atualiza a fatia de 90 dias no ciclo de sincronização em background.
- [ ] Todos os testes unitários e de integração passam e o `cargo clippy` passa estritamente sem warnings.
