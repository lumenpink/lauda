# Spec #147 — Apontar `apk_url` do Lauda para `https://bucket.2lp.in/pratilera/lauda.apk`

- **Ticket:** `#147`
- **Escopo:** `scripts/sync-version-manifests.sh`, `site/version.json`, `site/index.html`, `lauda-client/tests/update_domain_test.rs`, `tests/test-apk-metadata.sh`

## Contexto e Árvore de Decisões
O `kombinat` passa a publicar APKs exclusivamente sob o prefixo `pratilera/<app>.apk` no R2 (`twolp-bucket`), eliminando a duplicação de binários na raiz do bucket.
- **Escolha adotada:** atualizar todas as referências de download direto do Lauda (`https://bucket.2lp.in/lauda.apk`) para `https://bucket.2lp.in/pratilera/lauda.apk`.
- **Alternativa descartada:** manter cópia duplicada na raiz de `twolp-bucket`.
