# Especificação Técnica — Remoção de Campo Permissions Incompatível com Forgejo (#60)

- **ID do Ticket:** #60
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0 / Forgejo Actions

## 1. Contexto & Motivação
O Forgejo Actions runner emite avisos e falhas estruturais quando workflows contêm a chave top-level ou job-level `permissions:`, como:
```yaml
permissions:
  contents: write
```
No ecossistema soberano hospedado no Forgejo (`gambi.2lp.in`), a concessão de credenciais e permissões é gerenciada diretamente pelo token de execução injetado pelo runner soberano, tornando o campo `permissions:` do GitHub Actions não suportado e causa de degradação ou rejeição na execução da esteira.

## 2. Requisitos Técnicos
1. **Remoção de `permissions:` em `.forgejo/workflows/ci.yml`:**
   - Eliminar a diretiva global `permissions:` e sub-chaves associadas (`contents: write`).
2. **Conformidade Sintática e Operacional:**
   - O workflow deve permanecer sintaticamente válido em YAML.
   - Manter as demais diretivas (`concurrency`, `jobs`, containers, steps) inalteradas.
3. **Auditoria Automatizada:**
   - Criar teste em `tests/test-ci-permissions.sh` que audite todos os arquivos em `.forgejo/workflows/*.yml` assegurando que nenhuma ocorrência de `permissions:` persista.
   - Incluir verificação de mutação comprovando que a reintrodução de `permissions:` faz o teste falhar.
