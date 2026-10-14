# Especificação Técnica: Extração Léxica de Participantes em Eventos de Calendário

> **Documento de Especificação Técnica (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#48 - feat(core): extrair participantes a partir de conectivos de companhia em eventos](https://codeberg.org/lumenpink/lauda/issues/48)  
> **Norma Base:** GOST 1.4.0 / RFC 5545  

---

## 1. Visão Geral e Princípios

Esta especificação define o mecanismo determinístico e sem chamadas externas para reconhecimento e vinculação de acompanhantes/pessoas no texto livre de criação de eventos de calendário no `CalendarIntentParser` (`lauda-types`).

1. **Detecção por Conectivos:** Identificação automática de pessoas após conectivos de companhia (`com`, `junto a`, `junto de`, `junto com`, `em companhia de`, `na companhia de`).
2. **Separação Rígida de Título e Participante:** Nomes extraídos são transferidos exclusivamente para `attendees: Vec<EventAttendee>`, removendo-os do `summary` do evento. O nome/trecho só permanece no título se estiver expressamente delimitado por aspas (`"..."` ou `'...'`).
3. **Múltiplos Nomes e E-mails:** Suporte a listagens separadas por vírgula e conjunção `e` (ex: `com Alice, Bob e Carol`), nomes compostos (ex: `com João Paulo`) e endereços de correio eletrônico (ex: `com dev@lohn.in`).
4. **Isolamento de Notificações (Zero Spam):** A extração apenas popula a estrutura de dados local com `RsvpStatus::NeedsAction` sem acionar disparos de rede, SMTP, iTip ou mensageria externa.

---

## 2. Contratos de Dados

### 2.1. `ParsedCalendarIntent`
Inclusão do campo `attendees`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCalendarIntent {
    pub summary: String,
    pub start: EventTime,
    pub end: EventTime,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub rrule: Option<RecurrenceRule>,
    pub attendees: Vec<EventAttendee>,
}
```

### 2.2. Construção de `EventAttendee`
- Se o token contiver `@`: `EventAttendee { email: token.to_string(), name: None, status: RsvpStatus::NeedsAction }`
- Se for texto nominal: `EventAttendee { email: String::new(), name: Some(name.to_string()), status: RsvpStatus::NeedsAction }`

---

## 3. Gramática Léxica e Delimitadores

### 3.1. Conectivos Suportados
- `com`
- `junto a` / `junto de` / `junto com`
- `em companhia de` / `na companhia de`

### 3.2. Término da Cláusula de Participantes
A lista de participantes é consumida até encontrar:
- Delimitador temporal: `às`, `as`, `das`, `amanhã`, `amanha`, `hoje`, `depois`, `segunda`..`domingo`, `findi`, `dia`, `natal`, etc.
- Delimitador espacial: `no`, `na`, `em`, `at`.
- Fim da entrada.

### 3.3. Escape Literal por Aspas
- Trechos protegidos por aspas no início/corpo (ex.: `"Almoço com Marcelo" amanhã`) preservam `"Almoço com Marcelo"` no título e não extraem participantes.
- Aspas dentro da cláusula de participante (ex.: `com "Marcelo Silva" e "Ana Paula"`) agrupam o nome composto intacto para o respectivo `EventAttendee`.

---

## 4. Critérios de Aceite (DD-TDD)

1. `parse("Almoço com Marcelo amanhã às 12h30", &ref)`:
   - `summary`: `"Almoço"`
   - `attendees`: `[EventAttendee { name: Some("Marcelo"), email: "", status: NeedsAction }]`
2. `parse("Reunião com Alice e Bob sexta", &ref)`:
   - `summary`: `"Reunião"`
   - `attendees`: `["Alice", "Bob"]`
3. `parse("Alinhamento junto a Carlos, Daniel e Elisa às 14h", &ref)`:
   - `summary`: `"Alinhamento"`
   - `attendees`: `["Carlos", "Daniel", "Elisa"]`
4. `parse("Almoço com dev@lohn.in amanhã", &ref)`:
   - `attendees`: `[EventAttendee { email: "dev@lohn.in", name: None, status: NeedsAction }]`
5. `parse("\"Almoço com Marcelo\" amanhã", &ref)`:
   - `summary`: `"Almoço com Marcelo"`
   - `attendees`: `[]`
