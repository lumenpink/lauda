# Especificação Técnica: Esteira Automatizada Multi-Alvo na CI (Web, Android APK, Desktop e Pratilera)

**ID do Ticket:** #52  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Atualmente, o processo de integração contínua (CI) do Lauda no Codeberg Actions compila apenas a variante WebAssembly para o Codeberg Pages de fallback, enquanto:
1. A publicação Web online oficial em `lauda.2lp.in` (Cloudflare Workers) depende de credenciais e de `wrangler deploy`, permanecendo defasada.
2. A instância Web local no servidor `obizero` (`lauda.lohn.in`) servida pelo Caddy em `/home/lumen/stacks/infra/static/lauda-app` requeria intervenção manual de cópia da branch `pages`.
3. O APK nativo Android (`lauda-latest.apk`) e a atualização do repositório/catálogo F-Droid da Pratilera (`banca.2lp.in/pratilera`) dependiam de build local.
4. O binário Desktop (`lauda-client` com `--features desktop`) não possuía automação contínua.
5. Os manifestos `site/version.json` e `site/version-nightly.json` ficavam desincronizados do arquivo canônico `VERSION`, quebrando testes automatizados de distribuição (`e2e_apk_test.rs`).

Esta especificação define o contrato, o fluxo determinístico e os scripts canônicos para empacotar e distribuir automaticamente todos os alvos suportados.

---

## 2. Alvos Suportados e Contratos de Distribuição

### 2.1. Web Online Oficial (`https://lauda.2lp.in/`)
- **Mecanismo:** Cloudflare Workers com Assets Estáticos (`wrangler deploy`).
- **Configuração:** `wrangler.jsonc` (rota customizada `lauda.2lp.in`, diretório estático `./site`, binding R2 `lauda-downloads`).
- **Pré-requisito:** Autenticação via `CLOUDFLARE_API_TOKEN` e `CLOUDFLARE_ACCOUNT_ID` ou OAuth local.
- **Artefatos:** `site/app/assets/lauda-client-<hash>.js`, `site/app/assets/lauda-client_bg-<hash>.wasm`, `site/app/index.html`.

### 2.2. Web Local Obizero (`https://lauda.lohn.in/`)
- **Mecanismo:** Caddy HTTP Server apontando para montagem local em `/home/lumen/stacks/infra/static/lauda-app`.
- **Artefatos:** Cópia síncrona dos arquivos de `site/` e `site/app/` para o diretório de assets do Caddy.

### 2.3. Mobile Android APK & Pratilera (`https://banca.2lp.in/pratilera`)
- **Mecanismo:** Compilação aarch64 via Dioxus CLI (`dx build -p lauda-client --android --renderer webview --release --target aarch64-linux-android`), strip com `llvm-strip` e empacotamento.
- **Localização:** `site/lauda-latest.apk`.
- **Integração Pratilera:** Cópia para `/home/lumen/src/banca/site/pratilera/lauda-latest.apk` e execução do gerador de catálogo F-Droid (`gerar-pratilera.py`), garantindo hash SHA-256 e assinatura de `index-v1.jar`.

### 2.4. Desktop Nativo
- **Mecanismo:** Compilação Rust nativa via `cargo build -p lauda-client --features desktop --release` (ou `dx build -p lauda-client --desktop --release`).
- **Localização:** `dist/desktop/lauda-desktop` ou `target/release/lauda-client`.

### 2.5. Manifestos de Versão (`site/version.json` e `site/version-nightly.json`)
- **Formato:**
  ```json
  {
    "version": "<VERSION_CANONICA>",
    "min_compatible_version": "0.1.0",
    "apk_url": "https://lauda.2lp.in/lauda-latest.apk",
    "package_id": "in.twolp.lauda",
    "release_notes": "Build contínua do Lauda Smolclient (commit <GIT_HASH>).",
    "published_at": "<ISO8601_TIMESTAMP>"
  }
  ```

---

## 3. Scripts Canônicos e Arquitetura do Workflow

### 3.1. `scripts/sync-version-manifests.sh`
Script utilitário leve e determinístico que:
1. Lê o conteúdo de `VERSION`.
2. Extrai o commit SHA curto (`git rev-parse --short HEAD`).
3. Gera/atualiza atomicamente `site/version.json` e `site/version-nightly.json`.
4. Garante conformidade imediata com o teste `test_e2e_apk_distribution_manifest_and_obtainium`.

### 3.2. `scripts/package-all.sh`
Orquestrador de empacotamento completo que executa:
1. `sync-version-manifests.sh`
2. Build WebAssembly WebApp (`dx build -p lauda-client --platform web --release`) e ajustes de caminhos no HTML.
3. Build Desktop (`cargo build -p lauda-client --features desktop --release`).
4. Build Android APK se o ambiente possuir Android NDK/SDK ou emular fallback estruturado.
5. Sincronização local com a Pratilera (`../banca/site/pratilera`) e instância do Obizero (`../stacks/infra/static/lauda-app`).

### 3.3. Workflow `.forgejo/workflows/deploy.yml`
Atualização dos jobs de deploy para:
- Executar `scripts/sync-version-manifests.sh` antes da publicação.
- Publicar para Cloudflare Workers se `CLOUDFLARE_API_TOKEN` estiver presente.
- Publicar para Codeberg Pages (`origin pages`).
- Integrar jobs de compilação de APK e Desktop.

---

## 4. Critérios de Aceite e Verificação
1. O comando `cargo test -p lauda-server --test e2e_apk_test` deve passar com 100% de sucesso.
2. `site/version.json` deve conter a mesma versão declarada em `VERSION`.
3. O script `scripts/sync-version-manifests.sh` deve ser idempotente.
4. `scripts/package-all.sh` deve suportar flags modulares (`--web`, `--desktop`, `--apk`, `--all`).
5. A documentação do projeto e checklist Gosplan de 9 fases cumpridos rigorosamente.
