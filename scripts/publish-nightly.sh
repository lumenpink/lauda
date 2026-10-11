#!/usr/bin/env bash
# ==============================================================================
# Lauda — Script de Compilação & Publicação Contínua do Canal Nightly
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"
BUCKET_NAME="lauda-downloads"
REMOTE_KEY="lauda-nightly.apk"
NIGHTLY_APK_PATH="$SITE_DIR/lauda-nightly.apk"
MANIFEST_PATH="$SITE_DIR/version-nightly.json"

export JAVA_HOME="/usr/lib/jvm/java-21-openjdk-amd64"
export ANDROID_HOME="/usr/lib/android-sdk"
export ANDROID_NDK_HOME="/opt/android-ndk/android-ndk-r27c"
export NDK_HOME="/opt/android-ndk/android-ndk-r27c"
export PATH="/usr/lib/jvm/java-21-openjdk-amd64/bin:/opt/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin:/home/lumen/.local/dx-0.7/bin:$PATH"

DX_BIN="/home/lumen/.local/dx-0.7/bin/dx"
if [ ! -x "$DX_BIN" ]; then
    DX_BIN="dx"
fi

LLVM_STRIP="/opt/android-ndk/android-ndk-r27c/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

echo "================================================================="
echo "Lauda — Compilação e Publicação do Canal Nightly"
echo "================================================================="

cd "$REPO_ROOT"

# 1. Metadados da versão Nightly
BASE_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
DATE_STR="$(date +%Y%m%d)"
GIT_HASH="$(git rev-parse --short HEAD)"
NIGHTLY_VERSION="${BASE_VERSION}-nightly.${DATE_STR}.${GIT_HASH}"
VERSION_CODE="$(date +%y%m%d%H)"
PUBLISHED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

echo "-> Versão Base:    $BASE_VERSION"
echo "-> Versão Nightly: $NIGHTLY_VERSION"
echo "-> versionCode:    $VERSION_CODE"
echo "-> Commit HEAD:    $GIT_HASH"

# 2. Compilar alvos nativos Android (x86_64 e aarch64) com dx
echo "-> Compilando alvos Android com dx (x86_64)..."
$DX_BIN build -p lauda-client --android --renderer webview --release --target x86_64-linux-android

echo "-> Compilando alvos Android com dx (aarch64)..."
$DX_BIN build -p lauda-client --android --renderer webview --release --target aarch64-linux-android

