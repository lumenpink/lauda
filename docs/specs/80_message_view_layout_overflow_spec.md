# Especificação Técnica: Contenção de Layout e Prevenção de Overflow no MessageView

**ID do Ticket:** #80  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Andamento (Fase 3: Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Ao visualizar mensagens de email no cliente (`MessageView`), conteúdos gerados pelo conversor de texto estruturado (`txt_auto`), renderizações Markdown, blocos pré-formatados extensos ou tabelas HTML podem transbordar os limites horizontais do container.

Sem contenção estrita (`min-width: 0`, `max-width: 100%`, `overflow-x: auto`, `overflow-wrap: break-word`), elementos com largura intrínseca maior que a tela forçam o container flexível a se expandir além da viewport, quebrando o layout geral da aplicação e desconfigurando cabeçalhos, barras de navegação e menus adjacentes.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Contenção no Contêiner Raiz do MessageView
- O container raiz de `MessageView` deve declarar explicitamente:
  - `min-width: 0` (impedindo que itens flex herdem `min-width: auto` que previne encolhimento).
  - `max-width: 100%` e `width: 100%`.
  - `box-sizing: border-box`.

### 2.2. Contenção no Painel de Conteúdo (`container_style`)
- O container do corpo da mensagem (`TxtAuto`, `TxtOriginal`, `Html`) deve conter:
  - `max-width: 100%`.
  - `min-width: 0`.
  - `box-sizing: border-box`.
  - `overflow-wrap: break-word` e `word-break: break-word`.
  - `overflow-x: auto` como salvaguarda para tabelas e blocos de código indecomponíveis.

### 2.3. Estilização Confinada de Elementos Internos
- **TxtAuto / Markdown:**
  - `pre` e `code`: quebra de linha preservada com `white-space: pre-wrap; word-break: break-all;`.
  - `table`: `max-width: 100%; display: block; overflow-x: auto;`.
  - `img`: `max-width: 100%; height: auto;`.
- **TxtOriginal:**
  - `pre`: `white-space: pre-wrap; word-break: break-word; overflow-wrap: break-word; max-width: 100%;`.
- **Html:**
  - `.lauda-html-body`: `max-width: 100%; overflow-x: auto; overflow-wrap: break-word; word-break: break-word;`.
  - Regras embutidas garantindo que tabelas e imagens não excedam 100% da largura útil.

### 2.4. Contenção no `<main>` Global
- O elemento `<main>` em `ui/mod.rs` deve assegurar `min-width: 0; width: 100%; max-width: 100%; box-sizing: border-box;`.

---

## 3. Critérios de Aceite

1. Mensagens com linhas sem espaço de tamanho superior à largura da tela sofrem quebra de palavra (`overflow-wrap: break-word`).
2. Tabelas largas e blocos de código extensos mantêm scrollbar horizontal interna confinada no container do corpo, sem esticar a página.
3. Menus e cabeçalho global permanecem estáveis e alinhados na largura da janela ao abrir qualquer mensagem.
4. Suíte de testes automatizada valida a presença dos atributos CSS de contenção e a higienização de overflow.
5. Conformidade estrita Gosplan (`cargo fmt`, `clippy -D warnings`, `cargo test`).

---

## 4. Registro de Fases DD-TDD

- [x] **Fase 1 (Ticket):** Ticket #80 aberto no Codeberg.
- [x] **Fase 2 (Branch):** Branch `fix/80-message-view-layout-overflow` criada a partir da `main`.
- [x] **Fase 3 (Spec):** Especificação técnica formalizada neste documento.
- [ ] **Fase 4 (Red Test):** Teste de layout falhando contra a implementação atual (ausência de regras de contenção).
- [ ] **Fase 5 (Green Implement):** Aplicação das regras de contenção e testes passando verde.
- [ ] **Fase 6 (Refactor):** Otimização e limpeza visual sem warnings.
- [ ] **Fase 7 (Mutation Test):** Testes de mutação verificando sensibilidade à remoção de regras de contenção.
- [ ] **Fase 8 (Quality Gate):** Validação com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test`.
- [ ] **Fase 9 (Finish & Close):** Reconciliação, merge via CCCP e fechamento do ticket.
