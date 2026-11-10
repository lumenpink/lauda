# Especificação Técnica: Sincronização Automática de Versão e Reinstalação dos Hooks do CCCP

**ID do Ticket:** #97  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Em Implementação (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

O arquivo canônico `VERSION` e os manifestos de distribuição (`site/version.json` e `site/version-nightly.json`) permaneceram estáticos na versão `0.5.0-dev.508+20261107.c6530c0` ao longo de mais de 50 commits subsequentes. Como consequência, as compilações contínuas de APK para Android e os metadados da Pratilera permaneceram congelados na versão `dev.508`.

A investigação técnica identificou duas causas raiz:
1. O hook Git `pre-commit` (responsável no CCCP por computar a versão preditiva, atualizar `VERSION`, sincronizar manifestos e adicioná-los à área de preparação) não estava ativo no diretório `.git/hooks/` do repositório local.
2. O script de orquestração de manifestos (`scripts/sync-version-manifests.sh`) operava de forma puramente passiva, apenas lendo o arquivo `VERSION` existente sem validar se ele estava alinhado ao histórico do Git.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Problema:** A versão do aplicativo congelou porque a sincronização dependia exclusivamente de intervenção manual quando o hook estava desativado.
- **Alternativa Descartada 1:** Apenas reinstalar os hooks locais (`cccp hook --install`).
  - *Motivo do descarte:* Se outro clone do repositório for usado, se um desenvolvedor fizer commit em um ambiente sem os hooks instalados ou se a esteira de CI rodar a compilação, o arquivo continuaria defasado. A esteira e os scripts de empacotamento devem ser autossuficientes.
- **Alternativa Descartada 2:** Remover o arquivo `VERSION` e calcular a versão apenas em tempo de compilação via `build.rs`.
  - *Motivo do descarte:* Viola a norma GOST (GOST-03-VERSION-SEMVER), que exige o arquivo `VERSION` na raiz do projeto como fonte canônica de versão para ferramentas externas, scanners de vulnerabilidade e esteiras CI.
- **Escolha Adotada (Dupla Proteção):**
  1. Instalação e manutenção dos hooks canônicos do CCCP (`cccp hook --install`) para manter o repositório sincronizado a cada ciclo de desenvolvimento.
  2. Implementação de atualização dinâmica e resiliente no script `scripts/sync-version-manifests.sh`: se invocado, ele garante que `VERSION` esteja sincronizado com o commit HEAD atual antes de gerar `version.json` e `version-nightly.json`. Ele usa `cccp gen-version -f dev -w` se disponível, com fallback nativo via Git (`git rev-list` e `git rev-parse`) em containers enxutos.

---

## 3. Requisitos e Contratos

1. **Reinstalação dos Hooks Git:** Instalar os hooks canônicos do CCCP no repositório (`commit-msg`, `pre-commit`, `pre-push`, `post-commit`, `reference-transaction`, `post-merge`).
2. **Atualização Dinâmica de Versão em Scripts:** O script `scripts/sync-version-manifests.sh` deve atualizar ativamente o arquivo `VERSION` antes de exportar os manifestos JSON para o canal de desenvolvimento contínuo (nightly).
3. **Sincronização de Workspace:** Sincronizar o arquivo `VERSION` e as versões dos pacotes no workspace (`Cargo.toml`).
4. **Verificação de Qualidade GOST:** Validar a conformidade com `cccp check --strict`.
