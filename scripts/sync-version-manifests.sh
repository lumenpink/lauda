#!/usr/bin/env bash
# ==============================================================================
# Lauda — Sincronização Automática de Manifestos de Versão
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"

GIT_HASH="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "head")"

# 1. Leitura estritamente passiva da versão canônica a partir de VERSION na raiz
if [ ! -f "$REPO_ROOT/VERSION" ]; then
    echo "❌ ERRO: Arquivo $REPO_ROOT/VERSION não encontrado!" >&2
    exit 1
fi

CANONICAL_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"

if [ -z "$CANONICAL_VERSION" ]; then
    echo "❌ ERRO: Arquivo $REPO_ROOT/VERSION está vazio!" >&2
    exit 1
fi

PUBLISHED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p "$SITE_DIR"

echo "-> Sincronizando manifestos de versão para: $CANONICAL_VERSION ($GIT_HASH)..."

# 2. Gerar site/version.json
cat <<EOF > "$SITE_DIR/version.json"
{
  "version": "$CANONICAL_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://bucket.2lp.in/pratilera/lauda.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build do Lauda Smolclient ($CANONICAL_VERSION).",
  "published_at": "$PUBLISHED_AT"
}
EOF

# 3. Gerar site/version-nightly.json
cat <<EOF > "$SITE_DIR/version-nightly.json"
{
  "version": "$CANONICAL_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-nightly.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build do Lauda Smolclient ($CANONICAL_VERSION).",
  "published_at": "$PUBLISHED_AT"
}
EOF

echo "✓ Manifestos site/version.json e site/version-nightly.json sincronizados com sucesso."
