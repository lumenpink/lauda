# Especificação Técnica: Formatação Amigável de Tempo Relativo e Reatividade Temporal

**ID do Ticket:** #59  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Contexto e Motivação

No destaque da Agenda Soberana (`lauda-client/src/ui/calendar_view.rs`), o tempo restante até os próximos eventos era calculado ingenuamente como `(ts - now_ts) / 60` e exibido apenas em minutos brutos (`"em {diff} min"`). Para eventos nos dias ou semanas seguintes, essa representação produzia números desproporcionais e pouco legíveis (ex: centenas ou milhares de minutos). Além disso, a ausência de um tique temporal reativo mantinha o valor visual congelado caso a tela permanecesse aberta sem interação.

---

## 2. Contrato de Interface e Comportamento

1. **Função Pura de Formatação:**
   - Em `lauda-client/src/ui/calendar_view.rs`:
     ```rust
     pub fn format_relative_event_time(event_time: &EventTime, now_ts: i64) -> String
     ```
   - **Regras para `EventTime::DateTimeUtc(ts)`:**
     - `diff_sec = ts - now_ts`.
     - `diff_sec <= 0` ou `< 60`: `"agora"`.
     - `60 <= diff_sec < 3600`: `"em {min} min"` (onde `min = diff_sec / 60`).
     - `3600 <= diff_sec < 86400`:
       - `h = diff_sec / 3600`, `m = (diff_sec % 3600) / 60`.
       - Se `m == 0`: `"em {h}h"`.
       - Se `m > 0`: `"em {h}h {m}min"`.
     - `diff_sec >= 86400`:
       - `days = diff_sec / 86400`, `h = (diff_sec % 86400) / 3600`.
       - Se `days == 1`:
         - Se `h > 0`: `"em 1 dia e {h}h"`.
         - Se `h == 0`: `"em 1 dia"`.
       - Se `days > 1`:
         - Se `h > 0`: `"em {days} dias e {h}h"`.
         - Se `h == 0`: `"em {days} dias"`.
   - **Regras para `EventTime::DateOnly { year, month, day }`:**
     - Compara a data civil do evento com a data civil de `now_ts` em UTC.
     - `diff_days <= 0`: `"Hoje"`.
     - `diff_days == 1`: `"Amanhã"`.
     - `diff_days > 1`: `"em {diff_days} dias"`.

2. **Reatividade Temporal Contínua:**
   - O componente `CalendarView` gerencia o sinal `current_now: Signal<i64>`.
   - Um efeito em segundo plano atualiza periodicamente `current_now` a cada 30 segundos usando `async_sleep`.
   - Essa atualização reativa dispara a renderização contínua dos marcadores temporais sem necessidade de recarregar a tela.

---

## 3. Critérios de Aceite e Verificação

1. **Legibilidade Natural:** Formatação humana em minutos, horas e dias combinados.
2. **Suporte a DateOnly:** Tradução para "Hoje", "Amanhã" ou "em X dias".
3. **Reatividade Temporal:** Atualização em tempo de execução via sinal periódico.
4. **Qualidade e Estilo:** Cobertura de testes unitários e de mutação, conformidade com `cargo fmt`, `cargo clippy` e `cccp check --strict`.
