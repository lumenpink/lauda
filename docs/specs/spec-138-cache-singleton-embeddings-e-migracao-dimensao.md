# Especificação Técnica #138 — Cache Singleton de Embeddings e Expurgo Automático de Dimensão Divergente

## 1. Contexto e Motivação
Em produção (`lauda-server` em contêiner Docker), observou-se consumo de memória superior a **2,65 GiB de RAM** (e ~173 GB de leitura em disco em 14 horas de execução). A investigação identificou duas causas:
1. **`DynamicEmbeddingProvider::from_path_or_fallback` sem cache singleton:** Cada chamada de `search_hybrid` (`sqlite_index.rs`), cada ciclo do worker de reconciliação em background (`maildir.rs`) e cada busca sem SQLite recarregava do zero o arquivo `model.safetensors`, o `BertModel` e o `tokenizer.json` (vocabulário XLM-RoBERTa de 250 mil tokens). Isso causava picos recorrentes de alocação no heap, fragmentação de memória e I/O excessivo.
2. **Uso do `multilingual-e5-base` (768 dimensões):** O modelo base em `F32` consome ~1,11 GB apenas em pesos + ~150 MB no tokenizer + o dobro de RAM no `FlatVectorIndex` em comparação ao `multilingual-e5-small` (384 dimensões, ~470 MB em pesos).

Esta especificação define:
1. Cacheamento global thread-safe em `DynamicEmbeddingProvider::from_path_or_fallback` via `LazyLock<RwLock<HashMap<PathBuf, DynamicEmbeddingProvider>>>`.
2. Exposição da dimensão do provedor ativo (`embedding_dim()`).
3. Detecção e expurgo automático no `SqliteIndexStore` de documentos indexados cuja dimensão vetorial armazenada divirja da dimensão do provedor ativo (`purge_mismatched_dimensions`), permitindo que o worker em background reindexe tudo automaticamente ao alternar entre `e5-base` (768d) e `e5-small` (384d).

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Manter `e5-base` apenas com singleton (`OnceLock`):**
  - *Motivo do descarte:* Embora o singleton elimine os picos por recarregamento repetido, o `e5-base` em `F32` mantém um piso fixo de ~1,2–1,4 GB de RAM em repouso, próximo demais do orçamento de memória desejado para o servidor. O `e5-small` reduz o consumo total do processo para a faixa de ~550–650 MB mantendo alta qualidade multilíngue.
- **Alternativa Descartada 2: Apagar manualmente o arquivo `lauda_index.db` ao trocar de modelo:**
  - *Motivo do descarte:* Frágil e dependente de intervenção manual. Se o administrador ou a Natasha alterar `LAUDA_MODEL_PATH` no futuro, o servidor deve detectar automaticamente a divergência de dimensão no SQLite e agendar a reindexação sem intervenção humana.
- **Alternativa Descartada 3: Injetar o provedor por parâmetro em todas as assinaturas de `SqliteIndexStore` e `MaildirStore`:**
  - *Motivo do descarte:* Quebraria dezenas de chamadas e testes existentes sem necessidade. Cachear internamente em `DynamicEmbeddingProvider::from_path_or_fallback` com `LazyLock<RwLock<HashMap<PathBuf, DynamicEmbeddingProvider>>>` preserva a API pública intacta, suporta múltiplos caminhos de modelo em testes e garante carregamento único por caminho em produção.
- **Escolha Adotada:**
  - Cache singleton por caminho em `lauda-index/src/index.rs`.
  - Método `embedding_dim(&self) -> usize` no `DynamicEmbeddingProvider` (e `EmbeddingProvider` / `CandleEmbeddingProvider`).
  - Método `purge_mismatched_dimensions(&self, expected_dim: usize) -> Result<usize, rusqlite::Error>` no `SqliteIndexStore`, invocado na inicialização do worker de reconciliação do `maildir.rs`.

---

## 3. Contratos de Domínio

### 3.1. `lauda-index/src/index.rs`
```rust
impl DynamicEmbeddingProvider {
    /// Retorna a instância cacheada (singleton por caminho) quando neural,
    /// evitando recarregamento repetido de pesos e tokenizer.
    pub fn from_path_or_fallback<P: AsRef<Path>>(path: P) -> Self;

    /// Retorna a dimensão vetorial produzida por este provedor (ex: 384 ou 768).
    pub fn embedding_dim(&self) -> usize;
}
```

### 3.2. `lauda-server/src/adapters/sqlite_index.rs`
```rust
impl SqliteIndexStore {
    /// Remove do índice documentos cujos vetores tenham dimensão diferente de `expected_dim`,
    /// limpando o FlatVectorIndex correspondente para que a reconciliação reindexe os UIDs.
    pub fn purge_mismatched_dimensions(&self, expected_dim: usize) -> Result<usize, rusqlite::Error>;
}
```

---

## 4. Critérios de Aceitação
1. Chamadas consecutivas a `DynamicEmbeddingProvider::from_path_or_fallback` para o mesmo caminho neural retornam o mesmo ponteiro `Arc` subjacente (sem recarregar disco).
2. `embedding_dim()` retorna exatamente a dimensão do modelo carregado (`384` para `e5-small` / fallback, `768` para `e5-base`).
3. `purge_mismatched_dimensions(384)` remove documentos salvos com 768 dimensões, preserva documentos já com 384 dimensões e limpa os índices `FlatVectorIndex` incompatíveis em RAM.
4. Ao reiniciar o `lauda-server` com `LAUDA_MODEL_PATH=/models/multilingual-e5-small`, o worker expurga os vetores 768d e inicia a reindexação automática em 384d.

---

## 5. Laudo de Fechamento e Homologação
- **Status:** Concluído e Aprovado no Quality Gate (Fase 8).
- **Implementação:**
  - Cache singleton thread-safe (`PROVIDER_CACHE: LazyLock<RwLock<HashMap<PathBuf, DynamicEmbeddingProvider>>>`) em `lauda-index/src/index.rs`.
  - Exposição de `embedding_dim()` em `EmbeddingProvider`, `CandleEmbeddingProvider` e `DynamicEmbeddingProvider`.
  - Método `purge_mismatched_dimensions(expected_dim)` em `lauda-server/src/adapters/sqlite_index.rs` integrado na inicialização do worker em `lauda-server/src/adapters/maildir.rs`.
- **Validação:** 7/7 testes em `candle_e5_provider_test`, 3/3 testes em `index_storage_test`, `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` aprovados sem alertas.
- **Data:** 2026-11-24

