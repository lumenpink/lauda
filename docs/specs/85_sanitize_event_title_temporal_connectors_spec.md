# Especificação Técnica: Higienização de Conectores Temporais e Fragmentos de Data no Título de Eventos

**ID do Ticket:** #85  
**Norma:** GOST 1.4.0 (DD-TDD)  
**Status:** Especificado (Fase 3: Doc/Spec)  
**Responsável:** Lumen Pink / Antigravity  
**Branch:** `fix/85-sanitize-event-title-temporal-connectors`

---

## 1. Visão Geral e Motivação

Durante a criação rápida de eventos via linguagem natural (ex: *"Reunião primeiro domingo de novembro às 15 pra quatro"* ou *"Almoço sexta as 12h"*), conectores temporais como "às", "as", "em", "para", "pra", "de", etc., frequentemente não eram consumidos ou sobravam nas extremidades do resumo resultante, gerando títulos corrompidos como *"Reunião as"* ou *"Almoço de negócios para"*.

Além disso, conforme diretriz explícita, o reconhecimento e higienização desses conectores devem operar de forma **insensível a acentuação e caixa** (ex: tratar "às" e "as", "à" e "a" de forma idêntica).

O objetivo deste ticket é:
1. Garantir que conectores predecessores a padrões temporais (como "às" ou "as" antes de minutos inversos ou horas isoladas) sejam plenamente consumidos durante a extração de data e hora.
2. Implementar rotina de poda higiênica nas bordas (início e final) do título resultante, removendo preposições e conectores temporais órfãos tanto com acento quanto sem acento.
3. Normalizar pontuações suspensas e espaços múltiplos remanescentes.

---

## 2. Requisitos Arquiteturais e Contratos

### 2.1. Conectores e Preposições Reconhecidos (Insensíveis a Acentos)
A lista canônica de conectores que não devem subsistir nas extremidades de um título de evento inclui:
- `a`, `à`, `ao`, `aos`, `as`, `às`
- `em`, `no`, `na`, `nos`, `nas`
- `para`, `pra`, `pro`, `pras`, `pros`
- `de`, `do`, `da`, `dos`, `das`
- `pelo`, `pela`, `pelos`, `pelas`
- `com` (quando isolado na borda final sem participante associado)

A normalização de comparação deve aplicar `to_lowercase()` e remoção de diacríticos (`à` -> `a`).

### 2.2. Consumo em Pass 2 (Minutos Inversos)
No Pass 2 (`"15 pra quatro"`, `"10 pras 8"`):
- Se o token imediatamente anterior a `i` for `às`, `as`, `a` ou `à`, e não estiver consumido, marcá-lo como consumido.

### 2.3. Função de Higienização de Summary
Ao final da construção de `summary_tokens`:
- Remover repetidamente da cauda (trim end) e da cabeça (trim start) quaisquer tokens que pertençam à lista de conectores temporais ou pontuações residuais (`:`, `-`, `,`, `.`).
- Se após a higienização o título ficar vazio, adotar `"Novo Evento"`.

---

## 3. Critérios de Aceitação

- [ ] `"Reunião primeiro domingo de novembro às 15 pra quatro"` resulta no título estritamente `"Reunião"`.
- [ ] `"Reunião primeiro domingo de novembro as 15 pra quatro"` (sem acento) resulta no título estritamente `"Reunião"`.
- [ ] `"Almoço de negócios amanhã para as 13h"` resulta no título estritamente `"Almoço de negócios"`.
- [ ] `"Consulta médica em 15/11 às 10h"` resulta no título estritamente `"Consulta médica"`.
- [ ] Pontuações suspensas ao final do título (como traços ou dois-pontos) são removidas.

---

## 4. Checklist DD-TDD (9 Fases GOST)

- [x] **Fase 1 - Ticket:** Issue #85 delimitada no Codeberg e sincronizada.
- [x] **Fase 2 - Branch:** Branch `fix/85-sanitize-event-title-temporal-connectors` criada a partir de `main`.
- [x] **Fase 3 - Doc/Spec:** Especificação preliminar e contratos definidos (Zero código de produção).
- [ ] **Fase 4 - Red:** Testes automatizados falhos comprovando a permanência de conectores com e sem acento no título.
- [ ] **Fase 5 - Green:** Implementação funcional da higienização com testes passando.
- [ ] **Fase 6 - Refactor:** Código idiomático e zero warnings.
- [ ] **Fase 7 - Mutation:** Testes de borda cobrindo múltiplos conectores encadeados e pontuações suspensas.
- [ ] **Fase 8 - Quality Gate:** Validação incremental rigorosa (`cargo fmt --check`, `cargo clippy`, `cargo test`).
- [ ] **Fase 9 - Close:** Atualização de documentação, changelog e merge.
