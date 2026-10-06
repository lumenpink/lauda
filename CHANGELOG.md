# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
