# Especificação Técnica: Difusão de Alterações Externas de Flags e Anti-Entropia Configurável

**ID do Ticket:** #100  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Em Especificação (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

1. **Atualizações de flags por clientes externos:** Quando mensagens de e-mail são marcadas como lidas ou sinalizadas por um cliente IMAP externo (ex.: Dovecot, SnappyMail, Thunderbird), o padrão Maildir renomeia o arquivo dentro de `cur/` adicionando a flag no sufixo (ex.: `:2,S` para lida, `:2,F` para flag). O monitoramento inotify captura o evento de renomeação perfeitamente. No entanto, o `start_maildir_watcher` filtrava o broadcast apenas quando a reconciliação do índice vetorial detectava inclusão ou remoção de novos UIDs (`!reconcile.to_index.is_empty() || !reconcile.to_remove.is_empty()`). Como o UID não muda ao marcar como lido, o evento `mailbox_updated` era suprimido, impedindo que o cliente Lauda refletisse o status de lido em tempo real.
2. **Intervalo do Indexador Contínuo de Anti-Entropia:** A varredura contínua de reconciliação em segundo plano (`start_continuous_indexer`) estava fixada em 15 segundos em `main.rs`. Com o inotify agora ativo e operando em tempo real, um ciclo de 15 segundos gera I/O redundante em acervos com dezenas de milhares de mensagens. O intervalo deve ser parametrizável via variável de ambiente (`LAUDA_INDEX_INTERVAL_SECS`), com valor padrão relaxado de 300 segundos (5 minutos) para servir como rede de segurança de consistência eventual (anti-entropia).

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Emissão Incondicional de `mailbox_updated` no Watcher Inotify:**
  - *Alternativa descartada:* Fazer diff detalhado de cada sufixo `:2,flags` de todos os arquivos no loop de inotify.
    - *Motivo do descarte:* Inspecionar e parsear flags de 50.000 arquivos a cada micro-evento do inotify consome tempo de CPU desnecessário. Como o debounce de 300ms já agrupa rajadas e o cliente Lauda faz apenas um `list_messages` leve de 50 envelopes quando acionado, emitir `mailbox_updated` após a estabilização de qualquer evento de escrita/renomeação no Maildir é atômico, robusto e garante que qualquer mudança de flag externa reflita instantaneamente no cliente.
  - *Escolha adotada:* No `start_maildir_watcher`, sempre que o debounce de alterações estabilizar, disparar a reconciliação do índice vetorial e transmitir `mailbox_updated` no `events_tx`.

- **Decisão 2: Parametrização do Fallback de Anti-Entropia:**
  - *Escolha adotada:* Em `main.rs`, ler `std::env::var("LAUDA_INDEX_INTERVAL_SECS")`, com fallback padrão de 300 segundos. Isso reduz em 95% os ciclos de leitura no Maildir sem desproteger a consistência em caso de falha de eventos do kernel.

---

## 3. Contratos e Requisitos de Teste

1. **Teste de Renomeação Externa de Flags:**
   - Adicionar teste em `websocket_sync_test.rs` onde um arquivo existente em `cur/` sem flags é renomeado para incluir `:2,S`.
   - O WebSocket conectado DEVE receber `mailbox_updated` via inotify em menos de 1 segundo.
2. **Verificação de Anti-Entropia:**
   - Verificar resolução da variável de ambiente `LAUDA_INDEX_INTERVAL_SECS`.
