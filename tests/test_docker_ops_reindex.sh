#!/usr/bin/env bash
# ==============================================================================
# Lauda — Validação de Deploy e Reindexação em Produção (#79)
# ==============================================================================
set -euo pipefail

IMAGE_NAME="gambi.2lp.in/lumen/lauda-server:latest"
CONTAINER_NAME="lauda-server"
MAIL_DIR="/home/lumen/stacks/mail/data/dms/mail-data/lumen.pink/hi"
INDEX_DB="$MAIL_DIR/.lauda_index.db"
TODAY="$(date +%Y-%m-%d)"

echo "-> 1. Verificando data de criação da imagem Docker ($IMAGE_NAME)..."
IMG_DATE=$(docker inspect "$IMAGE_NAME" --format '{{.Created}}' | cut -d'T' -f1)
if [ "$IMG_DATE" != "$TODAY" ]; then
    echo "❌ FALHA: A imagem Docker $IMAGE_NAME foi criada em $IMG_DATE (esperado: $TODAY)."
    exit 1
fi
echo "✔ Imagem Docker criada hoje ($IMG_DATE)."

echo "-> 2. Verificando se o contêiner $CONTAINER_NAME está ativo com a imagem atualizada..."
RUNNING_IMAGE_ID=$(docker inspect "$CONTAINER_NAME" --format '{{.Image}}')
LATEST_IMAGE_ID=$(docker inspect "$IMAGE_NAME" --format '{{.Id}}')
if [ "$RUNNING_IMAGE_ID" != "$LATEST_IMAGE_ID" ]; then
    echo "❌ FALHA: O contêiner $CONTAINER_NAME está executando uma versão antiga da imagem ($RUNNING_IMAGE_ID != $LATEST_IMAGE_ID)."
    exit 1
fi
echo "✔ Contêiner $CONTAINER_NAME executando a imagem mais recente ($LATEST_IMAGE_ID)."

echo "-> 3. Verificando integridade e recência da base de dados vetorial ($INDEX_DB)..."
if [ ! -f "$INDEX_DB" ]; then
    echo "❌ FALHA: $INDEX_DB não existe."
    exit 1
fi

DB_DATE=$(stat -c %y "$INDEX_DB" | cut -d' ' -f1)
if [ "$DB_DATE" != "$TODAY" ]; then
    echo "❌ FALHA: $INDEX_DB foi modificado em $DB_DATE (esperado reindexação em: $TODAY)."
    exit 1
fi
echo "✔ Base vetorial reindexada hoje com sucesso ($DB_DATE)."

echo "🎉 Todas as asserções de produção foram validadas com sucesso!"
