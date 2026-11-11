# Especificação Técnica: Sincronização em Tempo Real via WebSocket e Monitoramento Inotify no Maildir

**ID do Ticket:** #99  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Data de Fechamento:** 2026-11-11  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, o ecossistema Lauda opera com duas latências artificiais por amostragem periódica (polling):
1. **Cliente:** O `lauda-client` consulta a cada 10 segundos o endpoint de listagem de caixas e mensagens. Consequentemente, a detecção de queda de rede ou restabelecimento de conectividade leva até 10 segundos, e mensagens novas enviadas ao servidor demoram para refletir na interface.
2. **Servidor:** O `lauda-server` roda um loop a cada 15 segundos (`start_continuous_indexer`) inspecionando o diretório de arquivos para reconciliação e indexação. Entregas de e-mails feitas pelo Postfix/Dovecot diretamente no Maildir do sistema não disparam reação imediata.

O objetivo deste ticket é introduzir uma via reativa em tempo real com protocolo leve, eficiente e resiliente:
- **No servidor:** Monitoramento em nível de kernel (via `notify` / `inotify` no Linux) das pastas `new/` e `cur/` das caixas de correio do Maildir, com debounce contra rajadas, indexação instantânea e difusão de eventos via WebSocket (`/ws`).
- **No cliente:** Conexão WebSocket persistente ao servidor para obter presença e telemetria de conectividade em tempo real (queda instantânea = offline) e engatilhar revalidação/prefetch no exato momento em que o servidor emite o evento `mailbox_updated`.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: WebSocket vs. Server-Sent Events (SSE):**
  - *Alternativa descartada:* Expandir apenas o SSE existente (`/events`).
    - *Motivo do descarte:* SSE é estritamente unidirecional e depende de streaming HTTP/1.1 ou chunked transfer. Em clientes nativos Desktop com bibliotecas HTTP compactas ou ambientes restritos, detectar uma conexão caída (dead socket/silent timeout) sem frames de ping/pong integrados no nível da camada de transporte exige cabeçalhos e parsers extras. O WebSocket possui suporte nativo a controle de quadros (Ping/Pong/Close frames) em `axum::extract::ws`, além de ampla interoperabilidade tanto em Tokio (`tokio-tungstenite`) quanto no navegador (`web-sys::WebSocket`).
  - *Escolha adotada:* Rota WebSocket em `/ws` no `lauda-server`.

- **Decisão 2: Autenticação da Conexão WebSocket:**
  - *Alternativa descartada:* Passar o token JWT no handshake inicial via subprotocolo ou cabeçalhos customizados não suportados pela API `WebSocket` do navegador em WASM.
    - *Motivo do descarte:* A API padrão `new WebSocket(url)` dos navegadores não permite inclusão arbitrária do header `Authorization: Bearer <token>`.
  - *Escolha adotada:* Aceitar o token via Query Parameter (`/ws?token=<jwt>`) com validação de claims pelo extrator de autenticação do Axum, rejeitando conexões não autorizadas no próprio handshake HTTP antes do upgrade.

- **Decisão 3: Notificação de Maildir (Inotify com Debounce):**
  - *Alternativa descartada:* Disparar indexação pesada imediatamente a cada micro-evento de arquivo (ex.: criação de arquivo temporário pelo Postfix antes do rename).
    - *Motivo do descarte:* O fluxo padrão do Maildir escreve o e-mail em `tmp/` e depois faz um `rename` atômico para `new/`. Ouvir eventos crus sem debounce causaria múltiplos disparos e condições de corrida de leitura em arquivos ainda incompletos.
  - *Escolha adotada:* Utilizar o crate `notify` configurado para monitorar recursivamente os diretórios `new/` e `cur/` com debounce de 250ms a 500ms. Quando um evento de escrita/renomeação estabiliza, aciona-se a rotina de reconciliação e, havendo alteração efetiva de UIDs, transmite-se `mailbox_updated` aos clientes conectados via broadcast.

- **Decisão 4: Política de Resiliência do Cliente:**
  - *Escolha adotada:* O cliente não descarta completamente o loop de polling; ele o converte em um canal de contingência de baixa frequência (ex.: 60s). Se o WebSocket cair, o cliente marca imediatamente `is_online = false`, entra em modo de reconexão automática com backoff exponencial (1s, 2s, 4s, 8s, até 30s) e, caso a reconexão falhe, o polling de segurança preserva a funcionalidade mínima.

---

## 3. Protocolo e Contrato de Mensagens

### 3.1. Handshake
- **URL:** `GET /ws?token={jwt}`
- **Upgrade:** `101 Switching Protocols`

### 3.2. Formato das Mensagens (JSON)

**Servidor -> Cliente:**
```json
{
  "event": "mailbox_updated",
  "mailbox": "INBOX",
  "timestamp": 1731350000
}
```

```json
{
  "event": "message_action",
  "mailbox": "INBOX",
  "uid": 1234,
  "action": "read"
}
```

```json
{
  "event": "pong"
}
```

**Cliente -> Servidor (Opcional):**
```json
{
  "action": "ping"
}
```

---

## 4. Fases de Execução (GOST DD-TDD)

1. **Fase 3 (Spec):** Esta especificação técnica detalhando decisões e arquitetura.
2. **Fase 4 (Red Test):**
   - Teste de integração do endpoint `/ws` com handshake autenticado e rejeição de token inválido.
   - Teste do receptor de eventos com simulação de broadcast.
3. **Fase 5 (Green Implement):**
   - Habilitar `features = ["ws"]` em `axum` no `lauda-server`.
   - Implementar o handler `/ws` e o gerenciador de conexões ativas no `lauda-server`.
   - Integrar monitoramento `notify` no `MaildirBackend` ou disparar broadcast a partir de eventos do Maildir.
   - Implementar o cliente de conexão WebSocket em `lauda-client` com reatividade imediata no status `is_online`.
4. **Fase 6 & 7 (Refactor & Mutation Test):**
   - Teste de mutação contra falha silenciosa de reconexão e eventos espúrios.
5. **Fase 8 (Quality Gate):**
   - `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e execução de testes.
6. **Fase 9 (Finish & Close):**
   - Laudo e consolidação.

---

## 5. Resumo da Implementação e Validação (Fase 9)

- **Servidor (`lauda-server`):**
  - Rota `/ws` com suporte a autenticação via `?token=` ou header Bearer, upgrade HTTP 101 e heartbeat (ping/pong) nativo do WebSocket.
  - Monitoramento inotify via `notify::RecommendedWatcher` em `MaildirBackend::start_maildir_watcher`, com debounce de 300ms e broadcast imediato de `{"event":"mailbox_updated","mailbox":"..."}`.
  - Testes em `websocket_sync_test.rs`: 3 testes cobrindo rejeição de token (401), handshake e envio/recebimento de comandos de controle, difusão de ações (leitura) e acionamento real de inotify ao gravar arquivo no Maildir.
- **Cliente (`lauda-client`):**
  - Módulo `lauda-client/src/domain/realtime.rs` com suporte a parsing de eventos e loop de conexão com reconexão resiliente (backoff exponencial).
  - Integração no `ui/mod.rs`: o cliente agora reage instantaneamente aos eventos do servidor e reflete presença imediata (`is_online = true` ao conectar, `false` na queda), mantendo o timer periódico degradado de 60s apenas como contingência secundária.
  - Compatibilidade com WASM preservada e validada via `cargo check --target wasm32-unknown-unknown`.
- **Quality Gate:**
  - `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` aprovados com zero advertências.
