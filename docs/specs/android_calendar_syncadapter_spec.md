# Especificação Técnica de Contratos: Android Calendar SyncAdapter Soberano

> **Documento de Especificação Técnica e Contratos Preliminares (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#40 - feat(calendar): android calendar syncadapter sob in.twolp.lauda](https://codeberg.org/lumenpink/lauda/issues/40)  
> **Épico:** [#36 - feat(calendar): arquitetura soberana de agenda, caldav e fluxo continuo](https://codeberg.org/lumenpink/lauda/issues/36)  
> **Norma Base:** GOST 1.4.0 / RFC 5545 (iCalendar) / Android CalendarContract API (API 14+)

---

## 1. Visão Geral da Arquitetura

O `lauda-client` no Android integra-se diretamente ao subsistema nativo `CalendarProvider` do sistema operacional. Isso garante total interoperabilidade com:
- Widgets de tela inicial de qualquer fabricante (AOSP, Samsung, Google, etc.).
- Clientes de calendário locais (como Fossify/Simple Calendar, Etar).
- Notificações de compromissos geradas pelo próprio sistema operacional.
- Sincronização periódica e em segundo plano em modo econômico de bateria (JobScheduler/WorkManager).

```
                ┌──────────────────────────────────────────────┐
                │          lauda-server (Rust / Linux)         │
                │        SQLite Calendar Store + CalDAV        │
                └──────────────────────┬───────────────────────┘
                                       │ CalDAV / Sync-Collection (RFC 6578)
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Android Device (Obtainium)                          │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    lauda-types / lauda-client (Rust)                  │  │
│  │  • CalendarEvent: RFC 5545 puro (UID, DTSTART, DTEND, RRULE, STATUS)  │  │
│  │  • AndroidCalendarSync: Mapeador determinístico para CalendarContract │  │
│  │  • AndroidCalendarBatch: Lote atômico de operações ContentProvider    │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │ JNI / Bridge                         │
│  ┌───────────────────────────────────▼───────────────────────────────────┐  │
│  │   Android Calendar SyncAdapter & Service (in.twolp.lauda)             │  │
│  │   (LaudaCalendarSyncService, calendar_syncadapter.xml)                │  │
│  └───────────────────────────────────┬───────────────────────────────────┘  │
│                                      │ ContentProviderOperation.applyBatch  │
│  ┌───────────────────────────────────▼───────────────────────────────────┐  │
│  │               Android Native CalendarProvider                         │  │
│  │       (Widgets da Home, Fossify Calendar, Notificações do SO)         │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Metadados e Serviços Android (`lauda-client/android/`)

### 2.1 Identificador de Conta & Autoridade
- **Tipo de Conta:** `in.twolp.lauda` (compartilhado entre Correio, Contatos e Agenda).
- **Nome da Conta:** E-mail do usuário autenticado (ex: `hi@lumen.pink` ou `lumen`).
- **Autoridade Vinculada:** `com.android.calendar`.

### 2.2 Estrutura Declarativa XML (`res/xml/calendar_syncadapter.xml`)
```xml
<?xml version="1.0" encoding="utf-8"?>
<sync-adapter xmlns:android="http://schemas.android.com/apk/res/android"
    android:contentAuthority="com.android.calendar"
    android:accountType="in.twolp.lauda"
    android:userVisible="true"
    android:supportsUploading="true"
    android:allowParallelSyncs="false"
    android:isAlwaysSyncable="true" />
```

### 2.3 Serviço Kotlin (`LaudaCalendarSyncService.kt`)
Implementa `AbstractThreadedSyncAdapter` e orquestra a execução da sincronização consumindo as operações geradas pela camada Rust.

---

## 3. Mapeamento de Contratos (`CalendarEvent` ↔ `CalendarContract.Events`)

Cada evento Lauda é projetado nas tabelas do `CalendarProvider`:

| Campo `CalendarEvent` | Coluna Android (`CalendarContract.Events`) | Tipo / Transformação |
| :--- | :--- | :--- |
| `uid` | `_SYNC_ID` | String única soberana |
| `summary` | `TITLE` | String de título do evento |
| `description` | `DESCRIPTION` | Texto detalhado opcional |
| `location` | `EVENT_LOCATION` | String de local |
| `start` | `DTSTART` | Timestamp em milissegundos UTC |
| `end` | `DTEND` | Timestamp em milissegundos UTC |
| `is_all_day` | `ALL_DAY` | Inteiro: `1` (dia inteiro) ou `0` |
| `rrule` | `RRULE` | String iCalendar (ex: `FREQ=WEEKLY;INTERVAL=1`) |
| `status` | `STATUS` | `1` (Confirmed), `0` (Tentative), `2` (Cancelled) |
| Fuso Horário | `EVENT_TIMEZONE` | Estritamente `"UTC"` |
| Conta | `ACCOUNT_NAME` & `ACCOUNT_TYPE` | `{account_name}` & `"in.twolp.lauda"` |

---

## 4. Estruturas em Rust (`lauda-types::android_calendar_sync`)

```rust
pub const LAUDA_CALENDAR_AUTHORITY: &str = "com.android.calendar";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidCalendarOp {
    InsertEvent {
        calendar_id: i64,
        account_name: String,
        account_type: String,
        sync_id: String,
        title: String,
        description: Option<String>,
        location: Option<String>,
        dtstart_ms: i64,
        dtend_ms: i64,
        all_day: bool,
        rrule: Option<String>,
        status: i32,
    },
    UpdateEvent {
        sync_id: String,
        title: String,
        description: Option<String>,
        location: Option<String>,
        dtstart_ms: i64,
        dtend_ms: i64,
        all_day: bool,
        rrule: Option<String>,
        status: i32,
    },
    DeleteEvent {
        sync_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidCalendarBatch {
    pub account_name: String,
    pub operations: Vec<AndroidCalendarOp>,
}
```

---

## 5. Critérios de Aceite e Fases TDD

1. **Red Test:** Testes em `lauda-types/tests/android_calendar_sync_test.rs` validando mapeamento de eventos com horário fixo, dia inteiro, recorrência, tombstones de deleção e conversão de status.
2. **Green Implement:** Implementação do módulo `android_calendar_sync` em `lauda-types` e os arquivos nativos em `lauda-client/android`.
3. **Quality Gate:** 100% dos testes da workspace passando, zero warnings no Clippy e auditoria GOST (`cccp check --strict`) aprovada com 100%.
