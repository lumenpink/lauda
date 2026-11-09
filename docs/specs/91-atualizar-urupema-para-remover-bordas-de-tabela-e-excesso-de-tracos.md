# Especificação Técnica — Ticket #91
## Atualizar Urupema para Remover Bordas de Tabela e Excesso de Traços

- **Ticket:** #91
- **Branch:** `fix/91-atualizar-urupema-para-remover-bordas-de-tabela-e-excesso-de-tracos`
- **Data:** 2026-11-08

---

### 1. Contexto e Motivação
Mensagens de e-mail de marketing e newsletters (ex.: Shein, Reddit, Pinterest, TikTok, AliExpress) utilizam layouts com múltiplas tabelas aninhadas (`<table><tr><td>`). Ao converter essas partes HTML para texto Markdown:
1. O conversor `html2text` gerava bordas completas de tabelas usando caracteres Unicode Box Drawing (`─`, `│`, `┬`, etc.), gerando centenas de milhares de caracteres de ruído visual.
2. Divisores horizontais consecutivos (`---`, `───`, etc.) e células vazias com apenas pontos espaçadores (`.`) poluíam o Markdown gerado (`txt auto`).
3. O `urupema` foi atualizado para a versão `0.1.2` no registro `gambi`, integrando `no_table_borders` e a higienização de divisores horizontais repetidos em `sanitize_markdown_noise`.

---

### 2. Requisitos Técnicos
1. Atualizar a dependência `urupema` para a versão `0.1.2` em `lauda-server/Cargo.toml` e `lauda-index/Cargo.toml`.
2. Sincronizar o `Cargo.lock` do workspace para a versão `0.1.2` da crate `urupema`.
3. Em `lauda-server/src/adapters/maildir.rs`, configurar o `html2text` com `no_table_borders()` alinhado com o `urupema`.
4. Adicionar teste de integração/unidade validando que um e-mail com tabelas HTML aninhadas e múltiplos `<hr>` repetidos é convertido sem caracteres de desenho de caixas (`─`) e sem `---` duplicados.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação.
- **Fase 4 (Red Test):** Teste unitário em `lauda-server/tests/` exigindo que HTML com tabelas e divisores repetidos seja convertido sem caracteres de borda (`─`) e sem duplicatas de `---`.
- **Fase 5 (Green Implement):** Elevação da dependência do `urupema` para `0.1.2`, atualização do `Cargo.lock` e adoção de `no_table_borders()` no adaptador `maildir.rs`.
- **Fase 6 (Refactor & Mutation):** Testes de condições de contorno e preservação de texto semântico.
- **Fase 7 (Style & Changelog):** Formatação com `cargo fmt`, clippy e registro em `CHANGELOG.md`.
- **Fase 8 (Quality Gate):** Checagem rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento e merge canônico via CCCP.

---

### 4. Status
- **Status:** CONCLUÍDO (CONCLUDED)
- **Validação:** Testes em `lauda-server/tests/sanitizer_and_reindex_test.rs` aprovados com `urupema` 0.1.2 e sem caracteres de borda.
