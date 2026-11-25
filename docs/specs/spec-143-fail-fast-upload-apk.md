# Especificação Técnica — Ticket #143
## Fail-Fast no Upload de APK e Alinhamento de `tests/test-ci-android-deploy.sh`

### 1. Contexto e Problema
1. Em `.forgejo/workflows/ci.yml`, o loop `for apk in target/dist/apk/*.apk site/*.apk` saía com status 0 caso nenhum `.apk` fosse encontrado.
2. Em `tests/test-ci-android-deploy.sh`, a asserção de fail-fast ainda buscava pela mensagem antiga `'Nenhum APK Android encontrado para upload no Cloudflare R2'` (de antes da migração para o Forgejo Generic Packages / `cccp package upload`).

### 2. Árvore de Decisões (Registro para a Natasha)
- **Decisão: Adicionar contador `UPLOADED` com `exit 1` no `ci.yml` e atualizar `test-ci-android-deploy.sh` para Generic Packages**
  - *Alternativa descartada:* Remover a checagem de fail-fast do `test-ci-android-deploy.sh`.
  - *Motivo:* O contrato de fail-fast continua válido e necessário, apenas o destino mudou de R2 direto para Generic Packages (`cccp package upload`).

### 3. Critérios de Aceite
- [x] `ci.yml` aborta com `exit 1` (`Nenhum APK Android encontrado para upload nos Generic Packages`) se nenhum APK existir.
- [x] `tests/test-ci-android-deploy.sh` e sua mutação 3 validam essa mensagem atualizada e passam.
- [x] `cccp check --strict` aprovado (24/24).
