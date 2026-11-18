# Especificação Técnica: Leitura Passiva de VERSION na Sincronização de Manifestos

**ID do Ticket:** #110  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Em Especificação (Fase 3)  
**Responsável:** Lumen Pink / Antigravity  

---

## 1. Visão Geral e Motivação

No ticket #97, o script `scripts/sync-version-manifests.sh` recebeu uma rotina de "autorrecuperação ativa" que executava `cccp gen-version -f dev -w` e blocos de fallback com `git describe` para reescrever o arquivo `VERSION` na raiz do repositório a cada invocação.

Em tempo de desenvolvimento local, os hooks do Git gerenciados pelo CCCP já mantêm `VERSION` sincronizado a cada commit. Porém, na esteira de CI (`.forgejo/workflows/ci.yml`), a execução desse script em um ambiente de contêiner com checkout raso (sem histórico completo ou tags Git) causou falha no cálculo do `cccp gen-version`. Isso fez com que a esteira regredisse e reescrevesse ativamente o arquivo `VERSION` para `0.0.2-dev.1+20261113.5b5c4c4`, ignorando a versão real commitada (`0.6.0-dev...`).

A mutação ativa de arquivos de metadados canônicos na esteira viola o princípio da reprodutibilidade e determinismo. O script `scripts/sync-version-manifests.sh` e a esteira de CI devem atuar de forma estritamente passiva em relação a `VERSION`, apenas consumindo o conteúdo commitado na raiz do repositório.

---

## 2. Árvore de Decisão Técnica (Para a Natasha)

- **Problema:** A esteira de CI corrompe o arquivo canônico `VERSION` e gera manifestos/pacotes com versão indevida (`0.0.2`) ao tentar recalcular ativamente a versão do projeto em tempo de compilação.
- **Alternativa Descartada 1: Forçar `fetch-depth: 0` na esteira para que o cálculo dinâmico funcione.**
  - *Motivo do descarte:* A esteira de compilação não deve ter a responsabilidade de gerar ou mutar metadados canônicos de release. Se o repositório já versiona o arquivo `VERSION` na raiz a cada commit (garantido pelos hooks de desenvolvimento do CCCP), a CI deve ser determinística e usar o que foi commitado. Qualquer cálculo dinâmico na CI introduz risco de divergência entre o que o desenvolvedor assinou localmente e o que foi gerado remotamente.
- **Alternativa Descartada 2: Manter autorrecuperação apenas condicional (ex: `if [ "$CI" != "true" ]`).**
  - *Motivo do descarte:* Cria comportamento assimétrico entre desenvolvimento local e CI. O local para sincronizar `VERSION` é o ciclo de commit via hooks do CCCP (`pre-commit`), e não um script de exportação de manifestos JSON.
- **Escolha Adotada (Consumo Estritamente Passivo):**
  - O script `scripts/sync-version-manifests.sh` deve **apenas ler** o arquivo `$REPO_ROOT/VERSION`.
  - Se `$REPO_ROOT/VERSION` existir e for legível, seu conteúdo limpo é utilizado como `CANONICAL_VERSION`.
  - É expressamente proibida qualquer chamada a `cccp gen-version`, `git tag` ou escrita em `$REPO_ROOT/VERSION` dentro de `scripts/sync-version-manifests.sh`.
  - O teste de regressão `tests/test-version-sync.sh` deve validar que o arquivo `VERSION` permanece inalterado (idêntico byte a byte) durante a execução do script.

---

## 3. Arquivos Afetados

1. `scripts/sync-version-manifests.sh` — remoção da autorrecuperação e escrita em `VERSION`; leitura passiva do arquivo existente.
2. `tests/test-version-sync.sh` — atualização dos testes: remoção da exigência de autorrecuperação mutável e inclusão de asserção de imutabilidade de `VERSION`.
3. `docs/specs/110-ler-version-passivamente-na-esteira-sem-reescrever-arquivo-spec.md` — esta especificação técnica.

---

## 4. Requisitos e Critérios de Aceite

1. **Imutabilidade de `VERSION`:** A execução de `scripts/sync-version-manifests.sh` não deve alterar o arquivo `VERSION` sob nenhuma circunstância.
2. **Sincronia de Manifestos:** `site/version.json` e `site/version-nightly.json` devem ser preenchidos exatamente com a versão contida em `VERSION`.
3. **Resiliência em Ambientes Enxutos:** O script deve rodar com sucesso em ambientes sem ferramentas extras (sem necessidade de `cccp` ou árvore Git profunda).
4. **Validação de Testes:** O script de teste `tests/test-version-sync.sh` deve passar com código de saída 0.
5. **Quality Gate:** `cargo test` e validação estrita via `cccp check --strict`.
