# Especificação Técnica: Reconhecimento de Horários e Datas Coloquiais na Agenda

> **Documento de Especificação Técnica (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#47 - feat(core): reconhecer horarios e datas coloquiais no gerador de calendario](https://codeberg.org/lumenpink/lauda/issues/47)  
> **Norma Base:** GOST 1.4.0 / RFC 5545  

---

## 1. Visão Geral e Objetivos

O módulo `CalendarIntentParser` em `lauda-types` é o motor determinístico offline-first para criação rápida de eventos a partir de uma única linha de texto. Esta especificação expande o parser para suportar expressões idiomáticas brasileiras com alta fidelidade:

1. **Normalizador Léxico:** Intercambialidade total entre números por extenso e dígitos arábicos (`duas` $\leftrightarrow$ `2`, `quinze` $\leftrightarrow$ `15`), com tolerância a sufixos temporais (`h`, `hr`, `hrs`, `horas`, `min`, `m`).
2. **Minutos Inversos:** Interpretação da construção regressiva de horário (`15 pras 2` $\to$ 13:45, `20 pra meia noite` $\to$ 23:40).
3. **Termos Nominais e Ambiguidade:** Tratamento de `meio-dia`, `meia-noite` e aplicação de heurística de horário comercial (1 a 6 $\to$ 13h a 18h) quando não especificado turno.
4. **Datas Ordinais e Meses Flexíveis:** Suporte ao `primeiro de <mês>` (dia 1), meses por extenso (`outubro`), 3 letras (`out`) e numéricos (`10`, `1/10`, `01/10`).
5. **Datas Coloquiais e Feriados Nacionais:** Reconhecimento de `findi` / `fim de semana`, e feriados como `natal`, `ano novo` / `reveillon`, `independência`, `tiradentes` e `dia do trabalhador`, com inferência automática de virada de ano.
6. **Mecanismo de Escape Literal:** Aspas (`"..."` ou `'...'`) para proteção estrita de literais contra o parser, e heurística contextual para logradouros/estabelecimentos (`no clube 12 de agosto`).

---

## 2. Gramática e Regras de Parsing

### 2.1. Normalização de Numerais por Extenso
Mapeamento determinístico de numerais para dígitos inteiros:
- Horas e dias: `um`/`uma` (1), `dois`/`duas` (2), `três`/`tres` (3), `quatro` (4), `cinco` (5), `seis` (6), `sete` (7), `oito` (8), `nove` (9), `dez` (10), `onze` (11), `doze` (12).
- Dezenas e frações: `quinze` (15), `vinte` (20), `trinta` (30), `quarenta` (40), `quarenta e cinco` (45), `cinquenta` (50).
- Expressão de meia hora: `meia` (em contexto de minutos, ex: `duas e meia`, `e meia`) $\to$ 30.

### 2.2. Minutos Inversos ("X pras Y")
Padrão: `<minutos> <preposição_regressiva> <hora_alvo> [turno]`
- Preposições regressivas: `pra`, `pras`, `pro`, `pros`, `para`, `para as`, `para os`.
- Cálculo:
  $$\text{minutos\_finais} = 60 - \text{minutos}$$
  $$\text{hora\_base} = (\text{hora\_alvo} + 24 - 1) \pmod{24}$$
- Ajuste comercial da hora base: Se `hora_alvo` estiver entre 1 e 6 e sem `da manhã`, presume-se tarde comercial antes de subtrair (ex: `15 pras 2` $\to$ alvo 14h $\to$ 13:45).

### 2.3. Presunção de Turno e Ambiguidade Comercial
- Turnos explícitos:
  - `da manhã` / `da madrugada` / `am`: hora entre 0..11.
  - `da tarde` / `pm`: se hora < 12, soma 12.
  - `da noite`: se hora < 12 e hora != 0, soma 12 (ex: `8 da noite` $\to$ 20h, `12 da noite` $\to$ 00h).
- Sem turno explícito:
  - `1..=6` $\to$ assume tarde (`13:00` a `18:00`).
  - `7..=12` $\to$ assume manhã (`07:00` a `12:00`).
  - `13..=23` $\to$ formato 24h.

### 2.4. Mapeamento de Meses e Datas
Meses suportados nas três modalidades:
| Mês | Extenso | 3 Letras | Numérico |
| :--- | :--- | :--- | :--- |
| 1 | janeiro | jan | 1, 01 |
| 2 | fevereiro | fev | 2, 02 |
| 3 | março / marco | mar | 3, 03 |
| 4 | abril | abr | 4, 04 |
| 5 | maio | mai | 5, 05 |
| 6 | junho | jun | 6, 06 |
| 7 | julho | jul | 7, 07 |
| 8 | agosto | ago | 8, 08 |
| 9 | setembro | set | 9, 09 |
| 10 | outubro | out | 10 |
| 11 | novembro | nov | 11 |
| 12 | dezembro | dez | 12 |

Sintaxes válidas:
- `primeiro de <mes>`, `1º de <mes>`, `1o de <mes>`, `dia primeiro` $\to$ Dia 1.
- `D de <mes>`, `D/M`, `DD/MM`, `D/<mes_3_letras>`, `D de <mes_3_letras>`.
- `findi`, `fim de semana`, `final de semana` $\to$ próximo sábado a partir de `ref_time`.

### 2.5. Feriados Nacionais
- `natal` $\to$ 25 de Dezembro.
- `ano novo`, `reveillon`, `réveillon` $\to$ 01 de Janeiro.
- `independência`, `independencia` $\to$ 07 de Setembro.
- `tiradentes` $\to$ 21 de Abril.
- `trabalhador`, `dia do trabalho`, `dia do trabalhador` $\to$ 01 de Maio.
- **Resolução de ano:** Se a data calculada for anterior à data de `ref_time`, projeta para o ano seguinte (`ref_time.year + 1`).

### 2.6. Escapes e Proteção Literal
- **Aspas Literais:** Trechos entre `"..."` ou `'...'` são extraídos primeiro e protegidos com tokens sentinela. Nunca são decompostos em datas ou horários.
- **Heurística de Local:** Quando precedido de `no`, `na`, `em` seguido de termos como `clube`, `rua`, `avenida`, `hospital`, `bar`, `restaurante`, `edifício`, a expressão de data contígua não é consumida como data do evento se houver outro marcador temporal explícito no comando.

---

## 3. Critérios de Aceite e Casos de Teste (DD-TDD)

1. `CalendarIntentParser::parse("Almoço 15 pras 2", &ref)` $\to$ Start: 13:45, Summary: "Almoço".
2. `CalendarIntentParser::parse("Pizza 20 pra meia noite", &ref)` $\to$ Start: 23:40, Summary: "Pizza".
3. `CalendarIntentParser::parse("Reunião duas e meia da tarde", &ref)` $\to$ Start: 14:30.
4. `CalendarIntentParser::parse("Festa no \"Clube 12 de Agosto\" amanhã às 20h", &ref)` $\to$ Location: "Clube 12 de Agosto", Start: Amanhã 20:00.
5. `CalendarIntentParser::parse("Entrega primeiro de novembro às 10h", &ref)` $\to$ Date: 01/11, Time: 10:00.
6. `CalendarIntentParser::parse("Plantão 1/out às 14h", &ref)` $\to$ Date: 01/10, Time: 14:00.
7. `CalendarIntentParser::parse("Churrasco no findi às 12h", &ref)` $\to$ Date: Próximo sábado, Time: 12:00.
8. `CalendarIntentParser::parse("Ceia de natal às 21h", &ref)` $\to$ Date: 25/12, Time: 21:00.
