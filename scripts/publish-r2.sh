#!/usr/bin/env bash
# ==============================================================================
# Lauda — Script de Publicação do APK no Cloudflare R2 & Deploy do Worker
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APK_PATH="$REPO_ROOT/site/lauda-latest.apk"
BUCKET_NAME="lauda-downloads"
REMOTE_KEY="lauda-latest.apk"

echo "================================================================="
echo "Lauda — Publicação no Cloudflare R2 & Deploy Cloudflare Worker"
echo "================================================================="

if [ ! -f "$APK_PATH" ]; then
    echo "ERRO: APK não encontrado em $APK_PATH."
    echo "Gere o APK primeiro compilando o cliente Android."
    exit 1
fi

APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
echo "-> APK encontrado: $APK_PATH ($APK_SIZE)"

# 1. Upload do APK para o Bucket R2
echo "-> Enviando $REMOTE_KEY para o bucket R2: $BUCKET_NAME..."
npx wrangler r2 object put "$BUCKET_NAME/$REMOTE_KEY" \
    --file="$APK_PATH" \
    --content-type="application/vnd.android.package-archive" \
    --remote

echo "✅ Upload do APK concluído com sucesso no Cloudflare R2."

# 2. Deploy do Worker e Assets
echo "-> Fazendo deploy do Worker e Static Assets (lauda.2lp.in)..."
npx wrangler deploy

echo ""
echo "================================================================="
echo "🎉 Publicação concluída com sucesso em https://lauda.2lp.in!"
echo "================================================================="
