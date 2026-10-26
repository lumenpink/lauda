# Especificação Técnica — CD Soberano no Nó Central (obizero) via SSH (#64)

- **ID do Ticket:** #64
- **Status:** Aprovado
- **Autora:** Lumen
- **Alinhamento:** GOST 1.5.0 (Seção 9.2)

## 1. Contexto & Motivação
Na esteira do Forgejo Runner, o contêiner de execução possui o socket `/var/run/docker.sock` e a chave SSH montada (`~/.ssh/id_obitwo`), porém não possui o caminho de arquivos do host (`/home/lumen/stacks`) montado diretamente em seu sistema de arquivos local.
A tentativa de invocar `docker compose -f /home/lumen/stacks/mail/compose.yml` diretamente de dentro do contêiner do runner falha com `open /home/lumen/stacks/mail/compose.yml: no such file or directory`.
A solução canônica e soberana, alinhada com o projeto `lacraia`, consiste em disparar a atualização através de sessão SSH autenticada contra o host (`172.17.0.1` / `lumen`), mantendo a verificação ativa de integridade (healthcheck), o protocolo de rollback automático em caso de falha e a purga de imagens órfãs.

## 2. Requisitos Técnicos
1. **Conexão SSH Soberana:**
   - Obter host via `${OBIZERO_SSH_HOST:-172.17.0.1}` e usuário via `${OBIZERO_SSH_USER:-lumen}`.
   - Utilizar a chave SSH disponível (`~/.ssh/id_obitwo` ou `~/.ssh/id_ed25519` ou `${OBIZERO_SSH_KEY}`).
   - Utilizar `-o StrictHostKeyChecking=accept-new -o ConnectTimeout=15`.
2. **Execução no Host:**
   - Navegar até `~/stacks/mail` (ou referenciar `/home/lumen/stacks/mail/compose.yml`).
   - Invocar:
     ```bash
     docker compose -f /home/lumen/stacks/mail/compose.yml up -d --no-build lauda-server
     ```
   - Sondar a estabilidade do contêiner `lauda-server` por até 15 segundos (`docker inspect --format='{{.State.Status}}' lauda-server`).
   - Em caso de falha de estabilização, acionar rollback para a imagem anterior salva (`OLD_IMAGE_ID`) e abortar.
   - Purgar camadas órfãs após a conclusão (`docker image prune -f`).
3. **Critérios de Validação:**
   - Conformidade com `tests/audit_cd_workflow.sh`.
   - `cccp check --strict`.
