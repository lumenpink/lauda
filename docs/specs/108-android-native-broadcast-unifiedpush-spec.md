# Especificação Técnica: BroadcastReceiver Nativo para UnifiedPush sem Dependências Externas

**ID do Ticket:** #108  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Especificação Preliminar (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Durante a execução da esteira de CI do Lauda no Forgejo (Job `deploy-apk`), a etapa de montagem do APK via Gradle falhou ao tentar resolver o artefato `org.unifiedpush.android:connector:2.1.1` no Google Maven e Maven Central. A biblioteca oficial do conector é hospedada exclusivamente no JitPack (`com.github.UnifiedPush:android-connector`), que é inacessível dentro do contêiner hermético de CI.

Como o protocolo UnifiedPush é uma especificação aberta baseada puramente em quatro `Broadcast Intents` padrão do Android, não há necessidade de adicionar bibliotecas externas ou depender de repositórios de terceiros para receber as notificações.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: BroadcastReceiver Nativo vs Dependência do JitPack:**
  - *Alternativa descartada:* Injetar o repositório `https://jitpack.io` no `settings.gradle.kts` ou `build.gradle.kts`.
    - *Motivo do descarte:* Viola o princípio de builds reprodutíveis e independentes da infraestrutura do Lauda. Ambientes de CI herméticos e proxies corporativos bloqueiam ou falham ao baixar artefatos dinâmicos do JitPack.
  - *Escolha adotada:* **`LaudaPushReceiver` herdando diretamente de `android.content.BroadcastReceiver` nativo**.
    - O protocolo UnifiedPush define formalmente:
      - `ACTION_MESSAGE` = `"org.unifiedpush.android.connector.MESSAGE"`
      - `ACTION_NEW_ENDPOINT` = `"org.unifiedpush.android.connector.NEW_ENDPOINT"`
      - `ACTION_UNREGISTERED` = `"org.unifiedpush.android.connector.UNREGISTERED"`
      - `ACTION_REGISTRATION_FAILED` = `"org.unifiedpush.android.connector.REGISTRATION_FAILED"`
    - O receptor intercepta essas ações nativamente via `onReceive(context, intent)` e extrai os campos extras `endpoint`, `instance`, `message` e `bytes`.
    - Mantém a **Regra de Ouro da Natasha: "Recebe, Enfileira e Morre"** (< 50ms) sem qualquer dependência externa no Gradle.

- **Decisão 2: Limpeza do `build.gradle.kts`:**
  - *Escolha adotada:* Remover a linha `implementation("org.unifiedpush.android:connector:2.1.1")` do script `publish-nightly.sh`. O build do APK passa a depender exclusivamente do SDK Android padrão e do `androidx.work:work-runtime-ktx` (já disponível no Google Maven).

---

## 3. Contratos e Roteiro de Implementação

1. **Receptor Kotlin (`LaudaPushReceiver.kt`):**
   - Herdar de `BroadcastReceiver`.
   - Implementar `onReceive` despachando para `onMessage`, `onNewEndpoint`, `onRegistrationFailed` e `onUnregistered`.
   - Manter `OneTimeWorkRequestBuilder<LaudaSyncWorker>()` com `setExpedited` e `ExistingWorkPolicy.KEEP`.
2. **Ajuste em `scripts/publish-nightly.sh`:**
   - Remover injeção de `org.unifiedpush.android:connector`.
   - Preservar a injeção do `<receiver>` no `AndroidManifest.xml` com as ações do UnifiedPush.
3. **Atualização dos Testes de Contrato:**
   - Atualizar `lauda-client/tests/unifiedpush_receiver_test.rs` para verificar que `LaudaPushReceiver` estende `BroadcastReceiver` nativo e não depende de pacote externo no build.
