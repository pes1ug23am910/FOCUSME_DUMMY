# FocusMe Extension Icons

## Source

The icon source is `focusme_icon.svg` — a shield with a focus crosshair/target design in the FocusMe brand gradient (blue to teal).

## Generating PNG Icons

The Chrome Web Store and browser extension manifests require PNG icons at specific sizes. Generate them from the SVG source using one of the methods below.

### Method 1: rsvg-convert (recommended — Linux/macOS)

```bash
# Install: apt install librsvg2-bin (Ubuntu) or brew install librsvg (macOS)

rsvg-convert -w 16 -h 16 focusme_icon.svg -o icon16.png
rsvg-convert -w 48 -h 48 focusme_icon.svg -o icon48.png
rsvg-convert -w 128 -h 128 focusme_icon.svg -o icon128.png
```

### Method 2: Inkscape (cross-platform)

```bash
# Install: https://inkscape.org/

inkscape focusme_icon.svg --export-type=png --export-width=16 --export-filename=icon16.png
inkscape focusme_icon.svg --export-type=png --export-width=48 --export-filename=icon48.png
inkscape focusme_icon.svg --export-type=png --export-width=128 --export-filename=icon128.png
```

### Method 3: ImageMagick (cross-platform)

```bash
# Install: https://imagemagick.org/

magick convert -background none -resize 16x16 focusme_icon.svg icon16.png
magick convert -background none -resize 48x48 focusme_icon.svg icon48.png
magick convert -background none -resize 128x128 focusme_icon.svg icon128.png
```

### Method 4: PowerShell + Inkscape (Windows)

```powershell
$inkscape = "C:\Program Files\Inkscape\bin\inkscape.exe"

& $inkscape focusme_icon.svg --export-type=png --export-width=16 --export-filename=icon16.png
& $inkscape focusme_icon.svg --export-type=png --export-width=48 --export-filename=icon48.png
& $inkscape focusme_icon.svg --export-type=png --export-width=128 --export-filename=icon128.png
```

## Required Sizes

| File | Size | Used By |
|------|------|---------|
| `icon16.png` | 16×16 | Browser toolbar icon |
| `icon48.png` | 48×48 | Extension management page |
| `icon128.png` | 128×128 | Chrome Web Store listing, install dialog |

## Android App Icon

For the Android app icon (512×512), export at higher resolution:

```bash
rsvg-convert -w 512 -h 512 focusme_icon.svg -o icon512.png
```

Then use Android Studio's Asset Studio (Image Asset) to generate adaptive icon variants from the 512×512 PNG.

## Design Notes

- **Shape:** Shield with inner focus crosshair (concentric circles + cross lines)
- **Colors:** Linear gradient from `#2563EB` (blue-600) to `#0891B2` (cyan-600)
- **Border:** `#1E40AF` (blue-800), 2px stroke
- **Crosshair/target:** White (`#FFFFFF`) with slight opacity variation
- **Style:** Flat design, no shadows, no text — scales well at small sizes
