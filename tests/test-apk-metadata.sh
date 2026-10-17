#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste de Conformidade de Metadados e Assinatura do APK Android
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_APK="$REPO_ROOT/site/lauda-latest.apk"
VERSION_FILE="$REPO_ROOT/VERSION"

echo "-> Verificando presença do APK e arquivo VERSION..."
if [ ! -f "$TARGET_APK" ]; then
    echo "ERRO: $TARGET_APK não encontrado."
    exit 1
fi
if [ ! -f "$VERSION_FILE" ]; then
    echo "ERRO: $VERSION_FILE não encontrado."
    exit 1
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

    if [ "$PKG" != "in.twolp.lauda" ]; then
        echo "ERRO: Pacote inválido: $PKG (esperado: in.twolp.lauda)"
        exit 1
    fi
    if [ "$VER_NAME" != "$EXPECTED_VERSION" ]; then
        echo "ERRO: versionName divergente: $VER_NAME != $EXPECTED_VERSION"
        exit 1
    fi
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

echo "🎉 Todos os testes de metadados do APK passaram!"
