#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste de Conformidade: CI Android Deploy Conteinerizado (#59)
# ==============================================================================
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="${ROOT_DIR}/.forgejo/workflows/ci.yml"
PUBLISH_SCRIPT="${ROOT_DIR}/scripts/publish-nightly.sh"
PACKAGE_SCRIPT="${ROOT_DIR}/scripts/package-all.sh"

echo "=== [TEST] Validação da Esteira Android Conteinerizada (#59) ==="

FAILED=0

check_present() {
    local target="$1"
    local pattern="$2"
    local desc="$3"
    if grep -qE -- "$pattern" "$target"; then
        echo "  [PASS] $desc"
    else
        echo "  [FAIL] $desc (padrão '$pattern' não encontrado em $(basename "$target"))"
        FAILED=1
    fi
}

check_not_present() {
    local target="$1"
    local pattern="$2"
    local desc="$3"
    if grep -qE -- "$pattern" "$target"; then
        echo "  [FAIL] $desc (encontrado '$pattern' indevidamente em $(basename "$target"))"
        FAILED=1
    else
        echo "  [PASS] $desc"
    fi
}

# 1. Verificar container canônico no job deploy-apk
check_present "$WORKFLOW" "image:\s*gambi\.2lp\.in/lumen/gost-ci-android:latest" "Job deploy-apk utiliza container canônico gost-ci-android:latest"

# 2. Verificar parametrização dinâmica de variáveis em publish-nightly.sh
check_present "$PUBLISH_SCRIPT" 'JAVA_HOME="?\$\{JAVA_HOME:-' "JAVA_HOME parametrizado dinamicamente com fallback"
check_present "$PUBLISH_SCRIPT" 'ANDROID_HOME="?\$\{ANDROID_HOME:-' "ANDROID_HOME parametrizado dinamicamente com fallback"
check_present "$PUBLISH_SCRIPT" 'ANDROID_NDK_HOME="?\$\{ANDROID_NDK_HOME:-' "ANDROID_NDK_HOME parametrizado dinamicamente com fallback"

# 3. Ausência de atribuição estática rígida das variáveis Android
check_not_present "$PUBLISH_SCRIPT" '^export ANDROID_NDK_HOME="/opt' "Ausência de export rígido estático de ANDROID_NDK_HOME"
check_not_present "$PUBLISH_SCRIPT" '^export JAVA_HOME="/usr' "Ausência de export rígido estático de JAVA_HOME"

# 4. Derivação de LLVM_STRIP dinamicamente
check_present "$PUBLISH_SCRIPT" 'LLVM_STRIP="?\$\{ANDROID_NDK_HOME\}' "LLVM_STRIP derivado de ANDROID_NDK_HOME"

# 5. [MUTAÇÃO] Sensibilidade da validação a regressões
echo "-> [MUTAÇÃO] Testando sensibilidade do teste contra mutantes..."
TEMP_WORKFLOW=$(mktemp)
cp "$WORKFLOW" "$TEMP_WORKFLOW"

# Mutação 1: Remoção do container gost-ci-android
sed -i '/gost-ci-android/d' "$TEMP_WORKFLOW"
if grep -qE "image:\s*gambi\.2lp\.in/lumen/gost-ci-android:latest" "$TEMP_WORKFLOW"; then
    echo "  [FAIL] Mutação 1 falhou: remoção de container não foi detectada!"
    FAILED=1
else
    echo "  [PASS] Mutação 1: remoção do container detectada com sucesso."
fi
rm -f "$TEMP_WORKFLOW"

if [ "$FAILED" -ne 0 ]; then
    echo "❌ [FALHA] Validação da esteira Android falhou."
    exit 1
fi

echo "🎉 [SUCESSO] Todos os requisitos da esteira Android conteinerizada foram aprovados!"
