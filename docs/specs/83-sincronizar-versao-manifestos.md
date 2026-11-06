# Especificação Técnica: Sincronização de Versão e Manifestos do Lauda

- **Ticket:** [#83](https://gambi.2lp.in/lumen/lauda/issues/83)
- **Status:** Em Andamento
- **Autor:** Lumen Pink (Baquara)

---

## 1. Objetivo

Atualizar o arquivo canônico `VERSION` no repositório `lauda` para refletir o progresso recente do projeto (`0.5.0-dev.487+20261106.a8acaa9`) e sincronizar os manifestos de versão (`site/version.json` e `site/version-nightly.json`), garantindo que tanto a Pratilera quanto o verificador interno de atualizações reconheçam e notifiquem novas builds.

---

## 2. Critérios de Aceite

1. O arquivo `VERSION` deve conter a versão canônica no formato SemVer 2.0.0.
2. Os manifestos `site/version.json` e `site/version-nightly.json` devem ser atualizados via `scripts/sync-version-manifests.sh`.
3. Verificações de conformidade estrita e suíte de testes devem ser aprovadas com 100% de sucesso.
2026-11-06 21:10:19 UTC
