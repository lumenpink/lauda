# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **desktop:** integrar icone xdg e atalho desktop apos instalacao (closes #71)
- **ci:** cd soberano do lauda-server no obizero via ssh autenticado (closes #64)

### Changed
- **updater:** explicitar atualizacao manual e descontinuar auto-update (closes #55)
- **deps:** migrar urupema para crate gambi e converter testes para e2e real (closes #61)
- **ci:** remover campo permissions incompativel com forgejo actions (closes #60)
- **ci:** sanear pipeline para operacao offline, pull-through cache crates e gambi (closes #57)
- **build:** otimizar tamanho de binario com profile.release e habilitar linking estatico musl crt-static (closes #53)

### Features
- **sync:** implementar servico resiliente de sincronizacao em background (closes #72)
- **search:** busca hibrida por bm25 e ia com interpretacao de linguagem natural (closes #70, #66)
- **sync:** sincronizar integralmente corpos de mensagens recentes dos ultimos 90 dias (closes #68)
- **ci:** continuous deployment do lauda-server no obizero, healthcheck de 15s, rollback e remocao do watchtower (closes #89)
- **updater:** integrar auto-atualizador nativo e distribuicao na banca (closes #88)
- **index:** implementar instrucao e5 e busca semantica neural local (#87)
- **search:** busca bm25 direta no sqlite e aviso de indexacao incompleta (#86)
- **calendar:** exibir ate tres proximos eventos no destaque da agenda (#58)
- **calendar:** padronizar placeholder da quick-add bar para formato generico (#57)
- **ui:** botao de dispensa e exibicao condicional no banner de atualizacao (#56)
- **ci:** esteira completa de compilacao multi-alvo e deploy continuo no push (closes #61)
- **client:** resolver comparacao de versao com LAUDA_APP_VERSION (#61)

### Bug Fixes
- **backend:** despacho smtp nativo conteinerizado e remocao de falso positivo (closes #58)
- **calendar:** higienizar conectores temporais do titulo de eventos (#85)
- **calendar:** interpretar expressoes de data ordinal por dia da semana e mes (#84)
- **ci:** estabilizar esteira com imagem funcional e publicacao de apk (closes #77)
- **client:** definir servidor padrao localhost:8432 em instalacao limpa (closes #75)
- **pratilera:** sincronizar versao 0.5.0 do apk com a pratilera (closes #73)
- **calendar:** formatacao amigavel de tempo relativo e tique reativo (#59)

## [0.4.0] - 2026-10-14

### Features
- **client:** busca heuristica e vinculacao de contatos (closes #51)
- **ui:** exibir chip de data e horario no EventCard (closes #49)
- **ci:** esteira completa de deploy multi-alvo (closes #52)
- **calendar:** extrair participantes por conectivos (closes #48)
- **calendar:** reconhecer horarios e datas coloquiais (closes #47)
- **calendar:** android calendar syncadapter (closes #40)
- **client:** quick-add bar, timeline e rsvp de convites (closes #39)
- **caldav:** implementar persistencia sqlite e endpoints (closes #38)
- **calendar:** implementar parser e modelos rfc 5545 (closes #37)
- **ci:** adicionar script soberano de assinatura apksigner (#35)
- **contacts:** implementar normalizador e syncadapter nativo (#35)
- **rules:** editor duplo de regras e gerador preditivo (#28)
- **server:** adicionar endpoints REST /rules para ManageSieve (#28)
- **rules:** implementar modelo, parser e predicao de regras Sieve (#28)
- **ui:** implementar navegacao colunar e drawer de pastas (#30)
- **ui:** implementar arvore hierarquica e papeis de pastas (#30)
- **cache:** motor sqlite para mensagens e envelopes offline (#16)
- **crypto:** badges cripto e sanitizacao de preambulo (#29)
- **contacts:** ui and composer instant autocomplete (closes #34)
- **contacts:** implement ui and composer autocomplete (ref #34)
- **contacts:** client local cache and incremental sync (closes #33)
- **contacts:** implement client local cache and sync (ref #33)
- **carddav:** native carddav endpoints and sync-collection (closes #32)
- **carddav:** implement native carddav endpoints and sync (ref #32)
- **contacts:** sqlite storage and incremental sync (closes #31)
- **contacts:** implement sqlite store and sync (ref #31)

### Bug Fixes
- **ci:** assinar APKs com keystore soberana in.twolp (closes #54)
- **ci:** unificar esteira em ci.yml com needs [test] (closes #53)
- **calendar:** persistir eventos no indexeddb via rexie (closes #46)
- **wasm:** restringir parse_iso_date_to_unix a nao-wasm (closes #44)
- **ci:** deps em deploy, usar VERSION e icones no apk (closes #43)
- **ci:** instalar nodejs antes de checkout e arrumar pages (closes #42)
- **ci:** prefixar acoes docker com github em deploy (closes #41)

### Refactoring
- **android:** unificar accountType com package id in.twolp.lauda

### Documentation
- **core:** especificar arquitetura de convites de eventos (closes #50)
- **calendar:** especificar arquitetura de convites iTIP/iMIP (#50)
- **calendar:** especificar busca heuristica e vinculacao (#51)
- **ui:** especificar chip de data e horario no EventCard (#49)
- **ci:** especificar unificacao de ci.yml com needs [test] (#53)
- **ci:** especificar esteira multi-alvo na CI (#52)
- **calendar:** especificar extracao de participantes (#48)
- **calendar:** especificar parsing coloquial (#47)
- **calendar:** especificar android calendar syncadapter (#40)
- **calendar:** especificar quick-add bar e timeline stream (#39)
- **caldav:** especificar persistência sqlite e endpoints rfc 4791 (#38)
- **calendar:** especificar smart intent parser e modelos rfc 5545 (#37)
- **contacts:** spec do android syncadapter e phone normalizer (#35)
- especificacao do motor de regras ManageSieve (#28)
- especificacao de redesign e apresentacao de pastas (#30)
- **cache:** especificacao do cache local de mensagens (#16)
- **contacts:** spec for ui and autocomplete (ref #34)
- **contacts:** spec for client contacts cache (ref #33)
- **carddav:** spec for carddav routes and sync (ref #32)
- **contacts:** sqlite schema and sync contracts (ref #31)

### Testing
- **calendar:** adicionar testes red para contratos iTIP (#50)
- **client:** adicionar testes red para busca heuristica (#51)
- **ui:** adicionar teste para format_event_datetime_chip (red) (#49)
- **ci:** adicionar teste de unificacao do workflow (red test) (#53)
- **ci:** testes para esteira multi-alvo e manifestos (red test) (#52)
- **calendar:** testes de extracao de participantes (red test) (#48)
- **calendar:** adicionar testes de parsing coloquial (red test) (#47)
- **calendar:** adicionar testes para android calendar sync (#40)
- **calendar:** adicionar testes para cache de eventos e ics (#39)
- **caldav:** adicionar teste de integracao caldav e sqlite (#38)
- **calendar:** adicionar testes falhantes para parser e ical (#37)
- **contacts:** casos de borda e similaridade de contatos (#35)
- **contacts:** adicionar testes red para normalizador e sync (#35)
- adicionar testes de gerador e parser de regras Sieve (#28)
- adicionar testes de hierarquia e arvore de pastas (#30)
- **cache:** teste red para backend sqlite do cache de mensagens (#16)
- **contacts:** add failing test for autocomplete and crud (ref #34)
- **contacts:** add failing test for local contacts cache (ref #33)
- **carddav:** add failing integration test for carddav routes (ref #32)
- **contacts:** failing test for sqlite store and sync (ref #31)

### Maintenance
- **version:** sincronizar arquivo VERSION com semver (closes #45)
- **contacts:** formatar codigo conforme padroes canonicos (#35)

## [0.2.0] - 2026-10-05

### Added
- Backend Maildir nativo com suporte a caixas Dovecot reais (`INBOX`, `Sent`, `Drafts`, `Trash`, `Junk`) e preservação de UIDs.
- Despacho real de e-mails via MTA / Postfix com cópia automática em `.Sent`.
- Ações de mensagem no Smolclient: marcar como lida/não lida e mover para lixeira/exclusão definitiva.
- Barra de busca e filtro reativo instantâneo na caixa de entrada por assunto e remetente.
- Suporte a HTTP Range (206) e autodescoberta do APK no Obtainium via `lauda.2lp.in`.
- Configuração de rede Android permitindo tráfego HTTP cleartext em redes locais e Tailscale.
- Suíte exaustiva de testes E2E herméticos para Urupema, Ishcondidim Autocrypt, Papeli e Lauda.
- Interface mobile-first completa em Dioxus 0.7 e Rust Edition 2024 com estética Trash Polka.

## [0.1.0] - 2026-10-02

### Added
- Inicialização do repositório em conformidade com o padrão GOST.
- Contratos de dados compartilhados em `lauda-types`.
- Micro-daemon inicial axum em `lauda-server`.
- Estrutura base do cliente Dioxus em `lauda-client`.
