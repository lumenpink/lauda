# Especificação Técnica — Correção da Localização de APKs e Assinatura no Deploy Android (#82)

- **ID do Ticket:** #82
- **Status:** Concluído (Fase 9: Finish & Close)
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0
- **Data:** 2026-11-06

## 1. Contexto & Motivação
Na esteira de CI/CD do `lauda` (`.forgejo/workflows/ci.yml`), o job `deploy-apk` falhou na etapa de empacotamento e assinatura (`Package Android APK & Sign`).
Com o isolamento da geração de binários em `target/dist/apk/` (introduzido no Ticket #77 para manter a conformidade GOST-24 de zero binários no repositório), os arquivos deixaram de ser gerados em `site/`.
O workflow anterior continha a instrução:
```bash
if [ -f scripts/sign-release.sh ]; then
  [ -f site/lauda.apk ] && ./scripts/sign-release.sh site/lauda.apk
  [ -f site/lauda-latest.apk ] && ./scripts/sign-release.sh site/lauda-latest.apk
fi
```
Sob a diretiva `set -e` do runner do Forgejo, a ausência de `site/lauda-latest.apk` fez o comando `[ -f ... ]` retornar código de saída 1. Como era o comando final do bloco, o runner interpretou como falha do step (`exitcode 1: failure`).
Além disso, a etapa subsequente de upload para o Cloudflare R2 inspecionava apenas caminhos legados em `site/`.

## 2. Requisitos Técnicos

### 2.1 Resolução Segura de Caminhos e Assinatura
Na etapa de empacotamento e assinatura do job `deploy-apk`:
- Inspecionar os APKs gerados nos diretórios canônicos (`target/dist/apk/lauda.apk`, `target/dist/apk/lauda-latest.apk`) e nos diretórios legados (`site/lauda.apk`, `site/lauda-latest.apk`).
- Utilizar estruturas de controle completas (`if [ -f "$apk" ]; then ...; fi`), prevenindo retornos não-zero quando um dos caminhos alternativos não estiver presente.

### 2.2 Suporte a `target/dist/apk/` no Upload para Cloudflare R2
Na etapa de upload para o Cloudflare R2:
- Priorizar a busca do artefato em `target/dist/apk/lauda.apk` ou `target/dist/apk/lauda-latest.apk`.
- Manter fallback para `site/lauda.apk` e `site/lauda-latest.apk`.
- Manter o fail-fast caso nenhum APK seja encontrado em nenhum dos locais esperados.

### 2.3 Auditoria Automatizada e Testes de Mutação
- Atualizar o teste de auditoria da esteira Android (`tests/test-ci-android-deploy.sh`) para validar a presença da resolução de `target/dist/apk/` e a ausência do encadeamento frágil `[ -f site/... ] &&`.
- Adicionar mutantes para garantir a detecção de regressões.

## 3. Critérios de Aceitação
- Teste `tests/test-ci-android-deploy.sh` aprovado com 100% de sucesso e mutantes eliminados.
- Formatação limpa em `cargo fmt -- --check`.
- Zero alertas em `cargo clippy --workspace -- -D warnings`.
- Índice Gosplan em 100% via `cccp check --strict`.

## 4. Checklist GOST DD-TDD
- [x] **Fase 1: Ticket** — Ticket #82 criado no Forgejo.
- [x] **Fase 2: Branch** — Branch `fix/82-corrigir-localizacao-de-apks-e-assinatura-no-deploy-android` criada.
- [x] **Fase 3: Spec** — Especificação técnica redigida.
- [x] **Fase 4: Red Test** — Testes de auditoria em `tests/test-ci-android-deploy.sh` falhando pela ausência do suporte a `target/dist/apk`.
- [x] **Fase 5: Green Implement** — Workflow corrigido e testes passando.
- [x] **Fase 6: Refactor** — Limpeza e padronização idiomática do workflow.
- [x] **Fase 7: Mutation Test** — Cobertura completa de mutantes.
- [x] **Fase 8: Quality Gate** — Validação com fmt, clippy, testes e cccp check strict.
- [x] **Fase 9: Finish & Close** — Fechamento do ticket e integração à base.
