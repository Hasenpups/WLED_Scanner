"""Generates the app icon (icon.png, icon.ico, icon.icns) for WLED Scanner.

Run: python3 assets/make_icon.py   (requires Pillow)
"""
import colorsys
import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

S = 1024  # drawing resolution, downsampled for smooth edges
OUT = Path(__file__).resolve().parent


def rounded_background():
    # vertical navy gradient clipped to a rounded square
    grad = Image.new("RGBA", (S, S))
    top, bottom = (28, 36, 64), (10, 12, 26)
    px = grad.load()
    for y in range(S):
        t = y / (S - 1)
        c = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,)
        for x in range(S):
            px[x, y] = c
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle((24, 24, S - 24, S - 24), radius=210, fill=255)
    bg = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    bg.paste(grad, mask=mask)
    return bg, mask


def main():
    img, mask = rounded_background()
    cx = cy = S / 2
    ring_r = 330

    # radar sweep: translucent wedge fading from the leading edge
    sweep = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(sweep)
    lead = -50  # degrees, PIL measures clockwise from 3 o'clock
    for i in range(70):
        alpha = round(150 * (1 - i / 70) ** 1.6)
        d.pieslice((cx - ring_r + 40, cy - ring_r + 40, cx + ring_r - 40, cy + ring_r - 40),
                   lead - i - 1, lead - i, fill=(90, 220, 255, alpha))
    img.alpha_composite(sweep)

    # faint range rings
    d = ImageDraw.Draw(img)
    for r in (110, 200):
        d.ellipse((cx - r, cy - r, cx + r, cy + r), outline=(90, 220, 255, 70), width=10)
    # sweep leading line
    a = math.radians(lead)
    d.line((cx, cy, cx + (ring_r - 40) * math.cos(a), cy + (ring_r - 40) * math.sin(a)),
           fill=(150, 235, 255, 255), width=18)

    # rainbow LED ring with glow
    leds = 12
    glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    dots = []
    for i in range(leds):
        ang = math.radians(-90 + i * 360 / leds)
        x, y = cx + ring_r * math.cos(ang), cy + ring_r * math.sin(ang)
        r, g, b = (round(v * 255) for v in colorsys.hsv_to_rgb(i / leds, 0.85, 1.0))
        dots.append((x, y, (r, g, b)))
        gd.ellipse((x - 95, y - 95, x + 95, y + 95), fill=(r, g, b, 150))
    glow = glow.filter(ImageFilter.GaussianBlur(40))
    img.alpha_composite(glow)
    d = ImageDraw.Draw(img)
    for x, y, c in dots:
        d.ellipse((x - 62, y - 62, x + 62, y + 62), fill=c + (255,))
        d.ellipse((x - 26, y - 34, x + 14, y + 2), fill=(255, 255, 255, 140))  # highlight

    # center node
    d.ellipse((cx - 46, cy - 46, cx + 46, cy + 46), fill=(150, 235, 255, 255))

    # keep everything inside the rounded square
    clipped = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    clipped.paste(img, mask=mask)

    clipped.resize((256, 256), Image.LANCZOS).save(OUT / "icon.png")
    clipped.resize((512, 512), Image.LANCZOS).save(OUT / "icon-512.png")
    clipped.save(OUT / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    clipped.save(OUT / "icon.icns")  # macOS .app bundle


if __name__ == "__main__":
    main()
