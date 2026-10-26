# Especificação Técnica — Saneamento Soberano da CI/CD (#62)

## 1. Contexto e Objetivos
A esteira de integração e entrega contínua do Lauda (`.forgejo/workflows/ci.yml`) continha inconformidades de autonomia e soberania:
1. `Dockerfile`: Apontamentos para registros externos públicos (`docker.io/lumenpink/gost-ci-cross:latest` e `debian:bookworm-slim`), que sofrem timeout de rede ou violam o princípio hermético local.
2. `ci.yml`: Utilização da credencial legada `GIT_2LP_TOKEN` e ausência da passagem da credencial no job `deploy-server`.
3. Veto irrestrito a forges externas: Veto e remoção de qualquer referência a `github.com`, `docker.io` e `codeberg.org` em workflows de CI, scripts de distribuição e imagens OCI.

## 2. Requisitos Técnicos
1. **Dockerfile**:
   - Builder: `FROM gambi.2lp.in/lumen/gost-ci-cross:latest AS builder`
   - Runtime: `FROM gambi.2lp.in/lumen/mirror-debian:bookworm-slim`
2. **Workflow `.forgejo/workflows/ci.yml`**:
   - Substituição de todas as referências `${{ secrets.GIT_2LP_TOKEN }}` e `$GIT_2LP_TOKEN` por `GAMBI_2LP_IN_TOKEN`.
   - Adição do bloco `env:` com `GAMBI_2LP_IN_TOKEN: ${{ secrets.GAMBI_2LP_IN_TOKEN }}` no job `deploy-server`.
3. **Auditoria Soberana e Testes**:
   - `tests/audit_cd_workflow.sh` atualizado para validar a ausência absoluta de `docker.io`, `github.com` e `codeberg.org` no `Dockerfile` e no `ci.yml`.
   - `lauda-server/tests/e2e_apk_test.rs` atualizado para assegurar a utilização de imagens OCI soberanas em `gambi.2lp.in`.
