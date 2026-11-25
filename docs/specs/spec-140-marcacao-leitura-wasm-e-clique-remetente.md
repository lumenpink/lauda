# Especificação Técnica #140 — Marcação de Leitura no WASM e Clique no Remetente

## 1. Contexto e Motivação
Na versão web (WASM) do Lauda, duas regressões afetam a marcação de mensagens como lidas:
1. **Interceptação de clique no remetente (`inbox_view.rs`):** O `onclick` com `e.stop_propagation()` que alterna seleção múltipla foi colocado no container inteiro do remetente (`flex: 1 1 140px`), fazendo com que toques/cliques no nome do remetente apenas selecionem o card em vez de abrir a mensagem e marcá-la como lida.
2. **Ordem de flush e reconciliação no WASM (`ui/mod.rs`):** No cliente WASM, o flush das ações pendentes no IndexedDB (`list_actions_idb`) ocorria apenas ao final do loop de sincronização (após `list_messages` e `reconcile_ui_messages` e um `sleep` de 60s). Se `list_messages` executasse antes da confirmação do `PATCH /messages/{uid}/read` (ou em modo offline/evento WebSocket), `reconcile_ui_messages` sobrescrevia `is_read` de volta para `false`.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Remover o atalho de seleção por clique no marcador e deixar apenas long-press:**
  - *Motivo do descarte:* No desktop sem tela touch, clicar na bolinha `●`/`○` é o atalho mais prático para entrar em seleção múltipla sem precisar segurar `Shift`. Restringir o `onclick` + `e.stop_propagation()` estritamente ao elemento do marcador (`span` da bolinha) resolve a interceptação mantendo o atalho intacto.
- **Alternativa Descartada 2: Desativar `reconcile_ui_messages` no WASM:**
  - *Motivo do descarte:* Quebraria o recebimento de novos e-mails e atualizações de outras sessões. A abordagem correta é:
    1. Despachar `list_actions_idb()` **antes** de `list_messages` no ciclo do WASM (espelhando o comportamento desktop).
    2. Criar `reconcile_ui_messages_preserving_pending` em `domain/sync.rs` para reaplicar quaisquer `QueuedAction` ainda pendentes (`MarkRead`, `MarkFlagged`, `Delete`, `Move`) sobre os envelopes recebidos do servidor antes de atualizar o signal `messages`.

---

## 3. Contratos de Domínio (`domain/sync.rs`)
```rust
pub fn reconcile_ui_messages_preserving_pending(
    current_box: &str,
    synced_box: &str,
    current_view_messages: &[MessageEnvelope],
    server_messages: &[MessageEnvelope],
    pending_actions: &[QueuedAction],
) -> Option<Vec<MessageEnvelope>>;
```

---

## 4. Critérios de Aceitação
1. Clicar no nome do remetente (quando não há seleção ativa) abre a mensagem e dispara `mark_read` normalmente; clicar apenas no marcador `●`/`○`/`[✓]` alterna seleção.
2. No loop de sincronização WASM em `ui/mod.rs`, `list_actions_idb()` é despachado antes de `list_messages`.
3. `reconcile_ui_messages_preserving_pending` preserva `MarkRead`, `MarkFlagged`, `Delete` e `Move` pendentes da caixa ativa mesmo se o servidor retornar estado antigo.

---

## 5. Laudo de Fechamento e Homologação
*(A preencher na Fase 8 - Quality Gate)*
