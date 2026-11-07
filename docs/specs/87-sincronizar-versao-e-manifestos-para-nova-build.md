# Especificação Técnica: Sincronização de Versão e Manifestos para Nova Build

**ID do Ticket:** #87  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualizar o arquivo canônico `VERSION` no repositório `lauda` para refletir as entregas recentes (Tickets #84, #85 e #86: correção do roteamento web sob `/app`, sincronização rápida fast-first, retry contra oscilações de rede, token contínuo de 1 ano no APK, 12h na Web e wipe com preservação de login) com a versão `0.5.0-dev.508+20261107.c6530c0`.

---

## 2. Requisitos e Contratos

1. Atualizar `VERSION` para `0.5.0-dev.508+20261107.c6530c0`.
2. Sincronizar `site/version.json` e `site/version-nightly.json` com a nova versão.
3. Garantir conformidade nos testes de integridade e auditoria `cccp check --strict`.
