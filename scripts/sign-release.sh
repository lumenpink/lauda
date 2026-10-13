#!/usr/bin/env bash
# ==============================================================================
# Lauda — Script de Assinatura Soberana de APK com Keystore Oficial in.twolp
# ==============================================================================
set -euo pipefail

TARGET_APK="${1:-}"

if [ -z "$TARGET_APK" ] || [ ! -f "$TARGET_APK" ]; then
    echo "Uso: $0 <caminho-para-o-apk>"
    exit 1
fi

KEYSTORE_PATH="${LAUDA_KEYSTORE_PATH:-$HOME/.android/in_twolp_release.keystore}"
PROPS_PATH="${LAUDA_KEYSTORE_PROPS:-$HOME/.android/in_twolp_release.properties}"

if [ ! -f "$KEYSTORE_PATH" ] || [ ! -f "$PROPS_PATH" ]; then
    echo "ERRO: Keystore ou arquivo de propriedades não encontrado em:"
    echo "  - $KEYSTORE_PATH"
    echo "  - $PROPS_PATH"
    exit 1
fi

STORE_PASS="$(grep '^storePassword=' "$PROPS_PATH" | cut -d'=' -f2-)"
KEY_ALIAS="$(grep '^keyAlias=' "$PROPS_PATH" | cut -d'=' -f2-)"
KEY_PASS="$(grep '^keyPassword=' "$PROPS_PATH" | cut -d'=' -f2-)"

echo "-> Assinando APK: $TARGET_APK"
echo "-> Usando Keystore: $KEYSTORE_PATH (alias: $KEY_ALIAS)"

apksigner sign \
    --ks "$KEYSTORE_PATH" \
    --ks-key-alias "$KEY_ALIAS" \
    --ks-pass "pass:$STORE_PASS" \
    --key-pass "pass:$KEY_PASS" \
    "$TARGET_APK"

echo "-> Verificando assinatura do APK..."
apksigner verify --verbose "$TARGET_APK"

echo "✅ APK assinado e verificado com sucesso pela chave soberana in.twolp!"
