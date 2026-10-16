# LAUDA — Especificação do Smolclient & Micro-Daemon de Correio

**Versão da Norma:** GOST `1.0.0`  
**Status:** `CONCLUÍDO` (Branch: `feat/71-client-non-blocking-ui-io`)  
**Licença:** GPL-3.0 (com satélites `LICENSE-MEDIA` CC-BY-SA-4.0 e `LICENSE-LOGO` CC-BY-ND-4.0)  
**Ticket de Referência:** `#71` (Anteriores: `#70`, `#69`, `#68`, `#67`, `#66`, `#65`, `#64`, `#63`, `#62`, `#1`)

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
  - `txt_auto`: Markdown limpo gerado a partir do HTML higienizado (ou fallback de texto puro) com quebras de linha preservadas (padrão).
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
  - `txt_auto`: `Option<String>` (prioriza conversão estruturada de `html_sanitized` em Markdown, recorrendo a `txt_original` como fallback).
  - `txt_original`: `Option<String>`
  - `html_sanitized`: `Option<String>`
- **Resolução de `txt_auto` e Renderização**:
  - **Prioridade de Fonte**: Em mensagens `multipart/alternative`, o `html_sanitized` é a fonte primária para o `txt_auto`, preservando tags de parágrafo (`<p>`), quebras (`<br>`), listas e entidades de acentuação sem corrupção de charset. O `txt_original` é utilizado exclusivamente quando não existir corpo HTML.
  - **Preservação de Quebras de Linha**: Na conversão de Markdown para HTML no cliente (`parse_markdown_to_html`), quebras suaves e quebras simples intencionais em parágrafos e linhas de texto são convertidas em `<br />` ou parágrafos separados, e espaços não-separáveis isolados são normalizados para evitar colapso de texto.

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

---

## 17. Padronização dos Ícones Oficiais do Launcher e Vitrine em 8-Bit RGBA (Ticket #63)

A identidade visual do ícone soberano do aplicativo **Lauda** (`in.twolp.lauda`) segue rigorosamente a estética visual Trash Polka Postal com supersampling vetorial 8-bit RGBA (Proposta C2):

1. **Geometria & Composição Visual:**
   - **Fundo:** Papel envelhecido off-white (`#f4efe6`) com retícula pontilhada halftone (`#0a0a0a` a 15% de opacidade).
   - **Faixas Diagonais:** Vermelho carmim (`#dc143c`) e bordô profundo (`#8b0000`) em cortes assimétricos.
   - **Envelope Brutalista Central:** Corpo em branco puro (`#ffffff`) com bordas pretas grossas (`#0a0a0a`), vincos de dobra e sombra dura deslocada.
   - **Lacre de Cera Central:** Bloco maciço preto puro (`#0a0a0a`) com topo triangular carmim, sem letras ou glifos internos.
   - **Carimbo Postal Cancelador Reforçado (Proposta C2):**
     - Anel circular com traço ultra pesado (10px) em carmim puro (`#dc143c`).
     - Barras de cancelamento horizontais de 5px acima e abaixo do texto.
     - Texto `" S M O L "` em tipografia mono expressiva, posicionado estrategicamente (`cx=175, cy=205`, ângulo de -6°) de forma que 100% dos glifos fiquem sobre o fundo branco do envelope.
     - Sobreposição em camada direta por cima da aba e corpo do envelope.
   - **Ausência de Texto de Rodapé:** Sem tarja inferior "LAUDA", permitindo respiro simétrico e equilíbrio vetorial em qualquer tamanho.

2. **Densidades e Recursos Android & Vitrine:**
   - **Mestre Canônico da Vitrine e F-Droid:** `site/lauda-logo.png` (512x512, 8-bit RGBA).
   - **Densidades Launcher Android:** `res/mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher.png` (48x48, 72x72, 96x96, 144x144, 192x192).
   - **Variante Circular (Pixel/Round Launchers):** `res/mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/ic_launcher_round.png`.
   - **Ícones Adaptativos:** `res/drawable/ic_launcher_foreground.png` e definições em `res/mipmap-anydpi-v26/`.
   - **Modo de Cor Estrito:** Todos os arquivos de saída devem ser salvos em 8-bit RGBA com canais alfa limpos nos cantos.

