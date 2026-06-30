#!/usr/bin/env bash
# Build crates/git-same-core/assets/workspace-folder.icns from the
# `folder-icon` variant in toolkit/icons/generate-icons.swift.
#
# This ICNS is embedded into the git-same binary via include_bytes! and painted
# onto every workspace root via NSWorkspace.setIcon (see
# crates/git-same-core/src/macos/folder_icon.rs).
#
# Usage:  bash toolkit/icons/build-workspace-folder-icns.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GEN="$ROOT/toolkit/icons/generate-icons.swift"
OUT_DIR="$ROOT/crates/git-same-core/assets"
OUT="$OUT_DIR/workspace-folder.icns"

if [ ! -f "$GEN" ]; then
    echo "missing generator at $GEN" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# 1. Render the folder-icon master at 1024.
swift "$GEN" --variant folder-icon --out "$TMP" --size 1024 >/dev/null
MASTER="$TMP/folder-icon.png"

# 2. Assemble macOS .icns from an iconset (same size list promote.sh uses).
ISET="$TMP/WorkspaceFolder.iconset"
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

iconutil -c icns -o "$OUT" "$ISET"

echo "wrote $OUT"
ls -lh "$OUT"
