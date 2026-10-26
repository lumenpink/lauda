#!/bin/bash
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="${ROOT_DIR}/.forgejo/workflows/ci.yml"
DOCKERFILE="${ROOT_DIR}/Dockerfile"
CARGO_CONFIG="${ROOT_DIR}/.cargo/config.toml"

echo "=== [TEST] Auditoria do Workflow de CD Soberano (lauda) ==="

check_not_present() {
    local target="$1"
    local pattern="$2"
    local desc="$3"
    if grep -qF -- "$pattern" "$target"; then
        echo "  [FAIL] $desc (encontrado '$pattern' indevidamente em $(basename "$target"))"
        return 1
    else
        echo "  [PASS] $desc"
    fi
}

check_present() {
    local target="$1"
    local pattern="$2"
    local desc="$3"
    if grep -qF -- "$pattern" "$target"; then
        echo "  [PASS] $desc"
    else
        echo "  [FAIL] $desc (padrão '$pattern' não encontrado em $(basename "$target"))"
        return 1
    fi
}

FAILED=0

# 1. Descomissionamento absoluto do Watchtower
check_not_present "$WORKFLOW" 'WATCHTOWER' "Ausência de variáveis e webhooks do Watchtower" || FAILED=1
check_not_present "$WORKFLOW" 'watchtower' "Ausência de referências literais ao Watchtower" || FAILED=1

# 2. Deploy local soberano no obizero via SSH (#64)
check_present "$WORKFLOW" 'OBIZERO_SSH_HOST' "Presença de configuração de SSH para deploy local no obizero" || FAILED=1
check_present "$WORKFLOW" 'ssh $SSH_OPTS' "Invocação de deploy soberano via SSH no obizero" || FAILED=1
check_present "$WORKFLOW" 'docker compose -f /home/lumen/stacks/mail/compose.yml up -d --no-build lauda-server' "Comando de deploy local sem download redundante" || FAILED=1

# 3. Sondagem ativa de healthcheck
check_present "$WORKFLOW" 'docker inspect --format=' "Sondagem ativa de healthcheck com docker inspect" || FAILED=1

# 4. Protocolo de rollback
check_present "$WORKFLOW" 'Rollback' "Protocolo de rollback automatizado" || FAILED=1

# 5. Purga de camadas órfãs
check_present "$WORKFLOW" 'docker image prune -f' "Limpeza automática de camadas órfãs via docker image prune" || FAILED=1

# 6. Ausência do domínio legado git.2lp.in (Saneamento Gambi #57)
check_not_present "$DOCKERFILE" 'git.2lp.in' "Ausência de git.2lp.in legado no Dockerfile" || FAILED=1
check_not_present "$WORKFLOW" 'git.2lp.in' "Ausência de git.2lp.in legado no ci.yml" || FAILED=1

# 7. Pull-through cache crates.2lp.in configurado (#57)
check_present "$CARGO_CONFIG" 'sparse+https://crates.2lp.in/index/' "Pull-through cache configurado em .cargo/config.toml" || FAILED=1
check_present "$DOCKERFILE" 'crates.2lp.in' "Pull-through cache configurado no Dockerfile" || FAILED=1

# 8. Ausência de clones manuais e configuração do registro gambi (#61)
check_not_present "$DOCKERFILE" 'git clone' "Ausência de clones de dependências no Dockerfile" || FAILED=1
check_not_present "$WORKFLOW" 'ishcondidim.git' "Ausência de clones de dependências no ci.yml" || FAILED=1
check_present "$CARGO_CONFIG" 'https://gambi.2lp.in/api/packages/lumen/cargo/' "Registro soberano gambi configurado em .cargo/config.toml" || FAILED=1
check_present "$DOCKERFILE" 'https://gambi.2lp.in/api/packages/lumen/cargo/' "Registro soberano gambi configurado no Dockerfile" || FAILED=1

# 9. Soberania irrestrita e saneamento de forges externas (#62)
check_not_present "$DOCKERFILE" 'docker.io' "Ausência de docker.io no Dockerfile" || FAILED=1
check_not_present "$WORKFLOW" 'docker.io' "Ausência de docker.io no ci.yml" || FAILED=1
check_not_present "$DOCKERFILE" 'github.com' "Ausência de github.com no Dockerfile" || FAILED=1
check_not_present "$WORKFLOW" 'github.com' "Ausência de github.com no ci.yml" || FAILED=1
check_not_present "$DOCKERFILE" 'codeberg.org' "Ausência de codeberg.org no Dockerfile" || FAILED=1
check_not_present "$WORKFLOW" 'codeberg.org' "Ausência de codeberg.org no ci.yml" || FAILED=1
check_not_present "$WORKFLOW" 'GIT_2LP_TOKEN' "Ausência de secret legado GIT_2LP_TOKEN no ci.yml" || FAILED=1
check_present "$WORKFLOW" 'GAMBI_2LP_IN_TOKEN' "Presença de GAMBI_2LP_IN_TOKEN no ci.yml" || FAILED=1
check_present "$DOCKERFILE" 'gambi.2lp.in/lumen/gost-ci-cross' "Uso de gost-ci-cross soberano no Dockerfile" || FAILED=1
check_present "$DOCKERFILE" 'gambi.2lp.in/lumen/mirror-debian' "Uso de mirror-debian soberano no Dockerfile" || FAILED=1

# 10. Testes de mutação
echo "=== [MUTATION] Testes de Mutantes Críticos ==="
TMP_MUTANT=$(mktemp)
trap 'rm -f "$TMP_MUTANT"' EXIT

echo "docker.io" >> "$TMP_MUTANT"
if grep -q "docker.io" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante E (reintrodução de docker.io) isolado"
fi

echo "GIT_2LP_TOKEN" >> "$TMP_MUTANT"
if grep -q "GIT_2LP_TOKEN" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante F (reintrodução de GIT_2LP_TOKEN) isolado"
fi

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

echo "git.2lp.in" >> "$TMP_MUTANT"
if grep -q "git.2lp.in" "$TMP_MUTANT"; then
    echo "  [PASS] Mutante D (reintrodução de git.2lp.in) isolado"
fi

if [ $FAILED -ne 0 ]; then
    echo "❌ [RED TEST] Auditoria falhou: pendências de saneamento detectadas."
    exit 1
fi

echo "✔ [GREEN TEST] Auditoria aprovada: CD soberano e saneamento validados."
