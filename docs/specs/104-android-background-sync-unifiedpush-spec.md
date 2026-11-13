# Especificação Técnica: Sincronização Nativa em Segundo Plano no Android (SyncAdapter, WorkManager e UnifiedPush)

**ID do Ticket:** #104  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Especificação Preliminar (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O aplicativo móvel Lauda (`lauda-client`) necessita de uma estratégia robusta e energeticamente eficiente para manter os dados atualizados em segundo plano no ecossistema Android, cobrindo duas realidades funcionais distintas:

1. **Mundo 1 (Contatos e Calendário):** Devem ser cidadãos de primeira classe no sistema operacional. Os contatos devem estar disponíveis diretamente no discador telefônico nativo, no WhatsApp e no Signal; os compromissos devem aparecer no calendário do sistema, widgets de tela inicial e disparar alarmes nativos.
2. **Mundo 2 (Correio Eletrônico):** O Android não possui um `ContentProvider` nem `SyncAdapter` canônico do sistema para e-mails de terceiros. A sincronização de mensagens exige entrega quase em tempo real (2-5 segundos) sem recorrer ao Google Play Services (FCM) e sem manter um processo com socket TCP permanentemente acordado (o que drenaria bateria e seria sumariamente encerrado pelo gerenciador de energia do Android / Doze Mode).

Para atender a esses dois mundos de forma autônoma e colaborativa, estabelecemos uma arquitetura dividida:
- Para Contatos e Agenda: **SyncAdapter Nativo** conectado aos contratos `ContactsContract` e `CalendarContract` do Android, acionado pelo `ContentResolver`.
- Para E-mails: **Abordagem Dupla (UnifiedPush + WorkManager)**, combinando acionamento imediato por push livre com rede de convergência periódica de 15 minutos (anti-entropia).

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

### Decisão 1: Mundo 1 — SyncAdapter Nativo vs Armazenamento Isolado no App
- **Alternativa descartada:** Armazenamento isolado de contatos e compromissos apenas no SQLite local do Lauda, acessível apenas pela UI Web/Dioxus do app.
  - *Motivo do descarte:* Inutilizaria o propósito de uma suíte colaborativa no celular. Ao receber uma ligação telefônica, o discador do Android não saberia o nome do contato; o WhatsApp/Signal não reconheceriam as pessoas na agenda; e a pessoa usuária não veria seus eventos nos widgets do relógio ou calendário do sistema.
- **Escolha adotada:** **SyncAdapter Nativo com Conta de Usuário (`in.twolp.lauda`)**.
  - O Lauda registra uma conta no `AccountManager` do Android e dois serviços de sincronização (`LaudaSyncService` para `com.android.contacts` e `LaudaCalendarSyncService` para `com.android.calendar`).
  - O módulo Rust `lauda-types` já fornece os transformadores determinísticos `android_sync.rs` e `android_calendar_sync.rs`, gerando lotes atômicos de `ContentProviderOperation`. O Java/Kotlin apenas aplica o lote via `ContentResolver.applyBatch`. O Android cuida de agendar e coordenar as sincronizações sem necessidade de daemons manuais.

### Decisão 2: Mundo 2 — Abordagem Dupla (UnifiedPush + WorkManager) vs Alternativas
- **Alternativa descartada 1:** Foreground Service permanente com notificação fixa na barra de status mantendo conexão TCP/WebSocket IMAP aberta.
  - *Motivo do descarte:* Drena severamente a bateria do celular, degrada a experiência do usuário com poluição visual obrigatória na barra de status e frequentemente é encerrado por fabricantes agressivos (Xiaomi MIUI, Samsung OneUI) quando a memória aperta.
- **Alternativa descartada 2:** Uso exclusivo do Firebase Cloud Messaging (FCM) do Google.
  - *Motivo do descarte:* Viola frontalmente os princípios de soberania e autonomia técnica do Lauda. Cria dependência obrigatória do Google Play Services e exige contas corporativas centralizadas no console do Google.
- **Alternativa descartada 3:** Apenas WorkManager periódico de 15 minutos (sem push).
  - *Motivo do descarte:* Latência inaceitável para recebimento de comunicações urgentes. O usuário teria que aguardar até 15 minutos para saber de uma nova mensagem.
