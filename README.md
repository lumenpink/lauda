# lauda (Leitura e Escrita Aconchegante de Correio)

> Cliente de correio eletrônico acolhedor, focado na experiência aconchegante de ler cartas com calma e escrever com tranquilidade e sem ruído, construído em Rust com **Dioxus 0.7**, backend em **Axum** e motor inteligente de indexação **lauda-index**.

[![GOST Compliance](https://img.shields.io/badge/GOST-1.5.0-crimson)](https://gambi.2lp.in/lumen/gost)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue)](./LICENSE)
[![Website](https://img.shields.io/badge/portal-lauda.2lp.in-red)](https://lauda.2lp.in)

---

## ⚡ Visão Geral e Arquitetura

O **`lauda`** substitui a complexidade pesada dos clientes tradicionais por uma micro-API JSON enxuta e clientes nativos multiplataforma.

- **`lauda-types`**: Contratos de dados e modelos serde compartilhados.
- **`lauda-index`**: Motor nativo de busca com parser Gmail, BM25 e fuzzy search.
- **`lauda-server`**: Micro-daemon HTTP (axum + tokio) conectado ao Maildir/Dovecot.
- **`lauda-client`**: Smolclient Dioxus multiplataforma (Android NDK + Desktop).

Consulte o documento normativo de arquitetura e decisões em [`SPEC.md`](./SPEC.md).

---

## 🚀 Como Executar

```bash
# Iniciar o Micro-Daemon (lauda-server)
LAUDA_LISTEN_ADDR="0.0.0.0:8432" cargo run -p lauda-server

# Iniciar o Smolclient Desktop (lauda-client)
cargo run -p lauda-client --features desktop

# Executar suíte de testes de unidade e conformidade
cargo test --all-targets --all-features
```

---

## 📄 Licenciamento

- **Código-fonte:** [GPL-3.0](./LICENSE)
- **Mídias e Ilustrações:** [CC-BY-SA-4.0](./LICENSE-MEDIA)
- **Marcas e Insígnias:** [CC-BY-ND-4.0](./LICENSE-LOGO)
