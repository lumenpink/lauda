# Especificação Normativa — Ticket #89
## Continuous Deployment Soberano do lauda-server no Nó Central (obizero) e Descomissionamento do Watchtower

**Norma de Referência:** GOST v1.5.0 (Perfil Lumen — Seção 9)  
**Status:** Aprovado para Implementação  
**Data:** 2026-10-20  

---

### 1. Escopo e Objetivos

Esta especificação define o pipeline soberano de entrega contínua (Continuous Deployment - CD) do serviço de correio **`lauda-server`** no nó central de execução **`obizero`**, substituindo em definitivo o mecanismo legado de webhooks baseado no Watchtower:

1. **Construção Hermética da Imagem:**
   - A imagem Docker `lumenpink/lauda-server:latest` é gerada diretamente no Docker daemon do nó `obizero` via runner nativo do Forgejo.
   - Preservação do ID da imagem anterior para contingência:
     ```bash
     OLD_IMAGE_ID=$(docker inspect --format='{{.Id}}' lumenpink/lauda-server:latest 2>/dev/null || true)
     ```
2. **Descomissionamento Compulsório do Watchtower:**
   - Remoção definitiva da etapa `Trigger Watchtower Webhook` no `.forgejo/workflows/ci.yml`.
   - Veto irrestrito a chamadas HTTP para `WATCHTOWER_WEBHOOK_URL` e `WATCHTOWER_TOKEN`.
   - Veto ao rótulo `com.centurylinklabs.watchtower.enable`.
3. **Deploy Local Imediato Sem Download Redundante:**
   - Acionamento imediato da nova imagem no Docker Compose runtime de correio:
     ```bash
     docker compose -f /home/lumen/stacks/mail/compose.yml up -d --no-build lauda-server
     ```
4. **Sondagem de Healthcheck Ativo & Rollback Automático:**
   - Sondagem ativa de até 15 segundos confirmando que o contêiner `lauda-server` permanece em estado `running`:
     ```bash
     for i in $(seq 1 15); do
       STATUS=$(docker inspect --format='{{.State.Status}}' lauda-server 2>/dev/null || echo "unknown")
       if [ "$STATUS" = "running" ]; then
         echo "✔ Container lauda-server saudável e ativo."
         exit 0
       fi
       sleep 1
     done
     ```
   - Em caso de falha de inicialização, aciona imediatamente o rollback para a imagem anterior salva e aborta o job com erro.
5. **Purga Automática de Camadas Órfãs:**
   - Execução de `docker image prune -f` após o deploy concluído com sucesso.
