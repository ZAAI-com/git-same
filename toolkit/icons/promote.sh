#!/usr/bin/env bash
# Promote a variant from toolkit/icons/generate-icons.swift into the live
# crates/git-same-app/icons/ asset set: regenerates 1024x1024 master,
# downsamples all PNG sizes, rebuilds icon.icns, and refreshes icon.ico.
#
# Usage:  bash toolkit/icons/promote.sh <variant>
#   <variant>: twin | sync | folder-pair | wordmark | tui-banner

set -euo pipefail

VARIANT="${1:-}"
if [ -z "$VARIANT" ]; then
    echo "usage: $0 <variant>" >&2
    echo "  variants: twin sync folder-pair wordmark tui-banner" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ICONS="$ROOT/crates/git-same-app/icons"
GEN="$ROOT/toolkit/icons/generate-icons.swift"

if [ ! -f "$GEN" ]; then
    echo "missing generator at $GEN" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# 1. Render the chosen variant at 1024.
swift "$GEN" --variant "$VARIANT" --out "$TMP" --size 1024 >/dev/null
MASTER="$TMP/$VARIANT.png"
if [ ! -f "$MASTER" ]; then
    echo "generator did not produce $MASTER" >&2
    exit 1
fi

# 2. Replace master + the sizes referenced by tauri.conf.json.
cp "$MASTER" "$ICONS/1024x1024.png"
cp "$MASTER" "$ICONS/icon.png"

# Only the sizes referenced by crates/git-same-app/tauri.conf.json.
# 64x64 is gitignored Tauri-CLI output we don't ship, but we regenerate it
# here so the icons/ directory matches what `pnpm tauri icon` would produce.
for size in 32 64 128; do
    out="$ICONS/${size}x${size}.png"
    cp "$MASTER" "$out"
    sips -Z "$size" "$out" >/dev/null
done
cp "$MASTER" "$ICONS/128x128@2x.png"
sips -Z 256 "$ICONS/128x128@2x.png" >/dev/null

# 3. Assemble macOS .icns from an iconset.
ISET="$TMP/AppIcon.iconset"
mkdir -p "$ISET"
declare -a entries=(
    "16:icon_16x16.png"
    "32:icon_16x16@2x.png"
    "32:icon_32x32.png"
    "64:icon_32x32@2x.png"
    "128:icon_128x128.png"
    "256:icon_128x128@2x.png"
    "256:icon_256x256.png"
    "512:icon_256x256@2x.png"
    "512:icon_512x512.png"
    "1024:icon_512x512@2x.png"
)
for entry in "${entries[@]}"; do
    size="${entry%%:*}"
    name="${entry##*:}"
    cp "$MASTER" "$ISET/$name"
    sips -Z "$size" "$ISET/$name" >/dev/null
done
iconutil -c icns -o "$ICONS/icon.icns" "$ISET"

# 4. Regenerate icon.ico from the 256 PNG via sips.
ICO_SRC="$TMP/ico-source.png"
cp "$MASTER" "$ICO_SRC"
sips -Z 256 "$ICO_SRC" >/dev/null
sips -s format ico "$ICO_SRC" --out "$ICONS/icon.ico" >/dev/null

echo "promoted '$VARIANT' to:"
echo "  $ICONS/1024x1024.png"
echo "  $ICONS/{32,64,128}x{32,64,128}.png"
echo "  $ICONS/128x128@2x.png"
echo "  $ICONS/icon.png"
echo "  $ICONS/icon.icns"
echo "  $ICONS/icon.ico"
