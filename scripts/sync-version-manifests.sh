#!/usr/bin/env bash
# ==============================================================================
# Lauda — Sincronização Automática de Manifestos de Versão
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"

GIT_HASH="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || echo "head")"
DATE_TAG="$(date -u +"%Y%m%d")"

# 1. Garantir que VERSION reflita o commit atual (autorrecuperação ativa)
if command -v cccp >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cccp gen-version -f dev -w >/dev/null 2>&1 || true)
fi

# Fallback resiliente se VERSION não contiver o commit atual ou se cccp não estiver presente
CURRENT_VERSION_VAL="$(cat "$REPO_ROOT/VERSION" 2>/dev/null | tr -d '[:space:]' || true)"
if [ -z "$CURRENT_VERSION_VAL" ] || [[ "$CURRENT_VERSION_VAL" != *"$GIT_HASH"* ]]; then
    DEV_COUNT="$(git -C "$REPO_ROOT" rev-list --count HEAD 2>/dev/null || echo "1")"
    BASE_SEMVER="0.5.0"
    NEW_DEV_VER="${BASE_SEMVER}-dev.${DEV_COUNT}+${DATE_TAG}.${GIT_HASH}"
    echo "$NEW_DEV_VER" > "$REPO_ROOT/VERSION"
fi

CANONICAL_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
PUBLISHED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p "$SITE_DIR"

echo "-> Sincronizando manifestos de versão para: $CANONICAL_VERSION ($GIT_HASH)..."

# 2. Gerar site/version.json
cat <<EOF > "$SITE_DIR/version.json"
{
  "version": "$CANONICAL_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://bucket.2lp.in/lauda.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
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
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
  "published_at": "$PUBLISHED_AT"
}
EOF

echo "✓ Manifestos site/version.json e site/version-nightly.json sincronizados com sucesso."
