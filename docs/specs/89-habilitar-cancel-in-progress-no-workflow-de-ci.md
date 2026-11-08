# Especificação Técnica — Ticket #89
## Habilitar `cancel-in-progress: true` no Workflow de CI

- **Ticket:** #89
- **Branch:** `ci/89-habilitar-cancel-in-progress-no-workflow-de-ci`
- **Data:** 2026-11-08

---

### 1. Contexto e Motivação
No arquivo `.forgejo/workflows/ci.yml`, o grupo de concorrência estava configurado como:
```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: false
```
Com `cancel-in-progress: false`, caso um run anterior entre em estado retido, bloqueado ou demorado na mesma referência (`refs/heads/main` ou branches de trabalho), novas execuções enfileiradas não conseguem iniciar e ficam em espera indefinida.

Ao adotar `cancel-in-progress: true`, o Forgejo cancela automaticamente qualquer execução anterior ainda ativa/pendente para a mesma referência assim que um novo commit/push é recebido, liberando a fila e evitando retenções desnecessárias.

---

### 2. Requisitos Técnicos
1. Configurar expressamente `cancel-in-progress: true` sob `concurrency` em `.forgejo/workflows/ci.yml`.
2. Validar via teste de auditoria em `tests/audit_cd_workflow.sh` que a diretiva `cancel-in-progress: true` está presente e contratualmente assegurada.
3. Assegurar teste de mutação para falhar caso `cancel-in-progress: false` seja reintroduzido.

---

### 3. Fases GOST DD-TDD
- **Fase 3 (Spec):** Elaboração deste documento de especificação.
- **Fase 4 (Red Test):** Adição de asserção em `tests/audit_cd_workflow.sh` exigindo `cancel-in-progress: true`, falhando enquanto o workflow estiver com `false`.
- **Fase 5 (Green Implement):** Alteração de `cancel-in-progress: false` para `cancel-in-progress: true` em `.forgejo/workflows/ci.yml`.
- **Fase 6 (Refactor):** Revisão de formatação e integridade do workflow.
- **Fase 7 (Mutation Test):** Verificação de que mutante com `cancel-in-progress: false` é detectado e morto.
- **Fase 8 (Quality Gate):** Checagem rigorosa com `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cccp check --strict`.
- **Fase 9 (Finish):** Solicitação de autorização para merge e push.