### 17.1 Critérios de Aceite e Verificação
- [x] Especificação formal aprovada e documentada na Seção 17 do `SPEC.md`.
- [x] Gerador vetorial determinístico implementado em `scripts/gerar-recursos-android.py` com supersampling 2x (1024x1024 -> 512x512/densidades).
- [x] Mestre `site/lauda-logo.png` e SVG canônico gerados em 8-bit RGBA.
- [x] Todas as 5 densidades Android geradas nos formatos padrão e circular.
- [x] Teste automatizado de validação de dimensões, formato e paleta aprovado.
- [x] Conformidade integral com `cargo fmt --check`, `cargo clippy` e `cccp check --strict`.

---

## 18. Motor de Busca Vetorial com Max-Pooling e Fatiamento Semântico Urupema (Ticket #64)

Para alimentar a busca de e-mails com precisão semântica e sem alocar memória de vídeo na GPU (mantendo 100% da VRAM dedicada a Baquara), o crate `lauda-index` é expandido para combinar BM25 com busca vetorial baseada em embeddings na CPU.

### 18.1 Fatiamento Semântico no `lauda-index`
- Cada e-mail indexado é fatiado via `urupema::domain::segment_email`.
- São extraídos blocos tipados:
  - `SegmentKind::Header`: Metadados essenciais (De, Para, Assunto, Data).
  - `SegmentKind::Body`: Chunks do corpo fatiados com limite de janela e estratégia Cabeça-Cauda (até 5 blocos).
  - `SegmentKind::Signature`: Fechos e assinaturas isolados.
- Cada segmento é mantido individualmente na estrutura `IndexedDoc`, gerando uma matriz de embeddings $K \times D$, onde $K \le 6$ e $D = 384$.

### 18.2 Motor de Embeddings em CPU (384 Dimensões)
- Embeddings gerados na CPU em float32 normalizados pela norma L2 ($\|v\|_2 = 1$).
- Modelo canônico: `multilingual-e5-small` (384 floats), especializado em Português, Inglês e Espanhol.
- 0 MB de VRAM consumida.
- O `lauda-index` provê trait e implementação padrão de `EmbeddingProvider` permitindo injeção de dependência e testes unitários determinísticos herméticos sem download de pesos de rede durante a compilação/testes.

### 18.3 Similaridade por Cosseno & Max-Pooling
- Dado um vetor de consulta $Q$ (normalizado L2) e uma matriz de segmentos de documento $\{S_1, \dots, S_K\}$:
  $$\text{sim}(Q, S_k) = Q \cdot S_k$$
  $$\text{Score}_{\text{sem}}(D) = \max_{k=1..K} \text{sim}(Q, S_k)$$
- O resultado da busca semântica inclui não apenas o UID do documento, mas também o `best_segment` (tipo de segmento que causou o match) e a pontuação semântica.

### 18.4 Ranqueamento Híbrido (BM25 + Vetorial)
- Fusão linear balanceada:
  $$\text{Score}_{\text{híbrido}} = \alpha \cdot \text{Score}_{\text{BM25}} + (1 - \alpha) \cdot \text{Score}_{\text{sem}}$$
  com $\alpha = 0.5$ configurável.
- Casamento de exatidão para códigos, números de processo, CPF e leis via BM25, e casamento conceitual/linguagem natural via similaridade semântica.

