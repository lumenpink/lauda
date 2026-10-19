# Especificação Técnica: Otimização do Tempo de Carregamento Inicial Pós-Login

**ID do Ticket:** #81  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Andamento (Fase 3: Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante a autenticação no cliente (`LoginView`), o fluxo anterior bloqueava o botão de login em estado `"CONECTANDO..."` enquanto executava requisições sequenciais adicionais para listar caixas de entrada (`list_mailboxes`) e listar as primeiras 50 mensagens da caixa principal (`list_messages`).

Essa abordagem causava retenção perceptível da tela de login, atrasando a navegação para a tela principal (`InboxView`) por múltiplos roundtrips de rede e processamento no servidor. Simultaneamente, ao atualizar o estado da sessão, o loop SWR em segundo plano de `ui/mod.rs` disparava requisições concorrentes e duplicadas para os mesmos endpoints.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Transição Imediata Pós-Login
- Logo após o recebimento do token JWT de autenticação (`TokenResponse`):
  1. A sessão é persistida em armazenamento local seguro (`save_session`).
  2. A caixa postal ativa é inicializada canonicamente como `"INBOX"`.
  3. O estado de carregamento da tela de login é encerrado (`is_loading = false`).
  4. A tela ativa é imediatamente alterada para `Screen::Inbox` sem aguardar a conclusão de chamadas de rede subsequentes.

### 2.2. Desacoplamento e Não-Bloqueio de Dados
- O carregamento das caixas e da lista de mensagens não deve bloquear a transição de telas.
- O ciclo reativo de SWR em segundo plano (em `ui/mod.rs`) e/ou rotinas assíncronas em background assumem a sincronização de envelopes e caixas, preenchendo a interface à medida que os dados chegam.

### 2.3. Indicador de Carregamento na Caixa de Entrada
- Caso a `InboxView` seja exibida com a lista de mensagens vazia enquanto a primeira sincronização ocorre, a interface deve exibir um indicador claro e leve de carregamento (`"CARREGANDO CAIXA DE ENTRADA..."`), mantendo menus e botões operacionais.

---

## 3. Critérios de Aceite

1. O tempo de transição da tela de login para a tela principal após o recebimento do token é instantâneo (< 50ms locais), sem esperar requisições de mensagens ou caixas.
2. Eliminação de chamadas de rede redundantes e sequenciais bloqueantes no manipulador de login.
3. Teste unitário e de fluxo automatizado comprovando o desacoplamento e a transição não-bloqueante pós-login.
4. Conformidade estrita Gosplan (`cargo fmt`, `clippy -D warnings`, `cargo test`).

---

## 4. Registro de Fases DD-TDD

- [x] **Fase 1 (Ticket):** Ticket #81 formalizado e delimitado.
- [x] **Fase 2 (Branch):** Branch `perf/81-login-initial-load-optimization` criada a partir da `main`.
- [x] **Fase 3 (Spec):** Especificação técnica formalizada neste documento.
- [ ] **Fase 4 (Red Test):** Teste comprovando a latência ou o bloqueio sequencial indevido no fluxo de login.
- [ ] **Fase 5 (Green Implement):** Desacoplamento do login e transição imediata para Inbox.
- [ ] **Fase 6 (Refactor):** Código idiomático, tipado e com zero warnings.
- [ ] **Fase 7 (Mutation Test):** Asserções verificando sensibilidade à reintrodução de chamadas bloqueantes.
- [ ] **Fase 8 (Quality Gate):** Validação com `cargo fmt --check`, `cargo clippy` e suíte de testes.
- [ ] **Fase 9 (Finish & Close):** Reconciliação, merge via CCCP e fechamento do ticket.
