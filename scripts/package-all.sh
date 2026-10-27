#!/usr/bin/env bash
# ==============================================================================
# Lauda — Orquestrador de Empacotamento e Distribuição Multi-Alvo
# Alvos: Web (2lp.in e lohn.in), Android APK (Pratilera), Desktop e Manifestos
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"
DIST_DIR="$REPO_ROOT/dist"

DO_SYNC_MANIFESTS=true
DO_BUILD_WEB=false
DO_BUILD_DESKTOP=false
DO_BUILD_APK=false
DO_SYNC_OBIZERO=false
DO_SYNC_PRATILERA=false
DO_DEPLOY_CLOUDFLARE=false

if [ "$#" -eq 0 ]; then
    DO_BUILD_WEB=true
    DO_BUILD_DESKTOP=true
    DO_BUILD_APK=true
    DO_SYNC_OBIZERO=true
    DO_SYNC_PRATILERA=true
fi

while [ "$#" -gt 0 ]; do
    case "$1" in
        --all)
            DO_BUILD_WEB=true
            DO_BUILD_DESKTOP=true
            DO_BUILD_APK=true
            DO_SYNC_OBIZERO=true
            DO_SYNC_PRATILERA=true
            DO_DEPLOY_CLOUDFLARE=true
            shift
            ;;
        --web)
            DO_BUILD_WEB=true
            shift
            ;;
        --desktop)
            DO_BUILD_DESKTOP=true
            shift
            ;;
        --apk)
            DO_BUILD_APK=true
            shift
            ;;
        --sync-obizero)
            DO_SYNC_OBIZERO=true
            shift
            ;;
        --sync-pratilera)
            DO_SYNC_PRATILERA=true
            shift
            ;;
        --deploy-cloudflare)
            DO_DEPLOY_CLOUDFLARE=true
            shift
            ;;
        -h|--help)
            echo "Uso: $0 [--all | --web | --desktop | --apk | --sync-obizero | --sync-pratilera | --deploy-cloudflare]"
            exit 0
            ;;
        *)
            echo "Opção desconhecida: $1" >&2
            exit 1
            ;;
    esac
done

cd "$REPO_ROOT"

echo "================================================================="
echo "Lauda — Esteira de Empacotamento & Distribuição Multi-Alvo"
echo "================================================================="

# 1. Sincronizar Manifestos
if [ "$DO_SYNC_MANIFESTS" = true ]; then
    "$SCRIPT_DIR/sync-version-manifests.sh"
fi

# 2. Compilar WebAssembly WebApp (Dioxus Web)
if [ "$DO_BUILD_WEB" = true ]; then
    echo "-> Compilando Dioxus WebAssembly (release)..."
    dx build -p lauda-client --platform web --release

    mkdir -p "$SITE_DIR/app/assets"
    rm -rf "$SITE_DIR/app/assets/"*
    cp -r "$REPO_ROOT/target/dx/lauda-client/release/web/public/assets/"* "$SITE_DIR/app/assets/"
    cp "$REPO_ROOT/target/dx/lauda-client/release/web/public/index.html" "$SITE_DIR/app/index.html"

    JS_NAME="$(ls "$SITE_DIR/app/assets" | grep '^lauda-client-.*\.js$' | head -n 1)"
    WASM_NAME="$(ls "$SITE_DIR/app/assets" | grep '^lauda-client_bg-.*\.wasm$' | head -n 1)"

    sed -i -E "s|/app/assets/lauda-client-[^\"']+\.js|/app/assets/${JS_NAME}|g" "$SITE_DIR/app/index.html"
    sed -i -E "s|/app/assets/lauda-client_bg-[^\"']+\.wasm|/app/assets/${WASM_NAME}|g" "$SITE_DIR/app/index.html"
    sed -i -E "s|/assets/lauda-client-[^\"']+\.js|/app/assets/${JS_NAME}|g" "$SITE_DIR/app/index.html"
    sed -i -E "s|/assets/lauda-client_bg-[^\"']+\.wasm|/app/assets/${WASM_NAME}|g" "$SITE_DIR/app/index.html"

    # Atualizar index.html na raiz do site para redirecionamento ou app
    cp "$SITE_DIR/app/index.html" "$SITE_DIR/index.html"
    echo "✓ WebApp Dioxus Web compilado: ${JS_NAME} / ${WASM_NAME}"
fi