### 18.5 Critérios de Aceite (Ticket #64)
- [x] Trait `EmbeddingProvider` e estruturas vetoriais implementadas em `lauda-index`.
- [x] Dissecação via `urupema::domain::segment_email` integrada no `lauda-index`.
- [x] Cálculo de similaridade por cosseno, Max-Pooling e seleção de melhor segmento implementados.
- [x] Fusão híbrida BM25 + Vetorial com pontuação unificada.
- [x] Testes unitários herméticos em `lauda-index/tests/vector_search_test.rs`.
- [x] Quality Gate 100% verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test -p lauda-index`).

---

## 19. Endpoint de Busca Semântica no Servidor e Integração no Cliente (Ticket #65)

Para expor o motor de busca vetorial e híbrida construído no `lauda-index`, o `lauda-server` adiciona rota de busca para as caixas postais e o `lauda-client` conecta sua barra de pesquisa à API remota.

### 19.1 Especificação do Endpoint HTTP (`lauda-server`)
- Rota: `GET /messages/search` e `GET /mailboxes/{mailbox}/search`
- Parâmetros Query:
  - `box`: Nome da caixa postal (padrão: `INBOX`, se acessado via `/messages/search`).
  - `q`: Termo de busca textual ou semântica.
  - `mode`: Modo de busca (`hybrid` [padrão], `semantic`, `bm25`).
  - `alpha`: Coeficiente de fusão linear para modo híbrido (padrão: `0.5`).
- Cabeçalho de autorização: `Authorization: Bearer <JWT>`
- Resposta: `200 OK` com `Vec<MessageEnvelope>` ordenados por relevância decrescente.
- Comportamento:
  - Disseca mensagens da caixa postal com `urupema::domain::segment_email`.
  - Executa ranqueamento via `lauda-index::MessageIndex::search_hybrid`.
  - Retorna envelopes ordenados pela pontuação combinada.

### 19.2 Integração no Smoliclient (`lauda-client`)
- Extensão do trait `MailApiClient`:
  - Adição do método assíncrono `search_messages(&self, token: &str, mailbox: &str, query: &str) -> Result<Vec<MessageEnvelope>, ClientError>`.
  - Implementação correspondente em `HttpApiClient` e `InMemoryApiClient`.
- Componente `InboxView`:
  - Hook reativo `use_resource` para busca assíncrona ao digitar na barra de pesquisa.
  - Fallback instantâneo em memória enquanto a requisição transita pela rede.
  - Manutenção dos filtros de status (`TODOS`, `NÃO LIDOS`, `FAVORITOS`, `COM ANEXOS`) aplicados sobre o resultado ranqueado.

### 19.3 Critérios de Aceite (Ticket #65)
- [x] Método `search_messages` adicionado à porta `MailBackend` e implementado nos adapters (`MaildirBackend` e `InMemoryMailBackend`).
- [x] Endpoints HTTP de busca registrados e funcionais no router do `lauda-server`.
- [x] Método `search_messages` implementado na porta `MailApiClient` e adapters do `lauda-client`.
- [x] Barra de busca do `InboxView` no `lauda-client` conectada à busca assíncrona da API.
- [x] Testes de integração cobrindo busca no servidor e cliente.
- [x] Quality Gate 100% verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 20. Paginação Maildir por `mtime` e Janela de Renderização no Cliente (Ticket #66)

### 20.1 Problema de Desempenho Identificado
Com caixas de e-mail volumosas (> 10k a 30k mensagens), a listagem ingênua do Maildir causava dois gargalos críticos:
1. `lauda-server` (`maildir.rs`): executava `read` e parse completo de cabeçalhos RFC 822 para todos os milhares de arquivos em disco de forma síncrona.
2. `lauda-client` (`inbox_view.rs`): o Dioxus tentava renderizar mais de 10.000 nós no DOM da Web simultaneamente, travando a thread principal do browser WASM.
3. Além disso, e-mails importados via migração possuem UIDs mais altos que e-mails recentes pré-existentes. A ordenação cronológica precisa usar o `mtime` do arquivo no filesystem, preservado pelo Dovecot/IMAP APPEND.

### 20.2 Especificação da API (`lauda-server` & `lauda-types`)
- Estender `MessagesQuery` com:
  - `limit: Option<usize>` (padrão: 50, com teto de segurança)
  - `offset: Option<usize>` (padrão: 0)
- Na porta `MailBackend`:
  - `list_messages(&self, username: &str, mailbox: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<MessageEnvelope>, BackendError>`
- No adaptador `MaildirBackend`:
  - Varrer `cur/` e `new/` coletando apenas `(path, filename, mtime)` via `entry.metadata()?.modified()` (sem ler conteúdo dos arquivos).
  - Ordenar entradas por `mtime` decrescente (mais recentes primeiro).
  - Aplicar paginação `.skip(offset).take(limit)`.
  - Ler bytes e executar `MessageParser::parse_headers` **estritamente** para as mensagens da fatia requisitada.

### 20.3 Especificação da Janela de Exibição (`lauda-client`)
- Na porta `MailApiClient`:
  - `list_messages(&self, token: &str, mailbox: &str, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<MessageEnvelope>, ClientError>`
- No componente `InboxView`:
  - Carregar inicialmente as primeiras 50 mensagens mais recentes.
  - Disponibilizar ação/botão `[+ CARREGAR MAIS 50 MENSAGENS]` que busca a próxima fatia e anexa à lista reativa de mensagens.
  - Manter fluidez a 60 FPS com limite de itens renderizados e prevenção de clone inútil de dezenas de milhares de mensagens.

### 20.4 Critérios de Aceite (Ticket #66)
- [x] Rota `/messages` aceita `limit` e `offset`.
- [x] `MaildirBackend` ordena por `mtime` decrescente e abre apenas a fatia `(offset..offset+limit)` de arquivos.
- [x] `InMemoryMailBackend` respeita `limit` e `offset`.
- [x] Testes unitários e de integração herméticos validando ordenação por `mtime` e paginação com `limit`/`offset`.
- [x] `lauda-client` implementa carregamento paginado mantendo a lista fluida no Dioxus.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 21. Auditoria e Eliminação de Operações Bloqueantes no Cliente (Ticket #67)

### 21.1 Diagnóstico de Gargalos na Thread Principal (WASM UI)
Após auditoria dos componentes Dioxus e hooks reativos no `lauda-client`, foram mapeados os seguintes gargalos potenciais de bloqueio da thread principal do browser:
1. **Fallback de Busca Síncrono no Render Body (`inbox_view.rs`)**: A cada caractere digitado na busca, antes do retorno assíncrono do servidor, o Dioxus instanciava um novo `MessageIndex`, iterava sobre todos os envelopes locais criando structs `Message` fictícias e executava o motor BM25 síncrono.
2. **Re-parsing Pesado de Markdown/HTML a Cada Frame (`message_view.rs`)**: Toda alteração de estado no `MessageView` (como troca de abas, clique em botões ou re-renders reativos) reexecutava `clean_comment_tags` e `parse_markdown_to_html` sobre corpos inteiros de e-mail (até centenas de kilobytes), sem memoização.
3. **Clonagem Recorrente de Coleções no Render Loop (`calendar_view.rs`)**: A lista de eventos inteira era clonada (`events.read().clone()`) em cada ciclo reativo, seguida de re-particionamento vetorial completo.
4. **Substituição Agressiva de Estado no Background Sync (`ui/mod.rs`)**: O loop em segundo plano chamava `messages.set(...)` incondicionalmente a cada 10 segundos, forçando um reflow global no DOM mesmo quando não havia alterações.

### 21.2 Especificações das Soluções
- **Busca Instantânea Zero-Allocation no Cliente**: Substituir a montagem síncrona do índice BM25 no corpo do render por um filtro direto e insensível a maiúsculas/minúsculas sobre `subject` e `sender` dos envelopes locais enquanto a busca semântica completa transita assincronamente via HTTP.
- **Memoização com `use_memo` no `MessageView`**: Armazenar o resultado de `parse_markdown_to_html` e `clean_comment_tags` em sinais memoizados que só recalculam caso o `active_message` realmente mude.
- **Iteração por Referência no `CalendarView`**: Eliminar `.clone()` redundante na lista de eventos e particionar a stream por referência.
- **Higiene de Re-render no Background Sync**: Atualizar `messages` no sync em segundo plano apenas se a lista retornada divergir do estado atual ou se o usuário não tiver expandido a janela além da página base.

### 21.3 Critérios de Aceite (Ticket #67)
- [x] Fallback de busca no `inbox_view.rs` opera com filtro direto em sub-milissegundo sem alocação de `MessageIndex`.
- [x] Transformações de corpo em `message_view.rs` usam `use_memo` evitando re-parsing de Markdown em re-renders de interação.
- [x] `calendar_view.rs` itera sobre eventos sem clonar o vetor raiz.
- [x] Testes automatizados herméticos cobrindo busca local rápida e memoização de visualização.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 22. Limiar de Busca em 3 Caracteres e Feedback Visual Explicativo (Ticket #68)

### 22.1 Motivação e Usabilidade
Ao digitar os primeiros caracteres na barra de pesquisa (1 ou 2 caracteres), a interface exibia prematuramente o aviso de lista vazia *"Nenhuma mensagem corresponde à pesquisa e filtros."*, induzindo o usuário ao falso negativo (acreditar que a busca havia sido concluída com zero resultados quando, na verdade, o sistema nem iniciou o processamento). Além disso, disparar consultas assíncronas ao servidor para termos de 1 ou 2 caracteres causa tráfego de rede e consumo de CPU desnecessários para stop-words.

### 22.2 Regras de Busca e Estados da Interface
1. **Limiar Mínimo de 3 Caracteres**:
   - Tanto a requisição assíncrona HTTP (`HttpApiClient::search_messages`) quanto o filtro local instantâneo (`fast_filter_envelopes`) só são ativados quando o termo digitado (`query.trim()`) possuir 3 ou mais caracteres (`query.trim().chars().count() >= 3`).
   - Com 3 caracteres, acrônimos e termos técnicos essenciais (PIX, DOC, PDF, DEV, XML, CEP, RFC, etc.) são buscados com total precisão.
2. **Feedback Visual Imediato (Anti-Falso Negativo)**:
   - Quando o campo de pesquisa contiver 1 ou 2 caracteres (`0 < query.trim().chars().count() < 3`), a interface exibe um aviso explicativo no estilo Trash Polka:
     `"DIGITE AO MENOS 3 CARACTERES PARA PESQUISAR..."`.
   - A listagem de mensagens da pasta ativa permanece visível (respeitando os filtros de abas como TODOS, NÃO LIDOS, etc.), evitando que a tela esvazie abruptamente antes do usuário completar a palavra.
   - Caso a pasta ativa esteja vazia com busca incompleta, o estado vazio reforça a mensagem explicativa de digitação mínima em vez de alegar falso negativo de busca.
3. **Feedback de Busca Concluída**:
   - Quando a busca for válida ($\ge 3$ caracteres) e retornar 0 resultados, o aviso informa claramente: `"Nenhuma mensagem encontrada para '<query>' nos filtros ativos."`.

### 22.3 Critérios de Aceite (Ticket #68)
- [x] Requisição assíncrona de busca ao backend é suprimida se `query.trim().chars().count() < 3`.
- [x] Filtro local não esvazia a listagem quando `query.trim().chars().count() < 3`.
- [x] Banner / aviso explicativo `"DIGITE AO MENOS 3 CARACTERES PARA PESQUISAR..."` é exibido quando houver 1 ou 2 caracteres.
- [x] Mensagem de lista vazia com busca ativa ($\ge 3$) informa explicitamente o termo pesquisado.
- [x] Testes unitários herméticos no cliente validando a lógica de limiar de busca e feedback.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 23. Indexador Persistente SQLite em Background para Caixas Volumosas (Ticket #69)

### 23.1 Diagnóstico do Gargalo de Busca
Anteriormente, o `MaildirBackend::search_messages` iterava sobre todos os arquivos do Maildir da pasta requisitada a cada chamada de busca, lendo o arquivo de cada e-mail e recalculando embeddings na hora. Em caixas reais com dezenas de milhares de mensagens (ex: 10.000 a 30.000 e-mails), essa abordagem tornava a busca inutilizável e bloqueava o I/O do servidor.

### 23.2 Arquitetura do `SqliteIndexStore` e Worker em Background
1. **Persistência em SQLite Local (`sqlite_index.rs`)**:
   - Componente `SqliteIndexStore` em `lauda-server`, gerenciando um banco SQLite dedicado (`.lauda_index.db` no diretório raiz do Maildir do usuário).
   - Modo WAL ativado para concorrência segura entre leituras de busca e escritas do worker.
   - Tabelas gerenciadas:
     - `indexed_documents`: metadados do envelope (`mailbox`, `uid`, `mtime`, `subject`, `sender`, `date`, `is_read`, `is_flagged`, `has_attachments`), `doc_len`, frequências de termos em JSON e embeddings vetoriais binários compactos (`BLOB` de floats `f32` para os segmentos semânticos).
     - `inverted_index`: mapeamento `(term, mailbox, uid, tf)` para busca lexical e cálculo de BM25 instantâneo.
2. **Reconciliação com `dovecot-uidlist`**:
   - O arquivo `dovecot-uidlist` de cada pasta Maildir é a autoridade canônica dos UIDs válidos.
   - O indexador compara o conjunto de UIDs ativos com os UIDs já gravados no SQLite:
     - UIDs ausentes no SQLite são enfileirados para indexação incremental em background.
     - UIDs que deixaram de existir no `dovecot-uidlist` (excluídos via IMAP/Dovecot) são expurgados do SQLite.
     - Flags de leitura/favorito são sincronizadas de forma leve caso tenham mudado.
3. **Worker em Background (0 MB VRAM, CPU Pura)**:
   - Processamento de mensagens em lotes assíncronos (fatias de 20 a 50 mensagens por iteração), cedendo CPU (`tokio::task::yield_now` / pausas curtas) para garantir zero impacto na latência dos endpoints HTTP do daemon.
   - Vetorização semântica determinística com `SimpleCpuEmbeddingProvider` de 384 dimensões do `lauda-index`.
4. **Busca Híbrida Direta em Sub-Milissegundo**:
   - `search_messages` no `MaildirBackend` utiliza o `SqliteIndexStore` e executa a busca lexical e vetorial diretamente sobre o banco persistente, retornando os envelopes ranqueados sem tocar no sistema de arquivos durante a busca.
5. **Endpoint de Fatia de Sincronização (`/sync/index-slice`)**:
   - Rota no servidor que exporta os metadados e vetores dos últimos N dias (padrão: 90 dias) para consumo offline pelos clientes.

### 23.3 Critérios de Aceite (Ticket #69)
- [x] Criação de `SqliteIndexStore` com esquema persistente para documentos, termos BM25 e embeddings binários.
- [x] Implementação da reconciliação com `dovecot-uidlist` (detecção de novos e-mails e exclusões).
- [x] Worker em background indexando mensagens em lote sem bloqueio de I/O e com 0 MB VRAM.
- [x] `search_messages` do `MaildirBackend` utiliza o índice persistente em vez de varredura direta no filesystem.
- [x] Endpoint `/mailboxes/{mailbox}/index-slice` ou `/sync/index-slice` retornando mensagens indexadas da janela recente.
- [x] Testes automatizados herméticos cobrindo indexação, reconciliação, remoção e busca persistente.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 24. Busca Offline Híbrida com Sincronização da Fatia de 90 Dias (Ticket #70)

### 24.1 Arquitetura Offline-First no Cliente
Clientes móveis (Android NDK) e desktops (Linux x86_64) operam frequentemente em condições intermitentes ou sem conectividade (modo avião, metrô, áreas sem cobertura 4G/5G). Para preservar a experiência de busca inteligente por IA sem conexão:
1. O servidor disponibiliza a fatia pré-computada dos últimos 90 dias via `/mailboxes/{mailbox}/index-slice`.
2. O cliente sincroniza essa fatia em seu SQLite local (`LocalMessageCache`).
3. Quando desconectado, o cliente vetoriza a pergunta na CPU local com `SimpleCpuEmbeddingProvider` (< 0,05 ms) e executa busca híbrida instantânea (BM25 + vetores semânticos) sobre os dados locais.

### 24.2 Especificação das Camadas do Cliente
1. **Contrato de API (`ports.rs` / `HttpApiClient`)**:
   - `fetch_index_slice(&self, token: &str, mailbox: &str, days: u32) -> Result<Vec<lauda_index::IndexedDoc>, ClientError>`.
2. **Persistência Local no `LocalMessageCache` (`cache.rs`)**:
   - Tabela `cache_indexed_documents (mailbox TEXT, uid INTEGER, doc_json TEXT, PRIMARY KEY (mailbox, uid))`.
   - Método `save_index_slice(&self, mailbox: &str, docs: &[IndexedDoc])`.
   - Método `load_index_slice(&self, mailbox: &str) -> Option<MessageIndex>`.
3. **Resolução de Busca no `InboxView` (`inbox_view.rs`)**:
   - Tenta requisição assíncrona HTTP ao backend.
   - Em caso de indisponibilidade de rede ou erro HTTP:
     - Realiza fallback automático para `load_index_slice`.
     - Executa `search_hybrid` localmente na CPU (0 MB VRAM).
     - Renderiza aviso visual estilizado em *Trash Polka*:
       `"⚡ MODO OFFLINE: PESQUISANDO NO HISTÓRICO LOCAL DOS ÚLTIMOS 90 DIAS"`.
4. **Sincronização em Background (`ui/mod.rs`)**:
   - Sincroniza a fatia de índice periodicamente em segundo plano.

### 24.3 Critérios de Aceite (Ticket #70)
- [x] `HttpApiClient` implementa `fetch_index_slice` consumindo a rota do servidor.
- [x] `LocalMessageCache` persiste e hidrata `IndexedDoc`s e constrói `MessageIndex` em memória.
- [x] Fallback automático para busca híbrida local quando offline ou sem sinal.
- [x] Interface exibe feedback explícito informando o modo offline e a janela de 90 dias.
- [x] Testes automatizados herméticos cobrindo serialização de fatia, busca offline e fallback.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).

---

## 25. Desacoplamento de Operações Bloqueantes de I/O e Persistência na Interface (Ticket #71)

### 25.1 Diagnóstico de Bloqueio da UI Thread
Em interfaces reativas baseadas em Dioxus no desktop e mobile, quaisquer operações de I/O síncrono no sistema de arquivos ou consultas SQLite executadas diretamente nos event handlers de clique ou digitação (`oninput`) bloqueiam o loop de renderização da aplicação.
Foram identificados os seguintes gargalos:
1. **Gravação Síncrona de Anexos**: `message_view.rs` executava `std::fs::write(&target, bytes)` no handler síncrono do clique em "SALVAR ANEXO".
2. **Autocomplete Síncrono de Contatos**: `compose_view.rs` e `calendar_view.rs` executavam `cache.search_contacts` / `cache.search_contacts_fuzzy` diretamente durante `oninput`, consultando SQLite síncrono a cada pressionamento de tecla.
3. **Mutações Síncronas de Agenda e Contatos**: `calendar_view.rs` e `contacts_view.rs` invocando escritas e leituras síncronas na thread principal da interface.

### 25.2 Arquitetura de Desacoplamento Assíncrono
1. **I/O de Anexos em Background**:
   - O salvamento de anexo deve ser delegado a uma task assíncrona (`spawn(async move { ... })`), executando a gravação de arquivos fora da thread de renderização da interface.
   - A interface deve exibir feedback visual instantâneo (ex: status "SALVANDO ANEXO...", "ANEXO SALVO COM SUCESSO: <caminho>" ou mensagem de erro).
2. **Autocomplete Reativo Desacoplado**:
   - A busca de contatos no `compose_view` e `calendar_view` deve ser disparada de forma não-bloqueante via `spawn`, atualizando um sinal reativo assincronamente sem congelar a digitação.
3. **Persistência e Mutações Locais Não-Bloqueantes**:
   - Operações de cache de contatos e agenda devem rodar em segundo plano (`spawn`), mantendo a UI fluida a 60/120 FPS.
4. **Sincronização de Manifestos de Versão**:
   - Sincronização dos manifestos `site/version.json` e `site/version-nightly.json` para garantir conformidade contínua nos testes de distribuição e packaging.

### 25.3 Critérios de Aceite (Ticket #71)
- [x] Salvamento de anexo desacoplado em tarefa assíncrona com feedback não-bloqueante na UI.
- [x] Autocomplete de contatos no `compose_view` e `calendar_view` desacoplado da thread de renderização com atualização assíncrona.
- [x] Operações de I/O e persistência local protegidas contra travamentos da interface.
- [x] Sincronização de manifestos de versão atualizada conforme o arquivo `VERSION`.
- [x] Testes automatizados herméticos cobrindo fluxos assíncronos e não-bloqueantes.
- [x] Quality gate estrito verde (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`).
