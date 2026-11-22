# Especificação Técnica - Ticket #118
## Otimização de Latência na Busca Remota SQLite e Atualização do Bundle Web (`lauda-server` / `lauda-client`)

### Contexto & Motivação
A busca remota de mensagens contra o endpoint `/messages/search` do `lauda-server` apresenta latência excessiva e eventuais timeouts em acervos com dezenas de milhares de mensagens (como as 58.962 mensagens indexadas no `.lauda_index.db` em produção).
Além disso, os artefatos estáticos WebAssembly servidos em `site/app` encontravam-se defasados (gerados em 10 de novembro), impedindo que os clientes navegadores usufruíssem das correções dos Tickets #112, #113 e #115 (normalização Unicode, eliminação de JOIN e cache local IndexedDB via Rexie com BM25 puro).

### Árvore de Decisão
- **Alternativa A (Descartada):** Aumentar indiscriminadamente o timeout de requisições HTTP para minutos.
  - *Motivo do descarte:* Não resolve a causa raiz e degrada a experiência da usuária com esperas inaceitáveis e telas travadas. Natasha rejeitaria essa gambiarra.
- **Alternativa B (Descartada):** Desativar a agregação de TF (Term Frequency) e ordenar puramente por UID.
  - *Motivo do descarte:* Destrói a qualidade do ranqueamento BM25, priorizando e-mails recentes em vez de e-mails semanticamente e lexicalmente relevantes para os termos buscados.
- **Alternativa C (Escolhida):**
  1. No `lauda-server`: Adicionar índice cobridor (covering index) `idx_inverted_bm25_covering` sobre `(mailbox, term, tf, uid)` no SQLite, permitindo que o SQLite execute a query BM25 com index-only scan sem tocar na tabela base.
  2. Implementar timeout defensivo e limites de segurança nas consultas agregadas para garantir respostas sub-segundo mesmo sob cargas pesadas.
  3. Recompilar o bundle WebAssembly do `lauda-client` e atualizar os hashes em `site/app/index.html` e `site/app/assets/`.

### Requisitos Técnicos
1. Garantir que a criação da tabela `inverted_index` no `SqliteIndexStore::init_schema` declare o índice cobridor `idx_inverted_bm25_covering` sobre `(mailbox, term, tf, uid)`.
2. Validar via teste de plano de execução que queries sobre `inverted_index` utilizam index-only scan (sem table access para `tf` e `uid`).
3. O endpoint de busca deve retornar resultados em tempo de resposta adequado sem expirar conexões ativas.
4. O artefato WebAssembly em `site/app/assets/` deve ser atualizado e sincronizado com os fontes vigentes.
