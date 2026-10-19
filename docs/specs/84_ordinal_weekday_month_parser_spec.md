# Especificação Técnica: Interpretação de Expressões de Data Ordinal por Dia da Semana e Mês

**ID do Ticket:** #84  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3: Doc/Spec)  
**Responsável:** Lumen Pink / Antigravity  
**Branch:** `fix/84-ordinal-weekday-month-parser`

---

## 1. Visão Geral e Motivação

No analisador de linguagem natural de calendário de `lauda-types` (`ParsedCalendarIntent::parse_colloquial`), expressões como:
* *"Reunião primeiro domingo de novembro"*
* *"Almoço segunda terça de outubro"*
* *"Fechamento última sexta de dezembro"*

falhavam em identificar a data específica pretendida porque o motor só compreendia datas fixas numéricas (*"15 de outubro"*) ou ordinais diretos de início de mês (*"primeiro de novembro"*).

O objetivo deste ticket é viabilizar o cálculo exato do dia do mês correspondente a `[ordinal] [dia da semana] [de] [mês]`, suportando:
1. Ordinais: "primeiro(a)", "segundo(a)", "terceiro(a)", "quarto(a)", "quinto(a)", "último(a)", bem como suas versões abreviadas ("1º", "1ª", "1o", "1a", etc.) e sem acento ("ultimo", "ultima").
2. Dias da semana: domingo a sábado, com e sem sufixo "-feira" e variações sem acento ("terca", "sabado").
3. Mês: nomes de meses e abreviações válidas.
4. Ajuste preditivo de ano: se a data calculada já tiver passado no ano corrente, avança para o próximo ano.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Função Auxiliar de Resolução de Data Ordinal
```rust
fn resolve_ordinal_weekday_in_month(
    year: i32,
    month: u32,
    target_weekday: Weekday,
    ordinal: i32, // 1..=5, ou -1 para o último
) -> Option<u32>;
```
- Calcula todos os dias $d \in [1, \text{dias\_no\_mês}]$ onde $\text{dia\_da\_semana}(Y, M, d) == \text{target\_weekday}$.
- Se `ordinal == -1`, retorna o último elemento da lista.
- Se `ordinal \in 1..=5`, retorna o $n$-ésimo elemento.

### 2.2. Integração no `parse_colloquial`
No laço de reconhecimento de datas de `ParsedCalendarIntent::parse_colloquial`:
- Reconhece a sequência de tokens: `[ordinal] [dia da semana] (de)? [mês]`.
- Marca todos os tokens correspondentes em `consumed_indices`.
- Define `specific_date = Some((y, m, d))` com cálculo civil exato.
- Respeita horários que acompanhem a expressão (ex: `às 15 pra quatro`).

---

## 3. Critérios de Aceitação

- [x] "primeiro domingo de novembro" em 2026 resolve para 01/11/2026.
- [x] "segunda terça de outubro" em 2026 resolve para 13/10/2026.
- [x] "última sexta de dezembro" em 2026 resolve para 25/12/2026.
- [x] Formas com acento ("último", "terça", "sábado") e sem acento ("ultimo", "terca", "sabado") funcionam igualmente.
- [x] Todos os tokens da expressão de data são consumidos, não vazando para o título do evento.

---

## 4. Checklist DD-TDD (9 Fases GOST)

- [x] **Fase 1 - Ticket:** Issue #84 delimitada no Codeberg e sincronizada.
- [x] **Fase 2 - Branch:** Branch `fix/84-ordinal-weekday-month-parser` criada a partir de `main`.
- [x] **Fase 3 - Doc/Spec:** Especificação técnica preliminar registrada (Zero código de produção).
- [x] **Fase 4 - Red:** Testes automatizados falhos comprovando a falta de suporte a ordinais de dias da semana no mês.
- [x] **Fase 5 - Green:** Implementação funcional com testes passando.
- [x] **Fase 6 - Refactor:** Código idiomático e zero warnings.
- [x] **Fase 7 - Mutation:** Validação de ordinais de borda (ex: 5º dia inexistente, anos bissextos).
- [x] **Fase 8 - Quality Gate:** Validação incremental com `cargo fmt --check`, `cargo clippy` e `cargo test`.
- [x] **Fase 9 - Close:** Atualização de documentação, changelog e merge.
