# Especificação Técnica #134 — Transição Instantânea e Fila Offline para Ações no MessageView

## 1. Contexto e Motivação
No `MessageView` do Lauda (`lauda-client`), as ações do usuário sobre o email aberto — especialmente Excluir (`handle_delete`), Arquivar (`handle_archive`) e Favoritar (`handle_toggle_flag`) — dependiam de chamadas assíncronas à rede antes de refletir visualmente ou persistir:
1. `handle_delete` e `handle_archive` aguardavam a resposta remota da API HTTP. A tela ficava parada na mensagem aberta até o servidor responder com sucesso. Se o usuário estivesse sem internet ou a rede falhasse, o email continuava aberto e nada era alterado localmente ou enfileirado.
2. `handle_toggle_flag` aplicava atualização otimista na tela, mas desfazia a alteração (rollback) se o request de rede falhasse, impedindo a triagem desconectada.

Esta especificação define o padrão estrito de **Offline-First com Resposta Imediata (0ms)** para todas as ações unitárias do `MessageView`.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Exibir um spinner/carregamento durante a exclusão:**
  - *Motivo do descarte:* Viola a premissa de um cliente autônomo e ágil. O usuário quer triar a caixa de correio rapidamente; esperar confirmação do servidor a cada exclusão destrói a usabilidade e inviabiliza o uso em trânsito ou conexões instáveis.
- **Alternativa Descartada 2: Rollback destrutivo em caso de erro de rede:**
  - *Motivo do descarte:* Em clientes de email modernos e no modelo do Lauda, a intenção do usuário é local e primária. A falha de rede deve resultar em persistência na fila de sincronização (`ActionsQueue`), não na reversão da ação do usuário.
- **Escolha Adotada (Otimismo Instantâneo + Fila Resiliente):**
  1. **UI Reativa (0ms):** Ao clicar em "🗑 EXCLUIR" ou "📦 ARQUIVAR":
     - Remover imediatamente a mensagem da lista (`messages.write().retain(|m| m.uid != uid)`).
     - Limpar `active_message.set(None)`.
     - Transicionar imediatamente para `Screen::Inbox` (ou mensagem vizinha na navegação sequencial se aplicável).
     - Se houver busca ativa, atualizar também `search_envelopes`.
  2. **Persistência Local Imediata:** Remover/marcar nos caches locais (`LocalMessageCache` no disco e IndexedDB no WASM).
  3. **Enfileiramento na `ActionsQueue`:** Registrar a ação (`ActionType::Delete`, `ActionType::Move { dest_box: "Archive" }`, `ActionType::MarkFlagged`) na fila persistente de reconciliação.
  4. **Tentativa Oportunista de Sincronização em Background:** Disparar a chamada de rede em tarefa desvinculada (`spawn`/`spawn_local`). Se tiver sucesso, remover da fila; se falhar ou estiver offline, a ação permanece na fila para sincronização eventual.

---

## 3. Contratos de Domínio e Comportamento

### 3.1. Exclusão (`handle_delete`)
```rust
// 1. Atualização otimista no estado da UI
messages.write().retain(|m| m.uid != uid);
if let Some(list) = search_envelopes.write().as_mut() {
    list.retain(|m| m.uid != uid);
}
active_message.set(None);
screen.set(Screen::Inbox);

// 2. Persistência local e enfileiramento offline (Desktop e WASM)
// Desktop: cache.delete_message(&box_name, uid) + actions.enqueue(...)
// WASM: LocalMessageCache::delete_message_idb(&box_name, uid) + LocalMessageCache::enqueue_action_idb(...)
```

### 3.2. Arquivamento (`handle_archive`)
- Semelhante à exclusão na UI atual: remove da caixa ativa atual, atualiza contadores e transiciona para `Screen::Inbox`.
- Enfileira `ActionType::Move { dest_box: "Archive".to_string() }`.

### 3.3. Destaque / Estrela (`handle_toggle_flag`)
- Atualiza `envelope.is_flagged = new_flagged` imediatamente.
- Enfileira `ActionType::MarkFlagged { flagged: new_flagged }` sem rollback em caso de falha de conexão.

---

## 4. Critérios de Aceitação
1. Clicar em "🗑 EXCLUIR" em uma mensagem no `MessageView` altera a tela para `Screen::Inbox` imediatamente, sem aguardar resposta HTTP.
2. A mensagem excluída não consta mais na lista `messages` nem nos resultados de `search_envelopes`.
3. Uma ação `ActionType::Delete` é registrada na `ActionsQueue` correspondente.
4. Clicar em "📦 ARQUIVAR" remove a mensagem e retorna à Inbox imediatamente, gerando a ação na fila.
5. Em cenário desconectado / falha de rede simulada, as ações permanecem enfileiradas e a UI não trava nem reverte.

---

## 5. Laudo de Fechamento e Homologação
- **Status:** Concluído e Aprovado no Quality Gate (Fase 8).
- **Implementação:** Transição de tela instantânea (0ms), remoção otimista em `apply_optimistic_removal` e enfileiramento em `ActionsQueue`/IDB sem rollback em `message_view.rs`.
- **Validação:** 22/22 testes aprovados em `client_test.rs`, clippy e fmt validados sem alertas.
- **Data:** 2026-11-24

