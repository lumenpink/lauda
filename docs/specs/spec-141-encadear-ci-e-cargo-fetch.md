# Especificação Técnica — Ticket #141
## Encadeamento de Dependências de Jobs e `cargo fetch` Antes de `dx build --offline`

### 1. Contexto e Problema
1. No `.forgejo/workflows/ci.yml` do `lauda`, `deploy-web` falhou com `failed to download crunchy v0.2.4 — attempting to make an HTTP request, but --offline was specified`.
2. Como `deploy-web`, `deploy-apk`, `deploy-banca` e `deploy-server` rodavam todos em paralelo (`needs: [test]`), o `deploy-banca` subiu os binários e disparou o webhook para o `gossnab` mesmo com `deploy-web` falho, gerando loop de 404 em `site.tar.gz`.

### 2. Árvore de Decisões (Registro para a Natasha)
- **Decisão 1: Adicionar `cargo fetch` antes de `dx build --offline`**
  - *Alternativa descartada:* Remover `--offline` do `dx build`.
  - *Motivo:* O contrato `wasm_ui_contract_test.rs` exige `--offline` na linha do `dx build`. Executar `cargo fetch` antes garante que crates faltantes no cache da imagem `gost-ci-wasm` sejam baixadas previamente, mantendo o `dx build --offline` determinístico.
- **Decisão 2: Encadear `needs:` sequencialmente (`test` → `deploy-web` → `deploy-apk` → `deploy-banca` → `deploy-server`)**
  - *Alternativa descartada:* Manter jobs paralelos após `test`.
  - *Motivo:* Se qualquer etapa anterior falhar, toda a cadeia de entrega subsequente é abortada antes de publicar o pacote final de binários ou container.

### 3. Critérios de Aceite
- [x] `cargo fetch` adicionado antes de `dx build ... --offline` em `deploy-web`.
- [x] Cadeia de `needs:` configurada entre `deploy-web`, `deploy-apk`, `deploy-banca` e `deploy-server`.
- [x] `wasm_ui_contract_test`, `test-ci-android-deploy.sh` e `cccp check --strict` aprovados.
