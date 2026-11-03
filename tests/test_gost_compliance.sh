#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste de Conformidade Estrita GOST & Isolamento de Artefatos (#77)
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "-> 1. Verificando teto normativo de linhas do README.md (<= 50 linhas)..."
README_LINES=$(wc -l < "$REPO_ROOT/README.md")
if [ "$README_LINES" -gt 50 ]; then
    echo "❌ FALHA: README.md possui $README_LINES linhas (limite: 50)."
    exit 1
fi
echo "✔ README.md possui $README_LINES linhas (dentro do limite)."

echo "-> 2. Verificando ausência de binários APK no diretório site/..."
SITE_APKS=$(find "$REPO_ROOT/site" -maxdepth 2 -name "*.apk" 2>/dev/null || true)
if [ -n "$SITE_APKS" ]; then
    echo "❌ FALHA: Encontrados APKs em site/:"
    echo "$SITE_APKS"
    exit 1
fi
echo "✔ Nenhum arquivo APK detectado em site/."

echo "-> 3. Executando auditoria estrita cccp check --strict..."
cccp check --strict
echo "✔ Auditoria GOST 100% aprovada!"
