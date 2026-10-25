# Especificação Técnica — Migração do urupema para Crate Gambi e Desacoplamento E2E (#61)

- **ID do Ticket:** #61
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
Anteriormente, o `lauda` dependia de código-fonte de repositórios irmãos (`ishcondidim`, `papeli` e `urupema`) via referências relativas de sistema de arquivos (`path = "../../..."`).
Isso forçava a esteira de CI e o `Dockerfile` a clonar manualmente os três repositórios a cada ciclo de compilação ou teste.
Com a publicação da biblioteca `urupema` no registro soberano de pacotes Cargo (`gambi.2lp.in`), o `lauda-index` e `lauda-server` passam a consumi-la diretamente como crate soberana (`registry = "gambi"`).
Paralelamente, removem-se as `dev-dependencies` dos repositórios irmãos e os testes são consolidados para validar o ecossistema de ponta a ponta sem necessidade de clonar ou compilar as árvores de código externas.

## 2. Requisitos Técnicos
1. **Configuração do Registro Soberano Gambi:**
   - Declarar o registro `gambi` em `.cargo/config.toml`:
     ```toml
     [registries.gambi]
     index = "sparse+https://gambi.2lp.in/api/packages/lumen/cargo/"
     ```
   - No `Dockerfile`, configurar o Cargo para resolver tanto `crates.2lp.in` quanto `gambi.2lp.in`.
2. **Consumo da Crate `urupema`:**
   - Em `lauda-index/Cargo.toml` e `lauda-server/Cargo.toml`:
     ```toml
     urupema = { version = "0.1", registry = "gambi" }
     ```
3. **Eliminação de Clones Irmãos:**
   - Remover steps de clonagem de `ishcondidim`, `papeli` e `urupema` de `.forgejo/workflows/ci.yml` (jobs `test`, `deploy-web`, `deploy-banca`).
   - Remover instrução `RUN git clone ...` do `Dockerfile`.
   - Atualizar a auditoria em `tests/audit_cd_workflow.sh` para verificar a ausência de clones manuais e a presença do registro `gambi`.
4. **Desacoplamento dos Testes E2E:**
   - Purgar imports internos e testes de unidade espelhados de `ishcondidim` e `papeli` em `lauda-server/tests/`.
   - Manter as suítes E2E integradas (`e2e_full_chain_test.rs`, `e2e_full_loop_test.rs`, `e2e_resilience_test.rs`) validando o fluxo real de recepção, higienização pelo Urupema, armazenamento Maildir, e visualização/composição pelo Lauda Client.

## 3. Critérios de Aceitação
- Build e testes do workspace passando com `cargo test --workspace`.
- Auditoria do workflow soberano em `tests/audit_cd_workflow.sh` aprovada com validação de mutantes.
- Conformidade estrita Gosplan via `cccp check --strict`.
