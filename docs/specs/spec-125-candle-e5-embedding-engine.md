# Especificação Técnica - Ticket #125
## Integração do Motor Neural Multilingual-E5-Small em CPU via Candle (`lauda-index` / `lauda-server`)

### Contexto & Motivação
A especificação do Lauda (`SPEC.md` e Ticket #116) definiu que o canal semântico da busca híbrida RRF utiliza o modelo `multilingual-e5-small` (384 dimensões em ponto flutuante, normalizado em norma $L_2 = 1.0$).
No entanto, o carregamento do modelo ONNX na implementação prévia operava apenas como fallback sintético (`SimpleCpuEmbeddingProvider`), impedindo generalização semântica e buscas por linguagem natural cotidiana em base real de e-mails.

A usuária solicitou a integração real do modelo E5 e optou pelo framework **Candle** (Hugging Face) em vez de `fastembed-rs`, visando:
1. **Zero dependências nativas C/C++:** Código 100% Rust, sem dependência dinâmica de `libonnxruntime.so` nem complicações de toolchains de C++ no build hermético de CI.
2. **Execução no cliente:** Viabilidade de compilação direta para WebAssembly (`wasm32-unknown-unknown`) e APK Android (`aarch64-linux-android`) compartilhando exatamente a mesma biblioteca de inferência.
3. **Desacoplamento e Resiliência:** Preservação estrita do fallback determinístico caso os pesos locais (`model.safetensors` e `tokenizer.json`) não estejam presentes no disco.

---

### Árvore de Decisão
- **Alternativa A (Descartada): `fastembed-rs` (ONNX Runtime via bindings C++)**
  - *Motivo do descarte:* Exige bibliotecas C++ compartilhadas (`libonnxruntime.so`), inviabilizando compilação direta para `wasm32-unknown-unknown` no cliente web e complicando o pipeline NDK do Android.
- **Alternativa B (Descartada): Inferência remota via API externa**
  - *Motivo do descarte:* Fere a premissa fundamental de soberania, privacidade dos dados de e-mail e operação 100% local/offline do ecossistema Baquara.
- **Alternativa C (Escolhida): Motor de inferência neural local em CPU com `candle-core` e `candle-transformers`**
  - *Motivo da escolha:* Rust puro, compilação nativa em qualquer target do workspace, suporte direto a modelos baseados em BERT/E5 (`BertModel`) via `.safetensors`, com extração do token `[CLS]` ou Max/Mean-Pooling e normalização Euclidiana ($L_2$).

---

### Requisitos Técnicos e Arquitetura

1. **Estrutura do Provedor (`CandleEmbeddingProvider`):**
   - Implementa a trait `EmbeddingProvider` existente em `lauda-index`.
   - Carrega o modelo a partir de um diretório ou arquivos específicos contendo:
     - `model.safetensors` (pesos do `multilingual-e5-small`).
     - `tokenizer.json` (tokenizador Hugging Face).
   - Suporta seleção de dispositivo (`Device::Cpu`).
   - Aplica tokenização, inferência do `BertModel`, Mean Pooling dos embeddings dos tokens (com attention mask) e normalização $L_2$:
     $$\hat{v} = \frac{v}{\|v\|_2}$$
   - Retorna rigorosamente um vetor contíguo `Vec<f32>` de 384 elementos.

2. **Compatibilidade com Instruções E5:**
   - Compatível com `E5InstructionProvider`: prefixo `query: ` para consultas e `passage: ` para segmentos indexados.

3. **Integração no `DynamicEmbeddingProvider`:**
   - `DynamicEmbeddingProvider::from_path_or_fallback(path)`:
     - Se `path` contiver os arquivos do modelo Candle (`model.safetensors` e `tokenizer.json`), instancia `DynamicEmbeddingProvider::new_neural(Arc::new(CandleEmbeddingProvider::load(path)?))`.
     - Caso contrário, recai graciosamente em `DynamicEmbeddingProvider::Fallback(SimpleCpuEmbeddingProvider)`.

4. **Isolamento de Feature Flag (`candle` / `neural`):**
   - As dependências `candle-core`, `candle-transformers` e `tokenizers` ficam encapsuladas em `lauda-index` sob a feature opcional `candle` (ou `neural`).
   - Builds herméticos e targets WASM que não habilitarem a feature continuam compilando normalmente com o fallback em CPU.

---

### Critérios de Aceitação (GOST DD-TDD)

- [ ] **Fase 4 (Red Test):** Teste unitário e de contrato comprovando carregamento de pesos mock/estruturados e inferência de 384 dimensões normalizadas com tolerância a erro.
- [ ] **Fase 5 (Green Implement):** Implementação de `CandleEmbeddingProvider` e conexão com `DynamicEmbeddingProvider::from_path_or_fallback`.
- [ ] **Fase 6 (Refactor):** Código modular, idiomático, zero dependências soltas e formatação canônica.
- [ ] **Fase 7 (Mutation Test):** Testes de mutação validando pooling, normalização $L_2$ e fallback determinístico quando arquivos estão corrompidos ou ausentes.
- [ ] **Fase 8 (Quality Gate):** `cargo fmt --check`, `cargo clippy --all-targets -p lauda-index -p lauda-server -- -D warnings` e testes de módulo passando 100% offline.
- [ ] **Fase 9 (Close):** Commit canônico, registro em documentação e fechamento via `cccp close`.
