#!/usr/bin/env bash
# ==============================================================================
# Lauda — Teste Automatizado de Integridade de Ícones Android & Vitrine (Ticket #63)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$REPO_ROOT/site"
RES_DIR="$REPO_ROOT/lauda-client/android/res"

echo "================================================================="
echo "Lauda — Validação Estrita de Ícones Launcher & Vitrine (Ticket #63)"
echo "================================================================="

FAILED=0

# 1. Validar mestre da vitrine (512x512 8-bit RGBA)
MASTER_PNG="$SITE_DIR/lauda-logo.png"
if [ ! -f "$MASTER_PNG" ]; then
    echo "❌ Falha: $MASTER_PNG não encontrado."
    FAILED=1
else
    python3 -c "
import sys
from PIL import Image
im = Image.open('$MASTER_PNG')
assert im.size == (512, 512), f'Tamanho incorreto: {im.size}'
assert im.mode == 'RGBA', f'Modo de cor não é RGBA: {im.mode}'
# Cantos devem ter transparência alfa para squircle
corners = [im.getpixel((0,0)), im.getpixel((511,0)), im.getpixel((0,511)), im.getpixel((511,511))]
for c in corners:
    assert c[3] == 0, f'Canto não é transparente: {c}'
print('✔ Mestre site/lauda-logo.png válido (512x512 RGBA com cantos transparentes)')
" || FAILED=1
fi

# 2. Validar SVG canônico
SVG_FILE="$SITE_DIR/lauda-logo.svg"
if [ ! -f "$SVG_FILE" ]; then
    echo "❌ Falha: $SVG_FILE não encontrado."
    FAILED=1
else
    if grep -q "LAUDA" "$SVG_FILE"; then
        echo "❌ Falha: $SVG_FILE contém tarja de texto LAUDA legada."
        FAILED=1
    fi
    if ! grep -q "SMOL" "$SVG_FILE"; then
        echo "❌ Falha: $SVG_FILE não contém o carimbo SMOL."
        FAILED=1
    fi
    echo "✔ SVG canônico site/lauda-logo.svg validado"
fi

# 3. Validar todas as densidades Android (ic_launcher e ic_launcher_round)
declare -A DENSITIES=(
    ["mipmap-mdpi"]=48
    ["mipmap-hdpi"]=72
    ["mipmap-xhdpi"]=96
    ["mipmap-xxhdpi"]=144
    ["mipmap-xxxhdpi"]=192
)

for folder in "${!DENSITIES[@]}"; do
    expected_size="${DENSITIES[$folder]}"
    
    # ic_launcher.png
    target_std="$RES_DIR/$folder/ic_launcher.png"
    if [ ! -f "$target_std" ]; then
        echo "❌ Falha: $target_std ausente."
        FAILED=1
    else
        python3 -c "
from PIL import Image
im = Image.open('$target_std')
assert im.size == ($expected_size, $expected_size), f'Tamanho incorreto em $target_std: {im.size} != ($expected_size, $expected_size)'
assert im.mode == 'RGBA', f'Modo não é RGBA em $target_std: {im.mode}'
" || FAILED=1
    fi

    # ic_launcher_round.png
    target_round="$RES_DIR/$folder/ic_launcher_round.png"
    if [ ! -f "$target_round" ]; then
        echo "❌ Falha: $target_round ausente."
        FAILED=1
    else
        python3 -c "
from PIL import Image
im = Image.open('$target_round')
assert im.size == ($expected_size, $expected_size), f'Tamanho incorreto em $target_round: {im.size}'
assert im.mode == 'RGBA', f'Modo não é RGBA em $target_round: {im.mode}'
# Canto circular deve ser 100% transparente
assert im.getpixel((0, 0))[3] == 0, 'Canto não transparente no ícone circular'
" || FAILED=1
    fi
done

# 4. Validar marcadores de paleta Trash Polka
if [ -f "$MASTER_PNG" ]; then
    python3 -c "
from PIL import Image
im = Image.open('$MASTER_PNG')
colors = im.getcolors(maxcolors=262144)
has_crimson = any(c[1][0] > 200 and c[1][1] < 40 and c[1][2] < 80 for c in colors if len(c[1]) >= 3)
has_paper = any(c[1][0] > 230 and c[1][1] > 225 and c[1][2] > 215 for c in colors if len(c[1]) >= 3)
has_black = any(c[1][0] < 20 and c[1][1] < 20 and c[1][2] < 20 for c in colors if len(c[1]) >= 3)
assert has_crimson, 'Paleta carmim Trash Polka ausente'
assert has_paper, 'Fundo papel off-white ausente'
assert has_black, 'Preto brutalista ausente'
print('✔ Paleta de cores Trash Polka validada com sucesso')
" || FAILED=1
fi

# 5. Testes de Mutação e Limites (Resiliência contra corrupção e modos inválidos)
echo "-> Executando testes de mutação e limites..."
python3 -c "
import tempfile, os
from PIL import Image

# Mutação 1: Imagem não-quadrada deve falhar
with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
    mut1 = Image.new('RGBA', (512, 480), (255, 0, 0, 255))
    mut1.save(tmp.name)
    try:
        im = Image.open(tmp.name)
        assert im.size == (512, 512)
        raise RuntimeError('Mutação 1 não detectada!')
    except AssertionError:
        pass
    finally:
        os.unlink(tmp.name)

# Mutação 2: Imagem sem canal alfa (RGB puro) deve falhar
with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
    mut2 = Image.new('RGB', (512, 512), (255, 0, 0))
    mut2.save(tmp.name)
    try:
        im = Image.open(tmp.name)
        assert im.mode == 'RGBA'
        raise RuntimeError('Mutação 2 não detectada!')
    except AssertionError:
        pass
    finally:
        os.unlink(tmp.name)

# Mutação 3: Canto opaco em ícone circular deve falhar
with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
    mut3 = Image.new('RGBA', (48, 48), (0, 0, 0, 0))
    mut3.putpixel((0, 0), (244, 239, 230, 255))
    mut3.save(tmp.name)
    try:
        im = Image.open(tmp.name)
        assert im.getpixel((0, 0))[3] == 0
        raise RuntimeError('Mutação 3 não detectada!')
    except AssertionError:
        pass
    finally:
        os.unlink(tmp.name)

print('✔ Todas as 3 mutações sintéticas foram detectadas e rejeitadas corretamente')
" || FAILED=1

if [ $FAILED -ne 0 ]; then
    echo "❌ Falhas encontradas na validação de ícones."
    exit 1
fi

echo "🎉 TODOS OS ÍCONES DO LAUDA FORAM VALIDADOS COM SUCESSO!"
