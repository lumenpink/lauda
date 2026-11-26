# Spec #148 — Usar `upload-artifact@v3` e `download-artifact@v3` no CI do Lauda

- **Ticket:** `#148`
- **Escopo:** `.forgejo/workflows/ci.yml`

## Contexto e Árvore de Decisões
O runner do Forgejo/Gambi rejeitou `actions/upload-artifact@v4` e `actions/download-artifact@v4` com `GHESNotSupportedError: @actions/artifact v2.0.0+, upload-artifact@v4+ and download-artifact@v4+ are not currently supported on GHES`.
- **Escolha adotada:** utilizar `https://github.com/actions/upload-artifact@v3` e `https://github.com/actions/download-artifact@v3`.
- **Alternativa descartada:** manter `@v4`, incompatível com o servidor Forgejo Actions atual.
