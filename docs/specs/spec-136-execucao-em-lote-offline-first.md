# Especificação Técnica #136 — Execução em Lote Offline-First das Ações da Barra Contextual

## 1. Contexto e Motivação
Com a seleção múltipla implementada no Ticket #135, a Barra de Ações Contextual (CAB) exibe os botões de ação em lote: `🗑 EXCLUIR`, `📦 ARQUIVAR`, `✉ LIDA/NÃO LIDA` e `★ FAVORITAR`.

Atualmente, o clique nas ações aplicava mutação simplificada apenas em memória local na UI, sem garantir a persistência em lote no cache local, o enfileiramento resiliente na fila offline (`ActionsQueue` no desktop/mobile e IndexedDB no WASM) e o despacho assíncrono em background para sincronização com o servidor.

Esta especificação define o pipeline **Offline-First Completo para Ações em Lote**:
1. **0ms na UI (Otimismo Instantâneo):** Ações refletem imediatamente na tela (`messages`, `mailboxes`, `search_envelopes`). A seleção é desfeita de imediato para liberar o fluxo de trabalho do usuário.
2. **Persistência Local Imediata:** Remoção ou atualização atômica no armazenamento local (`LocalMessageCache` SQLite/FS e IndexedDB).
3. **Fila Offline Resiliente:** Enfileiramento das ações unitárias em lote na `ActionsQueue`/IDB, garantindo que mesmo se a aplicação for fechada imediatamente ou se a conexão estiver inoperante, nenhuma intenção de mutação seja perdida.
4. **Despacho em Background Sem Rollback:** Tentativa oportuna de sincronização via `spawn`/`spawn_local`. Se o servidor responder com sucesso, o item correspondente é removido da fila; se falhar ou estiver offline, a ação permanece na fila para o ciclo de anti-entropia.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Enviar um único request de batch no backend HTTP (ex.: POST /api/batch):**
  - *Motivo do descarte:* O protocolo atual e a API do smolserver operam com rotas unitárias de mutação (`delete_message`, `move_message`, `mark_read`). Criar um endpoint monolítico quebraria a granularidade do worker de sincronização offline e exigiria alterações no servidor com risco de incompatibilidade com versões instaladas. Despachar ações individuais em tarefas paralelas na fila é mais resiliente e mantém idempotência por UID.
- **Alternativa Descartada 2: Bloquear a interface com modal de progresso ("Excluindo 20 e-mails..."):**
  - *Motivo do descarte:* Viola a regra de ouro de reatividade e postura offline-first. O usuário não deve esperar o servidor para continuar lendo ou navegando.
- **Escolha Adotada (Funções Puras de Lote no Domínio + Despacho Oportunista):**
  - Funções puras em `domain/sync.rs` para aplicação de mutações e geração das ações enfileiradas (`QueuedAction`).
  - Execução desvinculada na interface sem bloqueio de thread.

---

## 3. Contratos de Domínio (`domain/sync.rs`)

```rust
use lauda_types::MessageEnvelope;
use crate::domain::sync::{ActionType, QueuedAction};

/// Aplica remoção em lote otimista na lista da caixa postal e na busca ativa.
/// Retorna a quantidade de mensagens efetivamente removidas.
pub fn apply_batch_removal(
    uids: &[u32],
    mailbox_messages: &mut Vec<MessageEnvelope>,
    search_envelopes: Option<&mut Vec<MessageEnvelope>>,
) -> usize;

/// Cria um vetor de `QueuedAction` a partir de um lote de UIDs e gerador de ação.
pub fn create_batch_queued_actions<F>(
    uids: &[u32],
    mailbox: &str,
    action_fn: F,
    base_unix_timestamp: u64,
) -> Vec<QueuedAction>
where
    F: Fn(u32) -> ActionType;

/// Aplica alteração em lote do status de leitura nos envelopes em memória.
/// Retorna quantas mensagens tiveram seu status alterado.
pub fn apply_batch_read_status(
    uids: &[u32],
    messages: &mut [MessageEnvelope],
    read: bool,
) -> usize;

/// Aplica alteração em lote do status de destaque (estrela) nos envelopes em memória.
/// Retorna quantas mensagens tiveram seu status alterado.
pub fn apply_batch_flag_status(
    uids: &[u32],
    messages: &mut [MessageEnvelope],
    flagged: bool,
) -> usize;
```

---

## 4. Integração no `InboxView` (`ui/inbox_view.rs`)
- **🗑 EXCLUIR:** Chama `apply_batch_removal`, apaga do cache local, enfileira `ActionType::Delete` para cada UID, dispara task de rede e limpa `selected_uids`.
- **📦 ARQUIVAR:** Chama `apply_batch_removal`, apaga da caixa ativa no cache local, enfileira `ActionType::Move { dest_box: "Archive" }`, dispara task de rede e limpa `selected_uids`.
- **✉ LIDA/NÃO LIDA:** Se houver qualquer mensagem não lida no lote selecionado, marca todas como lidas; caso contrário, marca todas como não lidas. Aplica no estado reativo, atualiza contadores de pastas, salva no cache local, enfileira `ActionType::MarkRead { read }` e limpa seleção.
- **★ FAVORITAR:** Se houver qualquer mensagem não favoritada, favorita todas; caso contrário, desfavorita todas. Aplica no estado reativo, salva no cache local, enfileira `ActionType::MarkFlagged { flagged }` e limpa seleção.

---

## 5. Critérios de Aceitação
1. Funções de domínio para lote tratam listas vazias, UIDs inexistentes e preservam invariantes de contagem.
2. Ações da CAB no `InboxView` executam em 0ms visualmente, esvaziam a seleção e fecham a barra contextual.
3. Todas as ações geram entradas na fila persistente local (`ActionsQueue` ou IndexedDB).
4. Sem rollback visual em cenários desconectados ou falhas transitórias de conexão.

---

## 6. Laudo de Fechamento e Homologação
- **Status:** Concluído e Aprovado no Quality Gate (Fase 8).
- **Implementação:**
  - Funções puras em lote em `domain/sync.rs`: `apply_batch_removal`, `create_batch_queued_actions`, `apply_batch_read_status` e `apply_batch_flag_status`.
  - Helper centralizado `enqueue_and_dispatch_batch` no `InboxView` para persistência atômica na `ActionsQueue`/IndexedDB e despacho assíncrono sem rollback.
  - Ações da Barra de Ações Contextual (`🗑 EXCLUIR`, `📦 ARQUIVAR`, `✉ LIDA/NÃO LIDA`, `★ FAVORITAR`) operando com 0ms na UI e atualização de contadores de pastas e busca ativa.
- **Validação:** 26/26 testes aprovados em `client_test.rs`, `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` aprovados sem alertas.
- **Data:** 2026-11-24