- **Alternativa descartada 4:** Apenas UnifiedPush (sem WorkManager).
  - *Motivo do descarte:* Falta de resiliência. Se o aparelho ficar temporariamente sem sinal, no modo avião, reiniciar ou o aplicativo distribuidor sofrer uma falha transitória de socket, a notificação push pode se perder e a caixa ficaria desatualizada até a próxima abertura manual do app.
- **Escolha adotada:** **Abordagem Dupla Cinto-e-Suspensório**.
  - **UnifiedPush (Sob Demanda / Tempo Real):** Quando chega um e-mail novo no servidor, o `lauda-server` (já notificado pelo `inotify` no Maildir implementado no Ticket #99) dispara um `HTTP POST` simples para a URL de endpoint registrada pelo app. O distribuidor no celular (ex: ntfy, Gotify, Conversations) repassa o cutucão ao Lauda via `BroadcastReceiver`, que agenda uma sincronização imediata com alta prioridade.
  - **WorkManager (Anti-Entropia Periódica):** Um `PeriodicWorkRequest` de 15 minutos roda com restrição de rede ativa (`NetworkType.CONNECTED`). Ele atua como um coletor de entropia para reconciliar tudo caso algum evento push tenha se perdido ou falhado.

### Decisão 3: A Regra de Ouro no Push — "Recebe, Enfileira e Morre"
- **Contexto Operacional e Risco:**
  Quando um push chega no `MessagingReceiver` (UnifiedPush) ou no `FirebaseMessagingService` (FCM), o sistema operacional Android (especialmente distribuições com agressivo gerenciamento de energia como HyperOS/MIUI e OneUI) abre uma janela de execução extremamente curta e monitorada (geralmente entre 10 e 20 segundos).
- **Três Riscos Fatais ao Tentar Sincronizar Direto no Receptor:**
  1. *Morte Súbita por Timeout (ANR / Background Crash):* Se o 4G/Wi-Fi oscilar ou o servidor demorar mais de 10s para responder ao handshake TLS/IMAP, o Android encerra o processo sumariamente no meio da escrita no SQLite, corrompendo o estado da sincronização.
  2. *Rebaixamento de Cota (App Standby Buckets):* O Android monitora rigorosamente o tempo que o receptor retém o processador acordado via wakelock. Se demorar, o app é rebaixado para baldes restritivos (*Rare* ou *Restricted*), atrasando os próximos pushes.
  3. *Sem Retentativa Automática:* BroadcastReceivers não possuem mecanismo nativo de retry. Se houver falha de rede ou timeout dentro dele, o evento se perde.
- **Fronteira Estrita de Responsabilidade:**
  - **O Receptor de Push (`LaudaPushReceiver`):** atua unicamente como carteiro que toca a campainha e vai embora. Seu tempo de vida deve ser inferior a 50 milissegundos. Ele apenas extrai o mínimo de metadados, enfileira um `OneTimeWorkRequest` acelerado (`setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)`) com política `ExistingWorkPolicy.KEEP` (ou `APPEND_OR_REPLACE`), e retorna imediatamente, liberando o wakelock. **É terminantemente proibido abrir conexões de rede ou tocar no SQLite pesado dentro do receptor.**
  - **O WorkManager (`LaudaSyncWorker`):** é a única entidade autorizada a abrir sockets de rede, negociar TLS, baixar mensagens e executar I/O no banco. Ele obtém janela de execução estendida (até 10 minutos), faz *debouncing* automático de rajadas de e-mails (múltiplos pushes em segundos consolidam em uma única execução) e garante entrega resiliente caso a conexão caia temporariamente.

### Decisão 4: Concorrência e Integridade no SQLite Local (`lauda.db`)
- **Problema identificado:** A UI do Dioxus (rodando na WebView) e os processos em segundo plano (`SyncAdapter` e `LaudaSyncWorker`) acessam simultaneamente o arquivo SQLite do cliente.
- **Escolha adotada:** Configurar obrigatoriamente no inicializador da base SQLite local:
  - `PRAGMA journal_mode = WAL;` (permite leitores e escritores concorrentes sem travamento de arquivo).
  - `PRAGMA busy_timeout = 5000;` (espera até 5 segundos em caso de contenção pontual de lock em vez de abortar com `SQLITE_BUSY`).
  - `PRAGMA synchronous = NORMAL;` (alta performance e integridade garantida no modo WAL).

---

## 3. Arquitetura e Fluxo de Dados

### 3.1. Fluxo de Notificação em Tempo Real (UnifiedPush)

```
[Mundo Exterior]
       │
       ▼ (Entrega SMTP)
┌────────────────────────────────────────────────────────┐
│ lauda-server (Linux / VPS)                             │
│  1. Mensagem gravada no Maildir (new/)                 │
│  2. inotify detecta criação de arquivo                 │
│  3. Consulta tabela de push endpoints registrados      │
│  4. Dispara HTTP POST assíncrono para o endpoint       │
└──────────────────────────┬─────────────────────────────┘
                           │ HTTP POST (sem payload sensível)
                           ▼
┌────────────────────────────────────────────────────────┐
│ Servidor Distribuidor UnifiedPush (ex: ntfy.sh)        │
└──────────────────────────┬─────────────────────────────┘
                           │ WebSocket / SSE seguro
                           ▼
┌────────────────────────────────────────────────────────┐
│ Celular da Pessoa Usuária                              │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │ App Distribuidor (ntfy / Gotify / NextPush)       │  │
│  │ (Mantém socket único para vários apps do celular)│  │
│  └───────────────────────┬──────────────────────────┘  │
│                          │ Intent Broadcast local      │
│                          ▼                             │
│  ┌──────────────────────────────────────────────────┐  │
│  │ lauda-client (Android)                           │  │
│  │  1. LaudaPushReceiver acorda (< 50ms)            │  │
│  │  2. setExpedited + ExistingWorkPolicy.KEEP       │  │
│  │  3. LaudaSyncWorker invoca JNI sync_all_cycle()  │  │
│  │  4. Baixa cabeçalhos e prefetch de mensagens     │  │
│  │  5. Emite notificação nativa de novo e-mail      │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

### 3.2. Fluxo de Sincronização Nativa de Contatos e Agenda (SyncAdapter)

```
┌────────────────────────────────────────────────────────┐
│ Android OS (ContentResolver / SyncManager)            │
│  - Disparo periódico, sob demanda ou por conta nativa  │
└──────────────────────────┬─────────────────────────────┘
                           │ onPerformSync(account, authority, ...)
                           ▼
┌────────────────────────────────────────────────────────┐
│ LaudaSyncService / LaudaCalendarSyncService (Kotlin)  │
│  - Recebe evento do Android OS no processo em bg      │
└──────────────────────────┬─────────────────────────────┘
                           │ JNI C-ABI
                           ▼
┌────────────────────────────────────────────────────────┐
│ lauda-client / lauda-types (Rust)                      │
│  1. Reconciliação incremental com lauda-server         │
│  2. Normalização telefônica (E.164 BR) / BDAY vCard    │
│  3. Geração de lote ContentProviderOperation atômico   │
└──────────────────────────┬─────────────────────────────┘
                           │ Retorna lote de operações para JVM
                           ▼
┌────────────────────────────────────────────────────────┐
│ ContentResolver.applyBatch(authority, operations)      │
│  - ContactsContract: Discador, WhatsApp, Signal        │
│  - CalendarContract: Calendário do SO, Alarme, Widgets │
└────────────────────────────────────────────────────────┘
```

---

## 4. Roteiro de Implementação em Fases

Para manter a conformidade estrita com o GOST DD-TDD e facilitar a revisão da Natasha, o trabalho é particionado em 3 frentes incrementais:

### Fase A: Mundo 1 — Integração dos SyncAdapters Nativo no APK
1. **Manifesto e Permissões:**
   - Injetar no `AndroidManifest.xml` durante `scripts/publish-nightly.sh`:
     - `READ_CONTACTS`, `WRITE_CONTACTS`
     - `READ_CALENDAR`, `WRITE_CALENDAR`
     - `GET_ACCOUNTS`, `MANAGE_ACCOUNTS`, `AUTHENTICATE_ACCOUNTS`
     - `READ_SYNC_SETTINGS`, `WRITE_SYNC_SETTINGS`
2. **Serviços Kotlin:**
   - Garantir a compilação de `LaudaAuthenticatorService.kt`, `LaudaSyncService.kt` e `LaudaCalendarSyncService.kt` em `lauda-client/android/`.
   - Ligar a invocação do `sync_contacts` e `sync_calendar` via JNI C-ABI, consumindo os vetores de `ContentProviderOperation` de `lauda-types`.

### Fase B: Mundo 2 — WorkManager Anti-Entropia Periódico
1. **Dependências Gradle:**
   - Adicionar `androidx.work:work-runtime-ktx:2.9.0` no `app/build.gradle`.
2. **Implementação do Worker:**
   - Criar `LaudaSyncWorker.kt` (estendendo `CoroutineWorker`).
   - Configurar `PeriodicWorkRequestBuilder<LaudaSyncWorker>(15, TimeUnit.MINUTES)` na inicialização da `MainActivity`.
   - Adicionar restrições: `Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()`.
3. **Ponte JNI:**
   - Exportar `Java_in_twolp_lauda_LaudaSyncWorker_nativeSyncAll` em `lauda-client` para executar o ciclo incremental de mensagens em Rust sem abrir a interface gráfica.

### Fase C: Mundo 2 — Conector UnifiedPush (Sob Demanda)
1. **No Cliente (Android):**
   - Adicionar `org.unifiedpush.android:connector:2.1.1` no `build.gradle`.
   - Implementar `LaudaPushReceiver : MessagingReceiver` seguindo à risca a regra *"Recebe, Enfileira e Morre"* (< 50ms):
     - `onNewEndpoint`: despacha o endpoint ao servidor em tarefa assíncrona desacoplada (`POST /api/push/register`).
     - `onMessage`: apenas cria e despacha um `OneTimeWorkRequestBuilder<LaudaSyncWorker>()` configurado com `setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)` e `ExistingWorkPolicy.KEEP`. Retorna imediatamente sem abrir rede nem I/O.
     - `onUnregistered`: notifica o servidor via `DELETE /api/push/unregister`.
2. **No Servidor (`lauda-server`):**
   - Endpoints REST para registro e desregistro de endpoints de push associados à conta da usuária.
   - Integração do disparador de webhook no loop do `inotify` do `MaildirBackend`: ao detectar novo arquivo na pasta `new/`, dispara `POST` assíncrono para o endpoint com timeout curto (3s) e retry leve.

---

## 5. Estratégia de Testes e Quality Gate (DD-TDD)

1. **Testes Unitários no Servidor (`lauda-server`):**
   - Teste de cadastro, consulta e invalidação de endpoints de push no banco.
   - Teste de despacho de notificação HTTP em mock local de endpoint.
2. **Testes de Concorrência SQLite:**
   - Teste automatizado de concorrência abrindo múltiplas conexões em threads simultâneas simulando escrita em lote pelo worker e leitura pela UI, garantindo zero ocorrências de `SQLITE_BUSY` sob WAL mode.
3. **Validação de Build e Empacotamento:**
   - Validação da injeção do `AndroidManifest.xml` e compilação do APK via `./gradlew assembleDebug` no `scripts/publish-nightly.sh`.

---

## 6. Cuidados e Manutenção Futura (Guardiã da Natasha)

1. **Sobrescrita do Dioxus CLI:** O comando `dx build` recria arquivos temporários no diretório de build. O script `scripts/publish-nightly.sh` é o guardião responsável por manter e reinjetar os arquivos Kotlin, XMLs e manifesto após o comando `dx`, garantindo que nada seja apagado em builds limpos.
2. **Gerenciamento de Bateria do Distribuidor:** O Lauda **não** deve pedir para sair da otimização de bateria; quem precisa de permissão de segundo plano irrestrito no celular é exclusivamente o aplicativo distribuidor (ntfy, Gotify, etc.), já que é ele quem centraliza a conexão TCP com a internet para todos os apps UnifiedPush do aparelho.
