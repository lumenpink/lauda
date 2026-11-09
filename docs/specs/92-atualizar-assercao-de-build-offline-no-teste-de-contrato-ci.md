# Especificação Técnica — Ticket #92
## Atualizar Asserção de Build Offline no Teste de Contrato CI

- **Ticket:** #92
- **Branch:** `fix/92-atualizar-assercao-de-build-offline-no-teste-de-contrato-ci`
- **Data:** 2026-11-09

---

### 1. Contexto e Motivação
No ticket #90, a flag `--base-path /app/` foi adicionada à chamada `dx build` no arquivo `.forgejo/workflows/ci.yml` para garantir que o cliente WebAssembly do Lauda localize seus assets corretamente quando servido sob `/app/`.
Contudo, o teste de contrato `test_ci_workflow_has_dx_offline_flag` em `lauda-client/tests/wasm_ui_contract_test.rs` realizava uma busca por substring exata:
`"dx build -p lauda-client --platform web --release --offline"`
A adição de `--base-path /app/` antes de `--offline` provocou a quebra do teste no CI (Run #1876), abortando a esteira antes dos jobs de deploy.

---

### 2. Requisitos Técnicos
1. Atualizar o teste `test_ci_workflow_has_dx_offline_flag` em `lauda-client/tests/wasm_ui_contract_test.rs` para:
   - Verificar que a linha de comando `dx build` para `lauda-client` inclui tanto a flag `--offline` quanto `--base-path /app/`.
   - Garantir resiliência na checagem de parâmetros de build do Dioxus sem quebrar por variações de ordenação ou inclusão de flags de caminho.
2. Validar que `cargo test -p lauda-client --test wasm_ui_contract_test` passe com sucesso.
3. Registrar a alteração em `CHANGELOG.md` referenciando o ticket #92.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação.
- **Fase 4 (Red Test):** Validação da falha atual do teste `wasm_ui_contract_test`.
- **Fase 5 (Green Implement):** Ajuste do teste em `lauda-client/tests/wasm_ui_contract_test.rs`.
- **Fase 6 (Refactor & Mutation):** Testes adicionais de mutação e conformidade.
- **Fase 7 (Style & Changelog):** Formatação `cargo fmt`, clippy e atualização de `CHANGELOG.md`.
- **Fase 8 (Quality Gate):** Checagem rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento e merge canônico via CCCP.
