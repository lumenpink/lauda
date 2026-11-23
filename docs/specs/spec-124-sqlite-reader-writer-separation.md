# Especificação Técnica #124: Separação de Conexões de Leitura e Escrita no SQLite Index Store

## 1. Contexto e Problema
Durante a operação com caixas de correio reais de grande porte (~59.000 mensagens, banco SQLite de 2.6 GB e índice invertido com mais de 16 milhões de registros), a rota de busca `/messages/search` enfrentou travamentos de quase 1 hora sem responder à usuária.

A investigação do processo revelou:
1. O worker de background de indexação contínua (`start_continuous_indexer` e watcher inotify) processa lotes de e-mails novos ou pendentes, chamando `store.save_document()`.
2. Para cada mensagem, `save_document` adquire o lock de `self.conn: Arc<Mutex<Connection>>`.
3. Como o `SqliteIndexStore` compartilhava uma única conexão SQLite entre leituras e escritas, a requisição HTTP da busca (que chama `store.get_sync_status` e `store.search_hybrid`) ficava bloqueada esperando o mutex da conexão (`futex_wait_queue`), enquanto o indexador iterava mensagens e centenas de termos.
4. Embora o SQLite estivesse em modo WAL (`PRAGMA journal_mode = WAL`), a vantagem nativa do WAL (múltiplos leitores concorrentes em paralelo com um escritor) foi completamente anulada pelo encapsulamento em um único `Mutex<Connection>`.

## 2. Árvore de Decisão e Alternativas Descartadas (Para a Natasha)

### Alternativa A: Manter mutex único e diminuir granularidade do lock
- *Ideia:* Adquirir e soltar o mutex para cada statement isolado dentro do indexador.
- *Por que foi descartada:* Em indexação massiva (ex: 160 mensagens com 500 termos cada = 80.000 inserts), a disputa constante pelo mutex gera "thrashing" e não impede que a busca espere na fila de lock. Além disso, inserts individuais sem transação são ordens de magnitude mais lentos no SQLite.

### Alternativa B (Escolhida): Conexões Separadas de Leitura e Escrita
- *Ideia:* Abrir conexões SQLite distintas no mesmo arquivo de banco:
  - `write_conn: Arc<Mutex<Connection>>`: Utilizada exclusivamente por operações de mutação (`save_document`, `remove_document`, `clear_mailbox_index`, `init_schema`).
  - `read_conn: Arc<Mutex<Connection>>`: Utilizada exclusivamente por operações de consulta (`search_bm25`, `search_hybrid`, `get_sync_status`, `get_indexed_uids`, `load_mailbox_index`, `get_recent_slice`).
- *Por que foi escolhida:* O motor WAL do SQLite garante que leitores nunca bloqueiem escritores e escritores nunca bloqueiem leitores em conexões diferentes. Buscas e queries de sync continuam respondendo em milissegundos mesmo com indexação massiva ativa.

### Otimização Adicional em `save_document`: Transação Explícita
- Atualmente, `save_document` executa múltiplos statements (`INSERT` de documento, `DELETE` de termos anteriores e loop de `INSERT` de termos) em autocommit. Envolver essas operações em uma transação explícita reduz o tempo de escrita em até 50x e minimiza o tempo em que o lock de escrita é retido.

### Timeout de Segurança na Camada HTTP
- Configurar timeout de 10 segundos na rota de busca para garantir que falhas externas ou travamentos nunca deixem a conexão HTTP pendurada indefinidamente.

## 3. Arquitetura Proposta

```
                     ┌──────────────────────────────────┐
                     │         MaildirBackend           │
                     └───────────────┬──────────────────┘
                                     │
                    ┌────────────────┴────────────────┐
                    │        SqliteIndexStore         │
                    │  (PathBuf, FlatVectorIndex RAM) │
                    └───────┬──────────────────┬──────┘
                            │                  │
               ┌────────────▼─────────┐      ┌─▼──────────────────┐
               │      write_conn      │      │     read_conn      │
               │ Arc<Mutex<Connection>>│     │Arc<Mutex<Connection>>│
               └───────────┬──────────┘      └─────────┬──────────┘
                           │                           │
                   (INSERT/UPDATE/DELETE)          (SELECT)
                           │                           │
                           └───────────┬───────────────┘
                                       │
                           ┌───────────▼───────────┐
                           │   SQLite DB (WAL)     │
                           └───────────────────────┘
```

## 4. Plano de Implementação (GOST DD-TDD)

- **Fase 3 (Spec):** Esta especificação.
- **Fase 4 (Red Test):** Teste de concorrência em `lauda-server/tests/sqlite_concurrency_test.rs`:
  - Thread 1 (escritora contínua): executa um loop de `save_document` simulando indexador pesado.
  - Thread 2 (leitora): executa `search_hybrid` simultaneamente.
  - Verificação: A busca deve ter sucesso e retornar em tempo < 100ms, sem deadlocks.
- **Fase 5 (Green Implement):**
  - Refatorar `SqliteIndexStore` para instanciar `write_conn` e `read_conn`.
  - Configurar `PRAGMA busy_timeout = 5000;` em ambas e `PRAGMA query_only = ON;` em `read_conn`.
  - Transacionar a inserção de termos em `save_document`.
- **Fase 6 (Refactor):**
  - Ajustar escopo dos locks de leitura em `search_hybrid_with_provider` para não reter a conexão durante KNN e RRF em RAM.
- **Fase 7 (Mutation Test):**
  - Validar que buscas e leituras isoladas continuam operacionais mesmo com banco em somente leitura.
- **Fase 8 (Quality Gate):**
  - `cargo fmt --check`
  - `cargo clippy --all-targets -p lauda-server -- -D warnings`
  - `cargo test -p lauda-server`
- **Fase 9 (Finish & Close):**
  - Fechar ticket, mergear e atualizar o container `lauda-server`.
