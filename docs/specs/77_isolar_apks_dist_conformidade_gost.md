# Spec: Isolamento de APKs em dist/, Poda de README e Conformidade GOST (#77)

## Contexto
A auditoria estrita `cccp check --strict` acusou 84% de conformidade com duas falhas críticas:
1. `GOST-20`: `README.md` excede o teto normativo de 50 linhas (atualmente 109 linhas).
2. `GOST-24`: Presença de binários compilados (`lauda.apk`, `lauda-latest.apk`, `lauda-nightly.apk`) residindo no diretório rastreado `site/`.

## Objetivo
1. Redirecionar o destino de saída da compilação e publicação dos APKs de `site/` para `dist/apk/` em `scripts/publish-nightly.sh`, `scripts/package-all.sh` e testes associados.
2. Expurgar fisicamente os APKs do diretório `site/` e garantir que `dist/` esteja devidamente ignorado no `.gitignore`.
3. Podar o `README.md` para menos de 50 linhas, delegando a documentação aprofundada para `SPEC.md` e `docs/`.
4. Atingir 100% de conformidade estrita no `cccp check --strict`.

## Critérios de Aceitação
- `README.md` com $\le 50$ linhas.
- Nenhum artefato binário `.apk` localizado sob `site/`.
- `scripts/publish-nightly.sh` e `scripts/package-all.sh` gravam APKs em `dist/apk/`.
- Suítes de teste de ícones e metadados (`test-android-icons.sh`, `test-apk-metadata.sh`) ajustadas para inspecionar `dist/apk/lauda.apk`.
- `cccp check --strict` atinge 100% (13/13 conformidades).
