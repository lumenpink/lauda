# Especificação Técnica: Smart Intent Parser e Modelos iCalendar RFC 5545

> **Documento de Especificação Técnica (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#37 - feat(calendar): smart intent parser e tipos iCalendar RFC 5545 no lauda-types](https://codeberg.org/lumenpink/lauda/issues/37)  
> **Épico:** [#36 - feat(calendar): arquitetura soberana de agenda, caldav e fluxo continuo](https://codeberg.org/lumenpink/lauda/issues/36)  
> **Norma Base:** GOST 1.4.0 / RFC 5545 (Internet Calendaring and Scheduling Core Object Specification - iCalendar)

---

## 1. Visão Geral e Princípios de Design

A gestão tradicional de calendários falha pela sobrecarga cognitiva de formulários com dezenas de campos para registrar compromissos rotineiros. O `lauda-types` introduz o núcleo determinístico e interoperável da agenda soberana do Lauda:

1. **Modelos Canônicos Puros:** Estruturas de dados em Rust puro (`CalendarEvent`, `EventTime`, `RecurrenceRule`, `Attendee`), serializáveis via `serde`, compatíveis com compilação WebAssembly (Wasm) e zero dependências pesadas de runtime.
2. **Smart Intent Parser (Linguagem Natural PT-BR / EN):** Motor de interpretação de texto livre determinístico, offline e sem IA externa, capaz de extrair título, datas relativas, horários, intervalos, locais e regras de repetição a partir de uma única linha.
3. **iCalendar Engine (RFC 5545):** Parser e serializador bidirecional estrito para `VEVENT` e `RRULE`, garantindo conformidade com padrões globais, CalDAV e processamento de convites `.ics`.

---

## 2. Modelos Canônicos de Dados (`calendar.rs`)

### 2.1. `CalendarEvent`
Estrutura canônica de um evento de calendário:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,                           // UUID canônico ou UID RFC 5545
    pub summary: String,                       // Título limpo do evento
    pub description: Option<String>,           // Descrição / anotações
    pub location: Option<String>,              // Local (ex: "Fasano", "Sala 2")
    pub start: EventTime,                      // Timestamp de início
    pub end: EventTime,                        // Timestamp de término
    pub is_all_day: bool,                      // Evento de dia inteiro
    pub rrule: Option<RecurrenceRule>,         // Regra de recorrência RFC 5545
    pub status: EventStatus,                   // Confirmed, Tentative, Cancelled
    pub organizer: Option<EventAttendee>,      // Organizador do evento
    pub attendees: Vec<EventAttendee>,         // Participantes e status RSVP
    pub sequence: u32,                         // Versão sequencial de revisão
    pub created_at: Option<i64>,               // Unix epoch (segundos)
    pub updated_at: Option<i64>,               // Unix epoch (segundos)
}
```

### 2.2. `EventTime`
Representação universal para instantes de calendário:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EventTime {
    DateTimeUtc(i64),                          // Timestamp Unix em segundos
    DateOnly { year: i32, month: u32, day: u32 }, // Dia inteiro sem fuso
}
```

Métodos canônicos:
- `to_ical_string(&self) -> String`: Formata como `20261014T153000Z` ou `20261014`.
- `from_ical_string(s: &str) -> Result<Self, CalendarError>`.
- `to_timestamp_utc(&self) -> i64`.

### 2.3. `RecurrenceRule` & `RecurrenceFreq`
Representação tipada de regras `RRULE`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFreq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub freq: RecurrenceFreq,
    pub interval: u32,                         // Padrão: 1
    pub count: Option<u32>,                    // Número de repetições
    pub until: Option<EventTime>,              // Data limite
    pub by_day: Vec<Weekday>,                  // Dias da semana aplicáveis
}
```

Métodos canônicos:
- `to_rrule_string(&self) -> String`: Gera string conforme RFC 5545 (ex: `FREQ=WEEKLY;INTERVAL=1;BYDAY=TU`).
- `from_rrule_string(s: &str) -> Result<Self, CalendarError>`.

### 2.4. `EventAttendee` e `RsvpStatus`
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RsvpStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub status: RsvpStatus,
}
```

---

## 3. Especificação do Smart Intent Parser

O parser converte uma string livre em um `ParsedCalendarIntent`.

### 3.1. Assinatura do Parser
```rust
pub struct CalendarReferenceTime {
    pub year: i32,
    pub month: u32,   // 1..=12
    pub day: u32,     // 1..=31
    pub hour: u32,    // 0..=23
    pub minute: u32,  // 0..=59
    pub weekday: Weekday,
    pub timestamp_utc: i64,
}

pub struct ParsedCalendarIntent {
    pub summary: String,
    pub start: EventTime,
    pub end: EventTime,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub rrule: Option<RecurrenceRule>,
}

impl CalendarIntentParser {
    pub fn parse(input: &str, ref_time: &CalendarReferenceTime) -> Result<ParsedCalendarIntent, CalendarError>;
}
```

