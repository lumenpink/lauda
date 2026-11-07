# Especificação Técnica: Resiliência de Rede, Sincronização em Camadas e Cache Local na Abertura de Mensagens

**ID do Ticket:** #85  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Desenvolvimento (Fase 3 - Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Entre a versão `dev.289` e as versões posteriores, a rotina unificada de sincronização em segundo plano (`sync_all_cycle`) passou a solicitar envelopes com `client.list_messages(token, mailbox, None, None)`. Em caixas de correio com grande volume (ex.: mais de 58.000 mensagens), essa requisição monolítica sem limite causava timeouts na rede móvel e estouro de memória no Android, abortando todo o ciclo com erro antes que o prefetch dos corpos de mensagens pudesse ser realizado.

Como consequência, as mensagens não eram salvas no cache local SQLite do aparelho. Ao clicar em uma mensagem na interface, o cliente tentava buscá-la diretamente no servidor e, em caso de instabilidade na conexão, o erro cru era disparado para a usuária (`Falha ao abrir mensagem #58357: Erro de rede / requisição: error sending request for url`).

---

## 2. Requisitos e Contratos

### 2.1. Sincronização em Camadas (Fast-First + 90 Dias)
1. **Prioridade Imediata (Fast-First):**
   - Ao sincronizar uma caixa de correio ativa, o cliente deve garantir primeiro a busca e o prefetch eager das mensagens mais recentes (as primeiras 100 mensagens com `limit: Some(100)`).
   - O prefetch dos corpos dessas 100 mensagens é executado com prioridade máxima, garantindo abertura instantânea (0 RTT / offline) no SQLite local.
2. **Segundo Plano (Acervo de 90 Dias):**
   - O worker de segundo plano mantém a cobertura e retenção completa dos 90 dias, sem fazer requisições desmedidas de 58.000 itens de uma só vez, processando lotes controlados para popular o acervo local de 90 dias e expurgando as mensagens mais antigas.

### 2.2. Resiliência de Transporte HTTP (`HttpApiClient`)
- Implementar mecanismo de repetição com backoff exponencial para operações idempotentes de leitura (`get_message`, `list_messages`, etc.) contra erros transientes de socket (`error sending request for url`, timeouts e resets de conexão).
- Configurar parâmetros de keep-alive e timeout no pool HTTP do `reqwest` adequados ao ambiente mobile.

### 2.3. Interface e Tratamento de Erro (UI / UX)
- A tela de leitura de mensagens (`inbox_view.rs`) mantém o fast-path de abertura direta a partir do cache local SQLite.
- Caso ocorra falha de rede ao buscar uma mensagem não presente em cache, exibir mensagem amigável com opção de repetição imediata.

---

## 3. Critérios de Aceite e Verificação

1. **Fase 4 (Red Test):**
   - Teste automatizado simulando falha temporária de rede em `get_message` e comprovando que a estratégia de retry recupera a chamada com sucesso.
   - Teste garantindo que `sync_all_cycle` prioriza a sincronização em janela controlada sem tentar carregar infinitos envelopes em caixas densas.
2. **Fase 5 (Green Implement):**
   - Ajustar `HttpApiClient::get_message` e chamadas de sincronização em `sync.rs` e `ui/mod.rs`.
3. **Fase 8 (Quality Gate):**
   - `cargo fmt --check` 100% aderente.
   - `cargo clippy --all-targets -- -D warnings` sem alertas.
   - Suíte de testes aprovada.
