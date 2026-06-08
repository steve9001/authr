# authr — Icon Specification

**Status:** approved direction — Concept #1 "Keyhole Monogram"
**Last updated:** 2026-06-07
**Owner:** design/brand
**Purpose:** Authoritative record of the authr application icons so they can be
regenerated exactly at any future date. If the SVG sources are lost, the two
icons can be rebuilt from the geometry and the full source embedded in
[Appendix A](#appendix-a--canonical-svg-source).

---

## 1. Concept

**authr** is a secure, encrypted vault / password manager desktop app (Tauri,
macOS-first). The brand mark is a **lowercase "a" whose enclosed counter is a
classic keyhole** — tying the product initial to the security metaphor in a
single shape.

The system has **two distinct deliverables** with different constraints:

| Deliverable | Where it lives | Constraints |
|-------------|----------------|-------------|
| **App / Finder icon** | Dock, Finder, Launchpad, About box | Full color, rich detail, macOS "squircle" tile, rendered large (down to 16px but usually ≥32px) |
| **Tray / menu-bar icon** | macOS status bar | Monochrome **template** image (black + alpha), no background, must read at **16–22px** |

> **Intentional divergence:** the Finder glyph uses a **double-story "a"** (the
> two-bowl typographic form) for character at large sizes; the tray glyph uses a
> **single-story "a"** (round bowl + right stem) because it survives small sizes
> far better. Both share the keyhole-as-counter idea, so they read as the same
> brand. See [§6 Design decisions](#6-design-decisions).

---

## 2. Color palette

| Token | Hex | Use |
|-------|-----|-----|
| `indigo-700` | `#4338CA` | Finder background gradient — top-left stop |
| `indigo-600` | `#4F46E5` | Finder background gradient — mid stop (55%) |
| `violet-600` | `#7C3AED` | Finder background gradient — bottom-right stop |
| `glyph-white` | `#F6F6FF` | The "a"/keyhole glyph (very slightly cool white, not pure `#FFFFFF`) |
| `carve-shadow` | `rgba(33, 26, 92, 0.6)` | Inner-shadow color for the carved keyhole (matrix RGB `0.13, 0.10, 0.36`, alpha `0.6`) |
| `sheen-white` | `#FFFFFF` @ 16%→0% | Top-down glassy highlight overlay |
| `tray-black` | `#000000` | Tray template glyph (macOS inverts automatically) |

---

## 3. App / Finder icon

**File:** `01-keyhole-monogram-finder.svg`
**Canvas:** `1024 × 1024`, `viewBox="0 0 1024 1024"`

### 3.1 Background tile — macOS "squircle"
- macOS Big Sur+ superellipse, **corner radius ≈ 22.37% of side ≈ 229px**.
- Corner spans ~229px; sides stay nearly straight between corners.
- Clip path (`squircleClip`):
  ```
  M 229 0 L 795 0 C 940 0 1024 84 1024 229
  L 1024 795 C 1024 940 940 1024 795 1024
  L 229 1024 C 84 1024 0 940 0 795
  L 0 229 C 0 84 84 0 229 0 Z
  ```
- Fill 1: diagonal gradient `bg`, `userSpaceOnUse` from `(0,0)`→`(1024,1024)`:
  `#4338CA` @0 → `#4F46E5` @0.55 → `#7C3AED` @1.
- Fill 2: vertical `sheen` gradient overlay `(0,0)`→`(0,1024)`:
  `#FFFFFF`@16% opacity @0 → `#FFFFFF`@0% @0.45.

### 3.2 Glyph — double-story "a" with keyhole counter
- **Single white path**, `fill="#F6F6FF"`, **`fill-rule="evenodd"`**:
  - Subpath 1 = outer "a" silhouette.
  - Subpath 2 = keyhole counter (round head + tapering slot), punched out by even-odd.
- **Bounding box:** x `312..712` (~400px, ~39% of width), y `286..712`.
- **Horizontally centered** (mid-x = 512); sits in the central band.
- Wrapped in the `carve` filter (see §3.3).
- Exact path data: see [Appendix A](#appendix-a--canonical-svg-source).

### 3.3 "Carve" inner-shadow filter
Makes the keyhole counter read as recessed/carved:
```
feOffset   dx=0 dy=7  on SourceAlpha
feGaussianBlur stdDeviation=7
feComposite operator="out"  (in=blur, in2=SourceAlpha)  -> inner ring
feColorMatrix -> rgba(0.13, 0.10, 0.36, alpha*0.6)
feMerge: SourceGraphic + shadow
```
Filter region: `x=-25% y=-25% width=150% height=150%`.

---

## 4. Tray / menu-bar icon

**File:** `01-keyhole-monogram-tray.svg`
**Canvas:** `32 × 32`, `viewBox="0 0 32 32"`
**Type:** macOS **template image** — pure `#000000`, transparent background,
fill + alpha only (no gradients, no color, no filters). macOS auto-inverts for
light/dark menu bars.

### 4.1 Construction — single-story "a", keyhole as the only counter
The glyph is a `<g fill="#000000">` containing two shapes, with the keyhole
removed via a `<mask>`:

| Element | Geometry | Notes |
|---------|----------|-------|
| Bowl | `<circle cx="14" cy="16.8" r="9"/>` | Round body |
| Stem | `<rect x="18.5" y="5.6" width="5" height="22.4" rx="2.5"/>` | Right stroke; **overlaps deep into the bowl** (stem left `x=18.5` < bowl right edge `x=23`) so they fuse over a tall seam |
| Keyhole head | `<circle cx="12" cy="14" r="3.0"/>` (in mask, black = cut) | |
| Keyhole slot | `<path d="M 10.8 13.8 L 13.2 13.8 L 14.0 20.6 L 10.0 20.6 Z"/>` (in mask) | Flares downward |

The keyhole is the bowl's **single** counter — this is the critical detail that
keeps it legible at 16px (see [§6](#6-design-decisions)).

### 4.2 Stateful variant (optional, recommended)
Concept #2's approach can be borrowed for live lock-state feedback: show a
**locked** glyph when the vault is locked and the keyhole "a" when unlocked.
Not part of the approved baseline; flagged here as a future option.

---

## 5. Required output sizes

> **Destination:** all generated app and tray assets go in
> `authr_app/src-tauri/icons/` (the path Tauri's `bundle.icon` resolves against).

### 5.1 App icon — `.icns` ladder (render from the 1024 SVG)
| Size | @1x | @2x |
|------|-----|-----|
| 16   | `icon_16x16.png` | `icon_16x16@2x.png` (32) |
| 32   | `icon_32x32.png` | `icon_32x32@2x.png` (64) |
| 128  | `icon_128x128.png` | `icon_128x128@2x.png` (256) |
| 256  | `icon_256x256.png` | `icon_256x256@2x.png` (512) |
| 512  | `icon_512x512.png` | `icon_512x512@2x.png` (1024) |

Tauri also wants: `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`
(macOS), `icon.ico` (Windows), and `icon.png` (1024, Linux/source).

### 5.2 Tray icon (render from the 32 SVG)
| Asset | Size | Notes |
|-------|------|-------|
| `tray.png` | 16×16 | @1x menu bar |
| `tray@2x.png` | 32×32 | Retina menu bar |

Render tray PNGs as **template** images (black + alpha) and set
`iconAsTemplate: true` (see §7).

---

## 6. Design decisions

1. **Keyhole = the letter's counter, not a separate shape.** The first tray
   draft drew a double-story "a" *and* punched a separate keyhole inside its
   bowl — two competing round counters (bowl ring + keyhole head) that merged
   into a featureless blob by 16px. Fix: the "a" is a **solid body** and the
   keyhole is its **only** negative space.
2. **Single-story "a" for the tray.** A single-story "a" has exactly one counter,
   so the keyhole-as-counter trick works cleanly. The double-story form (used on
   the Finder icon) has competing interior shapes that don't survive small sizes.
3. **Deep stem overlap.** A circle only *touches* a vertical bar over a short
   band, leaving white "bays" above/below the joint → the stem read as a detached
   block ("keyhole + domino"). Pushing the stem's left edge (`x=18.5`) well inside
   the bowl's right edge (`x=23`) fuses them over a tall seam so it reads as one
   "a".
4. **Cool white glyph (`#F6F6FF`)** rather than pure white — softer against the
   saturated indigo/violet background.
5. **Template tray icon** (not colored) so it obeys macOS light/dark menu bars.

### Rejected / alternative concepts (kept for reference in this folder)
- `02-shield-pulse-*` — Shield + auth pulse; **best stateful tray story** (lock/unlock).
- `03-vault-dial-*` — Brushed-metal combination dial; handsome but least authr-specific.
- `04-folded-key-*` — Gold key/"a" ligature; gorgeous but the bow read as a "9".
- `05-fingerprint-loop-*` — Biometric whorl + keyhole; Finder-only (too busy at 16px).

---

## 7. Tauri integration

Config lives in `authr_app/src-tauri/tauri.conf.json`; the icon paths below are
relative to that file (i.e. `authr_app/src-tauri/icons/`).

In `tauri.conf.json` (Tauri v2 shown; adjust path keys to your version):

```jsonc
{
  "bundle": {
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

Tray icon (Rust / `TrayIconBuilder`) — mark it as a template so macOS inverts it:
```rust
let tray = TrayIconBuilder::new()
    .icon(Image::from_path("icons/tray.png")?)
    .icon_as_template(true)   // macOS light/dark menu bar
    .build(app)?;
```
> `icon_as_template(true)` is macOS-only and required for the menu bar to invert
> the black glyph correctly. On Windows/Linux supply a colored tray icon instead.

---

## 8. Regeneration

### 8.1 Tooling
The repo machine currently has **`qlmanage`** and **`sips`** (macOS built-ins).
For production-quality rasterization prefer one of:
- `rsvg-convert` (librsvg) — `brew install librsvg`
- `cairosvg` — `pip install cairosvg`

`qlmanage` does **not** scale an SVG up to the thumbnail size — it renders the
SVG at its intrinsic `width`/`height` and pads. When previewing the 32px tray
SVG, temporarily rewrite `width="32" height="32"` → a larger value (keep the
`viewBox`) before calling `qlmanage`. (See the preview recipe below.)

### 8.2 App icon → full `.icns` set
With `rsvg-convert` (recommended):
```bash
SVG=01-keyhole-monogram-finder.svg
mkdir -p authr.iconset
for s in 16 32 128 256 512; do
  rsvg-convert -w $s        -h $s        "$SVG" -o "authr.iconset/icon_${s}x${s}.png"
  rsvg-convert -w $((s*2))  -h $((s*2))  "$SVG" -o "authr.iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns authr.iconset -o icon.icns          # macOS .icns
```

Pure-macOS fallback (no extra installs; lower fidelity on filters):
```bash
SVG=01-keyhole-monogram-finder.svg
mkdir -p authr.iconset
for s in 16 32 128 256 512; do
  qlmanage -t -s $s       -o /tmp "$SVG" && mv "/tmp/$(basename $SVG).png" "authr.iconset/icon_${s}x${s}.png"
  qlmanage -t -s $((s*2)) -o /tmp "$SVG" && mv "/tmp/$(basename $SVG).png" "authr.iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns authr.iconset -o icon.icns
```

### 8.3 Tray icon → template PNGs
```bash
SVG=01-keyhole-monogram-tray.svg
rsvg-convert -w 16 -h 16 "$SVG" -o tray.png
rsvg-convert -w 32 -h 32 "$SVG" -o tray@2x.png
```

### 8.4 Preview recipe used during design (qlmanage + sips)
To inspect the 32px tray SVG and faithfully simulate its 16px menu-bar
appearance (high-res render → downsample to 16 → enlarge to view):
```bash
SVG=01-keyhole-monogram-tray.svg
sed -E 's/width="32" height="32"/width="512" height="512"/' "$SVG" > /tmp/big.svg
qlmanage -t -s 360 -o /tmp /tmp/big.svg            # clean full-detail render
sips -z 16 16  /tmp/big.svg.png --out /tmp/d16.png # downsample = true 16px AA
sips -z 240 240 /tmp/d16.png   --out sim16.png     # enlarge to inspect
```

---

## 9. File inventory (`icon-mockups/`)

| File | Role |
|------|------|
| `01-keyhole-monogram-finder.svg` | **Approved** app / Finder icon (1024) |
| `01-keyhole-monogram-tray.svg` | **Approved** menu-bar template icon (32) |
| `02-shield-pulse-*.svg` | Alternative concept (incl. locked/unlocked tray) |
| `03-vault-dial-*.svg` | Alternative concept |
| `04-folded-key-*.svg` | Alternative concept |
| `05-fingerprint-loop-*.svg` | Alternative concept (Finder-only) |
| `preview/` | Rendered PNG previews |
| `ICON-SPEC.md` | This document |

---

## Appendix A — Canonical SVG source

> Embedded verbatim so the two approved icons are reproducible even if the
> `.svg` files are lost. If these ever diverge from the files, **the files are
> authoritative** — update this appendix to match.

### A.1 `01-keyhole-monogram-finder.svg`
```xml
<svg width="1024" height="1024" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1024" y2="1024" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#4338CA"/>
      <stop offset="0.55" stop-color="#4F46E5"/>
      <stop offset="1" stop-color="#7C3AED"/>
    </linearGradient>
    <linearGradient id="sheen" x1="0" y1="0" x2="0" y2="1024" gradientUnits="userSpaceOnUse">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.16"/>
      <stop offset="0.45" stop-color="#ffffff" stop-opacity="0.0"/>
    </linearGradient>
    <filter id="carve" x="-25%" y="-25%" width="150%" height="150%">
      <feOffset dx="0" dy="7" in="SourceAlpha" result="off"/>
      <feGaussianBlur in="off" stdDeviation="7" result="blur"/>
      <feComposite in="blur" in2="SourceAlpha" operator="out" result="inner"/>
      <feColorMatrix in="inner" type="matrix"
        values="0 0 0 0 0.13
                0 0 0 0 0.10
                0 0 0 0 0.36
                0 0 0 0.6 0" result="shadow"/>
      <feMerge>
        <feMergeNode in="SourceGraphic"/>
        <feMergeNode in="shadow"/>
      </feMerge>
    </filter>
    <clipPath id="squircleClip">
      <path d="M 229 0
               L 795 0
               C 940 0 1024 84 1024 229
               L 1024 795
               C 1024 940 940 1024 795 1024
               L 229 1024
               C 84 1024 0 940 0 795
               L 0 229
               C 0 84 84 0 229 0 Z"/>
    </clipPath>
  </defs>
  <g clip-path="url(#squircleClip)">
    <rect x="0" y="0" width="1024" height="1024" fill="url(#bg)"/>
    <rect x="0" y="0" width="1024" height="1024" fill="url(#sheen)"/>
  </g>
  <g filter="url(#carve)">
    <path fill="#F6F6FF" fill-rule="evenodd" d="
      M 512 286
      C 416 286 350 330 322 396
      L 396 428
      C 412 388 448 362 504 362
      C 572 362 610 398 610 466
      L 610 472
      C 520 472 450 484 406 510
      C 358 538 334 580 334 632
      C 334 680 372 712 442 712
      C 508 712 566 684 610 636
      L 610 706
      L 704 706
      L 704 466
      C 704 350 638 286 512 286 Z
      M 547 466
      C 547 436 523 412 493 412
      C 463 412 439 436 439 466
      C 439 487 451 505 469 514
      L 451 628
      C 451 641 470 650 493 650
      C 516 650 535 641 535 628
      L 517 514
      C 535 505 547 487 547 466 Z
    "/>
  </g>
</svg>
```

### A.2 `01-keyhole-monogram-tray.svg`
```xml
<svg width="32" height="32" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
  <mask id="keyhole">
    <rect x="0" y="0" width="32" height="32" fill="#fff"/>
    <circle cx="12" cy="14" r="3.0" fill="#000"/>
    <path d="M 10.8 13.8 L 13.2 13.8 L 14.0 20.6 L 10.0 20.6 Z" fill="#000"/>
  </mask>
  <g fill="#000000" mask="url(#keyhole)">
    <circle cx="14" cy="16.8" r="9"/>
    <rect x="18.5" y="5.6" width="5" height="22.4" rx="2.5"/>
  </g>
</svg>
```
