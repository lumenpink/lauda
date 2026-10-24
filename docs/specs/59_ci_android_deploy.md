# Especificação Técnica — Integração do Container gost-ci-android no Workflow de Deploy (#59)

- **ID do Ticket:** #59
- **Status:** Concluído (Fases 1 a 8 validadas)
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
O aplicativo Android do Lauda distribuído via Pratilera (catálogo F-Droid em `banca.2lp.in/pratilera`) e via bucket Cloudflare R2 estava congelado na versão `dev-138` (com `versionCode: 26101811`), mesmo com versões posteriores (`dev-244` / `dev-252`) já desenvolvidas.
A causa raiz identificada foi a ausência de um ambiente hermético com Android SDK e NDK no job `deploy-apk` do workflow de CI ([`.forgejo/workflows/ci.yml`](file:///home/lumen/src/lauda/.forgejo/workflows/ci.yml)), fazendo com que a compilação nativa falhasse silenciosamente e reutilizasse o binário estático antigo com versionCode defasado.

Com a criação e publicação da imagem canônica `gambi.2lp.in/lumen/gost-ci-android:latest` (ticket #24 do `gost-ci`), o Lauda agora dispõe de um ambiente oficial contendo OpenJDK 17, Android SDK 34, Android NDK r27c, Dioxus CLI (`dx`) e toolchain Rust com os alvos `aarch64-linux-android` e `x86_64-linux-android`.

## 2. Requisitos Técnicos
1. **Containerização do Job `deploy-apk`:**
   - O job `deploy-apk` em `.forgejo/workflows/ci.yml` DEVE declarar:
     ```yaml
     container:
       image: gambi.2lp.in/lumen/gost-ci-android:latest
     ```
   - O job deve manter seus passos de compilação, assinatura e sincronização.
2. **Parametrização Flexível do Script `scripts/publish-nightly.sh`:**
   - As variáveis de ambiente de ferramentas devem utilizar valores do ambiente do container quando presentes, com fallback para caminhos legados caso executado localmente:
     - `JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}"`
     - `ANDROID_HOME="${ANDROID_HOME:-/opt/android-sdk}"`
     - `ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-/opt/android-sdk/ndk/27.2.12479018}"`
     - `NDK_HOME="${NDK_HOME:-$ANDROID_NDK_HOME}"`
   - O binário `LLVM_STRIP` deve ser resolvido dinamicamente a partir de `$ANDROID_NDK_HOME`:
     - `LLVM_STRIP="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"`
   - O binário do Dioxus CLI (`dx`) deve respeitar o binário do sistema se disponível:
     - `DX_BIN="$(command -v dx 2>/dev/null || echo "/home/lumen/.local/dx-0.7/bin/dx")"`
3. **Geração Incremental do `versionCode`:**
   - Garantir que cada compilação de nightly utilize `VERSION_CODE="$(date +%y%m%d%H)"` para que o versionCode seja estritamente monotônico e superior a `26101811`, permitindo que os clientes Android e F-Droid detectem a atualização imediatamente.
4. **Soberania de Repositórios Git:**
   - Quaisquer referências nos scripts a forges de código devem apontar estritamente para `gambi.2lp.in` (veto absoluto a `codeberg.org`).

## 3. Critérios de Aceitação
- Teste executável de conformidade em `tests/test-ci-android-deploy.sh` que verifique:
  - Declaração do container `gambi.2lp.in/lumen/gost-ci-android:latest` no job `deploy-apk`.
  - Fallbacks não-bloqueantes e variáveis de ambiente dinâmicas em `scripts/publish-nightly.sh`.
  - Ausência de caminhos hardcoded de NDK em `scripts/package-all.sh` e `scripts/publish-nightly.sh`.
  - Testes de mutação validando sensibilidade a regressões nessas configurações.
- Aprovação estrita no `cccp check --strict`.
