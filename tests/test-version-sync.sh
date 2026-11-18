#!/usr/bin/env bash
set -euo pipefail

echo "=== Test: Validar leitura passiva e imutabilidade de VERSION no Lauda ==="

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="$REPO_ROOT/VERSION"
SITE_VERSION="$REPO_ROOT/site/version.json"
SITE_NIGHTLY_VERSION="$REPO_ROOT/site/version-nightly.json"

# 1. Salvar backup do VERSION original
ORIGINAL_BACKUP="$(mktemp)"
cp "$VERSION_FILE" "$ORIGINAL_BACKUP"

cleanup() {
    if [ -f "$ORIGINAL_BACKUP" ]; then
        cp "$ORIGINAL_BACKUP" "$VERSION_FILE"
        rm -f "$ORIGINAL_BACKUP"
    fi
}
trap cleanup EXIT

# 2. Teste de Imutabilidade e Consumo Passivo:
# Definimos uma versão arbitrária commitada no repo. O sync-version-manifests.sh DEVE
# respeitar o arquivo sem mutá-lo em disco.
TEST_VERSION="0.9.9-test.42+20261118.immutable"
echo "$TEST_VERSION" > "$VERSION_FILE"
PRE_SHA=$(sha256sum "$VERSION_FILE" | awk '{print $1}')

echo "-> Executando scripts/sync-version-manifests.sh..."
"$REPO_ROOT/scripts/sync-version-manifests.sh"

POST_SHA=$(sha256sum "$VERSION_FILE" | awk '{print $1}')
POST_VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"

if [ "$PRE_SHA" != "$POST_SHA" ] || [ "$POST_VERSION" != "$TEST_VERSION" ]; then
    echo "❌ ERRO: sync-version-manifests.sh violou a imutabilidade de VERSION! Foi alterado para: $POST_VERSION"
    exit 1
fi

# 3. Verifica se os manifestos refletem com precisão o arquivo VERSION
if [ ! -f "$SITE_VERSION" ]; then
    echo "❌ ERRO: site/version.json não existe."
    exit 1
fi

SITE_VER_VAL=$(python3 -c "import json; print(json.load(open('$SITE_VERSION'))['version'])")
if [ "$SITE_VER_VAL" != "$TEST_VERSION" ]; then
    echo "❌ ERRO: site/version.json ($SITE_VER_VAL) diverge de VERSION ($TEST_VERSION)."
    exit 1
fi

SITE_NIGHTLY_VER_VAL=$(python3 -c "import json; print(json.load(open('$SITE_NIGHTLY_VERSION'))['version'])")
if [ "$SITE_NIGHTLY_VER_VAL" != "$TEST_VERSION" ]; then
    echo "❌ ERRO: site/version-nightly.json ($SITE_NIGHTLY_VER_VAL) diverge de VERSION ($TEST_VERSION)."
    exit 1
fi

# 4. Caso de borda: VERSION contendo espaços em branco ou quebras de linha
echo "  0.8.1-clean   " > "$VERSION_FILE"
"$REPO_ROOT/scripts/sync-version-manifests.sh"
CLEAN_VAL=$(python3 -c "import json; print(json.load(open('$SITE_VERSION'))['version'])")
if [ "$CLEAN_VAL" != "0.8.1-clean" ]; then
    echo "❌ ERRO: version.json não higienizou espaços em branco de VERSION!"
    exit 1
fi

echo "✓ Sucesso: VERSION permaneceu imutável e manifestos foram sincronizados passivamente com sucesso."
exit 0
