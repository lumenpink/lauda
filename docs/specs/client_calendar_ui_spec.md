# Especificação Técnica de Contratos: UI de Calendário, Timeline Stream e RSVP (lauda-client)

> **Documento de Especificação Técnica (Fase 3 - DD-TDD GOST)**  
> **Ticket Vinculado:** [#39 - feat(calendar): quick-add bar, timeline stream e rsvp de convites no lauda-client](https://codeberg.org/lumenpink/lauda/issues/39)  
> **Épico:** [#36 - feat(calendar): arquitetura soberana de agenda, caldav e fluxo continuo](https://codeberg.org/lumenpink/lauda/issues/36)  
> **Norma Base:** GOST 1.4.0 / Optimistic UI Pattern / Neo-Brutalist & Trash Polka Design System

---

## 1. Visão Geral da Funcionalidade

A interface do `lauda-client` substitui a experiência burocrática de calendários tradicionais por um fluxo contínuo e ágil integrado ao webmail e mobile:
1. **Quick-Add Bar (`calendar_view.rs`):** Entrada em linha única sem modais nem formulários com dezenas de campos, com parsing de intenção determinístico em tempo real via `CalendarIntentParser` de `lauda-types`.
2. **Timeline Stream:** Fluxo contínuo vertical mobile-first agrupado por dia (*Hoje*, *Amanhã*, *Esta Semana*), com indicador regressivo para o próximo compromisso.
3. **Leitor de E-mails Inteligente (`message_view.rs`):** Detecção automática de anexos `.ics` ou blocos iCalendar no corpo de mensagens, com cartão visual de convite e ações de RSVP em 1 clique (*Aceitar*, *Talvez*, *Recusar*).
4. **Cache Local Offline (`calendar_cache.rs`):** Persistência instantânea de eventos em SQLite local no desktop/mobile (e IndexedDB no Wasm), garantindo abertura imediata (0ms) mesmo sem rede.

---

## 2. Contratos da Quick-Add Bar (`calendar_view.rs`)

### 2.1 Parsing Reativo em Tempo Real
- Ao digitar na barra de entrada rápida:
  - O texto é processado por `CalendarIntentParser::parse(input, &ref_time)`.
  - Se válido (`Ok(intent)`), renderiza chip de prévia reativo abaixo do campo:
    - `[ 📅 Data & Horário | 📍 Local | 🔁 Recorrência | Título ]`
    - Botão de confirmação visual `[+ CRIAR]`.
- Se o input estiver vazio ou incompleto, o chip de prévia permanece oculto sem poluir a tela.

### 2.2 Criação e Persistência Imediata
- Ao teclar `Enter` ou clicar em `[+ CRIAR]`:
  - Gera um `CalendarEvent` com UID canônico único.
  - Salva imediatamente no `LocalCalendarCache`.
  - Atualiza a visualização da Timeline otimisticamente (0ms).
  - Limpa o campo de entrada e fecha o chip de prévia.
  - Sincroniza em segundo plano via `PUT /dav/calendars/{username}/default/{uid}.ics`.

---

## 3. Contratos da Timeline Stream

### 3.1 Agrupamento Temporal Mobile-First
- Eventos são ordenados cronologicamente e agrupados em seções:
  1. **Banner de Próximo Compromisso:** Se houver evento futuro hoje, exibe destaque:  
     `⚡ Próximo: [Título] às [Horário] (em X horas / minutos)`.
  2. **Hoje:** Lista de cartões de hoje.
  3. **Amanhã:** Lista de cartões do dia seguinte.
  4. **Esta Semana / Próximos Dias:** Eventos futuros subsequentes.

### 3.2 Estrutura do Cartão de Evento (Trash Polka)
- Cada cartão exibe:
  - **Badge de Horário:** `14:00 – 15:30` (fundo preto) ou `DIA INTEIRO` (fundo vermelho).
  - **Título Principal:** Tipografia em caixa alta / negrito.
  - **Local:** Ícone `📍` e nome do local se informado.
  - **Recorrência:** Badge `🔁` com frequência legível (ex: `Toda terça`).
  - **Ação Rápida:** Botão `🗑 EXCLUIR` que marca soft-delete e remove da tela.

---

## 4. Contratos de Convites e RSVP (`message_view.rs`)

### 4.1 Detecção de Convites
- Quando uma mensagem contiver anexo com extensão `.ics` ou corpo com `BEGIN:VCALENDAR`:
  - O payload é parseado via `CalendarEvent::from_ical`.
  - Exibe cartão de destaque no topo da mensagem:
    - Título da reunião, data/horário formatado, local e organizador.
    - Status atual do usuário (`NEEDS-ACTION`, `ACCEPTED`, `DECLINED`, `TENTATIVE`).

### 4.2 Ações de RSVP em 1 Toque
- Botões disponíveis no cartão de convite:
  - `[✓ ACEITAR]`: Define status para `Accepted`.
  - `[? TALVEZ]`: Define status para `Tentative`.
  - `[✕ RECUSAR]`: Define status para `Declined`.
- Ao clicar em uma ação:
  - Atualiza o evento no `LocalCalendarCache`.
  - Envia atualização para o servidor CalDAV.
  - Prepara/dispara e-mail de resposta para o organizador com cabeçalhos de RSVP.

---

## 5. Contratos do Cache Local (`calendar_cache.rs`)

- Módulo `LocalCalendarCache`:
  - `open(base_dir)`
  - `load_events() -> Result<Vec<CalendarEvent>, ClientError>`
  - `upsert_event(event: &CalendarEvent) -> Result<(), ClientError>`
  - `delete_event(id: &str) -> Result<bool, ClientError>`
  - `search_events(query: &str) -> Result<Vec<CalendarEvent>, ClientError>`
