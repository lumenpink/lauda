# Especificação Técnica - Ticket #115
## Cache IndexedDB via Rexie e Busca Local BM25 no WASM (`lauda-client`)

### Contexto & Motivação
No `lauda-client`, a persistência de mensagens e caixas para o alvo `wasm32-unknown-unknown` utilizava primitivas síncronas do `window.localStorage()`, sujeitas ao limite estrito de ~5 MB e capazes de congelar a interface durante gravações massivas. Além disso, as funções `save_index_slice` e `load_index_slice` no WASM estavam como no-ops retornando `Ok(None)`, o que inviabilizava a busca local offline na Web.
Na interface (`inbox_view.rs`), a busca local tentava instanciar o `SimpleCpuEmbeddingProvider` para cálculo híbrido RRF mesmo no ambiente web, quando a decisão arquitetural canônica para o WASM é operar exclusivamente via busca léxica/BM25 sobre os documentos cacheados.

### Árvore de Decisão
- **Alternativa A (Descartada):** Manter `localStorage` serializando fatias comprimidas em Base64/Gzip.
  - *Motivo do descarte:* `localStorage` continua sendo síncrono e bloqueante na thread principal. Uma fatia de 90 dias com dezenas de e-mails facilmente estoura a cota do navegador (DOMException QuotaExceededError).
- **Alternativa B (Descartada):** Portar SQLite via WASM com `sqlite3-wasm` / OPFS.
  - *Motivo do descarte:* Complexidade excessiva de build e tamanho de artefato WASM proibitivo. Natasha teria que manter dois wrappers de SQLite com flags de compilação distintas.
- **Alternativa C (Escolhida):** Padronizar a persistência web no **IndexedDB** via biblioteca **`rexie`**, seguindo o mesmo padrão arquitetural já estabelecido e aprovado em `calendar_cache.rs`.
  - *Vantagens:* Armazenamento assíncrono, capacidade de centenas de megabytes, zero bloqueio de UI e reaproveitamento do ecossistema de tipos do cliente.
  - *Busca no WASM:* No carregamento da fatia no WASM, o `MessageIndex` é reconstruído em memória a partir dos registros do IndexedDB. A busca local roda `local_index.search(&q)` (BM25 puro sem provedor vetorial). Em alvos nativos (`not(wasm32)`), o SQLite e a busca híbrida continuam inalterados.

### Critérios de Aceite
1. O banco `lauda_messages` no IndexedDB gerencia stores para `mailboxes`, `envelopes` e `index_slices`.
2. `save_index_slice_idb` e `load_index_slice_idb` (ou métodos assíncronos correspondentes no WASM) salvam e recuperam fatias de documentos pré-indexados.
3. A busca local no WASM (`inbox_view.rs`) utiliza `local_index.search(&q)` ordenado por relevância BM25, sem exigir provedor vetorial.
4. O cache nativo (`cfg(not(target_arch = "wasm32"))`) permanece 100% retrocompatível com SQLite.
5. Validação com testes unitários no target nativo e compilação limpa no target WASM.
