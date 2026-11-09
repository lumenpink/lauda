# Especificação Técnica — Ticket #90
## Corrigir Paths e Cópia de Artefatos Web no Workflow de Deploy

- **Ticket:** #90
- **Branch:** `fix/90-corrigir-paths-e-copia-de-artefatos-web-no-workflow-de-deploy`
- **Data:** 2026-11-08

---

### 1. Contexto e Motivação
A aplicação web Lauda (`https://lauda.2lp.in/app/`) é compilada com Dioxus WebAssembly.
Na esteira `.forgejo/workflows/ci.yml` (job `deploy-web`) e no script `scripts/package-all.sh`:
1. O comando `dx build -p lauda-client --platform web --release` precisa receber a flag `--base-path /app/`. Sem isso, o loader JavaScript embute requisições para `/./assets/lauda-client_bg-*.wasm`, que na raiz da Cloudflare Worker resolve para `/assets/` (resultando em 404 Not Found), em vez de `/app/assets/`.
2. O workflow de CI e o script de empacotamento devem copiar adequadamente os artefatos de `target/dx/lauda-client/release/web/public/assets/*` para `site/app/assets/`, mantendo `site/app/index.html` sincronizado com os nomes das tags de módulo geradas.
3. No worker Cloudflare (`worker/index.js`), adicionar fallback para requisições com prefixo `/assets/lauda-client` redirecionando internamente para `/app/assets/lauda-client`, garantindo resiliência total contra requisições que omitam o prefixo `/app`.

---

### 2. Requisitos Técnicos
1. Em `.forgejo/workflows/ci.yml`, o comando de build deve incluir `--base-path /app/`.
2. Em `scripts/package-all.sh`, o comando de build deve incluir `--base-path /app/`.
3. Em `worker/index.js`, adicionar fallback para requisições de assets do cliente solicitados na raiz `/assets/lauda-client`.
4. Em `tests/audit_cd_workflow.sh`, adicionar teste de contrato garantindo a presença de `--base-path /app/` no job `deploy-web`.
5. Adicionar teste de mutação para falhar caso `--base-path /app/` seja removido do workflow.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação.
- **Fase 4 (Red Test):** Adição de asserção em `tests/audit_cd_workflow.sh` exigindo `--base-path /app/` no build web.
- **Fase 5 (Green Implement):** Configuração de `--base-path /app/` no CI e script de empacotamento, e fallback resiliente no worker.
- **Fase 6 (Mutation Test):** Verificação de que mutante sem `--base-path /app/` é detectado e morto.
- **Fase 7 (Refactor & Style):** Revisão de formatação e registro em `CHANGELOG.md`.
- **Fase 8 (Quality Gate):** Checagem rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento e merge canônico via CCCP.
