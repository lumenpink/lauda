# Especificação Técnica - Ticket #122
## Otimização de Índice Cobridor BM25 e PRAGMAs de Memória no SQLite (`lauda-server`)

### Contexto & Motivação
A busca remota de mensagens em produção contra o `.lauda_index.db` (2.6 GB com 16.275.785 linhas no `inverted_index` e 58.962 mensagens) apresentava latência inaceitável (~6 minutos) para termos com poucas ocorrências como "lisiane" (441 ocorrências).
A investigação direta do plano de execução e das estatísticas de tabela revelou que:
1. A cardinalidade de `mailbox` é mínima (1 única caixa, `INBOX`), enquanto a cardinalidade de `term` é de 2.304.834 termos únicos.
2. O índice cobridor anterior `(mailbox, term, tf, uid)` forçava o SQLite a varrer todo o nó `INBOX` procurando pelo termo em disco sob concorrência de I/O.
3. Invertendo a chave para `(term, mailbox, tf, uid)`, o SQLite executa um salto B-Tree direto para as folhas do termo correspondente, reduzindo a busca de minutos para sub-segundo.
4. Além disso, a conexão SQLite rodava com PRAGMAs padrão de cache diminuto (2 MB) e sem `mmap_size`, gerando *iowait* excessivo.

### Árvore de Decisão
- **Alternativa A (Descartada):** Manter o índice `(mailbox, term, ...)` e aumentar timeout do HTTP.
  - *Motivo do descarte:* Viola frontalmente as diretrizes e a exigência de usabilidade. Natasha rejeitaria requisições com minutos de espera para 400 registros.
- **Alternativa B (Descartada):** Criar FTS5 virtual table paralela.
  - *Motivo do descarte:* Duplicaria o armazenamento para mais de 5 GB em disco sem necessidade, já que a tabela invertida com covering index resolve a agregação BM25 instantaneamente.
- **Alternativa C (Escolhida):**
  1. No `SqliteIndexStore`: Declarar o índice cobridor como `idx_inverted_bm25_covering ON inverted_index(term, mailbox, tf, uid)`.
  2. Configurar PRAGMAs de inicialização no SQLite: `PRAGMA mmap_size = 268435456;` (256 MB) e `PRAGMA cache_size = -64000;` (64 MB).
  3. Atualizar a suite de testes e validar plano de execução e desempenho.

### Requisitos Técnicos
1. `SqliteIndexStore::init_schema` deve criar o índice cobridor com a ordem `(term, mailbox, tf, uid)`.
2. A conexão SQLite deve aplicar os PRAGMAs otimizados de leitura e cache.
3. Testes de plano de execução devem confirmar que a consulta `INDEXED BY idx_inverted_bm25_covering` com filtro `term IN (...) AND mailbox = ?` continua sendo index-only scan.
