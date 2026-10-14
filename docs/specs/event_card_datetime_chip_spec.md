# Especificação Técnica: Chip Visual de Data e Horário no Visualizador de Eventos

**ID do Ticket:** #49  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral

Atualmente, o componente `EventCard` na visualização do calendário (`CalendarView`) exibe apenas uma badge lateral esquerda simplificada com o horário de início (ex: `14:00` ou `DIA INTEIRO`). A linha de metadados do evento contém chips dedicados para localização (`📍 Local`), recorrência (`🔁 WEEKLY`) e participantes (`👤 Participante`), mas não exibe a data completa do evento nem o intervalo de término.

Este ticket formaliza a adição de um chip padronizado de Data e Horário na linha de metadados do evento, permitindo que o usuário identifique de relance:
1. A data do evento (`DD/MM`).
2. O intervalo completo de horário (`HH:MM - HH:MM`).
3. O status de dia inteiro (`(Dia inteiro)`).
4. Variações entre dias distintos (eventos que atravessam a meia-noite).

---

## 2. Contrato de Formatação (`format_event_datetime_chip`)

A função auxiliar pura `format_event_datetime_chip(event: &CalendarEvent) -> String` deve produzir representações textuais concisas e amigáveis:

1. **Evento com Horário no Mesmo Dia:**
   - Exemplo: Início em 15/10 às 14:00, término às 15:30.
   - Formato: `"15/10 · 14:00 - 15:30"`
2. **Evento de Dia Inteiro (`is_all_day: true` ou `EventTime::DateOnly`):**
   - Exemplo: 15/10 (dia inteiro).
   - Formato: `"15/10 (Dia inteiro)"`
3. **Evento Overnight (atravessa meia-noite):**
   - Exemplo: Início em 15/10 às 23:00, término em 16/10 às 02:00.
   - Formato: `"15/10 23:00 → 16/10 02:00"`

---

## 3. Integração Visual no Componente `EventCard`

O chip deve seguir a mesma identidade visual dos demais chips de metadados:
```rust
span {
    style: "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;",
    "📅 {format_event_datetime_chip(&event)}"
}
```

O chip deve ser renderizado como o primeiro item da linha de metadados, antecedendo a localização, recorrência e participantes.

---

## 4. Critérios de Aceite

- [x] Função de formatação `format_event_datetime_chip` pura, determinística e testada unitariamente.
- [x] Renderização condicional no `EventCard` sem causar quebra de layout quando faltar localização ou participantes.
- [x] Suporte a `EventTime::DateTimeUtc` e `EventTime::DateOnly`.
- [x] Suporte a eventos overnight e multi-dia.
- [x] 100% de conformidade com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
