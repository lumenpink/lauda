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

---

## 6. Especificação do Daemon HTTP (`lauda-server`) — Ticket #2

O **`lauda-server`** é um serviço leve construído com `axum` e `tokio`, executado no servidor Debian local acessível via Tailscale. Ele atua como gateway de tradução entre o smolclient (HTTP/JSON) e a infraestrutura do servidor (Dovecot via UNIX socket e Postfix via sendmail).

### 6.1 Rotas HTTP REST

| Método | Caminho | Autenticação | Descrição |
|---|---|---|---|
| `POST` | `/auth/login` | Não | Recebe `LoginRequest`, valida contra o Dovecot e emite JWT |
| `GET` | `/mailboxes` | Bearer JWT | Lista as caixas disponíveis (`Vec<MailboxInfo>`) |
| `GET` | `/messages` | Bearer JWT | Lista envelopes de mensagens (`box`, `page`, `limit`) |
| `GET` | `/messages/:uid` | Bearer JWT | Retorna mensagem completa (`Message`) com corpos |
| `POST` | `/compose` | Bearer JWT | Recebe `ComposeForm` e despacha para `/usr/sbin/sendmail` |

### 6.2 Porta de Domínio & In-Memory Fake (`domain/ports.rs`)
Para assegurar a hermeticidade e velocidade dos testes (GOST Rust §4):
```rust
#[async_trait::async_trait]
pub trait MailBackend: Send + Sync {
    async fn authenticate(&self, username: &str, password: &str) -> Result<(), BackendError>;
    async fn list_mailboxes(&self, username: &str) -> Result<Vec<MailboxInfo>, BackendError>;
    async fn list_messages(&self, username: &str, mailbox: &str) -> Result<Vec<MessageEnvelope>, BackendError>;
    async fn get_message(&self, username: &str, mailbox: &str, uid: u32) -> Result<Message, BackendError>;
    async fn send_message(&self, username: &str, form: &ComposeForm) -> Result<(), BackendError>;
}
```
O adapter `InMemoryMailBackend` implementa este trait com dados em memória, permitindo testar todas as rotas axum sem necessidade de instância ativa do Dovecot ou Postfix durante o CI.

### 6.3 Configuração (`/etc/lauda/server.toml` ou fallback local)
Conforme GOST-13 (XDG):
```toml
[server]
listen = "127.0.0.1:8432"
jwt_secret = "chave-secreta-para-assinatura-jwt"
jwt_expiry_hours = 72

[mail]
imap_socket = "/var/run/dovecot/imap"
sendmail_bin = "/usr/sbin/sendmail"
```

### 6.4 Critérios de Aceite (Ticket #2)
- [x] Especificação do daemon e rotas consolidada neste `SPEC.md`.
- [x] Adição do membro `lauda-server` ao workspace Cargo.
- [x] Testes falhos criados cobrindo autenticação, middleware JWT e rotas `/mailboxes`, `/messages`, `/compose`.
- [x] Implementação do router `axum`, claims JWT e in-memory backend passando 100% nos testes.
- [x] Clippy zero warnings e formatação determinística verificada.

---

## 7. Especificação do Smolclient (`lauda-client`) — Ticket #3

O **`lauda-client`** é o cliente gráfico nativo multiplataforma (Android via NDK e Linux Desktop `x86_64`) construído com **Dioxus 0.6**. Ele consome exclusivamente a API JSON fornecida pelo `lauda-server` e opera como um leitor de texto e compositor rápido e sem atrito.

### 7.1 Visualizador de Mensagens com Alternador Trifásico
O leitor permite alternar instantaneamente entre três representações da mesma mensagem:
1. **`txt auto` (Padrão):** O Markdown estruturado gerado pelo Urupema é parseado via `pulldown-cmark` e renderizado em componentes nativos (cabeçalhos destacados, blocos de código com fundo diferenciado, links clicáveis).
2. **`txt original`:** Texto cru RFC 5322 com espaçamento fixo, quebras de linha estritas e tipografia monoespaçada typewriter.
3. **`html`:** Visualização tabular em container isolado (sandbox) para e-mails que demandem layout original (ex: extratos, faturas).

### 7.2 Editor de Composição Sem Travas
- **Campo `From`:** Caixa de texto livre permitindo enviar como qualquer alias autorizado, com suporte a sugestão/autocompletar dos aliases favoritos configurados.
- **Campos `To`, `Cc` e `Subject`:** Entradas ágeis.
- **Área de Texto do Corpo:** Fonte monoespaçada de máquina de escrever, sem formatação rica intrusiva.

### 7.3 Porta de Comunicação (`ports::MailApiClient`)
Abstração da comunicação HTTP para testes unitários headless rápidos:
```rust
#[async_trait::async_trait]
pub trait MailApiClient: Send + Sync {
    async fn login(&self, req: &LoginRequest) -> Result<TokenResponse, ClientError>;
    async fn list_mailboxes(&self, token: &str) -> Result<Vec<MailboxInfo>, ClientError>;
    async fn list_messages(&self, token: &str, mailbox: &str) -> Result<Vec<MessageEnvelope>, ClientError>;
    async fn get_message(&self, token: &str, mailbox: &str, uid: u32) -> Result<Message, ClientError>;
    async fn send_message(&self, token: &str, form: &ComposeForm) -> Result<(), ClientError>;
}
```

### 7.4 Configuração XDG (`~/.config/lauda/config.toml`)
```toml
[server]
url = "http://100.64.0.1:8432"

[identity]
default_from = "lumen@2lp.in"
aliases = ["lumen@2lp.in", "contato@2lp.in", "postmaster@2lp.in"]
```

### 7.5 Critérios de Aceite (Ticket #3)
- [ ] Especificação do cliente consolidada neste `SPEC.md`.
- [ ] Inclusão do crate `lauda-client` no workspace Cargo.
- [ ] Parser Markdown utilizando `pulldown-cmark` testado unitariamente.
- [ ] Implementação de `InMemoryApiClient` e máquina de estados de navegação (`Screen`, `ViewMode`).
- [ ] Componente raiz Dioxus e entrypoint desktop/mobile compilando com sucesso.
- [ ] Clippy zero warnings e rustfmt verificado.
