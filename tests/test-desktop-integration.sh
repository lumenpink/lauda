#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste Automatizado de Conformidade XDG Desktop Entry (Ticket #71)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DESKTOP_ENTRY="$REPO_ROOT/assets/lauda.desktop"
INSTALL_SCRIPT="$REPO_ROOT/scripts/install-desktop.sh"

echo "================================================================="
echo "Lauda — Validação Estrita XDG Desktop Entry & Ícones (Ticket #71)"
echo "================================================================="

# 1. Validar existência do arquivo .desktop
if [ ! -f "$DESKTOP_ENTRY" ]; then
    echo "❌ Falha: Arquivo $DESKTOP_ENTRY não encontrado."
    exit 1
fi

# 2. Validar campos essenciais da especificação XDG
grep -q "^\[Desktop Entry\]" "$DESKTOP_ENTRY" || { echo "❌ Falha: Cabeçalho [Desktop Entry] ausente."; exit 1; }
grep -q "^Type=Application" "$DESKTOP_ENTRY" || { echo "❌ Falha: Type=Application ausente."; exit 1; }
grep -q "^Name=Lauda" "$DESKTOP_ENTRY" || { echo "❌ Falha: Name=Lauda ausente."; exit 1; }
grep -q "^Exec=lauda-desktop" "$DESKTOP_ENTRY" || { echo "❌ Falha: Exec=lauda-desktop ausente."; exit 1; }
grep -q "^Icon=lauda" "$DESKTOP_ENTRY" || { echo "❌ Falha: Icon=lauda ausente."; exit 1; }
grep -q "^StartupWMClass=lauda" "$DESKTOP_ENTRY" || { echo "❌ Falha: StartupWMClass=lauda ausente."; exit 1; }
grep -q "Categories=.*Email" "$DESKTOP_ENTRY" || { echo "❌ Falha: Categories deve conter Email."; exit 1; }

echo "✔ Arquivo lauda.desktop validado conforme especificação XDG."

# 3. Validar script de instalação desktop
if [ ! -x "$INSTALL_SCRIPT" ]; then
    echo "❌ Falha: Script $INSTALL_SCRIPT não encontrado ou sem permissão de execução."
    exit 1
fi

# 4. Validar instalação em ambiente isolado (sandbox XDG)
TMP_XDG="$(mktemp -d)"
trap 'rm -rf "$TMP_XDG"' EXIT

export XDG_DATA_HOME="$TMP_XDG/data"
bash "$INSTALL_SCRIPT"

# Verificar atalho instalado
INSTALLED_DESKTOP="$XDG_DATA_HOME/applications/lauda.desktop"
if [ ! -f "$INSTALLED_DESKTOP" ]; then
    echo "❌ Falha: lauda.desktop não foi instalado em $INSTALLED_DESKTOP."
    exit 1
fi

# Verificar ícones hicolor instalados
for size in 48x48 72x72 96x96 144x144 192x192 512x512; do
    ICON_PATH="$XDG_DATA_HOME/icons/hicolor/$size/apps/lauda.png"
    if [ ! -f "$ICON_PATH" ]; then
        echo "❌ Falha: Ícone não instalado em $ICON_PATH."
        exit 1
    fi
done

SVG_PATH="$XDG_DATA_HOME/icons/hicolor/scalable/apps/lauda.svg"
if [ ! -f "$SVG_PATH" ]; then
    echo "❌ Falha: Ícone SVG não instalado em $SVG_PATH."
    exit 1
fi

echo "✔ Instalação na árvore XDG hicolor validada com sucesso!"
echo "✅ Teste de integração desktop concluído com sucesso."