# 3. Strip das bibliotecas JNI compartilhadas
echo "-> Removendo símbolos de depuração das bibliotecas JNI..."
$LLVM_STRIP --strip-all "$REPO_ROOT/target/dx/lauda-client/release/android/app/app/src/main/jniLibs"/*/*.so

# 4. Ajustar build.gradle.kts para versionCode, versionName e useLegacyPackaging
GRADLE_BUILD_KTS="$REPO_ROOT/target/dx/lauda-client/release/android/app/app/build.gradle.kts"
echo "-> Configurando packaging e versão em $GRADLE_BUILD_KTS..."

sed -i "s/versionCode = [0-9]\+/versionCode = $VERSION_CODE/g" "$GRADLE_BUILD_KTS"
sed -i "s/versionName = \".*\"/versionName = \"$NIGHTLY_VERSION\"/g" "$GRADLE_BUILD_KTS"
sed -i 's/applicationId = ".*"/applicationId = "in.twolp.lauda"/g' "$GRADLE_BUILD_KTS"
sed -i 's/namespace = ".*"/namespace = "in.twolp.lauda"/g' "$GRADLE_BUILD_KTS"

# Injetar useLegacyPackaging se não estiver presente
if ! grep -q "useLegacyPackaging = true" "$GRADLE_BUILD_KTS"; then
    python3 -c "
path = '$GRADLE_BUILD_KTS'
with open(path, 'r') as f:
    content = f.read()

# Substituir o bloco jniLibs padrão por useLegacyPackaging
old_block = '''            packaging {
                jniLibs.keepDebugSymbols.add(\"*/arm64-v8a/*.so\")
                jniLibs.keepDebugSymbols.add(\"*/armeabi-v7a/*.so\")
                jniLibs.keepDebugSymbols.add(\"*/x86/*.so\")
                jniLibs.keepDebugSymbols.add(\"*/x86_64/*.so\")
            }'''
new_block = '''            packaging {
                jniLibs {
                    useLegacyPackaging = true
                }
            }'''
if old_block in content:
    content = content.replace(old_block, new_block)
    with open(path, 'w') as f:
        f.write(content)
"
fi

# Injetar network_security_config permitindo cleartext HTTP em rede local/Tailscale
NETSEC_XML="$REPO_ROOT/target/dx/lauda-client/release/android/app/app/src/main/res/xml/network_security_config.xml"
echo "-> Habilitando cleartext HTTP para rede local/Tailscale em $NETSEC_XML..."
mkdir -p "$(dirname "$NETSEC_XML")"
cat <<EOF > "$NETSEC_XML"
<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <base-config cleartextTrafficPermitted="true">
        <trust-anchors>
            <certificates src="system" />
        </trust-anchors>
    </base-config>
</network-security-config>
EOF

MANIFEST_XML="$REPO_ROOT/target/dx/lauda-client/release/android/app/app/src/main/AndroidManifest.xml"
if [ -f "$MANIFEST_XML" ] && ! grep -q "usesCleartextTraffic" "$MANIFEST_XML"; then
    sed -i 's/<application /<application android:usesCleartextTraffic="true" /g' "$MANIFEST_XML"
fi

# Injetar MainActivity.kt com suporte a OnBackPressedDispatcher (gestos de voltar)
MAIN_KT="$REPO_ROOT/target/dx/lauda-client/release/android/app/app/src/main/kotlin/dev/dioxus/main/MainActivity.kt"
if [ -f "$REPO_ROOT/lauda-client/android/MainActivity.kt" ]; then
    echo "-> Configurando OnBackPressedDispatcher em $MAIN_KT..."
    cp "$REPO_ROOT/lauda-client/android/MainActivity.kt" "$MAIN_KT"
fi

# 5. Montar APK comprimido via Gradle
echo "-> Montando APK via Gradle (assembleDebug)..."
cd "$REPO_ROOT/target/dx/lauda-client/release/android/app"
./gradlew clean assembleDebug

# 6. Copiar APK gerado para site/lauda-nightly.apk
GEN_APK="$REPO_ROOT/target/dx/lauda-client/release/android/app/app/build/outputs/apk/debug/app-debug.apk"
if [ ! -f "$GEN_APK" ]; then
    echo "ERRO: APK não foi gerado em $GEN_APK."
    exit 1
fi

cp "$GEN_APK" "$NIGHTLY_APK_PATH"
cp "$GEN_APK" "$SITE_DIR/lauda-latest.apk"
APK_SIZE="$(du -h "$NIGHTLY_APK_PATH" | cut -f1)"
echo "✅ APK gerado em $NIGHTLY_APK_PATH ($APK_SIZE)"

# 7. Gerar manifestos site/version-nightly.json e site/version.json
echo "-> Atualizando manifestos de versão..."
cat <<EOF > "$MANIFEST_PATH"
{
  "version": "$NIGHTLY_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-nightly.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
  "published_at": "$PUBLISHED_AT"
}
EOF

cat <<EOF > "$SITE_DIR/version.json"
{
  "version": "$NIGHTLY_VERSION",
  "min_compatible_version": "0.1.0",
  "apk_url": "https://lauda.2lp.in/lauda-latest.apk",
  "package_id": "in.twolp.lauda",
  "release_notes": "Build contínua do Lauda Smolclient (commit $GIT_HASH).",
  "published_at": "$PUBLISHED_AT"
}
EOF

# 7. Compilar WebApp WebAssembly (Dioxus Web) e atualizar site/app
echo "-> Compilando WebApp WebAssembly com dx (target web)..."
(cd "$REPO_ROOT/lauda-client" && $DX_BIN build --platform web --release)
mkdir -p "$SITE_DIR/app"
cp -r "$REPO_ROOT/target/dx/lauda-client/release/web/public/"* "$SITE_DIR/app/"

# Ajustar index.html para caminhos relativos ao /app/ e registrar PWA Service Worker
python3 -c "
with open('$SITE_DIR/app/index.html', 'r') as f:
    html = f.read()
html_fixed = html.replace('/./assets/', '/app/assets/')
html_fixed = html_fixed.replace('dioxus | ⛺', 'LAUDA // Smolclient')
head_insert = '''    <link rel=\"manifest\" href=\"/app/manifest.json\">
    <link rel=\"icon\" type=\"image/svg+xml\" href=\"/lauda-logo.svg\">
    <meta name=\"theme-color\" content=\"#0a0a0a\">
    <style>
      body { margin: 0; padding: 0; background-color: #f4efe6; font-family: monospace; }
    </style>
'''
if '<link rel=\"manifest\"' not in html_fixed:
    html_fixed = html_fixed.replace('</head>', f'{head_insert}</head>')

sw_register = '''
<script>
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('/app/sw.js', { scope: '/app/' }).catch(() => {});
    });
  }
</script>
'''
if 'serviceWorker' not in html_fixed:
    html_fixed = html_fixed.replace('</body>', f'{sw_register}</body>')

with open('$SITE_DIR/app/index.html', 'w') as f:
    f.write(html_fixed)
"
echo "✅ WebApp atualizada em $SITE_DIR/app"

# 8. Upload dos APKs para o Bucket Cloudflare R2
echo "-> Enviando APKs para o bucket R2: $BUCKET_NAME..."
npx wrangler r2 object put "$BUCKET_NAME/lauda-nightly.apk" \
    --file="$NIGHTLY_APK_PATH" \
    --content-type="application/vnd.android.package-archive" \
    --remote

npx wrangler r2 object put "$BUCKET_NAME/lauda-latest.apk" \
    --file="$SITE_DIR/lauda-latest.apk" \
    --content-type="application/vnd.android.package-archive" \
    --remote

echo "✅ Upload dos APKs concluído com sucesso no R2."

# 9. Deploy do Worker e Assets
echo "-> Fazendo deploy do Worker e Static Assets (lauda.2lp.in)..."
cd "$REPO_ROOT"
npx wrangler deploy

echo ""
echo "================================================================="
echo "🎉 Publicação concluída com sucesso em lauda.2lp.in!"
echo "   Package ID:    in.twolp.lauda"
echo "   APK URL:       https://lauda.2lp.in/lauda-latest.apk"
echo "   Manifesto URL: https://lauda.2lp.in/version.json"
echo "================================================================="
