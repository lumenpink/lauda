# Especificação Técnica: Receptor de Notificação Push Sob Demanda via UnifiedPush

**ID do Ticket:** #106  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O Lauda requer notificação em tempo real (2 a 5 segundos) no recebimento de novas mensagens sem recorrer a serviços proprietários como Google FCM e sem drenar a bateria com sockets TCP ou daemons contínuos em segundo plano.

O protocolo aberto **UnifiedPush** permite que o celular delegue a conexão de rede a um aplicativo distribuidor único instalado pela usuária (como ntfy, Gotify, NextPush ou Conversations). Quando o servidor de correio detecta um novo e-mail, ele envia um HTTP POST simples ao endpoint gerado pelo distribuidor. O distribuidor acorda o Lauda via `BroadcastReceiver`.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Aplicação Estrita da Regra "Recebe, Enfileira e Morre" (< 50ms):**
  - *Contexto:* Ao disparar o `MessagingReceiver`, o Android impõe uma janela rigorosa de execução (10 a 20 segundos) e monitora agressivamente o *wakelock*. Qualquer tentativa de abrir socket TLS, falar IMAP ou escrever no SQLite dentro do receiver causa ANR, rebaixamento para baldes restritivos (*App Standby Buckets*) e perda do evento em caso de oscilação de rede.
  - *Escolha adotada:* O `LaudaPushReceiver` funciona como o carteiro que toca a campainha e vai embora imediatamente. Ele nunca abre sockets de rede nem toca no banco de dados. Ele se limita a criar um `OneTimeWorkRequest` para o `LaudaSyncWorker` com `setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)` e política `ExistingWorkPolicy.KEEP`, retornando em menos de 50ms.
- **Decisão 2: Agrupamento / Debouncing de Pushes Concorrentes:**
  - *Escolha adotada:* O uso de `ExistingWorkPolicy.KEEP` com tag única `"LaudaPushSync"` garante que, se múltiplos pushes chegarem em sequência rápida (ex.: 5 e-mails em 3 segundos), o WorkManager agrupa e executa uma única sincronização atômica consolidada.
- **Decisão 3: Desacoplamento do Envio do Endpoint:**
  - *Escolha adotada:* Ao receber `onNewEndpoint(endpoint)`, o app não faz chamada de rede síncrona; enfileira uma corrotina assíncrona desacoplada em `Dispatchers.IO` para despachar o registro ao `lauda-server`.

---

## 3. Contratos e Roteiro de Implementação

1. **Dependência Gradle:**
   - Adicionar `implementation("org.unifiedpush.android:connector:2.1.1")` no `build.gradle.kts` via `publish-nightly.sh`.
2. **Receptor Kotlin (`LaudaPushReceiver.kt`):**
   - Estender `MessagingReceiver`.
   - Implementar:
     - `onMessage(context: Context, message: ByteArray, instance: String)`
     - `onNewEndpoint(context: Context, endpoint: String, instance: String)`
     - `onRegistrationFailed(context: Context, instance: String)`
     - `onUnregistered(context: Context, instance: String)`
3. **Declaração no `AndroidManifest.xml`:**
   - Registrar `in.twolp.lauda.LaudaPushReceiver` com intent-filters:
     - `org.unifiedpush.android.connector.MESSAGE`
     - `org.unifiedpush.android.connector.NEW_ENDPOINT`
     - `org.unifiedpush.android.connector.REGISTRATION_FAILED`
     - `org.unifiedpush.android.connector.UNREGISTERED`
   - Configurar `android:exported="true"`.
4. **Automação no [publish-nightly.sh](file:///home/lumen/src/lauda/scripts/publish-nightly.sh):**
   - Sincronizar `LaudaPushReceiver.kt` para `src/main/kotlin/in/twolp/lauda/`.
   - Injetar dependência e `<receiver>` no manifesto de forma idempotente.

---

## 4. Testes e Validação (DD-TDD)

- Teste de contrato em `lauda-client/tests/unifiedpush_receiver_test.rs`:
  - Validar a existência de `LaudaPushReceiver.kt` com herança de `MessagingReceiver` e conformidade com `setExpedited` / `ExistingWorkPolicy.KEEP`.
  - Validar a declaração do `<receiver>` no manifesto em `publish-nightly.sh`.
  - Validar a injeção da dependência `org.unifiedpush.android:connector` no build Gradle.
