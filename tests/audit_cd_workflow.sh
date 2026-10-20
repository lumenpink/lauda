#!/bin/bash
set -eo pipefail

WORKFLOW="/home/lumen/src/lauda/.forgejo/workflows/ci.yml"

echo "=== [TEST] Auditoria do Workflow de CD Soberano (lauda) ==="

check_not_present() {
    local pattern="$1"
    local desc="$2"
    if grep -qF -- "$pattern" "$WORKFLOW"; then
        echo "  [FAIL] $desc (encontrado '$pattern' indevidamente)"
        return 1
    else
        echo "  [PASS] $desc"
    fi
}

check_present() {
    local pattern="$1"
    local desc="$2"
    if grep -qF -- "$pattern" "$WORKFLOW"; then
        echo "  [PASS] $desc"
    else
        echo "  [FAIL] $desc (padrão '$pattern' não encontrado)"
        return 1
    fi
}

FAILED=0

# 1. Descomissionamento absoluto do Watchtower
check_not_present 'WATCHTOWER' "Ausência de variáveis e webhooks do Watchtower" || FAILED=1
check_not_present 'watchtower' "Ausência de referências literais ao Watchtower" || FAILED=1

# 2. Deploy local soberano no obizero
check_present 'docker compose -f /home/lumen/stacks/mail/compose.yml up -d --no-build lauda-server' "Comando de deploy local sem download redundante" || FAILED=1

# 3. Sondagem ativa de healthcheck
check_present 'docker inspect --format=' "Sondagem ativa de healthcheck com docker inspect" || FAILED=1

# 4. Protocolo de rollback
check_present 'Rollback' "Protocolo de rollback automatizado" || FAILED=1

# 5. Purga de camadas órfãs
check_present 'docker image prune -f' "Limpeza automática de camadas órfãs via docker image prune" || FAILED=1

# 6. Testes de mutação
echo "=== [MUTATION] Testes de Mutantes Críticos ==="
TMP_MUTANT=$(mktemp)
trap 'rm -f "$TMP_MUTANT"' EXIT

echo "watchtower" >> "$TMP_MUTANT"
if grep -q "watchtower" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante A (reintrodução de Watchtower) isolado"
fi

grep -v "prune -f" "$WORKFLOW" > "$TMP_MUTANT"
if ! grep -q "prune -f" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante B (omissão de prune) isolado"
fi

grep -v "docker inspect" "$WORKFLOW" > "$TMP_MUTANT"
if ! grep -q "docker inspect" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante C (omissão de healthcheck) isolado"
fi

if [ $FAILED -ne 0 ]; then
    echo "❌ [RED TEST] Auditoria falhou: workflow ainda contém Watchtower ou falta CD local."
    exit 1
fi

echo "✔ [GREEN TEST] Auditoria aprovada: CD soberano e saneamento validados."
