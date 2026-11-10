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
TAG_AT_HEAD="$(git -C "$REPO_ROOT" tag --points-at HEAD 2>/dev/null | head -n 1 || true)"

if [ -n "$TAG_AT_HEAD" ]; then
    CLEAN_TAG="${TAG_AT_HEAD#v}"
    echo "$CLEAN_TAG" > "$REPO_ROOT/VERSION"
elif [ -z "$CURRENT_VERSION_VAL" ] || [[ "$CURRENT_VERSION_VAL" != *"$GIT_HASH"* ]]; then
    LAST_TAG="$(git -C "$REPO_ROOT" describe --tags --abbrev=0 2>/dev/null || echo "v0.4.0")"
    DEV_COUNT="$(git -C "$REPO_ROOT" rev-list "${LAST_TAG}..HEAD" --count 2>/dev/null || echo "1")"
    BASE_SEMVER="0.5.1"
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
