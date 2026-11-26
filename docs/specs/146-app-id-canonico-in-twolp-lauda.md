# Spec #146 — Restaurar `app_id` canônico `in.twolp.lauda` em `.gost/delivery.toml`

- **Ticket:** `#146`
- **Escopo:** `.gost/delivery.toml`

## Contexto e Árvore de Decisões
Na migração do CD declarativo (ph9, commit `1e71c92`), `.gost/delivery.toml` foi criado com `app_id = "org.pink.lauda"`, divergindo do `applicationId = "in.twolp.lauda"` definido em `lauda-client/Dioxus.toml` e `scripts/publish-nightly.sh`. Isso fez o `kombinat` criar uma entrada duplicada na Pratilera.
- **Escolha adotada:** alinhar `.gost/delivery.toml` para `in.twolp.lauda`, preservando compatibilidade com `AccountManager`, `SyncAdapter` e instalações Android existentes.
- **Alternativa descartada:** alterar o `applicationId` do app Android para `org.pink.lauda`, o que quebraria contas e dados locais no dispositivo.
