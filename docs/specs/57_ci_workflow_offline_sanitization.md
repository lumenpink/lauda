# Especificação Técnica — Saneamento do CI e Resiliência Offline Soberana (#57)

- **ID do Ticket:** #57
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0

## 1. Contexto & Motivação
O pipeline de integração contínua e o Dockerfile do `lauda` ainda guardam referências legadas a domínios obsoletos (`git.2lp.in` em vez de `gambi.2lp.in`) e dependem do crates.io público sem utilizar o pull-through cache soberano implantado no `obizero` (`crates.2lp.in`).
Adicionalmente, os passos de deploy na nuvem pública (Cloudflare Workers e R2) devem operar de modo tolerante a falhas ou quedas de WAN, assegurando que a geração de artefatos e o deploy local soberano no `obizero` nunca sejam bloqueados por oscilações externas.

## 2. Requisitos Técnicos
1. **Pull-Through Cache de Crates:**
   - Adicionar configuração do registro sparse soberano `crates.2lp.in` em `.cargo/config.toml`:
     ```toml
     [source.crates-io]
     replace-with = "sovereign-crates"

     [registries.sovereign-crates]
     index = "sparse+https://crates.2lp.in/index/"
     ```
   - No `Dockerfile` multi-stage do `lauda-server`, configurar o Cargo para utilizar `https://crates.2lp.in/index/`.
2. **Atualização de Domínio Soberano para Dependências Irmãs:**
   - No `Dockerfile`, substituir as URLs legadas `https://git.2lp.in/lumen/<dep>.git` por `https://gambi.2lp.in/lumen/<dep>.git`.
   - Garantir que não restem ocorrências do host legado `git.2lp.in` no repositório.
3. **Resiliência a Desconexão de WAN:**
   - Etapas dependentes de Cloudflare (`wrangler deploy`, `wrangler r2 object put`) devem verificar a presença das credenciais e tratar falhas de rede como advertências não-fatais sem abortar o build e deploy local no `obizero`.
4. **Deploy Local Contínuo:**
   - Manter o deploy estrito e verificado no Docker local do `obizero` (`deploy-server`) via `gambi.2lp.in` e `docker compose -f /home/lumen/stacks/mail/compose.yml up -d --no-build lauda-server`.

## 3. Critérios de Aceitação
- Teste automatizado `tests/audit_cd_workflow.sh` validando a ausência de `git.2lp.in` e a presença do registro `crates.2lp.in`.
- Validação de conformidade via `cccp check --strict`.
