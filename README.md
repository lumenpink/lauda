# lauda (Leitura e Escrita Aconchegante de Correio)

> Cliente de correio eletrônico acolhedor, focado na experiência aconchegante de ler cartas com calma e escrever com tranquilidade e sem ruído, construído em Rust com **Dioxus 0.7**, backend em **Axum** e motor inteligente de indexação **lauda-index**.

[![GOST Compliance](https://img.shields.io/badge/GOST-1.4.0-crimson)](https://codeberg.org/lumenpink/gost)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue)](./LICENSE)
[![Website](https://img.shields.io/badge/portal-lauda.2lp.in-red)](https://lauda.2lp.in)

---

## ⚡ Visão Geral do Sistema

O **`lauda`** substitui a complexidade pesada dos clientes tradicionais por uma micro-API JSON enxuta trafegando sobre rede segura privada (Tailscale / WireGuard):

* **Android NDK desde o Dia Zero:** Interface declarativa compilada nativamente para Android (`arm64-v8a` e `x86_64`) via Dioxus Mobile e Linux Desktop `x86_64`.
* **Alternador de Visão Trifásico:**
  - `txt auto` (Padrão): Renderiza o Markdown gerado pelo pipeline do Urupema com componentes estruturados nativos.
  - `txt original`: Texto cru RFC 5322 em fonte monoespaçada com quebras de linha preservadas.
  - `html`: Container isolado (sandbox) para e-mails transacionais e papéis de carta sem faixas espúrias.
* **Superbusca com BM25 & Sintaxe Gmail (`lauda-index`):**
  - Motor de busca nativo sem dependências externas.
  - Normalização completa com remoção de acentos e diacríticos.
  - Operadores de consulta: `from:`, `to:`, `subject:`, `has:attachment`, `is:unread`, `is:flagged`.
  - Expressões booleanas com `OR` explícito e negação com `-` ou `NOT`.
  - Ranqueamento **BM25** (com multiplicador de relevância no assunto e remetente).
  - **Fuzzy Search:** tolerância automática a erros de digitação (*typos*) no teclado móvel.
* **Resiliência Offline & Ciclo SWR (Stale-While-Revalidate):**
  - **Persistência Atômica:** cache particionado em disco com retenção de 90 dias para corpos completos.
  - **Optimistic UI & Outbox:** Envio não-bloqueante instantâneo (0ms de espera ao clicar em enviar) com fila persistente e retry em background.
  - **Reconciliação Offline:** fila de ações (`mark_read`, `mark_flagged`, `archive`, `delete`) sincronizadas com last-write-wins na reconexão.
  - **Badges de Conectividade:** indicador em tempo real de status `● ONLINE`, `○ OFFLINE (CACHE)` e contagem de itens na Outbox.
* **Editor Sem Travas:** Campo `From` livre com autocompletar de aliases favoritos e composição monoespaçada limpa.
* **Navegação Mobile Fluida:** Interceptação do gesto nativo de voltar do Android (`OnBackPressedDispatcher` e HTML5 History API).
* **Autenticação Direta:** Valida credenciais contra o Maildir/Dovecot e emite tokens JWT seguros.

---

## 🏗️ Topologia do Workspace Cargo

```text
lauda/
├── lauda-types/       # Contratos de dados e modelos serde compartilhados
├── lauda-index/       # Motor nativo de busca com parser Gmail, BM25 e fuzzy search
├── lauda-server/      # Micro-daemon HTTP (axum + tokio) conectado ao Maildir/Dovecot
└── lauda-client/      # Smolclient Dioxus multiplataforma (Android NDK + Desktop)
```

---

## 🧪 Testes e Qualidade

O projeto segue rigorosamente o ciclo **DD-TDD** e as diretrizes da norma [GOST](https://codeberg.org/lumenpink/gost):

```bash
# Executar todos os testes de unidade de todos os crates do workspace (45+ testes)
cargo test --all-targets --all-features

# Validar clippy em todo o workspace com zero avisos
cargo clippy --all-targets --all-features -- -D warnings

# Checar formatação determinística
cargo fmt --all -- --check
```

---

## 🚀 Como Executar

### 1. Iniciar o Micro-Daemon (`lauda-server`)
```bash
LAUDA_LISTEN_ADDR="0.0.0.0:8432" cargo run -p lauda-server
```

### 2. Iniciar o Smolclient Desktop (`lauda-client`)
```bash
cargo run -p lauda-client --features desktop
```

### 3. Compilar e Publicar o APK Nightly Android
```bash
./scripts/publish-nightly.sh
```

---

## 🎨 Vitrine e Documentação

A documentação pública segue o tema estético **Trash Polka** (halftones, recortes gráficos, carmesim e tipografia typewriter):
* **Portal online & Download:** [lauda.2lp.in](https://lauda.2lp.in)
* **Arquivos do Portal:** [`site/index.html`](./site/index.html) e [`site/_headers`](./site/_headers)
* **Especificação Técnica:** [`SPEC.md`](./SPEC.md)

---

## 📄 Licenciamento

* **Código-fonte:** [GPL-3.0](./LICENSE)
* **Mídias e Ilustrações:** [CC-BY-SA-4.0](./LICENSE-MEDIA)
* **Marcas e Insígnias:** [CC-BY-ND-4.0](./LICENSE-LOGO)
