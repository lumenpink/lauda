# Especificação — Sincronização Canônica do APK na Pratilera (Ticket #73)

## 1. Contexto e Problema
O catálogo e manifesto `index-v1.json` da loja Pratilera anunciava a versão `0.5.0` (obtida de `lauda/VERSION`), porém o binário instalado nos dispositivos dos usuários era a versão `0.4.1` (antigo `lauda-latest.apk`).

## 2. Solução Implementada
1. Recompilação do aplicativo Android com o Dioxus CLI e montagem via Gradle (`assembleDebug`).
2. Configuração canônica de `applicationId = "in.twolp.lauda"`, preservando o `namespace = "com.example.LaudaClient"` para compatibilidade com o binding do Kotlin.
3. Injeção estrita da versão canônica SemVer do CCCP (`0.5.0-dev...`) e `versionCode` atualizado.
4. Assinatura oficial com `~/.android/in_twolp_release.keystore` via `scripts/sign-release.sh`.
5. Sincronização do binário assinado com o repositório da Banca (`site/pratilera/lauda-latest.apk`) e regeneração dos manifestos F-Droid.
