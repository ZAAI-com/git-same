#!/usr/bin/env bash
# Render the git-same manpage into OUT_DIR/git-same.1.
#
# Usage:
#   gen-manpage.sh OUT_DIR [TARGET]
#
# OUT_DIR is created if missing. TARGET is an optional Rust target triple
# forwarded to `cargo run --target` so cross-compiled release builds can reuse
# their already-warm artifact cache.
#
# Run from the repo root. Requires `--features release-tools` deps (clap_mangen);
# cargo will pick those up automatically.

set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "Usage: $0 OUT_DIR [TARGET]" >&2
    exit 2
fi

OUT_DIR="$1"
TARGET_FLAG=()
if [ $# -eq 2 ]; then
    TARGET_FLAG=(--target "$2")
fi

mkdir -p "$OUT_DIR"
OUT_PATH="$OUT_DIR/git-same.1"

echo "==> manpage -> $OUT_PATH"
cargo run \
    --release \
    --features release-tools \
    --bin gen-manpage \
    ${TARGET_FLAG[@]+"${TARGET_FLAG[@]}"} \
    > "$OUT_PATH"

echo "==> Done."
