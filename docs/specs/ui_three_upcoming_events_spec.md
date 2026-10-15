# Especificação Técnica: Exibição dos Três Próximos Eventos no Destaque da Agenda

**ID do Ticket:** #58  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Contexto e Motivação

No topo da Agenda Soberana (`lauda-client/src/ui/calendar_view.rs`), o banner de destaque exibia exclusivamente 1 único compromisso (`next_event`). Para usuários que gerenciam múltiplas reuniões e compromissos diários, visualizar apenas um evento restringe a visibilidade da jornada imediata.

Faz-se necessária a evolução do banner de destaque para exibir até 3 próximos eventos cronológicos futuros, apresentados em um layout compacto, claro e elegante dentro da identidade visual Trash Polka.

---

## 2. Contrato de Interface e Comportamento

1. **Função Pura de Seleção:**
   - Em `lauda-client/src/ui/calendar_view.rs` (ou `domain/calendar_view`):
     ```rust
     pub fn select_upcoming_events(
         events: &[CalendarEvent],
         now_ts: i64,
         limit: usize,
     ) -> Vec<CalendarEvent>
     ```
   - **Regras:**
     1. Filtra eventos cujo início em UTC seja igual ou superior ao timestamp de referência (`ev.start.to_timestamp_utc() >= now_ts`).
     2. Ordena os eventos resultantes em ordem estritamente cronológica crescente pelo timestamp de início.
     3. Retorna uma fatia de tamanho até `limit` (default: 3).
2. **Apresentação Visual Trash Polka:**
   - Caso `upcoming_events.is_empty()`: o bloco de destaque é omitido (sem elementos vazios).
   - Caso contenha entre 1 e 3 eventos:
     - Container unificado: `border: 2px solid #0a0a0a; background: #0a0a0a; color: #fff; padding: 0.8rem 1rem; box-shadow: 3px 3px 0 #dc143c;`.
     - Cabeçalho: Badge vermelho (`background: #dc143c; color: #fff; font-weight: bold; font-size: 0.75rem; padding: 0.2rem 0.5rem;`) com texto `"PRÓXIMOS COMPROMISSOS"`.
     - Lista compacta de eventos: cada item exibe título, horário e marcador relativo, com separadores discretos entre itens caso haja múltiplos.

---

## 3. Critérios de Aceite e Verificação

1. **Limite de 3 Eventos:** Retorna exatamente os 3 primeiros eventos futuros em ordem cronológica. Se houver menos de 3, retorna todos os disponíveis; se houver zero, retorna lista vazia.
2. **Ordenação Cronológica:** Eventos fora de ordem na lista de entrada são devidamente ordenados pelo início.
3. **Resiliência e Layout:** A interface não quebra e mantém legibilidade com 0, 1, 2 ou 3 itens.
4. **Qualidade e Estilo:** Cobertura de testes unitários e de mutação, conformidade com `cargo fmt`, `cargo clippy` e `cccp check --strict`.
