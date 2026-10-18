# Especificação Técnica: Estabilização da Esteira de CI e Compilação de APK

**ID do Ticket:** #77  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Concluído (Fase 8)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

A esteira de integração contínua do Lauda (`.forgejo/workflows/ci.yml`) estava falhando sistematicamente logo no início da execução:
1. O job `test` apontava para a imagem OCI `codeberg.org/lumenpink/gost-ci-lauda:latest`. Essa imagem nunca pôde ser hospedada no registro do Codeberg devido ao limite estrito de tamanho por pacote (HTTP `413 Request Entity Too Large`). O runner abortava a execução em 2 segundos com erro de imagem não encontrada (`not found`).
2. Como todos os jobs subsequentes (`deploy-web`, `deploy-desktop`, `deploy-apk` e `trigger-pratilera`) dependiam de `test`, nenhuma compilação de artefatos ou publicação de APK era executada.
3. Além disso, o script de empacotamento Android (`package-all.sh`) executava no runner genérico sem as ferramentas de compilação Android (Dioxus CLI `dx`, `rustup target aarch64-linux-android`, Android NDK), caindo no fallback que apenas preservava o binário estático `site/lauda-latest.apk` defasado de 16/10 (`0.5.0-dev.110`).

Esta especificação define a correção estrutural da esteira para garantir execução contínua, resiliente e hermética.

---

## 2. Requisitos e Contratos

### 2.1. Job `test` (Rust Test, Lint & E2E)
- **Imagem de Execução:** Utilizar a imagem Docker funcional `docker.io/lumenpink/gost-ci-wasm:latest` (hospedada no Docker Hub sem restrição de cota 413, já utilizada com sucesso nos jobs `deploy-web` e `deploy-desktop`).
- **Resolução de Dependências Herméticas:** Checkout determinístico dos repositórios irmãos (`ishcondidim`, `papeli`, `urupema`) em `../` caso não estejam pré-montados em `/opt/gost/deps`.
- **Validações Canônicas:**
  - `scripts/sync-version-manifests.sh`
  - `cargo fmt -- --check`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test --workspace`

### 2.2. Job `deploy-apk` (Android APK & Distribuição)
- **Compilação e Assinatura:**
  - Se a toolchain nativa estiver disponível no runner, compilar o APK Android para `aarch64` com `dx build -p lauda-client --android --renderer webview --release --target aarch64-linux-android`.
  - Injetar `versionCode` e `versionName` canônicos extraídos de `VERSION`.
  - Realizar strip com `llvm-strip` nas bibliotecas compartilhadas JNI (`libmain.so`).
  - Assinar com a keystore soberana oficial `in_twolp_release.keystore` via `apksigner`.
- **Publicação R2:**
  - Fazer upload via `wrangler r2 object put` para `twolp-bucket/lauda.apk` e `twolp-bucket/lauda-latest.apk`.

### 2.3. Job `trigger-pratilera` (Sincronização da Vitrine)
- Disparar o workflow `update-store.yml` do repositório `banca` via `workflow_dispatch` na API do Codeberg ao final do deploy com sucesso, garantindo que a loja sempre aponte para a versão compilada e assinada mais recente.

---

## 3. Critérios de Aceite
1. `.forgejo/workflows/ci.yml` com sintaxe YAML estritamente válida e sem referências a imagens inexistentes no Codeberg Packages.
2. Job `test` configurado com `docker.io/lumenpink/gost-ci-wasm:latest` e checkout das dependências irmãs.
3. Job `deploy-apk` com fluxo de compilação, assinatura e upload para o R2.
4. Rastreabilidade Gosplan estrita: checklist 9 fases e `cccp check --strict` 100% conforme.
