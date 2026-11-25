# Especificação Técnica — Ticket #144
## Publicação Atômica de Todos os Pacotes Apenas no Estágio Final (`deploy-banca`)

### 1. Contexto e Problema
Mesmo com o encadeamento `deploy-web` → `deploy-apk` → `deploy-banca`, os jobs `deploy-web` e `deploy-apk` ainda executavam `cccp package upload` individualmente ao final de cada job. Isso disparava webhooks `package created` prematuros para o `gossnab` antes de todos os artefatos (`site.tar.gz`, `lauda.apk`, binários multiarch) existirem no pacote da mesma versão.

### 2. Árvore de Decisões (Registro para a Natasha)
- **Decisão: Compilar e validar nos jobs especializados (`deploy-web` e `deploy-apk`), transferir via `actions/upload-artifact@v4` e publicar tudo atomicamente no `deploy-banca` com a mesma `--version "$VERSION"`**
  - `deploy-web` compila o Wasm, atualiza `site/` e envia `lauda-site-bundle` como artifact de workflow.
  - `deploy-apk` compila e assina os APKs, valida que `target/dist/apk/*.apk` existe (fail-fast) e envia `lauda-apks` como artifact de workflow.
  - `deploy-banca` baixa `lauda-site-bundle` e `lauda-apks`, compila os binários multiarch e publica todos (`dist/packages/*`, `target/dist/apk/*.apk`, `site`) juntos para `--version "$VERSION"`.
  - *Alternativa descartada:* Manter `cccp package upload` em cada job.
  - *Motivo:* Apenas concentrar os uploads no estágio final garante um único ciclo de gatilho completo no `gossnab` e impede que qualquer falha intermediária publique pacotes parciais na Gambi.

### 3. Critérios de Aceite
- [x] `deploy-web` e `deploy-apk` não executam `cccp package upload` diretamente.
- [x] `deploy-banca` baixa os artifacts de `deploy-web` e `deploy-apk` e publica todos os pacotes com `--version "$VERSION"`.
- [x] `e2e_apk_test`, `wasm_ui_contract_test`, `test-ci-android-deploy.sh` e `cccp check --strict` aprovados.
