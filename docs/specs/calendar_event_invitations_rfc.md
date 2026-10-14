# RFC 0001: Arquitetura & Fluxo para Envio e Recepção de Convites de Eventos (iTIP / iMIP)

- **Ticket:** [#50](https://codeberg.org/lumenpink/lauda/issues/50)
- **Status:** Proposta Aprovada / Em Especificação
- **Data:** 2026-10-14
- **Autor:** Lumen Pink / Antigravity
- **Normas Base:** RFC 5545 (iCalendar), RFC 5546 (iTIP), RFC 6047 (iMIP), RFC 5322 (Internet Message Format)

---

## 1. Visão Geral & Motivação

O subsistema de agenda soberana do Lauda (`lauda-types`, `lauda-client`, `lauda-server`) permite a criação rápida e fluida de eventos através de linguagem natural (`Quick-Add`) e sincronização bidirecional via CalDAV (RFC 4791).

Para viabilizar a colaboração com participantes externos sem depender de plataformas fechadas (Google Calendar, Microsoft 365, Apple iCloud), esta RFC padroniza o ciclo de vida completo de envio, atualização, cancelamento e confirmação de presença (RSVP) em eventos através do protocolo padrão da Internet: **iTIP sobre iMIP**.

---

## 2. Princípios de Design & Ergonomia (UX)

### 2.1. Fricção Zero na Criação de Eventos
A criação e manipulação rápida de compromissos é sagrada:
- **Latência de 0ms:** O usuário pode redigir compromissos no `Quick-Add` (ex.: *"Almoço com Marcelo amanhã 12h"*) e teclar `Enter`. O evento é imediatamente gravado e renderizado na UI local.
- **Nenhum Modal Bloqueante:** A criação de eventos não deve ser interrompida por caixas de diálogo ou confirmações obrigatórias sobre disparo de e-mails.

### 2.2. Consentimento Ativo & Checkbox Desmarcada por Padrão
- **Desmarcada por Padrão (`[ ] Despachar convites por e-mail`):** Nenhum e-mail ou notificação é disparado silenciosamente. Se o usuário não marcar ativamente a opção, nada sai do dispositivo.
- **Transparência de Ação:** O usuário tem controle visual claro antes de qualquer comunicação com redes externas.

---

## 3. Protocolo Canônico: iTIP & iMIP (RFC 5546 / RFC 6047)

Em vez de criar APIs proprietárias que exigiriam clientes dedicados, o Lauda adota estritamente os padrões abertos do IETF:

### 3.1. Métodos iTIP Suportados
1. **`METHOD:REQUEST` (Convite Inicial ou Atualização):**
   - Enviado pelo organizador (`ORGANIZER`) para todos os convidados (`ATTENDEE`).
   - Contém o objeto `VCALENDAR` com `VEVENT`, carimbo `DTSTAMP`, `UID` canônico do evento e `SEQUENCE` sequencial (iniciando em 0 ou 1, incrementado a cada alteração de data/hora/local).
   - Atributos do participante no `.ics`:
     `ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE;CN="Nome":mailto:email@dominio`
2. **`METHOD:REPLY` (Resposta RSVP):**
   - Enviado pelo convidado de volta para o organizador.
   - Atualiza o estado de confirmação:
     - `PARTSTAT=ACCEPTED`: Presença confirmada.
     - `PARTSTAT=DECLINED`: Convite recusado.
     - `PARTSTAT=TENTATIVE`: Participação incerta.
3. **`METHOD:CANCEL` (Cancelamento do Evento):**
   - Enviado pelo organizador quando o evento é excluído ou cancelado.
   - O cliente de e-mail/agenda receptor remove o evento da agenda do convidado de forma automatizada.

---

## 4. Transporte & Encapsulamento MIME (iMIP)

### 4.1. Estrutura da Mensagem de Correio
Quando o usuário opta por despachar convites, o Lauda gera uma mensagem no padrão `multipart/alternative` compatível com todos os clientes de e-mail do mercado:

```text
From: organizador@2lp.in
To: convidado@dominio.com
Subject: Convite: Almoço de Alinhamento (15/10 às 12:00)
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="----=_Part_Lauda_Invite_12345"

------=_Part_Lauda_Invite_12345
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Você foi convidado para: Almoço de Alinhamento
Quando: 15/10/2026 das 12:00 às 13:30 (Horário de Brasília)
Onde: Fasano

Para responder a este convite, utilize as opções nativas do seu cliente de e-mail ou acesse o link de confirmação direta:
https://lauda.2lp.in/rsvp?token=eyJhbGciOi...

------=_Part_Lauda_Invite_12345
Content-Type: text/calendar; charset=UTF-8; method=REQUEST; name="invite.ics"
Content-Transfer-Encoding: 8bit
Content-Disposition: inline; filename="invite.ics"

BEGIN:VCALENDAR
PRODID:-//Lauda//PT-BR
VERSION:2.0
METHOD:REQUEST
BEGIN:VEVENT
UID:event-1700000000
SEQUENCE:1
DTSTAMP:20261014T210000Z
DTSTART:20261015T150000Z
DTEND:20261015T163000Z
SUMMARY:Almoço de Alinhamento
LOCATION:Fasano
ORGANIZER;CN=Organizador:mailto:organizador@2lp.in
ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE;CN=Convidado:mailto:convidado@dominio.com
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR

------=_Part_Lauda_Invite_12345--
```

### 4.2. Estratégia de Fallback (Links Web Diretos)
Para convidados cujo cliente de correio não possua leitor de `.ics` integrado (ex.: webmails básicos legados), o corpo em `text/plain` e `text/html` inclui botões e links com tokens HMAC/Ed25519 de clique único:
- `https://lauda.2lp.in/rsvp?token=...&action=accept`
- `https://lauda.2lp.in/rsvp?token=...&action=decline`
- `https://lauda.2lp.in/rsvp?token=...&action=tentative`

A resposta via link gera o processamento do RSVP no servidor e sincroniza o estado via CalDAV sem exigir login do convidado.

---

## 5. Segurança, Privacidade e Prevenção de Abuso

1. **Anti-Spam & Limites de Disparo:**
   - Limite prudencial de destinatários por evento (máximo de 30 por padrão) para prevenir envio de spam involuntário.
   - Rejeição de auto-convites circulares.
2. **Prevenção de Spoofing de Organizador:**
   - O campo `ORGANIZER` do iCalendar deve coincidir estritamente com a identidade autenticada do remetente (`From:` do SMTP e usuário da sessão do Lauda).
3. **Criptografia Autocrypt / PGP de Ponta a Ponta:**
   - Caso todos os participantes possuam chaves públicas registradas no `LocalContactsCache`, o e-mail contendo o `.ics` é assinado digitalmente e criptografado em PGP/MIME, impedindo a interceptação de reuniões confidenciais.
4. **Privacidade de Participantes Ocultos:**
   - Suporte a envio de cópia oculta (Bcc) individual onde cada participante recebe um `.ics` contendo apenas a si próprio e o organizador, evitando expor listas de participantes para terceiros quando necessário.

---

## 6. Contratos de Dados Preliminares (`lauda-types`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItipMethod {
    Request,
    Reply,
    Cancel,
}

impl ItipMethod {
    pub fn to_ical_str(&self) -> &'static str {
        match self {
            ItipMethod::Request => "REQUEST",
            ItipMethod::Reply => "REPLY",
            ItipMethod::Cancel => "CANCEL",
        }
    }
}

pub fn generate_itip_payload(
    event: &CalendarEvent,
    method: ItipMethod,
    organizer: &EventAttendee,
) -> String;
```

---

## 7. Critérios de Aceite da RFC
- [x] Documento RFC publicado em `docs/specs/` detalhando o fluxo e mensagens iTIP/iMIP.
- [x] Definição clara dos métodos RFC 5546 (`REQUEST`, `REPLY`, `CANCEL`) vs APIs de terceiros.
- [x] Regra de fricção zero e checkbox desmarcada por padrão consolidada.
- [x] Estratégia de fallback via tokens web e suporte à criptografia Autocrypt/PGP.
- [x] Contratos e testes unitários tipados em `lauda-types` validando a conformidade iTIP.
