# Especificação Técnica #135 — Seleção Múltipla com Long-press, Shift e Barra Contextual no InboxView

## 1. Contexto e Motivação
A triagem de e-mails em massa é uma operação essencial no cliente Lauda (`lauda-client`). Atualmente, o usuário só consegue interagir com uma mensagem por vez abrindo o `MessageView`.

Esta especificação define o comportamento de **seleção múltipla na lista de mensagens (`InboxView`)**:
1. Ativação via toque longo (*long-press*, ~400ms em telas touch/mobile), clique com `Shift` (intervalo contíguo no desktop), clique no marcador de status de leitura/seleção, ou atalhos de teclado (`Ctrl+A`, `x`).
2. Comportamento contextual: enquanto houver ao menos uma mensagem selecionada (`!selected_uids.is_empty()`), qualquer clique simples nos itens da lista alterna a seleção em vez de abrir o leitor.
3. Barra de Ações Contextual (CAB) no topo: substitui a barra de busca enquanto a seleção estiver ativa, exibindo contador de selecionadas, botão para desmarcar tudo (`✕`), botão para selecionar todas as visíveis, e as ações em lote (Excluir, Arquivar, Lida/Não Lida, Favoritar).
4. Limpeza automática de seleção na alternância de caixas postais (`current_mailbox`).

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Exibir caixas de seleção (checkboxes) permanentes em todas as linhas:**
  - *Motivo do descarte:* Consome largura horizontal preciosa em telas mobile e sobrecarrega visualmente a lista. A ativação via toque longo ou clique direto no indicador de leitura/seleção torna o fluxo limpo e natural.
- **Alternativa Descartada 2: Barra de ações fixa no rodapé:**
  - *Motivo do descarte:* Em dispositivos móveis e navegadores, barras de rodapé frequentemente colidem com barras de navegação do sistema, menus virtuais ou o teclado. Substituir a barra de busca superior (que não está em uso durante triagem em massa) oferece ergonomia consistente e previsível (padrão CAB).
- **Alternativa Descartada 3: Seleção contígua baseada em UIDs numéricos brutos:**
  - *Motivo do descarte:* A lista visível pode estar filtrada (por exemplo, busca ativa, filtro de "Não lidos" ou ordenação). O intervalo de seleção contígua com Shift deve respeitar estritamente a sequência dos itens visíveis no vetor `filtered_messages`.
- **Escolha Adotada (Funções Puras no Domínio + Sinais Reativos):**
  - Isolar toda a lógica de manipulação de conjuntos (`HashSet<u32>`) e intervalos em funções puras em `domain/view.rs`, permitindo testes de unidade e mutação 100% determinísticos.
  - Sinais de estado no `InboxView`: `selected_uids: Signal<HashSet<u32>>` e `last_interacted_uid: Signal<Option<u32>>`.

---

## 3. Contratos de Domínio e Assinaturas (`domain/view.rs`)

```rust
use std::collections::HashSet;
use lauda_types::MessageEnvelope;

/// Alterna a presença de um UID no conjunto de selecionados.
/// Retorna true se o UID passou a estar selecionado, ou false se foi removido.
pub fn toggle_selection(selected: &mut HashSet<u32>, uid: u32) -> bool;

/// Seleciona o intervalo contíguo entre from_uid e to_uid baseado na ordem de exibição em `messages`.
/// Se from_uid ou to_uid não existirem na lista, seleciona apenas to_uid se ele existir.
pub fn select_range(
    messages: &[MessageEnvelope],
    from_uid: u32,
    to_uid: u32,
    selected: &mut HashSet<u32>,
);

/// Seleciona todas as mensagens visíveis da lista fornecida.
pub fn select_all(messages: &[MessageEnvelope], selected: &mut HashSet<u32>);

/// Limpa todas as mensagens selecionadas.
pub fn clear_selection(selected: &mut HashSet<u32>);
```

---

## 4. Comportamento no `InboxView`
1. **Modo Normal (`selected_uids.is_empty()`):**
   - Barra de busca visível normalmente no topo.
   - Clique simples na mensagem abre o leitor (`Screen::MessageDetail(uid)`).
   - Clique no círculo de status ou clique com `Shift` seleciona a mensagem e ativa o modo de seleção.
   - Toque longo (400ms sem arrastar) seleciona a mensagem e ativa o modo de seleção.
2. **Modo de Seleção Ativo (`!selected_uids.is_empty()`):**
   - Barra de busca é ocultada e substituída pela **Barra de Ações Contextual (CAB)**.
   - Clique em qualquer parte do card da mensagem alterna seu estado de seleção (`toggle_selection`).
   - Clique com tecla `Shift` pressionada aciona `select_range` a partir do `last_interacted_uid`.
   - Clique no botão `✕` da CAB ou tecla `Escape` limpa a seleção (`clear_selection`) e restaura a barra de busca.
   - Botão `[TODAS]` na CAB seleciona todas as mensagens de `filtered_messages`.
3. **Reset de Estado:**
   - Ao trocar de pasta (`current_mailbox`), o conjunto `selected_uids` é esvaziado.

---

## 5. Critérios de Aceitação
1. Funções puras de seleção cobrem alternância, limites de intervalo direto/invertido, UIDs inexistentes e listas vazias.
2. No `InboxView`, quando `selected_uids` não está vazio, a barra de busca dá lugar à barra contextual com contador e ações.
3. Clicar no botão `✕` da barra contextual limpa todas as seleções.
4. Clique com Shift seleciona todos os itens no intervalo visível entre o último clicado e o atual.
5. A troca de caixa limpa a seleção ativa.

---

## 6. Laudo de Fechamento e Homologação
- **Status:** Concluído e Aprovado no Quality Gate (Fase 8).
- **Implementação:**
  - Funções puras em `domain/view.rs`: `toggle_selection`, `select_range`, `select_all` e `clear_selection`.
  - Reatividade no `InboxView`: `selected_uids: Signal<HashSet<u32>>` e `last_interacted_uid: Signal<Option<u32>>`.
  - Substituição da barra de busca pela Barra de Ações Contextual (CAB) com contador de mensagens selecionadas, ações em lote e botão de limpar seleção `✕`.
  - Suporte a seleção por clique no marcador de status, clique com tecla `Shift` para intervalo contíguo, toque longo via `oncontextmenu`, e clique simples quando em modo de seleção ativa.
  - Reset de seleção na troca de pasta/caixa postal (`current_mailbox`).
- **Validação:** 24/24 testes aprovados em `client_test.rs`, `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` sem alertas.
- **Data:** 2026-11-24
- **Fechamento:** Fase 9 concluída com sucesso.


