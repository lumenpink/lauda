# Especificação Técnica — Ticket #142
## Fail-Fast em `scripts/package-all.sh --apk` Quando NDK/SDK Android Estiver Ausente

### 1. Contexto e Problema
Em `scripts/package-all.sh`, quando `--apk` era passado em um ambiente sem `dx`, target `aarch64-linux-android` ou `ANDROID_NDK_HOME`, o script entrava em um `else` silencioso (`"mantendo APK existente em ..."`), retornando código 0. Isso permitia que um `.apk` antigo (ex.: `dev.167`) fosse empacotado e publicado junto com manifestos de versão novos (ex.: `dev.176`).

### 2. Árvore de Decisões (Registro para a Natasha)
- **Decisão: Abortar com `exit 1` se o ambiente Android NDK/SDK estiver ausente ao rodar `--apk`**
  - *Alternativa descartada:* Emitir apenas warning e reaproveitar o `.apk` anterior de `site/` ou `target/dist/apk/`.
  - *Motivo:* Reutilizar um APK antigo sob um rótulo de versão novo quebra a integridade da entrega na Pratilera.

### 3. Critérios de Aceite
- [ ] `scripts/package-all.sh` não possui fallback `"mantendo APK existente"`.
- [ ] `scripts/package-all.sh` aborta com `exit 1` caso `dx`, `aarch64-linux-android` ou NDK falte.
- [ ] Teste automatizado valida a ausência do fallback e o fail-fast.
