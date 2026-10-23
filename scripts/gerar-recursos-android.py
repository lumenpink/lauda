#!/usr/bin/env python3
"""
Lauda — Gerador Oficial de Ícones e Recursos Visuais Android (Ticket #63)
Estética Trash Polka Postal (Proposta C2: Cancelamento Ultra Pesado com SMOL no Branco)
Gera mestre 512x512 8-bit RGBA, SVG canônico e densidades Android normais e circulares.
"""
import os
import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO_ROOT = Path(__file__).resolve().parent.parent
SITE_DIR = REPO_ROOT / "site"
RES_DIR = REPO_ROOT / "lauda-client" / "android" / "res"

DENSITIES = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}

def render_lauda_icon(size=512, is_round=False):
    scale = 2
    dim = size * scale  # 1024 para 512
    
    im = Image.new("RGBA", (dim, dim), (0, 0, 0, 0))
    
    def s(v):
        return int(round(v * scale * (size / 512.0)))
    
    # Máscara de corte do chassi
    mask = Image.new("L", (dim, dim), 0)
    mask_draw = ImageDraw.Draw(mask)
    if is_round:
        mask_draw.ellipse([s(16), s(16), dim - s(16), dim - s(16)], fill=255)
    else:
        mask_draw.rounded_rectangle([s(8), s(8), dim - s(8), dim - s(8)], radius=s(100), fill=255)
        
    content = Image.new("RGBA", (dim, dim), (0, 0, 0, 0))
    cdraw = ImageDraw.Draw(content)
    
    # 1. Fundo papel envelhecido off-white (#f4efe6)
    paper_color = (244, 239, 230, 255)
    cdraw.rectangle([0, 0, dim, dim], fill=paper_color)
    
    # 2. Retícula halftone pontilhada
    step = s(16)
    dot_r = s(1.8)
    for x in range(0, dim + step, step):
        for y in range(0, dim + step, step):
            cdraw.ellipse([x + s(4) - dot_r, y + s(4) - dot_r, x + s(4) + dot_r, y + s(4) + dot_r], fill=(10, 10, 10, 38))
            cdraw.ellipse([x + s(12) - dot_r, y + s(12) - dot_r, x + s(12) + dot_r, y + s(12) + dot_r], fill=(10, 10, 10, 38))
            
    # 3. Faixas Diagonais Trash Polka
    red_crimson = (220, 20, 60, 255)  # #dc143c
    p1 = [(s(290), 0), (s(420), 0), (s(190), dim), (s(60), dim)]
    cdraw.polygon(p1, fill=red_crimson)
    
    red_dark = (139, 0, 0, 217)  # #8b0000 85%
    p2 = [(s(435), 0), (s(470), 0), (s(240), dim), (s(205), dim)]
    cdraw.polygon(p2, fill=red_dark)
    
    # 4. Envelope Brutalista Central
    env_w = s(320)
    env_h = s(220)
    env_x = s(96)
    env_y = s(155)  # Perfeitamente centralizado verticalmente sem a tarja
    
    # Sombra dura vetorial deslocada
    sh_offset = s(14)
    cdraw.rounded_rectangle(
        [env_x + sh_offset, env_y + sh_offset, env_x + env_w + sh_offset, env_y + env_h + sh_offset],
        radius=s(4), fill=(10, 10, 10, 255)
    )
    
    # Corpo base branco com contorno preto pesado
    cdraw.rounded_rectangle(
        [env_x, env_y, env_x + env_w, env_y + env_h],
        radius=s(4), fill=(255, 255, 255, 255), outline=(10, 10, 10, 255), width=s(10)
    )
    
    # Linhas de vinco da aba (V-flap)
    mid_x = env_x + env_w // 2
    flap_tip_y = env_y + s(135)
    cdraw.line([(env_x, env_y), (mid_x, flap_tip_y), (env_x + env_w, env_y)], fill=(10, 10, 10, 255), width=s(10))
    
    # Linhas diagonais inferiores
    cdraw.line([(env_x, env_y + env_h), (env_x + s(120), env_y + s(110))], fill=(10, 10, 10, 255), width=s(8))
    cdraw.line([(env_x + env_w, env_y + env_h), (env_x + env_w - s(120), env_y + s(110))], fill=(10, 10, 10, 255), width=s(8))
    
    # Lacre de cera: triângulo carmim superior + quadrado preto maciço SEM letras
    seal_w = s(52)
    seal_h = s(56)
    seal_x = mid_x - seal_w // 2
    seal_y = flap_tip_y - s(35)
    
    cdraw.polygon([(mid_x, seal_y - s(24)), (seal_x + seal_w, seal_y), (seal_x, seal_y)], fill=red_crimson, outline=(10, 10, 10, 255), width=s(3))
    cdraw.rectangle([seal_x, seal_y, seal_x + seal_w, seal_y + seal_h], fill=(10, 10, 10, 255))
    
    # 5. Carimbo Postal Cancelador Reforçado (Proposta C2)
    # Sobreposto em camada DIRETA por cima do envelope
    stamp_cx = s(175)
    stamp_cy = s(205)
    stamp_r = s(60)
    
    stamp_img = Image.new("RGBA", (stamp_r * 4, stamp_r * 4), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(stamp_img)
    scenter = stamp_r * 2
    
    # Anel externo ultra reforçado (10.5px)
    ring_w = s(10)
    sdraw.ellipse([scenter - stamp_r, scenter - stamp_r, scenter + stamp_r, scenter + stamp_r],
                  outline=red_crimson, width=ring_w)
                  
    # Barras de cancelamento horizontal (5px)
    bar_y1 = scenter - s(18)
    bar_y2 = scenter + s(18)
    bar_len = s(42)
    sdraw.line([(scenter - bar_len, bar_y1), (scenter + bar_len, bar_y1)], fill=red_crimson, width=s(5))
    sdraw.line([(scenter - bar_len, bar_y2), (scenter + bar_len, bar_y2)], fill=red_crimson, width=s(5))
    
    # Tipografia SMOL 100% sobre o fundo branco do envelope
    try:
        font_smol = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", s(21))
    except:
        font_smol = ImageFont.load_default()
        
    spaced_text = " S M O L "
    tbbox = font_smol.getbbox(spaced_text)
    tbw = tbbox[2] - tbbox[0]
    tbh = tbbox[3] - tbbox[1]
    sdraw.text((scenter - tbw // 2 - tbbox[0], scenter - tbh // 2 - tbbox[1]), spaced_text, font=font_smol, fill=red_crimson)
    
    # Rotação suave (-6°) simulando aplicação analógica manual
    rot_stamp = stamp_img.rotate(-6, resample=Image.Resampling.BICUBIC)
    content.alpha_composite(rot_stamp, (stamp_cx - scenter, stamp_cy - scenter))

    # Aplicação do chassi (corte com transparência nos cantos)
    clipped = Image.new("RGBA", (dim, dim), (0, 0, 0, 0))
    clipped.paste(content, (0, 0), mask=mask)
    
    # Moldura contínua preta
    draw_frame = ImageDraw.Draw(clipped)
    frame_w = s(12)
    if is_round:
        draw_frame.ellipse([s(16), s(16), dim - s(16), dim - s(16)], outline=(10, 10, 10, 255), width=frame_w)
    else:
        draw_frame.rounded_rectangle([s(8), s(8), dim - s(8), dim - s(8)], radius=s(100), outline=(10, 10, 10, 255), width=frame_w)
        
    # Redução com filtro Lanczos para fidelidade máxima de pixels
    return clipped.resize((size, size), Image.Resampling.LANCZOS)

def generate_canonical_svg():
    svg_content = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Textura Halftone Pontilhada -->
    <pattern id="halftone" x="0" y="0" width="16" height="16" patternUnits="userSpaceOnUse">
      <circle cx="4" cy="4" r="1.8" fill="#0a0a0a" opacity="0.15" />
      <circle cx="12" cy="12" r="1.8" fill="#0a0a0a" opacity="0.15" />
    </pattern>

    <clipPath id="rounded-icon">
      <rect width="512" height="512" rx="108" />
    </clipPath>
  </defs>

  <g clip-path="url(#rounded-icon)">
    <!-- Fundo do Ícone: Papel Envelhecido / Off-White -->
    <rect width="512" height="512" fill="#f4efe6" />
    <rect width="512" height="512" fill="url(#halftone)" />

    <!-- Faixas Diagonais Vermelhas Trash Polka -->
    <polygon points="290,0 420,0 190,512 60,512" fill="#dc143c" />
    <polygon points="435,0 470,0 240,512 205,512" fill="#8b0000" opacity="0.85" />

    <!-- Envelope Brutalista Central -->
    <g>
      <!-- Sombra Dura Vetorial Fiel -->
      <rect x="110" y="169" width="320" height="220" rx="4" fill="#0a0a0a" />

      <!-- Base do Envelope -->
      <rect x="96" y="155" width="320" height="220" rx="4" fill="#ffffff" stroke="#0a0a0a" stroke-width="10" stroke-linejoin="miter" />

      <!-- Linhas do Envelope em Linha Grossa -->
      <path d="M96 155 L256 290 L416 155" fill="none" stroke="#0a0a0a" stroke-width="10" stroke-linecap="square" stroke-linejoin="miter" />
      <path d="M96 375 L216 265" fill="none" stroke="#0a0a0a" stroke-width="8" stroke-linecap="square" />
      <path d="M416 375 L296 265" fill="none" stroke="#0a0a0a" stroke-width="8" stroke-linecap="square" />

      <!-- Lacre / Selo Central Maciço Preto (Sem Glifos) -->
      <polygon points="256,175 282,199 230,199" fill="#dc143c" stroke="#0a0a0a" stroke-width="3" />
      <rect x="230" y="199" width="52" height="56" fill="#0a0a0a" />
    </g>

    <!-- Carimbo Postal Cancelador Reforçado C2 (Sobreposto ao Envelope) -->
    <g transform="translate(175, 205) rotate(-6)">
      <!-- Anel Externo Pesado -->
      <circle cx="0" cy="0" r="60" fill="none" stroke="#dc143c" stroke-width="10.5" />
      <!-- Barras de Cancelamento -->
      <line x1="-42" y1="-18" x2="42" y2="-18" stroke="#dc143c" stroke-width="5" stroke-linecap="round" />
      <line x1="-42" y1="18" x2="42" y2="18" stroke="#dc143c" stroke-width="5" stroke-linecap="round" />
      <!-- Texto SMOL 100% sobre o fundo branco -->
      <text x="0" y="7" font-family="monospace" font-weight="900" font-size="21" fill="#dc143c" text-anchor="middle" letter-spacing="4">SMOL</text>
    </g>
  </g>

  <!-- Moldura Externa Contínua -->
  <rect x="8" y="8" width="496" height="496" rx="100" fill="none" stroke="#0a0a0a" stroke-width="12" />
</svg>
"""
    with open(SITE_DIR / "lauda-logo.svg", "w", encoding="utf-8") as f:
        f.write(svg_content)
    print("✓ SVG Canônico gerado: site/lauda-logo.svg")

def main():
    print("Gerando ícones oficiais do Lauda (Proposta C2: Cancelamento Ultra Pesado)...")
    
    # 1. Mestre 512x512 8-bit RGBA para vitrine e F-Droid
    master_png = render_lauda_icon(size=512, is_round=False)
    SITE_DIR.mkdir(parents=True, exist_ok=True)
    master_png.save(SITE_DIR / "lauda-logo.png", "PNG", optimize=True)
    print("✓ Mestre gerado: site/lauda-logo.png (512x512 8-bit RGBA)")
    
    # 2. SVG Canônico
    generate_canonical_svg()
    
    # 3. Densidades Android do Launcher
    for folder, size in DENSITIES.items():
        folder_dir = RES_DIR / folder
        folder_dir.mkdir(parents=True, exist_ok=True)
        
        # Padrão (squircle)
        ico_std = render_lauda_icon(size=size, is_round=False)
        ico_std.save(folder_dir / "ic_launcher.png", "PNG", optimize=True)
        
        # Circular (Pixel/Round Launcher)
        ico_round = render_lauda_icon(size=size, is_round=True)
        ico_round.save(folder_dir / "ic_launcher_round.png", "PNG", optimize=True)
        print(f"✓ Densidade Android gerada: {folder} ({size}x{size})")
        
    # 4. Ícone Adaptativo Foreground
    drawable_dir = RES_DIR / "drawable"
    drawable_dir.mkdir(parents=True, exist_ok=True)
    fg = render_lauda_icon(size=432, is_round=False)
    fg.save(drawable_dir / "ic_launcher_foreground.png", "PNG", optimize=True)
    print("✓ Adaptive foreground gerado: drawable/ic_launcher_foreground.png (432x432)")
    
    # 5. Strings de Recursos (Nome do App no Android)
    values_dir = RES_DIR / "values"
    values_dir.mkdir(parents=True, exist_ok=True)
    strings_xml = values_dir / "strings.xml"
    strings_content = '<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <string name="app_name">Lauda</string>\n</resources>\n'
    with open(strings_xml, "w", encoding="utf-8") as f:
        f.write(strings_content)
    print("✓ Recursos de texto gerados: values/strings.xml (app_name = Lauda)")
    
    print("🎉 Todos os recursos visuais do Lauda foram gerados com sucesso!")

if __name__ == "__main__":
    main()
