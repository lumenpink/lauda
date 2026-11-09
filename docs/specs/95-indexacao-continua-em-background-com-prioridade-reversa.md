# Especificação Técnica — Ticket #95
## Indexação Contínua em Background com Prioridade Reversa

- **Ticket:** #95
- **Branch:** `feat/95-indexacao-continua-em-background-com-prioridade-reversa`
- **Data:** 2026-11-09
- **Status:** CONCLUÍDO (Fase 9)

---

### 1. Contexto e Motivação
No `lauda-server`, a base SQLite de vetores e busca invertida (`.lauda_index.db`) é fundamental para alimentar a busca inteligente semântica e híbrida.
Entretanto, o comportamento anterior apresentava lacunas críticas:
1. **Indexação puramente sob demanda (lazy):** Se nenhuma busca for executada, nenhum e-mail é indexado pelo servidor após o boot.
2. **Perda de trabalho em reinicializações:** O worker em lote era um `tokio::spawn` isolado disparado dentro de `search_messages`. Ao reiniciar o contêiner, a tarefa morria.
3. **Ordem cronológica crescente:** Ao reconciliar os UIDs faltantes, a ordenação crescente (`to_index.sort()`) forçava a indexação a partir do UID 1 (e-mails de anos atrás), deixando os e-mails mais recentes desprovidos de busca por horas.
4. **Falta de sincronização contínua com a Maildir:** Chegadas de novos e-mails dependiam de buscas futuras para que a reconciliação os encontrasse.

---

### 2. Requisitos Técnicos

#### 2.1 Priorização Reversa de UID
- Em `lauda-server/src/adapters/sqlite_index.rs` e `maildir.rs`:
  - A ordenação de `to_index` deve ser invertida: `to_index.sort_by(|a, b| b.cmp(a))`.
  - As mensagens mais recentes (maiores UIDs) devem ser processadas imediatamente no topo da fila.

#### 2.2 Daemon de Indexação Contínua no MaildirBackend
- Em `lauda-server/src/adapters/maildir.rs`:
  - Adicionar controle atômico de concorrência (`indexing_active: Arc<AtomicBool>`) para impedir workers concorrentes duplicados.
  - Implementar método público `start_continuous_indexer(self: Arc<Self>, interval_secs: u64)`:
    - Inicia loop assíncrono em segundo plano (`tokio::spawn`).
    - Varre todas as mailboxes conhecidas (ou a pasta configurada, default `INBOX`).
    - Reconcilia UIDs com a Maildir via `reconcile_mailbox`.
    - Se houver `to_remove`, expurga imediatamente.
    - Se houver `to_index`, consome em lotes de 25-50 mensagens com prioridade decrescente e pausa cooperativa de 10-20ms entre lotes.
    - Quando todas as mensagens estiverem indexadas (`to_index.is_empty()`), dorme pelo intervalo configurado (`interval_secs`, ex: 15s) antes do próximo ciclo de reconciliação.
- No `main.rs`:
  - Disparar `backend.start_continuous_indexer(15)` logo após a inicialização do `MaildirBackend`.

#### 2.3 Expurgo Imediato em Operações de Escrita
- Em `delete_message` e `move_message`:
  - Se `sqlite_index` estiver presente, invocar `store.remove_document(mailbox, uid)` imediatamente após a exclusão/movimentação física do arquivo.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação técnica.
- **Fase 4 (Red Test):** Testes unitários e de integração em `lauda-server/tests/continuous_indexing_test.rs` cobrindo prioridade reversa de UID e execução do indexador contínuo.
- **Fase 5 (Green Implement):** Implementação do método `start_continuous_indexer`, ajuste da ordenação reversa e expurgo imediato.
- **Fase 6 (Refactor & Mutation):** Fortificação contra mutantes e limpeza idiomática.
- **Fase 7 (Style & Changelog):** Formatação com `cargo fmt`, clippy e atualização de documentação.
- **Fase 8 (Quality Gate):** Verificação rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Fechamento do ticket e commit final.
