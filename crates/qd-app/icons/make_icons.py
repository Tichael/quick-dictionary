"""Draw the app icon (a blue rounded square with "Aa") at the sizes Tauri needs.

Usage: python make_icons.py   (needs Pillow and the DejaVu Sans Bold font)
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).parent
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"


def draw(size: int) -> Image.Image:
    scale = 4  # draw large, then downsample for smooth edges
    big = size * scale
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(image)
    margin = big // 16
    d.rounded_rectangle(
        (margin, margin, big - margin, big - margin), radius=big // 5, fill=(37, 99, 235, 255)
    )
    font = ImageFont.truetype(FONT, int(big * 0.46))
    d.text((big / 2, big / 2), "Aa", font=font, fill="white", anchor="mm")
    return image.resize((size, size), Image.LANCZOS)


for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
    draw(size).save(HERE / name)
draw(256).save(HERE / "icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (256, 256)])
