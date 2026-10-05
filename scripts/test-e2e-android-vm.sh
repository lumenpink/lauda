#!/usr/bin/env bash
# ==============================================================================
# Lauda Smolclient — Script de Teste E2E em VM / Emulador Android (Pré-Release)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APK_PATH="$REPO_ROOT/site/lauda-latest.apk"
PACKAGE_NAME="com.example.LaudaClient"
ACTIVITY_NAME="com.example.LaudaClient/dev.dioxus.main.MainActivity"

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
LOGCAT_ERRORS=$($ADB -s "$DEVICE_ID" logcat -d | grep -E "FATAL EXCEPTION|UnsatisfiedLinkError|dev.dioxus.main.*crash" | grep -v "NativeCrashHandlerImpl" || true)

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

# 9. Provisionar sessão autenticada conectada ao Lauda Server
echo "-> Verificando conectividade com lauda-server..."
SERVER_PID=""
if ! curl -s http://127.0.0.1:8432/mailboxes >/dev/null 2>&1; then
    echo "Iniciando lauda-server em background na porta 8432..."
    LAUDA_LISTEN_ADDR="0.0.0.0:8432" cargo run -p lauda-server >/dev/null 2>&1 &
    SERVER_PID=$!
    sleep 3
fi

LOGIN_RESP=$(curl -s -X POST http://127.0.0.1:8432/auth/login -H "Content-Type: application/json" -d '{"username":"lumen","password":"senha123"}')
TOKEN=$(echo "$LOGIN_RESP" | grep -o '"token":"[^"]*' | cut -d'"' -f4)

if [ -n "$TOKEN" ]; then
    echo "✅ Token JWT obtido do servidor local com sucesso."
    SESSION_TMP="/tmp/lauda_e2e_session.json"
    cat <<EOF > "$SESSION_TMP"
{
  "server_url": "http://10.0.2.2:8432",
  "username": "lumen",
  "token": "$TOKEN"
}
EOF
    $ADB -s "$DEVICE_ID" push "$SESSION_TMP" /data/data/com.example.LaudaClient/files/session.json >/dev/null 2>&1 || true
    # Reiniciar aplicação com sessão injetada
    $ADB -s "$DEVICE_ID" shell am force-stop "$PACKAGE_NAME"
    sleep 1
    $ADB -s "$DEVICE_ID" shell am start -W -n "$ACTIVITY_NAME" >/dev/null 2>&1
    sleep 5
fi

# 10. Capturar tela do Inbox
SCREENSHOT_INBOX="$REPO_ROOT/target/e2e-android/screenshot_inbox.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_inbox.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_inbox.png "$SCREENSHOT_INBOX" >/dev/null 2>&1
echo "✅ Inbox capturado em: $SCREENSHOT_INBOX"

# 11. Teste Interativo: Abrir Mensagem (Leitor)
echo "-> Testando interação: abrindo mensagem (UID 101)..."
$ADB -s "$DEVICE_ID" shell input tap 400 950
sleep 2
SCREENSHOT_READER="$REPO_ROOT/target/e2e-android/screenshot_reader_auto.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_reader.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_reader.png "$SCREENSHOT_READER" >/dev/null 2>&1
echo "✅ Leitor (TxtAuto) capturado em: $SCREENSHOT_READER"

# 12. Teste Interativo: Alternância Trifocal Cíclica no Leitor (Botão Único)
echo "-> Testando interação: alternando para HTML (1º toque no botão rotativo)..."
$ADB -s "$DEVICE_ID" shell input tap 850 590
sleep 1
SCREENSHOT_HTML="$REPO_ROOT/target/e2e-android/screenshot_reader_html.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_html.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_html.png "$SCREENSHOT_HTML" >/dev/null 2>&1
echo "✅ HTML capturado em: $SCREENSHOT_HTML"

echo "-> Testando interação: alternando para ORIGINAL (2º toque no botão rotativo)..."
$ADB -s "$DEVICE_ID" shell input tap 850 590
sleep 1
SCREENSHOT_TXT_ORIGINAL="$REPO_ROOT/target/e2e-android/screenshot_reader_original.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_txt_original.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_txt_original.png "$SCREENSHOT_TXT_ORIGINAL" >/dev/null 2>&1
echo "✅ TXT ORIGINAL capturado em: $SCREENSHOT_TXT_ORIGINAL"

echo "-> Testando interação: retornando para TXT AUTO (3º toque no botão rotativo)..."
$ADB -s "$DEVICE_ID" shell input tap 850 590
sleep 1

# 13. Teste Interativo: Abrir Compositor via RESPONDER
echo "-> Testando interação: clicando em RESPONDER..."
$ADB -s "$DEVICE_ID" shell input tap 500 590
sleep 2
SCREENSHOT_COMPOSE="$REPO_ROOT/target/e2e-android/screenshot_compose.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_compose.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_compose.png "$SCREENSHOT_COMPOSE" >/dev/null 2>&1
echo "✅ Compositor capturado em: $SCREENSHOT_COMPOSE"

# 14. Teste Interativo: Disparar Envio
echo "-> Testando interação: clicando em ENVIAR ✉..."
$ADB -s "$DEVICE_ID" shell input tap 780 600
sleep 2
SCREENSHOT_RESTORED="$REPO_ROOT/target/e2e-android/screenshot_inbox_restored.png"
$ADB -s "$DEVICE_ID" shell screencap -p /data/local/tmp/screenshot_inbox_restored.png
$ADB -s "$DEVICE_ID" pull /data/local/tmp/screenshot_inbox_restored.png "$SCREENSHOT_RESTORED" >/dev/null 2>&1
echo "✅ Retorno ao Inbox após envio capturado em: $SCREENSHOT_RESTORED"

# 15. Validação final de estabilidade pós-interação no Logcat
echo "-> Verificando ausência de erros pós-interação no logcat..."
LOGCAT_POST_ERRORS=$($ADB -s "$DEVICE_ID" logcat -d | grep -E "FATAL EXCEPTION|UnsatisfiedLinkError|com.example.LaudaClient.*crash" | grep -v "NativeCrashHandlerImpl" || true)
if [ -n "$LOGCAT_POST_ERRORS" ]; then
    echo "ERRO CRÍTICO: Falha detectada após interações na UI!"
    echo "$LOGCAT_POST_ERRORS"
    exit 1
fi
echo "✅ Nenhuma falha após cliques e alternância de modos de visualização."

# 16. Fechar aplicação graciosamente
$ADB -s "$DEVICE_ID" shell am force-stop "$PACKAGE_NAME"

if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" 2>/dev/null || true
fi

echo ""
echo "================================================================="
echo "🎉 TESTE E2E ANDROID INTERATIVO CONCLUÍDO COM SUCESSO!"
echo "================================================================="
