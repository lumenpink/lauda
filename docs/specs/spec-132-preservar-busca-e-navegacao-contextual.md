# Especificação Técnica #132 — Preservação de Busca e Navegação Contextual

## 1. Contexto e Motivação
No smolclient Lauda (WebAssembly, Desktop e Android APK), o usuário realiza buscas por mensagens na caixa ativa. Ao clicar em um resultado de busca para ler o email (`Screen::MessageDetail(uid)`):
1. O componente `InboxView` era destruído, perdendo o estado local de `search_query` e os resultados filtrados. Ao voltar para a lista (via botão `← VOLTAR`, tecla `u` ou gesto do sistema), a aplicação exibia a caixa de entrada completa sem nenhum filtro de busca ativo.
2. A navegação de mensagens subsequente (`◀ ANTERIOR`, `PRÓXIMO ▶`, teclas `j` e `k`) no `MessageView` consultava exclusivamente `messages.read()` (a caixa inteira), ignorando o subconjunto ordenado/filtrado da busca em vigor.

Esta especificação define o gerenciamento de estado compartilhado e os contratos de navegação contextual para que a busca seja preservada entre transições de tela e governe a sequência de leitura.

---

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)
- **Alternativa Descartada 1: Persistência em `localStorage` / `IndexedDB` para a busca:**
  - *Motivo do descarte:* A busca é um estado volátil da sessão de navegação da UI. Gravar no armazenamento local geraria overhead desnecessário de I/O assíncrono e exigiria lógica extra para expirar buscas antigas entre logins ou sessões.
- **Alternativa Descartada 2: Context Provider global complexo:**
  - *Motivo do descarte:* Dioxus Signals permitem elevar o estado diretamente ao componente raiz `App` (`search_query: Signal<String>` e `search_envelopes: Signal<Option<Vec<MessageEnvelope>>>`), mantendo a árvore de dados explícita, reativa e tipada sem introduzir camadas extras de abstração.
- **Escolha Adotada:**
  - Elevar `search_query` e `search_envelopes` ao componente `App`.
  - Passar esses signals para `InboxView` (que os alimenta durante a digitação/busca) e `MessageView` (que utiliza `search_envelopes` como sequência candidata preferencial para `find_neighbor_uids`).
  - Resetar `search_query` e `search_envelopes` somente quando:
    - O usuário clicar em "✕ LIMPAR" ou esvaziar o input de busca.
    - O usuário alternar de caixa de correio (ex.: de INBOX para Sent).

---

## 3. Contratos de Domínio e Comportamento

### 3.1. Resolução de Envelopes Candidatos na Navegação (`resolve_navigation_envelopes`)
Função pura no domínio de visualização (`lauda-client::domain::view`):
```rust
pub fn resolve_navigation_envelopes<'a>(
    search_envelopes: Option<&'a [MessageEnvelope]>,
    mailbox_envelopes: &'a [MessageEnvelope],
) -> &'a [MessageEnvelope]
```
- Se `search_envelopes` for `Some(list)` e a busca estiver ativa, retorna a fatia da busca.
- Caso contrário (`None`), retorna `mailbox_envelopes`.

### 3.2. Preservação no `InboxView`
- O `use_resource` de busca e o valor do `<input>` vinculam-se ao `search_query` compartilhado.
- Quando o componente é desmontado e remontado na transição `MessageDetail -> Inbox`, o valor prévio de `search_query` é retido, reativando a exibição dos resultados locais e remotos instantaneamente.

### 3.3. Transição de Caixa
- Se `current_mailbox` mudar, `search_query.set(String::new())` e `search_envelopes.set(None)` são acionados para evitar que uma busca da caixa anterior filtre incorretamente a nova caixa selecionada.

---

## 4. Critérios de Aceitação
1. `resolve_navigation_envelopes` retorna a lista de busca quando `Some(list)` é fornecido e `mailbox_envelopes` quando `None`.
2. `find_neighbor_uids` calcula corretamente os vizinhos dentro do subconjunto da busca.
3. No `MessageView`, os botões `◀ ANTERIOR`, `PRÓXIMO ▶` e atalhos `j`/`k` pulam diretamente para os itens da busca ativa.
4. Ao navegar de volta de `MessageView` para `InboxView`, o input e os resultados da busca permanecem intactos.
5. Ao limpar a busca no `InboxView`, `search_envelopes` volta para `None`.
