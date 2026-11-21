#!/usr/bin/env bash
set -eo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="${ROOT_DIR}/.forgejo/workflows/ci.yml"
DELIVERY="${ROOT_DIR}/.gost/delivery.toml"

echo "=== [TEST] Auditoria de Delegacao de CD ao Kombinat (lauda) ==="
FAILED=0

if [ ! -f "$WORKFLOW" ]; then echo "  [FAIL] $WORKFLOW ausente"; FAILED=1; else echo "  [PASS] $WORKFLOW presente"; fi
if [ ! -f "$DELIVERY" ]; then echo "  [FAIL] $DELIVERY ausente"; FAILED=1; else echo "  [PASS] $DELIVERY presente"; fi

for forbidden in "watchtower" "WATCHTOWER" "wrangler" "docker compose pull" "docker compose up" "id_obi" "banca.git" "trigger-pratilera"; do
    if grep -qiF -- "$forbidden" "$WORKFLOW"; then
        echo "  [FAIL] Padrao legado de CD direto encontrado na CI: $forbidden"
        FAILED=1
    fi
done

if ! grep -qF "cccp package upload" "$WORKFLOW"; then
    echo "  [FAIL] Workflow deve publicar artefatos via 'cccp package upload'"
    FAILED=1
else
    echo "  [PASS] Publicacao via 'cccp package upload' presente na CI"
fi

if ! grep -qF "[services]" "$DELIVERY"; then
    echo "  [FAIL] Secao [services] ausente em .gost/delivery.toml"
    FAILED=1
else
    echo "  [PASS] Secao [services] declarada em .gost/delivery.toml"
fi

if [ $FAILED -ne 0 ]; then
    echo "❌ [RED TEST] Auditoria de delegacao ao Kombinat falhou."
    exit 1
fi
echo "✔ [GREEN TEST] CI limpa (sem deploys/resets diretos) e delegacao declarada em .gost/delivery.toml."
