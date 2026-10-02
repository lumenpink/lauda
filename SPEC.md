# LAUDA — Especificação do Smolclient & Micro-Daemon de Correio

**Versão da Norma:** GOST `1.0.0`  
**Status:** `CONCLUDED` (Branch: `feat/1-data-contracts`)  
**Licença:** GPL-3.0 (com satélites `LICENSE-MEDIA` CC-BY-SA-4.0 e `LICENSE-LOGO` CC-BY-ND-4.0)  
**Ticket de Referência:** `#1`

---

## 1. Visão Geral & Topologia

O **`lauda`** é um cliente de e-mail ultraleve focado em leitura pura e redação rápida. Ele abandona a complexidade dos clientes de e-mail tradicionais em favor de uma micro-API JSON sobre rede privada (Tailscale/WireGuard), desacoplando a interface gráfica do manuseio pesado de protocolos de rede.

```text
[ lauda-client (Android NDK / Linux Desktop) ]
                     │
            HTTP/JSON (Tailscale)
                     ▼
[ lauda-server (Daemon Rust axum) ]
        │                   │
  UNIX Socket          /usr/sbin/sendmail
        ▼                   ▼
 [ Dovecot IMAP ]     [ Postfix MTA ]
```

---

## 2. Estrutura do Workspace Cargo

```text
lauda/
├── lauda-types/       # Contratos de dados serde compartilhados
├── lauda-server/      # Micro-daemon HTTP axum + ponte IMAP Dovecot
└── lauda-client/      # App Dioxus Mobile (Android NDK) + Desktop (x86_64)
```

---

## 3. Contratos de Dados (`lauda-types`) — Ticket #1

O crate de biblioteca `lauda-types` centraliza os modelos serializáveis consumidos pelo cliente e pelo servidor:

### 3.1 Autenticação & Sessão
- **`LoginRequest`**:
  ```json
  {
    "username": "lumen",
    "password": "..."
  }
  ```
- **`TokenResponse`**:
  ```json
  {
    "token": "jwt...",
    "expires_at": 1727800000,
    "username": "lumen"
  }
  ```

### 3.2 Caixas de Correio
- **`MailboxInfo`**:
  ```json
  {
    "name": "INBOX",
    "total_messages": 42,
    "unread_messages": 3
  }
  ```

### 3.3 Envelopes e Mensagens
- **`ViewMode`**:
  - `txt_auto`: Markdown limpo gerado pelo Urupema (padrão).
  - `txt_original`: Texto cru com espaçamento fixo preservado.
  - `html`: Conteúdo HTML higienizado em sandbox.
- **`MessageEnvelope`**:
  - `uid`: Identificador único estável do Dovecot IMAP.
  - `subject`: Assunto da mensagem.
  - `sender`: Nome e e-mail do remetente.
  - `date`: Timestamp ISO-8601.
  - `is_read`: Flag booleana `\Seen`.
  - `is_flagged`: Flag booleana `\Flagged`.
- **`Message`**:
  - Todos os campos do envelope + corpos:
  - `txt_auto`: `Option<String>`
  - `txt_original`: `Option<String>`
  - `html_sanitized`: `Option<String>`

### 3.4 Composição e Envio
- **`ComposeForm`**:
  - `from`: Remetente livre (com suporte a aliases).
  - `to`: Destinatário principal.
  - `cc`: Opcional.
  - `subject`: Assunto.
  - `body`: Texto em fonte monoespaçada.

---

## 4. Estratégia de Testes (TDD)

1. Testes de unidade em `lauda-types` verificando:
   - Serialização e deserialização bidirecional (round-trip determinístico).
   - Suporte a campos opcionais e nulos.
   - Negação de desserialização em payloads malformados.
2. Zero warnings no `cargo clippy`.
3. Formatação determinística `cargo fmt -- --check`.

---

## 5. Critérios de Aceite (Ticket #1)

- [x] Especificação prévia consolidada neste `SPEC.md`.
- [x] Configuração do `Cargo.toml` raiz do workspace apontando para `lauda-types`.
- [x] Testes falhos criados para cada estrutura de dados em `lauda-types`.
- [x] Implementação de todos os modelos em `lauda-types` passando 100% nos testes.
- [x] Clippy zero warnings e rustfmt verificado.