### 3.2. Gramática e Regras de Reconhecimento

1. **Datas Relativas:**
   - `"hoje"` / `"today"`: data de referência.
   - `"amanhã"` / `"amanha"` / `"tomorrow"`: `ref_time + 1 dia`.
   - `"depois de amanhã"` / `"depois de amanha"`: `ref_time + 2 dias`.
   - Dias da semana: `"segunda"`, `"terça"`, `"quarta"`, `"quinta"`, `"sexta"`, `"sábado"`, `"domingo"` (ou `"próxima quarta"`, `"segunda que vem"`): avança para o próximo dia da semana correspondente.
   - Datas explícitas: `"dia 15"`, `"15/10"`, `"15/10/2026"`, `"15 de outubro"`.

2. **Horários & Intervalos:**
   - Horário pontual: `"às 14h"`, `"as 14h30"`, `"14:30"`, `"às 9h"`, `"14h"`:
     - `start`: horário informado.
     - `end`: `start + 60 min` (duração presumida de 1 hora).
     - `is_all_day`: `false`.
   - Intervalo explícito: `"das 10h às 11h30"`, `"das 14h as 16h"`, `"10:00-11:30"`:
     - `start`: horário inicial.
     - `end`: horário final.
     - Caso `end <= start` (ex: `"plantão das 19h às 07h"`), `end` avança automaticamente para o dia seguinte.
   - Sem horário especificado:
     - Evento classificado como `is_all_day = true`.

3. **Locais:**
   - Prefixos de local: `"no Fasano"`, `"na Sala 2"`, `"em Casa"`, `"at Room 101"`:
     - Extrai `location` e remove os tokens do título.

4. **Recorrências Simples:**
   - Expressões:
     - `"toda terça"` / `"todas as terças"` -> `FREQ=WEEKLY;BYDAY=TU`.
     - `"todo dia"` / `"diariamente"` -> `FREQ=DAILY;INTERVAL=1`.
     - `"toda semana"` / `"semanalmente"` -> `FREQ=WEEKLY;INTERVAL=1`.
     - `"a cada 15 dias"` / `"quinzenalmente"` -> `FREQ=WEEKLY;INTERVAL=2`.
     - `"todo mês"` / `"mensalmente"` -> `FREQ=MONTHLY;INTERVAL=1`.
     - `"todo ano"` / `"anualmente"` -> `FREQ=YEARLY;INTERVAL=1`.

5. **Título Limpo:**
   - Todos os tokens reconhecidos (data, hora, local, recorrência e preposições conectivas como `"às"`, `"das"`, `"em"`) são extraídos, resultando no título essencial do evento.
   - Exemplo:
     - Input: `"Almoço com Marcelo amanhã 12h30 no Fasano"`
     - Summary: `"Almoço com Marcelo"`
     - Location: `"Fasano"`
     - Start: `Amanhã 12:30`
     - End: `Amanhã 13:30`

---

## 4. Especificação do Motor iCalendar RFC 5545

### 4.1. Unfolding e Folding de Linhas (RFC 5545 §3.1)
- Linhas que excedem 75 octetos são desdobradas na leitura se a linha seguinte começar com espaço (` `) ou tab (`\t`).
- Na serialização, quebras usam estritamente `\r\n` (CRLF).

### 4.2. Geração e Extração de `VEVENT`
Campos obrigatórios em `VEVENT`:
- `UID`: Identificador único global.
- `DTSTAMP`: Timestamp UTC de emissão.
- `DTSTART`: Timestamp ou data do evento (`TZID` ou formato `Z`).
- `DTEND`: Timestamp ou data de término.
- `SUMMARY`: Título.
- `DESCRIPTION`: Texto descritivo (opcional).
- `LOCATION`: Localização (opcional).
- `RRULE`: Regra de recorrência (opcional).
- `STATUS`: `CONFIRMED`, `TENTATIVE` ou `CANCELLED`.
- `ATTENDEE`: Participantes com parâmetro `PARTSTAT`.

---

## 5. Critérios de Aceite e Fases GOST DD-TDD

- [ ] **Fase 4 (Red Test):** Testes unitários falhando para parsing de frases em PT-BR/EN, serialização/deserialização iCalendar e validação de `RRULE`.
- [ ] **Fase 5 (Green Implement):** Implementação dos módulos em `lauda-types` com compilação e aprovação em 100% dos testes.
- [ ] **Fase 6 (Refactor):** Otimização de alocações de string e clareza idiomática.
- [ ] **Fase 7 (Mutation Test):** Teste de mutação garantindo que operadores lógicos e condicionais de data/hora sejam cobertos.
- [ ] **Fase 8 (Quality Gate):** `cargo clippy --workspace --all-targets` limpo e `cargo test` aprovado.
