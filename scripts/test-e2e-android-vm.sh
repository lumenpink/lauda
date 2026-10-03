#!/usr/bin/env bash
# ==============================================================================
# Lauda Smolclient — Script de Teste E2E em VM / Emulador Android (Pré-Release)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APK_PATH="$REPO_ROOT/site/lauda-latest.apk"
PACKAGE_NAME="dev.dioxus.main"
ACTIVITY_NAME="dev.dioxus.main/.MainActivity"

# Localizar ADB
if command -v adb >/dev/null 2>&1; then
    ADB="adb"
elif [ -f "/usr/lib/android-sdk/platform-tools/adb" ]; then
    ADB="/usr/lib/android-sdk/platform-tools/adb"
else
    echo "ERRO: Ferramenta adb não encontrada no PATH nem em /usr/lib/android-sdk/platform-tools/adb"
    exit 1
fi

echo "================================================================="
echo "Lauda Android E2E — Teste em VM / Emulador / Dispositivo Real"
echo "================================================================="
echo "Usando ADB: $($ADB version | head -n 1)"
echo "Alvo APK:   $APK_PATH"

if [ ! -f "$APK_PATH" ]; then
    echo "ERRO: O APK não foi encontrado em $APK_PATH."
    echo "Compile primeiro com: cd lauda-client && dx build --platform android --release"
    exit 1
fi

# 1. Verificar se há dispositivos ou emuladores ativos
echo ""
echo "-> Verificando dispositivos/emuladores conectados..."
DEVICES=$($ADB devices | grep -v "List of devices" | grep "device$" || true)

if [ -z "$DEVICES" ]; then
    echo "Nenhum emulador ou dispositivo Android conectado via ADB no momento."
    echo ""
    echo "Para rodar em um emulador local headless:"
    echo "  1. Inicie o emulador: emulator -avd <nome_do_avd> -no-window -no-audio -gpu swiftshader_indirect &"
    echo "  2. Aguarde inicialização e rode este script novamente:"
    echo "     bash scripts/test-e2e-android-vm.sh"
    echo ""
    echo "Para rodar com celular físico conectado via USB / Wi-Fi ADB:"
    echo "  1. Conecte o celular com Depuração USB ativada."
    echo "  2. Autorize a depuração no visor do celular e rode este script."
    exit 0
fi

DEVICE_ID=$(echo "$DEVICES" | head -n 1 | awk '{print $1}')
echo "Dispositivo detectado: $DEVICE_ID"

# 2. Aguardar boot completo
echo "-> Aguardando boot completo do sistema Android..."
$ADB -s "$DEVICE_ID" wait-for-device
BOOT_COMPLETED=""
for i in {1..30}; do
    BOOT_COMPLETED=$($ADB -s "$DEVICE_ID" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
    if [ "$BOOT_COMPLETED" = "1" ]; then
        break
    fi
    sleep 2
done

if [ "$BOOT_COMPLETED" != "1" ]; then
    echo "AVISO: O dispositivo ainda não reportou sys.boot_completed=1, tentando prosseguir..."
fi

# 3. Limpar logs anteriores
echo "-> Limpando buffer do logcat..."
$ADB -s "$DEVICE_ID" logcat -c

# 4. Instalar APK
echo "-> Instalando $APK_PATH no dispositivo..."
$ADB -s "$DEVICE_ID" install -r "$APK_PATH"

# 5. Abrir a Activity Principal (medindo tempo de lançamento)
echo "-> Disparando Activity principal ($ACTIVITY_NAME)..."
START_OUT=$($ADB -s "$DEVICE_ID" shell am start -W -n "$ACTIVITY_NAME")
echo "$START_OUT"

if echo "$START_OUT" | grep -iq "Error"; then
    echo "FALHA ao iniciar a Activity!"
    exit 1
fi

# 6. Aguardar renderização inicial do Dioxus / WebView
echo "-> Aguardando 5 segundos para carga completa do WebView..."
sleep 5

# 7. Inspecionar Logcat por Exceções Fatais ou Erros de Linkagem Nativa
echo "-> Analisando logcat em busca de falhas de runtime..."
LOGCAT_ERRORS=$($ADB -s "$DEVICE_ID" logcat -d | grep -E "FATAL EXCEPTION|UnsatisfiedLinkError|dev.dioxus.main.*crash" || true)

if [ -n "$LOGCAT_ERRORS" ]; then
    echo "ERRO CRÍTICO: Falha de runtime detectada no Android Logcat!"
    echo "$LOGCAT_ERRORS"
    exit 1
fi
echo "✅ Nenhum crash, FATAL EXCEPTION ou UnsatisfiedLinkError detectado no Logcat."

# 8. Capturar dump da hierarquia de UI para inspecionar componentes
echo "-> Inspecionando hierarquia gráfica (UI Automator)..."
mkdir -p "$REPO_ROOT/target/e2e-android"
$ADB -s "$DEVICE_ID" shell uiautomator dump /data/local/tmp/uidump.xml >/dev/null 2>&1 || true
$ADB -s "$DEVICE_ID" pull /data/local/tmp/uidump.xml "$REPO_ROOT/target/e2e-android/uidump.xml" >/dev/null 2>&1 || true

if [ -f "$REPO_ROOT/target/e2e-android/uidump.xml" ]; then
    if grep -iq "WebView" "$REPO_ROOT/target/e2e-android/uidump.xml"; then
        echo "✅ Componente WebView ativo e renderizado na tela."
    else
        echo "ℹ️ UI dump capturado em target/e2e-android/uidump.xml."
    fi
fi

# 9. Capturar screenshot de evidência
SCREENSHOT_PATH="$REPO_ROOT/target/e2e-android/screenshot.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot.png "$SCREENSHOT_PATH" >/dev/null 2>&1
echo "✅ Screenshot capturado em: $SCREENSHOT_PATH"

# 10. Fechar aplicação graciosamente
$ADB -s "$DEVICE_ID" shell am force-stop "$PACKAGE_NAME"
echo ""
echo "================================================================="
echo "🎉 TESTE E2E ANDROID CONCLUÍDO COM SUCESSO!"
echo "================================================================="
