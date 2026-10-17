# Application icon

Approved for issue #27: a cyan/teal curve over three pale-blue axes on a navy
rounded tile, with transparent outer corners. No labels or fine grid details.
`source.png` is the final artwork generated with the built-in image-generation
tool and approved concept. Generation prompt: preserve the approved navy tile,
three-axis frame and cyan/teal wave; remove the exterior square background;
use transparent corners, crisp thick strokes and no text, border or shadow.

Run `python scripts/export-icons.py` with Pillow installed to regenerate the
committed exports. This is an asset-authoring tool; normal builds need neither
Python nor Pillow. Runtime assets are embedded or copied without regeneration.

- PNG: 16, 24, 32, 48, 64, 128, 192, 256, 512 and 1024 pixels.
- Windows ICO: 16, 24, 32, 48, 64, 128 and 256 pixels; embedded by `build.rs`.
- Web favicon ICO: 16, 32 and 48 pixels; PNG fallback 192 pixels.
- Apple touch icon: opaque 180 pixels (the platform applies its own mask).
- macOS ICNS: standard multiresolution artwork for future app-bundle packaging.
- `size-preview.png`: actual-size comparison on light/dark backgrounds.

The native window uses the 256px PNG through eframe. Windows Explorer uses the
embedded ICO; packaging checks for both RT_ICON and RT_GROUP_ICON resources.
Linux window-icon support depends on the window system; Wayland/compositor
desktop integration can require an installed desktop entry. macOS does not
support the winit window-icon API; the current raw-executable package is not a
`.app` bundle, so it includes ICNS artwork without promising Finder/Dock branding.
Native macOS/Linux visual verification requires those graphical platforms.
