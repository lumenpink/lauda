# Especificação Técnica - Ticket #126
## Isolamento da Busca Semântica no Servidor e Restrição do Cliente e Sincronização ao BM25

### Contexto & Motivação
A busca semântica neural híbrida (RRF) utilizando a família de modelos E5 foi integrada com sucesso no `lauda-server`.
No modelo anterior, cogitava-se embutir a inferência neural ou sincronizar tensores densos para os clientes (`lauda-client`) em WebAssembly (WASM) e Android APK.
Contudo, modelos neurais densos exigem centenas de megabytes de RAM e geram payloads de sincronização excessivamente pesados pela rede, sendo inadequados para dispositivos móveis ou sessões leves de navegador.

Definiu-se uma mudança de paradigma arquitetural:
1. **A busca semântica neural reside exclusivamente no servidor.**
2. **O cliente opera localmente em modo offline utilizando estritamente BM25** sobre os termos invertidos e metadados armazenados em IndexedDB (WASM) ou cache local (Android/Desktop).
3. **O protocolo de sincronização cliente-servidor purga os blobs de embeddings**, transmitindo apenas termos léxicos e frequências para alimentar o BM25 local.

---

### Árvore de Decisão
- **Alternativa A (Descartada): Embutir inferência neural no cliente via WebAssembly / NDK**
  - *Motivo do descarte:* Aumenta drasticamente o bundle do aplicativo (dezenas ou centenas de megabytes), degrada o tempo de inicialização, consome bateria excessiva e introduz latência perceptível durante a digitação de buscas locais.
- **Alternativa B (Descartada): Sincronizar blobs de embeddings de todos os segmentos para o cliente**
  - *Motivo do descarte:* Cada e-mail com múltiplos segmentos gera centenas de números em ponto flutuante em JSON, gerando um tráfego de rede proibitivo (megabytes) sem que o cliente realize cálculo vetorial local.
- **Alternativa C (Escolhida): Cliente puramente BM25 e sincronização de fatias enxuta**
  - *Motivo da escolha:* O servidor executa a busca híbrida unificada RRF (BM25 + Candle E5). O cliente mantém busca local instantânea via BM25 e filtros Gmail. A sincronização de fatias (`index-slice`) trafega apenas termos e frequências, economizando mais de 90% de tráfego de rede e armazenamento local.

---

### Requisitos Técnicos

1. **Desacoplamento de Dependências no Cliente (`lauda-client/Cargo.toml`):**
   - Configurar `lauda-index = { path = "../lauda-index", default-features = false }`.
   - Assegurar que o cliente não inclua `candle-core`, `candle-transformers`, `candle-nn` nem compiladores de tensores densos.

2. **Purga de Embeddings na Sincronização de Fatias (`get_recent_slice`):**
   - No adaptador SQLite (`sqlite_index.rs`), a exportação de `IndexedDoc` na API `/mailboxes/{mailbox}/index-slice` define `segment_embeddings` vazio.
   - Isso elimina a transmissão de matrizes densas pela rota HTTP.

3. **Busca Local Unificada no Cliente (`inbox_view.rs` / `in_memory.rs`):**
   - Unificar a busca local em `local_index.search(&q)` (BM25 puro) tanto no target `wasm32` quanto no desktop/Android (`not(target_arch = "wasm32")`).
   - Remover instanciações de provedores neurais sintéticos (`SimpleCpuEmbeddingProvider`) no cliente.

---

### Critérios de Aceitação (GOST DD-TDD)

- [x] **Fase 4 (Red Test):** Teste unitário no cliente e servidor validando que fatias exportadas possuem zero embeddings e que a busca local do cliente funciona estritamente com BM25 sem dependência neural.
- [x] **Fase 5 (Green Implement):** Implementação do desacoplamento de dependências, purga na exportação do slice e unificação da busca BM25 no cliente.
- [x] **Fase 6 (Refactor):** Limpeza de imports e formatação canônica.
- [x] **Fase 7 (Mutation Test):** Testes de mutação cobrindo integridade do BM25 local após recepção de fatias sem embeddings.
- [x] **Fase 8 (Quality Gate):** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e testes passando offline.
- [x] **Fase 9 (Close):** Fechamento do ticket via `cccp close` e integração na `main`.
