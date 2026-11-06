#!/usr/bin/env bash
set -euo pipefail

echo "=== Test: Validar atualização de versão do Lauda ==="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="$REPO_ROOT/VERSION"
SITE_VERSION="$REPO_ROOT/site/version.json"

EXPECTED_MIN_DEV=480

CURRENT_VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"
echo "Versão atual: $CURRENT_VERSION"

# Verifica se a versão tem dev >= EXPECTED_MIN_DEV
DEV_COUNT=$(echo "$CURRENT_VERSION" | sed -nE 's/.*-dev\.([0-9]+)\+.*/\1/p')

if [ -z "$DEV_COUNT" ] || [ "$DEV_COUNT" -lt "$EXPECTED_MIN_DEV" ]; then
    echo "❌ ERRO: dev count ($DEV_COUNT) é inferior a $EXPECTED_MIN_DEV."
    exit 1
fi

# Verifica se site/version.json está sincronizado
if [ ! -f "$SITE_VERSION" ]; then
    echo "❌ ERRO: site/version.json não existe."
    exit 1
fi

SITE_VER_VAL=$(python3 -c "import json; print(json.load(open('$SITE_VERSION'))['version'])")
if [ "$SITE_VER_VAL" != "$CURRENT_VERSION" ]; then
    echo "❌ ERRO: site/version.json ($SITE_VER_VAL) diverge de VERSION ($CURRENT_VERSION)."
    exit 1
fi

echo "✓ Sucesso: versão do Lauda atualizada e manifestos sincronizados."
exit 0
