"""Export approved artwork to platform formats. Requires Pillow (asset authoring only)."""
from pathlib import Path
from PIL import Image, ImageDraw

root = Path(__file__).resolve().parents[1] / "assets" / "icon"
source = Image.open(root / "source.png").convert("RGBA")
assert source.width == source.height and source.getpixel((0, 0))[3] == 0
for size in [16, 24, 32, 48, 64, 128, 192, 256, 512, 1024]:
    source.resize((size, size), Image.Resampling.LANCZOS).save(root / f"app-{size}.png")
source.save(root / "app.ico", sizes=[(s, s) for s in [16, 24, 32, 48, 64, 128, 256]])
source.save(root / "favicon.ico", sizes=[(s, s) for s in [16, 32, 48]])
source.save(root / "app.icns", sizes=[(s, s) for s in [16, 32, 64, 128, 256, 512, 1024]])
# Apple applies its own corner mask; supply an opaque square touch icon.
touch = Image.new("RGB", (180, 180), "#082052")
touch.paste(source.resize((180, 180), Image.Resampling.LANCZOS), (0, 0), source.resize((180, 180), Image.Resampling.LANCZOS))
touch.save(root / "apple-touch-icon.png")
# Actual pixel-size preview on light and dark surfaces, without enlarging the icons.
sheet = Image.new("RGB", (700, 260), "#ffffff")
draw = ImageDraw.Draw(sheet)
draw.rectangle((0, 130, 700, 260), fill="#171d29")
for y, color in [(0, "#222222"), (130, "#ffffff")]:
    x = 18
    for size in [16, 24, 32, 48, 64, 96]:
        icon = source.resize((size, size), Image.Resampling.LANCZOS)
        sheet.paste(icon, (x, y + 8), icon)
        draw.text((x, y + 108), f"{size}px", fill=color)
        x += max(size + 24, 75)
sheet.save(root / "size-preview.png")
print("Exported PNG, ICO, ICNS, touch icon and size preview")
