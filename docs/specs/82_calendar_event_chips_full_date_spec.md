# Especificação Técnica: Exibição Completa de Data e Processamento nos Chips de Evento

**ID do Ticket:** #82  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Em Andamento (Fase 3: Spec)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

Na interface da Agenda Soberana (`CalendarView`), na inclusão rápida de eventos via Quick-Add bar, à medida que o usuário digita uma descrição (ex.: `"Almoço amanhã às 12h30 no Fasano"` ou `"Plantão domingo às 19h"`), o `CalendarIntentParser` decompõe o texto e gera uma estrutura `ParsedCalendarIntent`.

Entretanto, o chip de prévia reativo anterior limitava-se a exibir o texto estático genérico `"📅 Dia Inteiro"` ou `"📅 Com horário definido"`, omitindo a data real processada (dia, mês e ano calculados a partir do contexto temporal) e os horários processados de início e fim. Isso impedia que o usuário validasse se a data calculada (ex.: para termos relativos como "amanhã", "próxima sexta", "dia 25") estava correta antes de criar o evento ou complementar os detalhes.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Helpers Canônicos de Formatação de Chips
Na camada de tipos e utilitários (`lauda-types::calendar`):
1. `format_intent_date_chip(intent: &ParsedCalendarIntent) -> String`:
   - Formata a data de calendário processada no padrão canônico `DD/MM/YYYY`.
   - Se o evento abranger datas distintas (ex.: overnight), formata como `DD/MM/YYYY → DD/MM/YYYY`.
2. `format_intent_time_chip(intent: &ParsedCalendarIntent) -> String`:
   - Para eventos de dia inteiro (`is_all_day == true`), retorna `"Dia inteiro"`.
   - Para eventos com horário, formata o intervalo processado `HH:MM - HH:MM` (ex.: `"12:30 - 13:30"`).
   - Para eventos que cruzam a meia-noite para outro dia, formata `HH:MM → HH:MM`.
3. `format_intent_datetime_chip(intent: &ParsedCalendarIntent) -> String`:
   - Formatação unificada amigável: `DD/MM/YYYY · HH:MM - HH:MM` ou `DD/MM/YYYY (Dia inteiro)`.

### 2.2. Componente de Prévia da Interface Visual (`CalendarView`)
No chip de prévia gerado a partir do `parsed_intent`:
- Em vez de rótulos estáticos de placeholder, exibir chips individuais bem formatados com estilo Trash Polka:
  - Chip de data processada: `📅 DD/MM/YYYY`
  - Chip de horário definido/processado: `⏰ HH:MM - HH:MM` (ou `⏰ Dia inteiro`)
- Assegurar que ambos os dados estejam perfeitamente visíveis, legíveis e sem ambiguidade para o usuário.

---

## 3. Critérios de Aceite

1. O chip de prévia do parser na inclusão de eventos na agenda deve exibir a data processada com dia, mês e ano.
2. O chip de prévia do parser deve exibir o horário de início e fim processados pelo parser ou a indicação clara de dia inteiro.
3. Existência de testes unitários e de integração validando cenários variados (mesmo dia, overnight, dia inteiro, datas futuras relativas).
4. Conformidade estrita Gosplan (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 4. Registro de Fases DD-TDD

- [x] **Fase 1 (Ticket):** Ticket #82 formalizado e delimitado.
- [x] **Fase 2 (Branch):** Branch `fix/82-calendar-event-chips-full-date` criada a partir da `main`.
- [x] **Fase 3 (Spec):** Especificação técnica formalizada neste documento.
- [ ] **Fase 4 (Red Test):** Testes unitários falhos para formatação de chips de data e horário do parser.
- [ ] **Fase 5 (Green Implement):** Implementação dos formatadores em `lauda-types` e renderização em `CalendarView`.
- [ ] **Fase 6 (Refactor):** Simplificação idiomática com zero warnings.
- [ ] **Fase 7 (Mutation Test):** Verificação de mutantes (omissão de ano, troca de ordem de início/fim, fallback incorreto de dia inteiro).
- [ ] **Fase 8 (Quality Gate):** Validação estrita Gosplan (`cargo fmt --check`, `cargo clippy`, testes de integração).
- [ ] **Fase 9 (Finish & Close):** Merge via CCCP e fechamento do ticket #82.
