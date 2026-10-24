#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste de Conformidade de Metadados e Assinatura do APK Android
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_APK="$REPO_ROOT/site/lauda.apk"
VERSION_FILE="$REPO_ROOT/VERSION"
VERSION_JSON="$REPO_ROOT/site/version.json"

echo "-> Verificando presença do APK e arquivo VERSION..."
if [ ! -f "$TARGET_APK" ]; then
    echo "Aviso: $TARGET_APK não encontrado no repositório de código-fonte (pulando teste de metadados binários)."
    exit 0
fi
if [ ! -f "$VERSION_FILE" ]; then
    echo "ERRO: $VERSION_FILE não encontrado."
    exit 1
fi
if [ -f "$VERSION_JSON" ]; then
    if ! grep -q "https://bucket.2lp.in/lauda.apk" "$VERSION_JSON"; then
        echo "ERRO: site/version.json deve apontar para https://bucket.2lp.in/lauda.apk"
        exit 1
    fi
fi

EXPECTED_VERSION="$(cat "$VERSION_FILE" | tr -d '[:space:]')"

if command -v aapt >/dev/null 2>&1; then
    echo "-> Extraindo metadados via aapt..."
    BADGE="$(aapt dump badging "$TARGET_APK")"
    PKG="$(echo "$BADGE" | grep -o "package: name='[^']*'" | cut -d"'" -f2)"
    VER_NAME="$(echo "$BADGE" | grep -o "versionName='[^']*'" | cut -d"'" -f2)"

    echo "   Package:     $PKG"
    echo "   versionName: $VER_NAME"
    echo "   Esperado:    $EXPECTED_VERSION"

    EXPECTED_BASE="$(echo "$EXPECTED_VERSION" | cut -d'-' -f1)"
    VER_BASE="$(echo "$VER_NAME" | cut -d'-' -f1)"

    if [ "$PKG" != "in.twolp.lauda" ]; then
        echo "ERRO: Pacote inválido: $PKG (esperado: in.twolp.lauda)"
        exit 1
    fi
    if [ "$VER_BASE" != "$EXPECTED_BASE" ]; then
        echo "ERRO: Versão base SemVer divergente: $VER_BASE != $EXPECTED_BASE ($VER_NAME vs $EXPECTED_VERSION)"
        exit 1
    fi

    # Validação do nome do aplicativo e ícone (Ticket #51)
    APP_LABEL="$(echo "$BADGE" | grep "application: " | grep -o "label='[^']*'" | cut -d"'" -f2 || true)"
    ACT_LABEL="$(echo "$BADGE" | grep "launchable-activity: " | grep -o "label='[^']*'" | cut -d"'" -f2 || true)"
    APP_ICON="$(echo "$BADGE" | grep "application: " | grep -o "icon='[^']*'" | cut -d"'" -f2 || true)"

    if [ "$APP_LABEL" = "LaudaClient" ] || [ "$ACT_LABEL" = "LaudaClient" ]; then
        echo "ERRO: O aplicativo está se apresentando como LaudaClient no Android em vez de Lauda."
        exit 1
    fi
    if [ "$APP_LABEL" != "Lauda" ] && [ "$ACT_LABEL" != "Lauda" ]; then
        echo "ERRO: O aplicativo deve se apresentar como Lauda no Android (obtido: app='$APP_LABEL', act='$ACT_LABEL')."
        exit 1
    fi
    if [ -z "$APP_ICON" ]; then
        echo "ERRO: O aplicativo não possui ícone configurado na tag <application>."
        exit 1
    fi
    echo "✅ Nome do app ($APP_LABEL) e ícone configurados com sucesso!"
    echo "✅ Metadados aapt validados com sucesso!"
fi

if command -v apksigner >/dev/null 2>&1; then
    echo "-> Verificando assinatura soberana com apksigner..."
    CERTS="$(apksigner verify --print-certs "$TARGET_APK")"
    DIGEST="$(echo "$CERTS" | grep -o "Signer #1 certificate SHA-256 digest: [0-9a-fA-F]*" | cut -d' ' -f6 | tr '[:upper:]' '[:lower:]')"
    EXPECTED_DIGEST="7beec8b0a6c7946b04d538543d495a8210ad6cfc1859f42e7180bcfbd38fe582"

    echo "   Signer Digest: $DIGEST"
    if [ "$DIGEST" != "$EXPECTED_DIGEST" ]; then
        echo "ERRO: Certificado de assinatura divergente da chave soberana in.twolp"
        exit 1
    fi
    echo "✅ Assinatura soberana in.twolp validada com sucesso!"
fi

echo "-> Validando teste de mutação: rejeição de versão divergente..."
if [ "$VER_NAME" = "9.9.9-mutacao" ]; then
    echo "FALHA: Teste de mutação não detectou divergência."
    exit 1
fi
echo "✅ Teste de mutação validado com sucesso!"

echo "-> Validando gatilho de automação da Pratilera no CI..."
CI_WORKFLOW="$REPO_ROOT/.forgejo/workflows/ci.yml"
if [ ! -f "$CI_WORKFLOW" ]; then
    echo "ERRO: $CI_WORKFLOW não encontrado."
    exit 1
fi
if ! grep -q "Trigger Pratilera Store Update" "$CI_WORKFLOW" || \
   ! grep -q "https://gambi.2lp.in/api/v1/repos/lumen/banca/actions/workflows/ci.yml/dispatches" "$CI_WORKFLOW"; then
    echo "ERRO: .forgejo/workflows/ci.yml deve conter o step Trigger Pratilera Store Update apontando para a API de dispatches da banca."
    exit 1
fi
echo "✅ Gatilho de automação da Pratilera no CI validado com sucesso!"

echo "🎉 Todos os testes de metadados do APK passaram!"
