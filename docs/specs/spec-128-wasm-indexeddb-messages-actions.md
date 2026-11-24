# Especificação Técnica — Ticket #128: Persistência de Mensagens e Fila de Ações no IndexedDB no WebAssembly

## 1. Contexto & Diagnóstico

No cliente Web (`target_arch = "wasm32"`), a persistência do corpo completo dos e-mails (`save_message` / `load_message`) estava sendo realizada de forma síncrona no `window.local_storage()`. 

O `localStorage` possui limitações críticas no ecossistema web:
1. **Teto de Quota Rígido (5 MB):** Ao atingir 5 MB de dados serializados (envelopes, fatias de índice e corpos), o navegador bloqueia qualquer nova gravação com `QuotaExceededError`. Como a escrita ignorava o erro (`let _ = storage.set_item(...)`), novos e-mails deixavam de ser armazenados silenciosamente.
2. **Latência no Leitor:** Toda mensagem não encontrada no cache do `localStorage` forçava uma requisição HTTP de rede (`client.get_message(...)`), gerando um atraso perceptível de centenas de milissegundos para abrir o e-mail.
3. **Inexistência de Fila Offline no Browser:** A estrutura `ActionsQueue` (responsável pelo enfileiramento resiliente de marcação de lido, favoritos e exclusão) operava apenas em disco sob `#[cfg(not(target_arch = "wasm32"))]`. No WASM, as ações eram descartadas sem envio ao servidor, impedindo que mensagens abertas na Web fossem marcadas como lidas de forma persistente.

## 2. Decisões Arquiteturais & Contratos

### 2.1. Evolução do Esquema IndexedDB (`lauda_messages` v2)
O banco IndexedDB gerenciado via biblioteca `rexie` será elevado para a versão `2`, incorporando duas novas object stores dedicadas:
- `"messages"` (KeyPath: `"key"`, no formato `"{mailbox}/{uid}"`): Armazena registros completos contendo `key`, `mailbox`, `uid` e `message_json`.
- `"actions"` (KeyPath: `"id"`): Armazena as ações da fila offline no formato `QueuedAction` serializado.

Constantes canônicas expostas em `LocalMessageCache`:
- `IDB_DATABASE_NAME = "lauda_messages"`
- `IDB_STORE_MAILBOXES = "mailboxes"`
- `IDB_STORE_ENVELOPES = "envelopes"`
- `IDB_STORE_INDEX_SLICES = "index_slices"`
- `IDB_STORE_MESSAGES = "messages"`
- `IDB_STORE_ACTIONS = "actions"`

### 2.2. Abertura Instantânea de Mensagens (0 ms)
- Ao abrir uma mensagem na Web:
  1. O leitor verifica o IndexedDB via `load_message_idb(mailbox, uid)`. Se presente, entrega imediatamente ao estado ativo (`active_message`).
  2. Caso contrário, faz o fetch HTTP de rede e, ao receber a resposta, persiste no IndexedDB em background via `save_message_idb`.
  3. Mensagens visualizadas uma vez abrem instantaneamente em acessos subsequentes ou offline.

### 2.3. Fila Offline-First de Ações no Browser
- Quando uma mensagem é lida, favoritada ou excluída no browser:
  1. A UI reflete imediatamente o novo estado (otimista).
  2. A ação é registrada na store `"actions"` do IndexedDB.
  3. Se o cliente estiver conectado (`is_online`), o despachador executa imediatamente a chamada de API no servidor e remove a ação concluída.
  4. Caso o cliente esteja offline ou a rede oscile, as ações permanecem no IndexedDB e são drenadas pelo ciclo SWR periódico do `ui/mod.rs` assim que a conectividade for restabelecida.

---

## 3. Critérios de Aceitação (GOST DD-TDD)

- [ ] **Fase 4 (Red Test):** Testes de contrato automatizados comprovando:
  1. Existência das constantes canônicas de stores `"messages"` e `"actions"` no `LocalMessageCache`.
  2. Contrato de serialização e desserialização de `MessageRecord` e `QueuedAction` para o IndexedDB.
- [ ] **Fase 5 (Green Implement):** 
  1. Atualização do `open_rexie` para versão 2 com stores `"messages"` e `"actions"`.
  2. Implementação de `save_message_idb` e `load_message_idb`.
  3. Implementação de `enqueue_action_idb`, `list_actions_idb` e `remove_action_idb`.
  4. Integração na abertura de mensagens (`inbox_view.rs`) e no ciclo SWR do WASM (`ui/mod.rs`).
- [ ] **Fase 6 (Refactor):** Limpeza de avisos, formatação canônica e documentação explícita para Natasha.
- [ ] **Fase 7 (Mutation Test):** Testes cobrindo mutações de chaves compostas, idempotência de drenagem de fila e tolerância a payloads corrompidos.
- [ ] **Fase 8 (Quality Gate):** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e suite de testes aprovados.
- [ ] **Fase 9 (Close):** Fechamento do ticket via `cccp close` e integração na `main`.
