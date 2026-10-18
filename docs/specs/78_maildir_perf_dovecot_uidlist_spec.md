# Especificação Técnica: Otimização de Performance Maildir e Resiliência Dovecot-UIDList

**ID do Ticket:** #78  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Em caixas de correio com grande volume (ex: 56.000+ e-mails no servidor de produção Obizero), a chamada `GET /messages?box=INBOX` congelava a thread do servidor por mais de 70 segundos, culminando em aborto por timeout no proxy reverso Caddy (`writing: client disconnected`) e deixando a interface do usuário travada indefinidamente em "CONECTANDO...".

### Causas Raízes Identificadas:
1. **Divergência de Formato no `dovecot-uidlist`**:
   O Dovecot registra entradas no formato `<uid> :<prefix>,S=<size>,W=<vsize>`. A função `load_uid_maps` removia apenas o `:` inicial, mantendo `,S=...,W=...`. Ao cruzar com os nomes reais dos arquivos em `cur/` e `new/` (cujos nomes são `<prefix>:2,<flags>`), a chave nunca coincidia.
2. **Explosão Algorítmica $O(N^2)$ em `unmapped`**:
   Como nenhum arquivo casava com as chaves corrompidas do mapa, todos os 56.000 arquivos caíam no fallback de não-mapeados. O código utilizava `Vec<String>` com `!unmapped.contains(&prefix)`, executando mais de 1,6 bilhão de comparações lineares de strings na mesma thread.
3. **Ausência de Timeout no Cliente HTTP**:
   O `reqwest::Client` em `lauda-client` era instanciado sem timeout padrão. Qualquer demora ou desconexão do backend travava o estado reativo `is_loading` para sempre.

---

## 2. Requisitos e Contratos

### 2.1. Sanitização de Prefixos em `load_uid_maps`
- No parsing de cada linha do `dovecot-uidlist`:
  - Extrair o token do prefixo após `:`
  - Descartar qualquer sufixo de metadados do Dovecot delimitado por vírgula (ex: `,S=1099,W=1125`), isolando estritamente a parte base do nome do arquivo (`prefix.split(',').next().unwrap_or(prefix)`).
- Na leitura dos arquivos físicos em `cur` e `new`:
  - Extrair o prefixo base descartando o sufixo de flags `:2,...` e eventuais metadados de tamanho `,S=...`.
- Garantir que o lookup `prefix_to_uid.contains_key(&base_prefix)` seja positivo para todas as mensagens indexadas pelo Dovecot.

### 2.2. Algoritmo $O(N)$ com `HashSet` para Arquivos Não Mapeados
- A estrutura temporária de detecção de UIDs ausentes deve utilizar `HashSet<String>` para verificação e inserção em tempo $O(1)$.
- A ordenação para atribuição sequencial de UIDs residuais (`max_uid + 1`) deve ser feita uma única vez ao final.

### 2.3. Timeout Resiliente no Cliente HTTP (`lauda-client`)
- Configurar timeout explícito de conexão e requisição (ex: 15 segundos) no `Client::builder()` do `HttpApiClient`.
- Em caso de timeout ou erro de rede, o cliente deve resetar `is_loading = false` e renderizar mensagem amigável no `error_msg`.

---

## 3. Critérios de Aceite e Verificação
1. **Teste Unitário / Benchmark**:
   - Teste unitário validando que uma pasta Maildir com `dovecot-uidlist` contendo sufixos `,S=...,W=...` mapeia 100% dos UIDs corretamente.
   - Teste de carga sintético com 5.000 arquivos simulados executando em menos de 100ms.
2. **Teste de Integração de Timeout**:
   - `HttpApiClient` deve possuir timeout configurado e abortar requisições zumbis em tempo determinístico.
3. **Quality Gate**:
   - Zero alertas em `cargo clippy --workspace -- -D warnings`.
   - Formatação 100% aderente com `cargo fmt -- --check`.
   - Todos os testes da suíte (`cargo test --workspace`) aprovados.
