# lauda (O Smolclient Minimalista & Micro-Daemon de Correio)

> Cliente de e-mail ultraleve focado em leitura pura e redação rápida sem a sobrecarga de clientes IMAP/SMTP tradicionais, construído em Rust com **Dioxus 0.6** e backend em **Axum**.

[![GOST Compliance](https://img.shields.io/badge/GOST-1.0.0-crimson)](https://codeberg.org/lumenpink/gost)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue)](./LICENSE)
[![Website](https://img.shields.io/badge/portal-lauda.2lp.in-red)](https://lauda.2lp.in)

---

## ⚡ Visão Geral do Sistema

O **`lauda`** substitui a complexidade pesada dos clientes tradicionais por uma micro-API JSON enxuta trafegando sobre rede segura privada (Tailscale / WireGuard):

* **Android NDK desde o Dia Zero:** Interface declarativa compilada nativamente para Android (Dioxus Mobile) e Linux Desktop `x86_64`.
* **Alternador de Visão Trifásico:**
  - `txt auto` (Padrão): Renderiza o Markdown gerado pelo Urupema com componentes estruturados nativos via `pulldown-cmark`.
  - `txt original`: Texto cru RFC 5322 em fonte monoespaçada com quebras de linha preservadas.
  - `html`: Container isolado (sandbox) para documentos tabulares inevitáveis.
* **Editor Sem Travas:** Campo `From` livre com autocompletar de aliases favoritos e redação monoespaçada limpa.
* **Autenticação Direta:** Valida credenciais contra o Dovecot via socket UNIX local e emite tokens JWT seguros.

---

## 🏗️ Topologia do Workspace Cargo

```text
lauda/
├── lauda-types/       # Contratos de dados e modelos serde compartilhados
├── lauda-server/      # Micro-daemon HTTP (axum + tokio) conectado ao Dovecot/Postfix
└── lauda-client/      # Smolclient Dioxus multiplataforma (Android NDK + Desktop)
```

---

## 🧪 Testes e Qualidade

O projeto segue rigorosamente o ciclo **DD-TDD** e as diretrizes da norma [GOST](https://codeberg.org/lumenpink/gost):

```bash
# Executar todos os testes de unidade de todos os crates do workspace
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
cargo run -p lauda-server
```

### 2. Iniciar o Smolclient Desktop (`lauda-client`)
```bash
cargo run -p lauda-client --features desktop
```

---

## 🎨 Vitrine e Documentação

A documentação pública segue o tema estético **Trash Polka** (halftones, recortes gráficos, carmesim e tipografia typewriter):
* **Portal online:** [lauda.2lp.in](https://lauda.2lp.in)
* **Arquivos:** [`site/index.html`](./site/index.html) e [`site/_headers`](./site/_headers)
* **Especificação Técnica:** [`SPEC.md`](./SPEC.md)

---

## 📄 Licenciamento

* **Código-fonte:** [GPL-3.0](./LICENSE)
* **Mídias e Ilustrações:** [CC-BY-SA-4.0](./LICENSE-MEDIA)
* **Marcas e Insígnias:** [CC-BY-ND-4.0](./LICENSE-LOGO)
