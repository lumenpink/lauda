# Especificação Técnica - Ticket #123
## Desacoplamento da Reconciliação Inline da Rota de Busca (`lauda-server`)

### Contexto & Motivação
Durante testes com a busca de e-mails em produção, a rota `/messages/search` ficou travada por mais de 25 minutos. A inspeção de threads (`/proc/<pid>/task/*/wchan`) revelou threads bloqueados em `futex_wait_queue`.
A causa raiz identificada foi:
1. `search_messages` em `lauda-server/src/adapters/maildir.rs` chamava `reconcile_mailbox` de forma síncrona na mesma thread da requisição HTTP, indexando documentos inline (`save_document`).
2. O `SqliteIndexStore` utilizava uma única conexão protegida por `Arc<Mutex<Connection>>`.
3. Quando a rotina de background `start_continuous_indexer` ou o watcher `inotify` estavam processando documentos em lotes, o lock do `Mutex` permanecia retido, impedindo que requisições HTTP de leitura adquirissem a conexão.

### Árvore de Decisão
- **Alternativa A (Descartada):** Manter a indexação síncrona em `search_messages` e diminuir o timeout do lock.
  - *Motivo do descarte:* Resultaria em erros 500 frequentes ou respostas vazias indevidas. Natasha não aprovaria tratar sintoma em vez da falha de design.
- **Alternativa B (Escolhida):**
  1. Tornar a rota de busca `search_messages` puramente voltada à leitura do índice já persistido, removendo qualquer reconciliação ou escrita síncrona durante o atendimento HTTP.
  2. Delegar a atualização do índice exclusivamente ao worker assíncrono de background e ao watcher inotify.
  3. No `SqliteIndexStore`, garantir que a busca direta não dispute locks de escrita ou retenha mutexes além do estritamente necessário.

### Requisitos Técnicos
1. `MaildirBackend::search_messages` não deve executar `reconcile_mailbox` nem chamadas a `save_document`.
2. A rota deve retornar o `sync_status` e a lista de mensagens correspondentes imediatamente.
3. Testes unitários e de integração devem garantir que `search_messages` retorne com êxito sem disparar reconciliação.
