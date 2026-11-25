# Especificação Técnica #139 — Responsividade Mobile em Barras e Cards do Cliente

## 1. Contexto e Motivação
Em auditoria via Playwright no viewport mobile `375x812`, o `InboxView` apresentou **+470px de estouro horizontal** (`docScrollW = 845px`), causando:
1. **Barra de Controle e Ações (`inbox_view.rs`):** Container de botões sem `flex-wrap: wrap`, empurrando botões (`CONTATOS`, `REGRAS`, `🔄`, `SAIR`) para fora da viewport ou quebrando rótulos de botões em múltiplas linhas.
2. **Cards da lista de e-mails (`inbox_view.rs`):** Linha de cabeçalho do card sem `flex-wrap: wrap` e remetente sem `min-width: 0` / truncamento com reticências (`overflow: hidden; text-overflow: ellipsis; white-space: nowrap`), fazendo remetentes longos empurrarem os badges (`[🔒]`, `[✓ PGP]`, `📎`, `★`) e a **data** para fora da box.
3. **Outras barras:** `MessageView` sem `white-space: nowrap` nos botões de ação; `ComposeView` e `CalendarView` com containers `flex` horizontais sem `flex-wrap: wrap`.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Ocultar botões da barra superior em um segundo menu hambúrguer:**
  - *Motivo do descarte:* Já existe um drawer para navegação de pastas (`☰ PASTAS`). Esconder ações primárias (`+ NOVA MENSAGEM`, `AGENDA`, `CONTATOS`, `REGRAS`) aumentaria o número de toques. Com `flex-wrap: wrap` + `white-space: nowrap`, os botões quebram naturalmente em linhas limpas sem quebrar o texto interno.
- **Alternativa Descartada 2: Quebrar o remetente em várias linhas com `word-break: break-all` no card:**
  - *Motivo do descarte:* Quebrar endereços como `<faturamento@provedor.com.br>` em 3 linhas de altura destrói a escaneabilidade da lista de e-mails no celular. Truncar o remetente em 1 linha com `min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;` dentro de um item flex com `flex: 1 1 0%; min-width: 0;` mantém a data sempre visível à direita dentro da box, enquanto o assunto abaixo quebra livremente com `overflow-wrap: anywhere`.
- **Escolha Adotada:**
  - Helpers/constantes de estilo mobile-safe em `domain/view.rs` + aplicação uniforme nos componentes `inbox_view.rs`, `message_view.rs`, `compose_view.rs` e `calendar_view.rs`.

---

## 3. Critérios de Aceitação
1. Em viewport `375x812`, `InboxView` (lista normal e modo CAB) deve ter `horizontalOverflowPx == 0` (`docScrollW <= 375`).
2. Nenhum botão nas barras de `InboxView`, `MessageView` e `ComposeView` deve quebrar o texto interno em duas linhas (`white-space: nowrap`).
3. Em cards com remetentes longos (> 80 caracteres), a data e os badges devem permanecer integralmente dentro dos limites da box (`right <= card.right <= 375`).

---

## 4. Laudo de Fechamento e Homologação
*(A preencher na Fase 8 - Quality Gate)*