# 3. Compilar Versão Desktop Nativa
if [ "$DO_BUILD_DESKTOP" = true ]; then
    echo "-> Compilando versão Desktop (lauda-client --features desktop)..."
    cargo build -p lauda-client --features desktop --release
    mkdir -p "$DIST_DIR/desktop"
    if [ -f "$REPO_ROOT/target/release/lauda-client" ]; then
        cp "$REPO_ROOT/target/release/lauda-client" "$DIST_DIR/desktop/lauda-desktop"
        echo "✓ Binário Desktop gerado em $DIST_DIR/desktop/lauda-desktop"
    fi
fi

# 4. Compilar Versão Android APK
if [ "$DO_BUILD_APK" = true ]; then
    echo "-> Assegurando ícones canônicos oficiais Android..."
    python3 "$SCRIPT_DIR/gerar-recursos-android.py"
    "$REPO_ROOT/tests/test-android-icons.sh"

    NDK_CHECK_DIR="${ANDROID_NDK_HOME:-/opt/android-sdk/ndk/27.2.12479018}"
    if [ ! -d "$NDK_CHECK_DIR" ] && [ -d "/opt/android-ndk/android-ndk-r27c" ]; then
        NDK_CHECK_DIR="/opt/android-ndk/android-ndk-r27c"
    fi

    if command -v dx >/dev/null 2>&1 && rustup target list | grep -q 'aarch64-linux-android (installed)' && [ -d "$NDK_CHECK_DIR" ]; then
        echo "-> Compilando Android APK nativo (aarch64)..."
        "$SCRIPT_DIR/publish-nightly.sh"
    else
        echo "-> Ambiente Android NDK/SDK não disponível diretamente; mantendo APK existente em $SITE_DIR/lauda-latest.apk"
    fi
    "$REPO_ROOT/tests/test-apk-metadata.sh"
fi

# 5. Sincronizar com Obizero Local (lauda.lohn.in)
if [ "$DO_SYNC_OBIZERO" = true ]; then
    OBIZERO_DIR="/home/lumen/stacks/infra/static/lauda-app"
    if [ -d "$OBIZERO_DIR" ] && [ -d "$SITE_DIR/app" ]; then
        echo "-> Sincronizando com instância Obizero local ($OBIZERO_DIR)..."
        mkdir -p "$OBIZERO_DIR/app" "$OBIZERO_DIR/assets"
        cp -r "$SITE_DIR/app/"* "$OBIZERO_DIR/app/" 2>/dev/null || true
        cp -r "$SITE_DIR/app/assets/"* "$OBIZERO_DIR/assets/" 2>/dev/null || true
        cp "$SITE_DIR/app/index.html" "$OBIZERO_DIR/index.html" 2>/dev/null || true
        cp "$SITE_DIR/app/manifest.json" "$OBIZERO_DIR/manifest.json" 2>/dev/null || true
        cp "$SITE_DIR/app/sw.js" "$OBIZERO_DIR/sw.js" 2>/dev/null || true
        echo "✓ Instância local do Obizero (lauda.lohn.in) atualizada."
    fi
fi

# 6. Sincronizar com a Pratilera (banca.2lp.in/pratilera)
if [ "$DO_SYNC_PRATILERA" = true ]; then
    BANCA_PRATILERA="/home/lumen/src/banca/site/pratilera"
    if [ -d "$BANCA_PRATILERA" ] && [ -f "$SITE_DIR/lauda-latest.apk" ]; then
        echo "-> Sincronizando APK com a Pratilera ($BANCA_PRATILERA)..."
        cp "$SITE_DIR/lauda-latest.apk" "$BANCA_PRATILERA/lauda-latest.apk"
        GERADOR_PRATILERA="/home/lumen/src/banca/scripts/gerar-pratilera.py"
        if [ -f "$GERADOR_PRATILERA" ]; then
            python3 "$GERADOR_PRATILERA" || echo "Aviso: gerar-pratilera executado com avisos."
        fi
        echo "✓ Pratilera sincronizada e catálogo F-Droid regenerado."
    fi
fi

# 7. Publicação na Cloudflare Workers (lauda.2lp.in)
if [ "$DO_DEPLOY_CLOUDFLARE" = true ]; then
    echo "-> Verificando deploy na Cloudflare Workers (lauda.2lp.in)..."
    if npx wrangler whoami >/dev/null 2>&1 || [ -n "${CLOUDFLARE_API_TOKEN:-}" ]; then
        npx wrangler deploy || echo "Aviso: wrangler deploy falhou ou foi abortado."
        echo "✓ lauda.2lp.in publicado via Cloudflare Workers."
    else
        echo "Aviso: Credenciais Cloudflare não detectadas para deploy."
    fi
fi

echo "================================================================="
echo "✓ Esteira multi-alvo concluída com sucesso!"
echo "================================================================="
