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
├── lauda-index/       # Motor nativo de busca com parser Gmail, BM25 e fuzzy search
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
- [x] Especificação do cliente consolidada neste `SPEC.md`.
- [x] Inclusão do crate `lauda-client` no workspace Cargo.
- [x] Parser Markdown utilizando `pulldown-cmark` testado unitariamente.
- [x] Implementação de `InMemoryApiClient` e máquina de estados de navegação (`Screen`, `ViewMode`).
- [x] Componente raiz Dioxus e entrypoint desktop/mobile compilando com sucesso.
- [x] Clippy zero warnings e rustfmt verificado.

---

## 8. Distribuição Android & Mecanismo de Auto-Update (Ticket #5)

Para viabilizar um ciclo de desenvolvimento ágil com testes em dispositivos Android reais sem atrito ou burocracia de lojas centralizadas, o **`lauda`** implementa uma arquitetura híbrida de distribuição e atualização automática.

### 8.1 Manifesto de Versão Remota (`version.json`)
Publicado na raiz do portal estático (`https://lauda.2lp.in/version.json`):
```json
{
  "version": "0.1.0",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-latest.apk",
  "release_notes": "Versão inicial de desenvolvimento com visualizador trifásico e redação sem travas.",
  "published_at": "2026-10-02T08:00:00Z"
}
```

