# Especificação Técnica: Sincronização Periódica de E-mails via Android WorkManager

**ID do Ticket:** #105  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Especificação Preliminar (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Como o sistema operacional Android não oferece um `SyncAdapter` canônico para provedores terceiros de correio eletrônico, o Lauda necessita de um orquestrador de segundo plano para garantir a convergência periódica da caixa de entrada (anti-entropia).

O objetivo deste ticket é introduzir o `LaudaSyncWorker` utilizando o `WorkManager` da Jetpack (`androidx.work:work-runtime-ktx`), programado para executar a cada 15 minutos sob condição de rede conectada (`NetworkType.CONNECTED`). Esse worker invoca diretamente o ciclo de sincronização em Rust através de uma ponte JNI C-ABI (`nativeSyncInbox`), garantindo concorrência segura com a interface gráfica do Dioxus via SQLite WAL mode e `busy_timeout`.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: WorkManager vs JobScheduler vs AlarmManager:**
  - *Alternativa descartada (AlarmManager):* Exige acordar o dispositivo manualmente (*exact alarm*), consumindo cota restritiva no Android 12+ e drenando bateria desnecessariamente se o celular estiver offline.
  - *Alternativa descartada (JobScheduler direto):* API de baixo nível com variações problemáticas entre versões do Android e em skins de fabricantes (HyperOS, OneUI).
  - *Escolha adotada:* **WorkManager (`androidx.work:work-runtime-ktx`)**. O WorkManager abstrai o agendamento em segundo plano respeitando o Doze Mode, limites de bateria do sistema e executando apenas quando a restrição de rede ativa (`NetworkType.CONNECTED`) estiver satisfeita.
- **Decisão 2: Periodicidade e Política de Enfileiramento:**
  - *Escolha adotada:* Periodicidade de **15 minutos** (o menor intervalo permitido pelo Android para tarefas periódicas) e política **`ExistingPeriodicWorkPolicy.KEEP`**. Ao reiniciar ou reabrir o aplicativo, o agendamento existente é mantido intacto, sem reiniciar o contador nem disparar requisições em duplicidade.
- **Decisão 3: Execução Nativa em Background via JNI:**
  - *Escolha adotada:* Exportar função C-ABI `Java_in_twolp_lauda_LaudaSyncWorker_nativeSyncInbox(env, clazz, data_dir)`. O worker não precisa carregar a WebView nem instanciar componentes visuais do Dioxus; ele inicializa apenas o adaptador local de armazenamento e invoca o ciclo de sincronização de e-mails em Rust.
- **Decisão 4: Proteção contra Concorrência no SQLite (`SQLITE_BUSY`):**
  - *Escolha adotada:* Abertura das conexões do banco de dados local com `PRAGMA journal_mode = WAL;` e `PRAGMA busy_timeout = 5000;`. Isso assegura que o worker de segundo plano possa escrever novos e-mails mesmo enquanto a usuária estiver lendo mensagens na UI.

---

## 3. Contratos e Roteiro de Implementação

1. **Dependências Gradle:**
   - Injetar `implementation 'androidx.work:work-runtime-ktx:2.9.0'` nas dependências do `build.gradle` no script `publish-nightly.sh`.
2. **Worker Kotlin (`LaudaSyncWorker.kt`):**
   - Criar `lauda-client/android/LaudaSyncWorker.kt` herdando de `CoroutineWorker`.
   - Declarar o método externo `external fun nativeSyncInbox(dataDir: String): Boolean`.
   - Executar `withContext(Dispatchers.IO)` e retornar `Result.success()` ou `Result.retry()`.
3. **Ponte JNI em Rust (`lauda-client`):**
   - Implementar a função compatível com C-ABI em `lauda-client/src/adapters/jni_worker.rs`:
     `pub extern "C" fn Java_in_twolp_lauda_LaudaSyncWorker_nativeSyncInbox(...) -> jboolean`.
4. **Agendamento na `MainActivity.kt`:**
   - Configurar o agendamento de 15 minutos em `onCreate` usando `WorkManager.getInstance(applicationContext).enqueueUniquePeriodicWork(...)`.

---

## 4. Testes e Validação (DD-TDD)

- Teste de contrato em `lauda-client/tests/workmanager_sync_test.rs`:
  - Verificar a existência do arquivo `LaudaSyncWorker.kt` com a assinatura canônica de `CoroutineWorker`.
  - Verificar a injeção da dependência `androidx.work:work-runtime-ktx` em `publish-nightly.sh`.
  - Verificar o registro do agendamento periódico na `MainActivity.kt`.
  - Testar a função de sincronização em segundo plano no SQLite em modo WAL sob concorrência.
