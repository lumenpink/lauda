# Especificação Técnica de Contratos: Persistência SQLite & Endpoints CalDAV

> **Documento de Especificação Técnica (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#38 - feat(caldav): persistência SQLite e endpoints CalDAV no lauda-server](https://codeberg.org/lumenpink/lauda/issues/38)  
> **Épico:** [#36 - feat(calendar): arquitetura soberana de agenda, caldav e fluxo continuo](https://codeberg.org/lumenpink/lauda/issues/36)  
> **Norma Base:** GOST 1.4.0 / RFC 4791 (CalDAV) / RFC 4918 (WebDAV) / RFC 6578 (Sync-Collection) / RFC 6764 (Service Discovery)

---

## 1. Visão Geral da Arquitetura

O `lauda-server` passa a gerenciar compromissos e calendários de forma soberana e persistente, implementando:
1. **Motor Relacional SQLite em Modo WAL (`calendar.db`):** Armazena eventos e metadados indexados por fatias temporais (`start_ts`, `end_ts`) e carimbos de sincronização com soft-delete para replicação idempotente.
2. **Endpoints Canônicos CalDAV (RFC 4791 / RFC 6578):** Expostos sobre Axum com suporte a clientes de mercado (Thunderbird, DAVx⁵, Fossify/Simple Calendar, iOS/macOS Calendar).
3. **Interoperabilidade Total:** Tradução direta entre os modelos canônicos do `lauda-types` (`CalendarEvent`) e os payloads brutos `.ics` (iCalendar RFC 5545).

---

## 2. Esquema Relacional SQLite (`calendar.db`)

Gerenciado pelo adaptador `SqliteCalendarStore`:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS calendar_events (
    id TEXT PRIMARY KEY,                       -- UUID v4 canônico ou UID RFC 5545
    username TEXT NOT NULL,                    -- Dono da agenda
    summary TEXT NOT NULL,                     -- Título do evento
    description TEXT,                          -- Descrição / notas
    location TEXT,                             -- Localização
    start_ts INTEGER NOT NULL,                 -- Timestamp Unix UTC em segundos
    end_ts INTEGER NOT NULL,                   -- Timestamp Unix UTC em segundos
    is_all_day INTEGER NOT NULL DEFAULT 0,     -- Booleano (1 se dia inteiro)
    rrule TEXT,                                -- String de recorrência RRULE se houver
    ics_raw TEXT NOT NULL,                     -- Payload canônico VEVENT em RFC 5545
    etag TEXT NOT NULL,                        -- Hash de integridade do payload ICS
    version INTEGER NOT NULL DEFAULT 1,        -- Versão sequencial de modificação
    created_at INTEGER NOT NULL,               -- Timestamp Unix Epoch (segundos)
    updated_at INTEGER NOT NULL,               -- Timestamp Unix Epoch (segundos)
    deleted_at INTEGER                         -- Tombstone para sync incremental
);

CREATE INDEX IF NOT EXISTS idx_calendar_user_time 
    ON calendar_events(username, start_ts, end_ts);

CREATE INDEX IF NOT EXISTS idx_calendar_user_updated 
    ON calendar_events(username, updated_at);

CREATE INDEX IF NOT EXISTS idx_calendar_user_deleted 
    ON calendar_events(username, deleted_at);
```

---

## 3. Endpoints CalDAV e Rotas HTTP

### 3.1 Descoberta de Serviço (RFC 6764)
- **`GET /.well-known/caldav`**:
  - Responde `301 Moved Permanently` com header `Location: /dav/calendars/`.

### 3.2 Capacidades e Home-Set (`OPTIONS` & `PROPFIND`)
- **`OPTIONS /dav/calendars/`** e **`/dav/calendars/{username}/default/`**:
  - Headers:
    - `Allow: OPTIONS, GET, HEAD, POST, PUT, DELETE, PROPFIND, REPORT`
    - `DAV: 1, 3, calendar-access, access-control, sync-collection`
- **`PROPFIND /dav/calendars/`**:
  - `Depth: 0`: Devolve `current-user-principal` e `calendar-home-set` apontando para `/dav/calendars/{username}/`.
- **`PROPFIND /dav/calendars/{username}/`**:
  - `Depth: 1`: Devolve informações da coleção `/dav/calendars/{username}/default/`:
    - `resourcetype`: `<D:collection/><C:calendar xmlns:C="urn:ietf:params:xml:ns:caldav"/>`
    - `supported-calendar-component-set`: `<C:comp name="VEVENT"/>`
    - `displayname`: `"Agenda Lauda"`
    - `sync-token`: Token de sincronização (ex: `ts:1728800000`).

### 3.3 Consultas, Relatórios e Sincronização (`REPORT`)
- **`REPORT /dav/calendars/{username}/default/`**:
  - Suporta três tipos de payload XML:
    1. **`calendar-query` (RFC 4791 §9.5):**
       - Filtra eventos pela faixa temporal (`<C:time-range start="..." end="..."/>`).
       - Retorna XML `multistatus` com os eventos correspondentes contendo `<C:calendar-data>` e `<D:getetag>`.
    2. **`calendar-multiget` (RFC 4791 §9.6):**
       - Extrai lista de `<D:href>` solicitados pelo cliente e devolve seus respectivos `.ics`.
    3. **`sync-collection` (RFC 6578):**
       - Extrai `<D:sync-token>` enviado pelo cliente.
       - Consulta alterações desde o token.
       - Retorna `<D:response>` com status `HTTP/1.1 200 OK` para novos/alterados e `HTTP/1.1 404 Not Found` para deletados (tombstones).
       - Devolve `<D:sync-token>` atualizado.

### 3.4 Manipulação de Eventos (`GET`, `PUT`, `DELETE`)
- **`GET /dav/calendars/{username}/default/{id}.ics`**:
  - Devolve payload iCalendar cru (`text/calendar; charset=utf-8`) com header `ETag`.
- **`PUT /dav/calendars/{username}/default/{id}.ics`**:
  - Recebe o `.ics` enviado pelo cliente.
  - Converte para `CalendarEvent` via `CalendarEvent::from_ical`.
  - Persiste no SQLite com geração de novo `ETag` e incremento de versão.
  - Responde `201 Created` ou `204 No Content` com header `ETag`.
- **`DELETE /dav/calendars/{username}/default/{id}.ics`**:
  - Marca `deleted_at = now()` no SQLite gerando tombstone para replicação.
  - Responde `204 No Content`.

---

## 4. Autenticação e Segurança
- Autenticação HTTP Basic (`Authorization: Basic base64(user:pass)`) ou Bearer Token validando as credenciais no backend Maildir/MUA existente.