### 8.2 Lógica do Verificador Semântico (`domain/update.rs`)
O cliente móvel consulta o manifesto e realiza a comparação via [Semantic Versioning 2.0.0](https://semver.org/):
- Se `remote_version > local_version`: o cliente sinaliza `UpdateStatus::Available(UpdateManifest)`.
- Se a versão local for menor que `min_compatible_version`: sinaliza `UpdateStatus::Required(UpdateManifest)`.
- Do contrário: `UpdateStatus::UpToDate`.

### 8.3 Métodos de Atualização Suportados
1. **Rastreamento Automático via Obtainium:**
   - O aplicativo Obtainium no Android rastreia diretamente as releases do Codeberg (`https://codeberg.org/lumenpink/lauda`) e executa o download e instalação em segundo plano.
2. **Download Direto na Vitrine Trash Polka:**
   - Link de download direto apontando para `https://lauda.2lp.in/lauda-latest.apk` e botões de atalho em `site/index.html`.
3. **In-App Updater:**
   - O smolclient exibe o aviso com a opção de baixar o novo `.apk` e invocar o instalador de pacotes do sistema via Intent `application/vnd.android.package-archive`.

### 8.4 Critérios de Aceite (Ticket #5)
- [x] Especificação do protocolo de atualização consolidada neste `SPEC.md`.
- [x] Criação do modelo `UpdateManifest` e lógica semântica `check_update()` testada unitariamente.
- [x] Disponibilização de `site/version.json` na vitrine pública.
- [x] Inclusão dos links e botões de download do APK no `site/index.html`.
- [x] Clippy zero warnings e rustfmt verificado.

---

## 9. Suíte de Testes de Integração Ponta a Ponta (E2E) — Ticket #6

Para validar a pirâmide de testes GOST (§6 da norma técnica), o sistema possui uma suíte de integração hermética conectando o filtro MIME **`urupema`** ao micro-daemon **`lauda-server`** e ao smolclient **`lauda-client`**.

### 9.1 Cenário do Ciclo Completo de Correio
```text
[ E-mail Bruto com Scripts e Trackers ]
                   │
                   ▼ (stdin)
          [ urupema::Pipeline ]
                   │
                   ▼ (multipart limpo + txt auto)
        [ lauda-server (HTTP API) ]
                   │
                   ▼ (JWT Bearer /messages/:uid)
     [ lauda-client (Dioxus / Markdown) ]
                   │
                   ▼ (POST /compose)
        [ MTA Sendmail Subprocess ]
```

1. **Ingestão:** Mensagem RFC 5322 maliciosa contendo `<script>alert('xss')</script>`, pixel 1x1 e tabelas promocionais.
2. **Higienização:** O `urupema` purifica a mensagem, remove os vetores perigosos e gera a representação `txt auto` em Markdown.
3. **Ponte de Dados:** A mensagem tratada é disponibilizada para o usuário no backend do servidor.
4. **Consumo Gráfico:** O cliente móvel Lauda autentica via JWT, lista a caixa de entrada, busca o e-mail higienizado e converte o Markdown em nós visíveis.
5. **Composição e Resposta:** O smolclient envia uma resposta rápida pelo endpoint `/compose` confirmando a entrega no sink final.


---

## 10. Suíte E2E da Cadeia Completa de Correio Soberano — Ticket #7

Para validar a integridade de ponta a ponta do ecossistema de e-mail soberano no ambiente de CI e em homologação, o repositório **`lauda`** concentra uma suíte exaustiva de testes de integração ponta a ponta conectando todos os elos da infraestrutura:

```text
               [ lauda-client (Dioxus / HttpApiClient) ]
                               │
                               ▼ (HTTP REST / JWT)
                 [ lauda-server (API Gateway) ]
                               │
                               ▼ (POST /compose)
 ┌───────────────────────────────────────────────────────────────┐
 │                   CADEIA DE SAÍDA (OUTBOUND)                  │
 │                                                               │
 │  1. Papéli Milter:                                            │
 │     ├─ Remetente gerenciado: envelopa em MJML + assinatura   │
 │     │   (suporte aos papéis de carta dinâmicos anos 80)       │
 │     └─ Remetente não gerenciado: BYPASS total (intocado)      │
 │                                                               │
 │  2. Ishcondidim Milter:                                       │
 │     ├─ Remetente gerenciado: gera par Ed25519/X25519 e        │
 │     │   injeta cabeçalho Autocrypt                            │
 │     ├─ Remetente não gerenciado: BYPASS total (intocado)      │
 │     ├─ Peer com chave conhecida: CIFRA em PGP/MIME (RFC 3156) │
 │     └─ Peer sem chave: envia em texto claro                   │
 └───────────────────────────────┬───────────────────────────────┘
                                 │
                                 ▼ (Entrega MTA / Rede)
 ┌───────────────────────────────────────────────────────────────┐
 │                  CADEIA DE ENTRADA (INBOUND)                  │
 │                                                               │
 │  3. Ishcondidim Inbound:                                      │
 │     ├─ Mensagem PGP/MIME: DECIFRA com chave local e injeta    │
 │     │   banner com Palavra Secreta de Validação               │
 │     ├─ Anti-Spoofing: neutraliza banners forjados em texto    │
 │     ├─ Aprendizado de Pares: absorve chave pública do peer    │
 │     └─ Anti-Lockout: detecta resposta sem Autocrypt e         │
 │         rebaixa com segurança para texto claro                │
 │                                                               │
 │  4. Urupema Post-Queue Filter:                                │
 │     ├─ Higienização de HTML (scripts, trackers e pixels)      │
 │     ├─ Geração determinística de txt auto em Markdown         │
 │     └─ Prevenção de loop com X-Urupema-Processed: 1          │
 └───────────────────────────────┬───────────────────────────────┘
                                 │
                                 ▼
                     [ Armazenamento / Maildir ]
                                 │
                                 ▼ (GET /messages/:uid)
               [ lauda-client (Visualizador Trifásico) ]
                 ├─ txt auto: Markdown limpo renderizado
                 ├─ txt original: Texto cru e cabeçalhos
                 └─ html: HTML com layout Papéli e selo Autocrypt
```

### 10.1 Casos de Teste Cobertos

1. **Papéli (Papel de carta & Assinaturas Inteligentes):**
   - **Envelopamento:** E-mail de remetente gerenciado é envelopado no papel de parede responsivo MJML (ex: tema anos 80) com assinatura corporativa.
   - **Bypass / Fora da lista:** E-mail de remetente não configurado na lista é ignorado e mantido 100% inalterado.
   - **Compactação de Histórico:** Citações em respostas (`<blockquote>`) têm assinaturas antigas compactadas.

2. **Ishcondidim (Autocrypt Level 1 & OpenPGP):**
   - **Key Bootstrap:** Primeiro envio de usuário gerenciado gera par Ed25519/X25519 e injeta cabeçalho `Autocrypt`.
   - **Bypass / Fora da lista:** Remetente não gerenciado não recebe cabeçalho Autocrypt e não altera keystore.
   - **Descoberta de Pares:** Recebimento de cabeçalho `Autocrypt` armazena chave pública do contato externo.
   - **Criptografia Oportunística:** Envio para peer com chave conhecida transforma a mensagem em `multipart/encrypted`.
   - **Decifragem Inbound:** Mensagem criptografada é decifrada com chave local e recebe banner com Palavra Secreta.
   - **Anti-Spoofing:** Tentativas de spam simulando o banner do Ishcondidim em texto claro são desarmadas.
   - **Anti-Lockout:** Resposta do peer sem cabeçalho Autocrypt rebaixa oportunisticamente para texto claro.

3. **Urupema + Lauda Client:**
   - Higienização e geração de `txt auto` estruturado em Markdown a partir do layout envelopado.
   - Consumo pelo smolclient Lauda com alternador trifásico (`txt auto`, `txt original`, `html`).
   - Composição e envio sem travas via API REST `/compose`.

### 10.2 Critérios de Aceite (Ticket #7)
- [x] Especificação da cadeia completa documentada neste `SPEC.md`.
- [x] Adição das crates irmãs no workspace de testes e implementação dos cenários exaustivos em `e2e_full_chain_test.rs`.
- [x] Execução hermética sem dependência de serviços externos no CI (`cargo test`).
- [x] Clippy zero warnings e rustfmt verificado.

---

## 11. Suíte Exaustiva de Resiliência e Simulação de Erros (Ticket #8)

Para consolidar o ecossistema Soberano com robustez de nível de produção e tolerância a falhas, o repositório **`lauda`** implementa uma bateria de testes de resiliência e simulação de erros cobrindo cenários anômalos e ataques maliciosos em todos os elos do fluxo.

### 11.1 Cenários e Vetores de Teste

1. **Autocrypt & OpenPGP Profundo (`ishcondidim`):**
   - **Rotação de Chaves:** Atualização segura de chave pública quando um contato publica novo par com timestamp mais recente (`msg_date > effective_date`).
   - **Defesa Anti-Replay:** Rejeição de substituição de chaves em mensagens antigas entregues fora de ordem (`msg_date < effective_date`), preservando a chave mais recente.
   - **Múltiplos Remetentes/Aliases:** Gestão isolada e independente de chaves Ed25519/X25519 para múltiplos endereços locais gerenciados (`lumen@2lp.in`, `contato@2lp.in`, `postmaster@2lp.in`).
   - **Decifragem Pré-Maildir com Metadados:** Decifragem com injeção do banner no corpo e de metadados nos cabeçalhos RFC 5322 (`X-Ishcondidim-Decrypted: 1`, `X-Autocrypt-Verified: valid`).
   - **Neutralização Anti-Spoofing:** Detecção de banners falsos em mensagens de texto claro e neutralização de tentativas de phishing.
   - **Tratamento de Payloads Corrompidos:** Captura graciosa de Base64 inválido, atributos faltantes e OpenPGP ciphertext adulterado sem ocorrência de pânico no serviço.

2. **Papéli & Envelopamento Resiliente (`papeli`):**
   - **HTML Imperfeito:** Tolerância a tags não fechadas, caracteres especiais e fragmentos DOM quebrados restaurados pelo sanitizador sem falha no parser MJML.
   - **Fallback de Temas Desconhecidos:** Uso de tema inexistente no assunto recorre automaticamente ao template padrão seguro.
   - **Preservação de Anexos e CIDs:** Garantia de integridade binária para arquivos anexados e imagens inline durante a reformulação do corpo.

3. **Urupema & Prevenção de Evasão (`urupema`):**
   - **Vetores Evasivos de XSS:** Bloqueio de tags `<SCRIPT>`, `<iframe src="...">`, atributos `onload=`, `onerror=` e esquemas `javascript:`.
   - **Pixels de Rastreamento:** Eliminação de imagens 1x1 e rastreadores invisíveis.
   - **Prevenção Estrita de Loops:** Mensagens marcadas com `X-Urupema-Processed: 1` contornam o processamento sem loop infinito.

4. **Lauda Gateway & Smolclient (`lauda-server` / `lauda-client`):**
   - **Rejeição de Tokens Forjados:** Rejeição imediata (`401 Unauthorized`) para tokens com assinatura inválida, segredo errado ou expirados.
   - **Fidelidade da Visualização Trifásica:** Comportamento confiável na alternância entre `txt auto`, `txt original` e `html`.

### 11.2 Critérios de Aceite (Ticket #8)
- [x] Especificação da suíte de resiliência e simulação de erros consolidada neste `SPEC.md`.
- [x] Implementação de `lauda-server/tests/e2e_resilience_test.rs` cobrindo todos os cenários de erro e borda.
- [x] Execução hermética sem dependências externas no CI (`cargo test`).
- [x] Zero warnings no Clippy e formatação determinística verificada.

---

## 12. Esteira Soberana de Compilação Multi-Alvo e Deploy Contínuo no Push (Ticket #61)

Conforme a doutrina GOST, a única via legítima de publicação e deploy é a automação da esteira de CI (`.forgejo/workflows/ci.yml`) acionada automaticamente no evento de push para a branch principal (`main`). Deploys manuais ou ad-hoc são vedados. Durante o ciclo contínuo de desenvolvimento, versões com identificador de desenvolvimento (`-dev...`) são tratadas como estáveis para todos os efeitos práticos de distribuição e consumo.

### 12.1 Contrato da Esteira Multi-Alvo no Push
A esteira é encadeada com dependência estrita do gate de testes (`needs: [test]`) e dispara simultaneamente a entrega de todos os canais:
1. **Web (Dioxus WebAssembly):** Compilação de artefatos estáticos PWA e deploy automático via Cloudflare Workers (`lauda.2lp.in`) e Codeberg Pages.
2. **Desktop (Linux Nativo):** Compilação release do binário `dist/desktop/lauda-desktop`.
3. **Android APK Nativo (`deploy-apk`):** Compilação do APK real via `dx` e Gradle, strip de símbolos JNI, assinatura criptográfica oficial soberana (`in_twolp_release.keystore`), publicação em bucket R2 e espelhamento automático na Pratilera (`banca.2lp.in/pratilera`).
4. **Pacotes & Manifestos:** Sincronização atômica dos manifestos de distribuição (`version.json` e `version-nightly.json`) com o arquivo canônico `VERSION`.

### 12.2 Resolução de Versão do Cliente (`lauda-client`)
- A verificação de atualização no cliente (`check_update`) consome estritamente o identificador canônico de build `env!("LAUDA_APP_VERSION")`, e não o `CARGO_PKG_VERSION` isolado.
- Durante o desenvolvimento, versões `dev` superiores ou iguais às versões publicadas remotamente no manifesto são consideradas atualizadas (`UpdateStatus::UpToDate`), prevenindo alertas espúrios de downgrade.

---

## 13. Banner de Atualização Disponível Dispensável (Ticket #56)

Para otimizar o aproveitamento da área visível em telas móveis e desktop, o banner de nova versão no smolclient deve ser dispensável interativamente:
1. **Controle de Estado:** O componente raiz mantém o identificador da versão dispensada (`dismissed_update_version`).
2. **Interação:** O banner exibe um botão de fechamento rápido (`✕`). Ao ser acionado, o banner é recolhido imediatamente sem interromper a navegação da caixa postal ou exigir recarregamento.
### 13.1 Critérios de Aceite e Verificação
- [x] Especificação formal documentada em `docs/specs/ui_dismissable_update_banner_spec.md`.
- [x] Lógica de filtragem `should_show_update_banner` implementada e testada.
- [x] Botão interativo de fechamento (`✕`) estilizado em Trash Polka no banner.
- [x] Testes unitários, de integração e de mutação cobrindo casos de borda e resiliência a espaços.
- [x] Conformidade integral com `cargo fmt`, `cargo clippy` e `cccp check --strict`.

---

## 14. Padronização do Placeholder da Quick-Add Bar (Ticket #57)

O texto de instrução e sugestão (placeholder) da barra de entrada rápida da Agenda Soberana (`lauda-client/src/ui/calendar_view.rs`) é padronizado com um exemplo genérico e profissional:
1. **Definição Canônica:** Declarado via constante pública `QUICK_ADD_DEFAULT_PLACEHOLDER`.
2. **Conteúdo:** `"Ex: Reunião de equipe amanhã às 14h na Sala 2"`, demonstrando título, horário/data e local.
3. **Consumo:** Aplicado no atributo `placeholder` do campo `<input>`.

### 14.1 Critérios de Aceite e Verificação
- [x] Especificação formal documentada em `docs/specs/ui_quick_add_generic_placeholder_spec.md`.
- [x] Constante `QUICK_ADD_DEFAULT_PLACEHOLDER` definida e consumida no componente `CalendarView`.
- [x] Teste automatizado assegurando o valor e estrutura da constante.
- [x] Conformidade integral com `cargo fmt`, `cargo clippy` e `cccp check --strict`.

---

## 15. Destaque dos Três Próximos Eventos na Agenda (Ticket #58)

O card superior de destaque na Agenda Soberana (`lauda-client/src/ui/calendar_view.rs`) exibe até 3 próximos compromissos futuros:
1. **Seleção e Ordenação:** A função `select_upcoming_events` filtra compromissos futuros (`start >= now_ts`), ordena-os cronologicamente e seleciona até 3 itens.
2. **Apresentação:** Renderiza um banner unificado em estética Trash Polka com o badge `"PRÓXIMOS COMPROMISSOS"`, listando os eventos com suas respectivas estimativas temporais.
3. **Resiliência:** Suporta perfeitamente 0, 1, 2 ou 3 eventos sem quebras ou lacunas visuais.

### 15.1 Critérios de Aceite e Verificação
- [x] Especificação formal documentada em `docs/specs/ui_three_upcoming_events_spec.md`.
- [x] Função `select_upcoming_events` implementada e testada para 0, 1, 2, 3 e >3 eventos.
- [x] Componente `CalendarView` atualizado para renderizar a lista tripla de destaque.
- [x] Conformidade integral com `cargo fmt`, `cargo clippy` e `cccp check --strict`.

---

## 16. Formatação Amigável de Tempo Relativo e Reatividade Temporal (Ticket #59)

A indicação do tempo restante até um compromisso é formatada de forma coloquial e atualizada reativamente:
1. **Formatação Humanizada:** A função `format_relative_event_time` decompõe deltas de tempo em minutos (`"em 25 min"`), horas com minutos (`"em 4h 15min"`), ou dias e horas (`"em 2 dias e 3h"`). Deltas passados ou inferiores a 1 minuto são apresentados como `"agora"`.
2. **Eventos DateOnly:** São comparados com a data civil corrente, gerando `"Hoje"`, `"Amanhã"` ou `"em X dias"`.
3. **Reatividade Contínua:** Sinal periódico a cada 30 segundos garante recálculo em tempo real no cliente Lauda.

### 16.1 Critérios de Aceite e Verificação
- [x] Especificação formal documentada em `docs/specs/ui_relative_event_time_format_spec.md`.
- [x] Função `format_relative_event_time` implementada e testada para minutos, horas, dias e DateOnly.
- [x] Tique reativo contínuo integrado em `CalendarView`.
- [x] Conformidade integral com `cargo fmt`, `cargo clippy` e `cccp check --strict`.







