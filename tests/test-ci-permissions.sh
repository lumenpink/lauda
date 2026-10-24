#!/bin/bash
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOWS_DIR="${ROOT_DIR}/.forgejo/workflows"

echo "=== [TEST] Auditoria de ausência de permissions no Forgejo CI (lauda) ==="

FAILED=0

for wf in "${WORKFLOWS_DIR}"/*.yml "${WORKFLOWS_DIR}"/*.yaml; do
    [ -f "$wf" ] || continue
    if grep -nE '^[[:space:]]*permissions:' "$wf"; then
        echo "  [FAIL] Campo 'permissions:' incompatível detectado em $(basename "$wf")"
        FAILED=1
    else
        echo "  [PASS] $(basename "$wf") livre de diretiva 'permissions:'"
    fi
done

if [ $FAILED -ne 0 ]; then
    echo "❌ [RED TEST] Auditoria falhou: encontrada diretiva permissions incompatível com Forgejo."
    exit 1
fi

echo "✔ [GREEN TEST] Todos os workflows Forgejo estão em conformidade sem permissions."
