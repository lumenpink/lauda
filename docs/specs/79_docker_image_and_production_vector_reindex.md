# Spec: Atualização da Imagem Docker e Reindexação Vetorial em Produção (#79)

## Contexto
O daemon `lauda-server` em execução na pilha de correio do nó `obizero` (`/home/lumen/stacks/mail/compose.yml`) operava com imagem compilada antes da integração do `urupema 0.1.1`.
Além disso, a base SQLite de vetores de produção localizada em `/var/mail/lumen.pink/hi/.lauda_index.db` permaneceu estática desde 22 de outubro, preservando representações vetoriais ruidosas (underlines repetitivos e divisores de newsletters).

## Objetivo
1. Compilar e empacotar a nova imagem Docker `gambi.2lp.in/lumen/lauda-server:latest` contendo a versão com Urupema 0.1.1 e o suporte a expurgo e reindexação vetorial.
2. Efetuar backup de segurança da base anterior `.lauda_index.db` na caixa postal de produção (`/home/lumen/stacks/mail/data/dms/mail-data/lumen.pink/hi/`).
3. Reiniciar o serviço `lauda-server` na stack de produção com a nova imagem.
4. Validar o healthcheck do contêiner, os logs de inicialização e a regeneração limpa do índice.

## Critérios de Aceitação
- Nova imagem `gambi.2lp.in/lumen/lauda-server:latest` compilada e registrada localmente no daemon Docker.
- Contêiner `lauda-server` recriado com status `Up` e sem erros de inicialização.
- Base vetorial `.lauda_index.db` inicializada em modo WAL com sanitização ativa do Urupema 0.1.1.
