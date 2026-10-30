#!/usr/bin/env bash
# ==============================================================================
# Lauda — Instalador de Integração Desktop XDG (Ticket #71)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$XDG_DATA_HOME/applications"
ICON_BASE="$XDG_DATA_HOME/icons/hicolor"

echo "-> Instalando integração desktop do Lauda em $XDG_DATA_HOME..."

# 1. Instalar arquivo de atalho .desktop
mkdir -p "$APP_DIR"
cp "$REPO_ROOT/assets/lauda.desktop" "$APP_DIR/lauda.desktop"
chmod 644 "$APP_DIR/lauda.desktop"
echo "✔ Atalho desktop instalado em $APP_DIR/lauda.desktop"

# 2. Instalar ícones nas densidades hicolor normativas
declare -A DENSITY_MAP=(
    ["48x48"]="$REPO_ROOT/lauda-client/android/res/mipmap-mdpi/ic_launcher.png"
    ["72x72"]="$REPO_ROOT/lauda-client/android/res/mipmap-hdpi/ic_launcher.png"
    ["96x96"]="$REPO_ROOT/lauda-client/android/res/mipmap-xhdpi/ic_launcher.png"
    ["144x144"]="$REPO_ROOT/lauda-client/android/res/mipmap-xxhdpi/ic_launcher.png"
    ["192x192"]="$REPO_ROOT/lauda-client/android/res/mipmap-xxxhdpi/ic_launcher.png"
    ["512x512"]="$REPO_ROOT/site/lauda-logo.png"
)

for dim in "${!DENSITY_MAP[@]}"; do
    TARGET_DIR="$ICON_BASE/$dim/apps"
    SRC_IMG="${DENSITY_MAP[$dim]}"
    if [ -f "$SRC_IMG" ]; then
        mkdir -p "$TARGET_DIR"
        cp "$SRC_IMG" "$TARGET_DIR/lauda.png"
    fi
done

# Ícone SVG escalável
SCALABLE_DIR="$ICON_BASE/scalable/apps"
if [ -f "$REPO_ROOT/site/lauda-logo.svg" ]; then
    mkdir -p "$SCALABLE_DIR"
    cp "$REPO_ROOT/site/lauda-logo.svg" "$SCALABLE_DIR/lauda.svg"
fi

echo "✔ Ícones hicolor instalados em $ICON_BASE"

# 3. Atualizar bancos de dados de desktop e ícones se ferramentas presentes
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f "$ICON_BASE" 2>/dev/null || true
fi

echo "✅ Integração desktop do Lauda instalada com sucesso!"
