# Especificação Técnica — Ticket #94
## Autocompletar e Identidades Configuráveis no Campo DE

- **Ticket:** #94
- **Branch:** `feat/94-autocompletar-e-identidades-configuraveis-no-campo-de`
- **Data:** 2026-11-09

---

### 1. Contexto e Motivação
Na tela de composição de mensagens (`ComposeView`), o campo "DE:" apresentava apenas um campo de texto simples pré-preenchido com o usuário da sessão.
1. Usuárias que utilizam múltiplos aliases, nomes de exibição personalizados (ex.: `Lumen Freitas <hi@lumen.pink>`) ou domínios alternativos precisavam redigitar manualmente todo o remetente a cada envio.
2. Era inviável cadastrar previamente identidades favoritas e reaproveitar remetentes já utilizados no passado.
3. Como o campo inicia sempre preenchido, ao clicar ou focar nele, todo o texto deve ser automaticamente selecionado para permitir substituição imediata ao digitar, ou edição posicional ao utilizar teclas direcionais (setas, Home, End).

---

### 2. Requisitos Técnicos

#### 2.1 Modelo de Dados e Persistência (`SessionConfig`)
- Em `lauda-client/src/domain/session.rs`, estender `SessionConfig` com:
  ```rust
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub identities: Vec<String>,
  ```
- Implementar métodos utilitários:
  - `add_identity(&mut self, identity: &str)`: insere a identidade no topo da lista sem duplicatas.
  - `remove_identity(&mut self, identity: &str)`: remove a identidade selecionada.
  - `get_or_default_identities(&self) -> Vec<String>`: retorna a lista de identidades salvas ou, se vazia, a identidade padrão baseada em `username` (ex.: `username@2lp.in` ou `username` caso já contenha `@`).

#### 2.2 Formatação e Parsing de Identidades (`RFC 5322`)
- Em `lauda-client/src/domain/contacts_cache.rs` ou `view.rs`:
  - `parse_identity(raw: &str) -> (Option<String>, String)`: extrai nome e e-mail a partir de `Nome <email@dominio>` ou `email@dominio`.
  - `filter_identities(identities: &[String], query: &str) -> Vec<String>`: busca case-insensitive por nome ou e-mail.

#### 2.3 Interface do Usuário (`ComposeView`)
- No campo `DE:`:
  - Atribuir `id: "compose-from-input"`.
  - No evento `onfocus` e `onclick`:
    - Disparar seleção total do texto via `dioxus::document::eval("document.getElementById('compose-from-input')?.select()")`.
    - Abrir dropdown de sugestões de identidades (`show_from_dropdown.set(true)`).
  - Teclas direcionais e cliques adicionais no interior do texto preservam o comportamento nativo de cursor e edição rápida.
  - No dropdown:
    - Exibir identidades correspondentes ao filtro da digitação.
    - Exibir botão `✕` em cada item para remover da lista persistida.
    - Se o texto digitado não estiver na lista salva e contiver formato de e-mail válido, exibir item de ação rápida `➕ Salvar "{from_input}" nas identidades`.
  - Ao despachar a mensagem (`handle_send`):
    - Se o remetente utilizado ainda não constar na lista de identidades, incluí-lo automaticamente via `session.write().add_identity(&from)` e persistir a sessão via `save_session`.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação técnica.
- **Fase 4 (Red Test):** Testes unitários para persistência de identidades em `SessionConfig` e parsing de identidades.
- **Fase 5 (Green Implement):** Implementação dos campos em `SessionConfig`, helpers de parsing e autocompletar na `ComposeView`.
- **Fase 6 (Refactor & Mutation):** Validação de mutantes e casos de contorno (identidades duplicadas, seleção total).
- **Fase 7 (Style & Changelog):** Formatação com `cargo fmt`, lints `clippy` e registro em `CHANGELOG.md`.
- **Fase 8 (Quality Gate):** Verificação rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento e merge canônico via CCCP.
