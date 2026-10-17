#!/usr/bin/env bash
# ==============================================================================
# Lauda — Script de Publicação do APK no Cloudflare R2 & Deploy do Worker
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APK_PATH="$REPO_ROOT/site/lauda.apk"
BUCKET_NAME="twolp-bucket"
REMOTE_KEY="lauda.apk"

echo "================================================================="
echo "Lauda — Publicação no Cloudflare R2 & Deploy Cloudflare Worker"
echo "================================================================="

if [ ! -f "$APK_PATH" ]; then
    if [ -f "$REPO_ROOT/site/lauda-latest.apk" ]; then
        cp "$REPO_ROOT/site/lauda-latest.apk" "$APK_PATH"
    else
        echo "ERRO: APK não encontrado em $APK_PATH."
        echo "Gere o APK primeiro compilando o cliente Android."
        exit 1
    fi
fi

APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
echo "-> APK encontrado: $APK_PATH ($APK_SIZE)"

# 1. Upload do APK para o Bucket R2 (lauda.apk e lauda-latest.apk)
echo "-> Enviando $REMOTE_KEY para o bucket R2: $BUCKET_NAME..."
npx wrangler r2 object put "$BUCKET_NAME/$REMOTE_KEY" \
    --file="$APK_PATH" \
    --content-type="application/vnd.android.package-archive" \
    --remote

npx wrangler r2 object put "$BUCKET_NAME/lauda-latest.apk" \
    --file="$APK_PATH" \
    --content-type="application/vnd.android.package-archive" \
    --remote || true

echo "✅ Upload do APK concluído com sucesso no Cloudflare R2 ($BUCKET_NAME)."

# 2. Deploy do Worker e Assets
echo "-> Fazendo deploy do Worker e Static Assets (lauda.2lp.in)..."
npx wrangler deploy

# 3. Anexar APK à release do Codeberg para autodescoberta pelo Obtainium
CURRENT_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
TAG_NAME="v$CURRENT_VERSION"
if command -v tea >/dev/null 2>&1; then
    echo "-> Sincronizando asset APK na release Codeberg ($TAG_NAME)..."
    VERSIONED_APK="/tmp/lauda-${CURRENT_VERSION}.apk"
    cp "$APK_PATH" "$VERSIONED_APK"
    tea release assets create "$TAG_NAME" "$VERSIONED_APK" >/dev/null 2>&1 || true
    rm -f "$VERSIONED_APK"
    echo "✅ Release $TAG_NAME no Codeberg possui asset APK compatível com Obtainium."
fi

echo ""
echo "================================================================="
echo "🎉 Publicação concluída com sucesso em https://lauda.2lp.in!"
echo "================================================================="

