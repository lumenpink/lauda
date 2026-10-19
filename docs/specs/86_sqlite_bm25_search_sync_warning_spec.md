# Especificação Técnica: Busca BM25 Direta no SQLite e Aviso de Indexação Incompleta

**ID do Ticket:** #86  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificação (Fase 3: Doc/Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O motor de indexação e busca do Lauda foi projetado para oferecer busca vetorial e híbrida (BM25 + embeddings). Em caixas postais reais com dezenas de milhares de mensagens (ex: mais de 56.000 e-mails), a implementação anterior apresentava três gargalos críticos:
1. **Bloqueio Síncrono no Request:** O endpoint de busca acionava a reconciliação e indexação sequencial de todas as mensagens pendentes no caminho crítico da requisição HTTP, travando o worker por minutos/horas.
2. **Reconstrução Ineficiente em Memória:** A cada busca, o servidor realizava `SELECT` em todos os documentos da caixa, desserializava dicionários de termos e reconstruía a estrutura `MessageIndex` na RAM, em vez de aproveitar os índices invertidos relacionais já persistidos no SQLite.
3. **Falta de Transparência de Sincronização:** Quando o acervo ainda está sendo indexado em segundo plano, o usuário recebia resultados vazios ou parciais sem saber que mensagens antigas ou recém-chegadas ainda não haviam sido processadas.

O objetivo deste ticket é viabilizar a busca BM25 ultrarrápida direta via SQL no SQLite (< 10ms) sobre o acervo já indexado, desacoplar a indexação do request e expor o estado de sincronização da base (`MailboxSyncStatus`), alertando o usuário no topo dos resultados quando a indexação ainda estiver em andamento.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Modelos de Domínio (`lauda-types`)
- **`MailboxSyncStatus`**:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
  pub struct MailboxSyncStatus {
      pub is_synced: bool,
      pub indexed_count: u64,
      pub total_count: u64,
      pub remaining_count: u64,
  }
  ```
- **`SearchResult`**:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
  pub struct SearchResult {
      pub messages: Vec<MessageEnvelope>,
      pub sync_status: MailboxSyncStatus,
  }
  ```

### 2.2. Acesso a Dados no SQLite (`SqliteIndexStore` em `lauda-server`)
- **`get_sync_status(mailbox: &str, total_count: u64) -> Result<MailboxSyncStatus, rusqlite::Error>`**:
  - Consulta `SELECT count(*) FROM indexed_documents WHERE mailbox = ?1` em tempo sub-milissegundo.
  - Compara `indexed_count` com `total_count`.
  - Define `is_synced = (indexed_count >= total_count)` e calcula `remaining_count = total_count.saturating_sub(indexed_count)`.
- **`search_bm25(mailbox: &str, query: &str, limit: usize) -> Result<Vec<MessageEnvelope>, rusqlite::Error>`**:
  - Tokeniza os termos da consulta em lowercase/alphanumeric.
  - Executa consulta SQL agregada direta:
    ```sql
    SELECT d.uid, d.subject, d.sender, d.date, d.is_read, d.is_flagged, d.has_attachments,
           SUM(i.tf) as score
    FROM inverted_index i
    JOIN indexed_documents d ON d.mailbox = i.mailbox AND d.uid = i.uid
    WHERE i.mailbox = ?1 AND i.term IN (...)
    GROUP BY d.uid
    ORDER BY score DESC
    LIMIT ?;
    ```
  - Mapeia as linhas para `Vec<MessageEnvelope>` instantaneamente.

### 2.3. Desacoplamento da Indexação e Endpoint HTTP
- O endpoint `search_messages` não bloqueia aguardando a indexação de milhares de mensagens pendentes. Ele responde imediatamente com os e-mails já indexados e o status de sincronização.
- Resposta HTTP com JSON `SearchResult` e cabeçalhos informativos:
  - `X-Lauda-Index-Synced: true/false`
  - `X-Lauda-Index-Indexed: <count>`
  - `X-Lauda-Index-Total: <count>`
  - `X-Lauda-Index-Remaining: <count>`

### 2.4. Interface com o Usuário (Web `lauda-client` e TUI `lauda-tui`)
- **Web (`InboxView`)**:
  - Quando em modo de busca e `!sync_status.is_synced`, exibe no topo da lista de mensagens um aviso visual Trash Polka:
    `⚠️ ÍNDICE EM SINCRONIZAÇÃO: {indexed_count} de {total_count} e-mails indexados (faltam {remaining_count}). A busca cobriu apenas o acervo já indexado; é possível que o e-mail procurado ainda não tenha entrado no índice.`
  - Quando `is_synced == true`, o aviso é omitido.
- **TUI (`lauda-tui`)**:
  - Em modo de busca, se o status for incompleto, exibe banner no cabeçalho da listagem informando a contagem de indexados vs pendentes.

---

## 3. Critérios de Aceite

1. `SqliteIndexStore::search_bm25` realiza busca direta no SQLite em menos de 50ms para dezenas de milhares de termos.
2. `SqliteIndexStore::get_sync_status` calcula com precisão `indexed_count`, `total_count` e `remaining_count`.
3. Requisições de busca não travam mesmo quando milhares de e-mails aguardam indexação na caixa postal.
4. O payload e/ou cabeçalhos HTTP de busca fornecem `MailboxSyncStatus` com transparência total.
5. A UI Web e a TUI informam logo no início da resposta de busca o estado da sincronização caso incompleta.
6. A suíte de testes unitários e de integração cobre cenários com base sincronizada e não sincronizada.
7. Validação estrita de Quality Gate Gosplan (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 4. Registro de Fases DD-TDD

- [x] **Fase 1 (Ticket):** Ticket #86 formalizado e sincronizado.
- [x] **Fase 2 (Branch):** Branch `feat/86-sqlite-bm25-search-sync-warning` criada a partir da `main`.
- [x] **Fase 3 (Spec):** Especificação técnica formalizada neste documento.
- [ ] **Fase 4 (Red Test):** Testes unitários falhos para busca BM25 direta no SQLite e cálculo de `sync_status`.
- [ ] **Fase 5 (Green Implement):** Implementação da busca SQL BM25, cálculo de sincronização, desacoplamento e exibição na UI.
- [ ] **Fase 6 (Refactor):** Código limpo, modular, com zero warnings e documentação integrada.
- [ ] **Fase 7 (Mutation Test):** Verificação de robustez contra mutantes de pontuação, cálculo residual e ordenação.
- [ ] **Fase 8 (Quality Gate):** Validação com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e testes direcionados.
- [ ] **Fase 9 (Finish & Close):** Commit de encerramento (`closes #86`), merge e deploy.
