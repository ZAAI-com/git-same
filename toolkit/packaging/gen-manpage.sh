#!/usr/bin/env bash
# Render the git-same manpage into OUT_DIR/git-same.1.
#
# Usage:
#   gen-manpage.sh OUT_DIR
#
# OUT_DIR is created if missing.
#
# Run from the repo root. Requires `--features release-tools` deps (clap_mangen);
# cargo will pick those up automatically.

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 OUT_DIR" >&2
    exit 2
fi

OUT_DIR="$1"

mkdir -p "$OUT_DIR"
OUT_PATH="$OUT_DIR/git-same.1"

echo "==> manpage -> $OUT_PATH"
cargo run \
    --release \
    -p git-same-cli \
    --features release-tools \
    --bin gen-manpage \
    > "$OUT_PATH"

echo "==> Done."
