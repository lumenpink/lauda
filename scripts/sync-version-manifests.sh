#!/usr/bin/env bash
# ==============================================================================
# Lauda — Sincronização Automática de Manifestos de Versão
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"

if [ ! -f "$REPO_ROOT/VERSION" ]; then
    echo "Erro: Arquivo VERSION não encontrado em $REPO_ROOT" >&2
    exit 1
fi

CANONICAL_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
GIT_HASH="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "head")"
PUBLISHED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p "$SITE_DIR"

echo "-> Sincronizando manifestos de versão para: $CANONICAL_VERSION ($GIT_HASH)..."

# 1. Gerar site/version.json
cat <<EOF > "$SITE_DIR/version.json"
{
  "version": "$CANONICAL_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-latest.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
  "published_at": "$PUBLISHED_AT"
}
EOF

# 2. Gerar site/version-nightly.json
cat <<EOF > "$SITE_DIR/version-nightly.json"
{
  "version": "$CANONICAL_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-nightly.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
  "published_at": "$PUBLISHED_AT"
}
EOF

echo "✓ Manifestos site/version.json e site/version-nightly.json sincronizados com sucesso."
