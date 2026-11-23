# Especificação Técnica — Ticket #127: Avaliação e Integração do Multilingual-E5-Base (768 Dimensões) em CPU

## 1. Contexto & Motivação

No Ticket #126, a busca vetorial neural foi isolada exclusivamente no `lauda-server`. O cliente e a sincronização HTTP de fatias de índice foram restritos ao BM25 léxico, reduzindo drasticamente o tráfego de rede e o consumo de recursos nos clientes móveis e Web.

Essa dissociação viabiliza a evolução do motor semântico no servidor para modelos neurais mais expressivos. Enquanto o modelo `multilingual-e5-small` opera com 384 dimensões e ~118M parâmetros, o modelo `multilingual-e5-base` conta com 768 dimensões e ~278M parâmetros (arquitetura XLM-RoBERTa base com 12 camadas, 12 attention heads e hidden size de 768). O modelo *base* proporciona maior capacidade de discernimento contextual, captura superior de sinônimos e ordenação mais precisa em buscas de linguagem natural complexas.

## 2. Diagnóstico Arquitetural e Gargalos Identificados

Uma análise no código do `lauda-server` (`lauda-server/src/adapters/sqlite_index.rs`) revelou acoplamentos rígidos à dimensão 384:
1. **Serialização Binária em SQLite (`encode_segment_embeddings` / `decode_segment_embeddings`)**:
   - O chunking binário assume `entry_size = 1 + 384 * 4` (1537 bytes por segmento).
   - Ao receber vetores de 768 dimensões (3073 bytes por segmento), a decodificação atual corrompe os limites dos segmentos e resulta em vetores truncados ou incorretos.
2. **Índice Vetorial em RAM (`FlatVectorIndex`)**:
   - No `save_document` e no carregamento inicial (`preload_vector_indexes`), existe a trava explícita `&& rep_vec.len() == 384` e `FlatVectorIndex::new(384)`.
   - Vetores de 768 dimensões são descartados silenciosamente da busca KNN em memória.
3. **Compatibilidade com Dados Existentes**:
   - Natasha precisa que a base existente com índices de 384 dimensões continue funcionando sem corrupção e sem exigir migração destrutiva.

## 3. Diretrizes de Projeto & Contratos Técnicos

### 3.1. Formato Binário Auto-descritivo (V2) com Retrocompatibilidade (V1)
Para suportar tanto 384 quanto 768 dimensões (e futuras extensões) preservando bancos SQLite já populados:
- **Formato V1 (Legado)**:
  - BLOB sem cabeçalho mágico onde o tamanho total é múltiplo de `1 + 384 * 4` (1537 bytes).
  - Decodificado utilizando dimensão fixa 384.
- **Formato V2 (Auto-descritivo)**:
  - BLOB com prefixo de 4 bytes: `b"V2"` (2 bytes) + `dim` (2 bytes, little-endian, ex.: 384 ou 768).
  - Em seguida, segmentos contíguos: `kind` (1 byte) + `dim * 4` bytes de floats IEEE-754 little-endian.
- Novos registros gerados pelo servidor utilizam o formato V2, gravando a dimensão real do provedor ativo.

### 3.2. Dimensão Dinâmica no `FlatVectorIndex` em RAM
- No `SqliteIndexStore`, o `FlatVectorIndex` associado a cada mailbox deve ser instanciado ou atualizado com a dimensão do vetor representativo extraído (`rep_vec.len()`).
- Se a dimensão dos novos vetores diferir da dimensão atual do índice em RAM (ex.: alternância de modelo via `LAUDA_MODEL_PATH`), o índice é reconfigurado para a dimensão do novo modelo.

### 3.3. Inferência Candle Multilingual-E5-Base
- `CandleEmbeddingProvider` já lê a dimensão `hidden_size` do `config.json`.
- Ao carregar `multilingual-e5-base`, o provider assume `dim = 768`, executa mean-pooling sobre os 768 canais ocultos e normalização euclidiana $L_2 = 1.0$.

---

## 4. Critérios de Aceitação (GOST DD-TDD)

- [x] **Fase 4 (Red Test):** Testes unitários comprovando que:
  1. `encode_segment_embeddings` e `decode_segment_embeddings` preservam vetores de 768 dimensões sem corrupção.
  2. Blobs legados de 384 dimensões continuam sendo decodificados corretamente.
  3. `SqliteIndexStore` indexa e busca documentos com vetores de 768 dimensões via KNN no `FlatVectorIndex`.
- [x] **Fase 5 (Green Implement):** Implementação da serialização V2 auto-descritiva e suporte à dimensão dinâmica no `FlatVectorIndex`.
- [x] **Fase 6 (Refactor):** Otimizações e limpeza de código.
- [x] **Fase 7 (Mutation Test):** Testes de mutação cobrindo alternância de dimensões, vetores malformados e integridade de norma euclidiana.
- [x] **Fase 8 (Quality Gate):** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e testes offline aprovados.
- [x] **Fase 9 (Close):** Fechamento do Ticket #127 e integração na `main`.
