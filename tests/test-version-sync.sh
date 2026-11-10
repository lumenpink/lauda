#!/usr/bin/env bash
set -euo pipefail

echo "=== Test: Validar atualização e autorrecuperação de versão do Lauda ==="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="$REPO_ROOT/VERSION"
SITE_VERSION="$REPO_ROOT/site/version.json"
SITE_NIGHTLY_VERSION="$REPO_ROOT/site/version-nightly.json"

EXPECTED_MIN_DEV=410
GIT_HASH="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "head")"

# Executa o script de sincronização de manifestos
echo "-> Executando scripts/sync-version-manifests.sh..."
"$REPO_ROOT/scripts/sync-version-manifests.sh"

CURRENT_VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"
echo "Versão canônica após sincronização: $CURRENT_VERSION"

# Verifica se a versão contém o hash do commit atual
if [[ "$CURRENT_VERSION" != *"$GIT_HASH"* ]]; then
    echo "❌ ERRO: VERSION ($CURRENT_VERSION) não contém o hash do commit HEAD ($GIT_HASH)."
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

SITE_NIGHTLY_VER_VAL=$(python3 -c "import json; print(json.load(open('$SITE_NIGHTLY_VERSION'))['version'])")
if [ "$SITE_NIGHTLY_VER_VAL" != "$CURRENT_VERSION" ]; then
    echo "❌ ERRO: site/version-nightly.json ($SITE_NIGHTLY_VER_VAL) diverge de VERSION ($CURRENT_VERSION)."
    exit 1
fi

# Teste de autorrecuperação: se VERSION estiver defasado, sync-version-manifests.sh deve corrigi-lo
echo "-> Testando autorrecuperação contra versão congelada..."
TEMP_BACKUP="$(mktemp)"
cp "$VERSION_FILE" "$TEMP_BACKUP"

echo "0.5.0-dev.100+20260101.stale00" > "$VERSION_FILE"
"$REPO_ROOT/scripts/sync-version-manifests.sh"

HEALED_VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"
cp "$TEMP_BACKUP" "$VERSION_FILE"
rm -f "$TEMP_BACKUP"

if [[ "$HEALED_VERSION" == *"stale00"* ]]; then
    echo "❌ ERRO: sync-version-manifests.sh não recuperou o arquivo VERSION defasado!"
    exit 1
fi

echo "✓ Sucesso: versão do Lauda validada, autorrecuperação comprovada e manifestos sincronizados."
exit 0
