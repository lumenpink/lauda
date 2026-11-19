# Especificação Técnica - Ticket #113
## Otimização da Busca Invertida SQLite (`lauda-server`)

### Contexto & Motivação
No `lauda-server`, a recuperação de candidatos de mensagens para ranqueamento BM25 em `SqliteIndexStore::search_hybrid` executava um `JOIN` entre `inverted_index i` e `indexed_documents d`:
```sql
SELECT d.uid
FROM inverted_index i
JOIN indexed_documents d ON d.mailbox = i.mailbox AND d.uid = i.uid
WHERE i.mailbox = ?1 AND i.term IN (...)
GROUP BY d.uid
ORDER BY SUM(i.tf) DESC, d.uid DESC
LIMIT 50
```
Com bases reais contendo dezenas de milhares de mensagens (ex.: 58.898 mensagens na INBOX), o planejador de queries do SQLite opta por iniciar a varredura pela tabela `indexed_documents` usando o índice `idx_indexed_docs_box_uid (mailbox=?)`. Isso força o SQLite a escanear dezenas de milhares de entradas de documentos e realizar um lookup pontual em `inverted_index` para cada documento.

Como a tabela `inverted_index` já é modelada com as colunas `(term, mailbox, uid, tf)` e indexada com chave primária composta `(term, mailbox, uid)`, todas as informações necessárias para a busca (`uid` e `tf`) já residem diretamente em `inverted_index`.

### Árvore de Decisão
- **Alternativa A (Descartada):** Manter o `JOIN` e forçar index hints via `INDEXED BY idx_inverted_term_box`.
  - *Motivo do descarte:* Index hints no SQLite são frágeis e quebram se os índices forem renomeados ou se Natasha precisar reestruturar migrações no futuro. Além disso, o `JOIN` não projeta nenhuma coluna exclusiva de `indexed_documents`, sendo conceitualmente desnecessário.
- **Alternativa B (Escolhida):** Remover completamente o `JOIN` com `indexed_documents` e consultar diretamente `inverted_index`:
  ```sql
  SELECT i.uid
  FROM inverted_index i
  WHERE i.mailbox = ?1 AND i.term IN (...)
  GROUP BY i.uid
  ORDER BY SUM(i.tf) DESC, i.uid DESC
  LIMIT 50
  ```
  - *Vantagens:* Otimização limpa, idiomática e sem acoplamento a hints frágeis. O planejador do SQLite utiliza diretamente `sqlite_autoindex_inverted_index_1 (term=? AND mailbox=?)` de forma imediata.

### Critérios de Aceite
1. A consulta SQL em `SqliteIndexStore::search_hybrid` não referencia a tabela `indexed_documents`.
2. A ordenação por relevância BM25 (`SUM(i.tf) DESC, i.uid DESC`) e o limite (`LIMIT 50`) permanecem idênticos.
3. Testes unitários e de integração de `lauda-server` validam a recuperação exata dos mesmos UIDs esperados.
