#!/usr/bin/env bash
# Render shell completions into OUT_DIR with the canonical filenames Homebrew
# expects (`_git-same`, `git-same.bash`, `git-same.fish`).
#
# Usage:
#   gen-completions.sh OUT_DIR [TARGET]
#
# OUT_DIR is created if missing. TARGET is an optional Rust target triple
# forwarded to `cargo run --target` so cross-compiled release builds can reuse
# their already-warm artifact cache.
#
# Run from the repo root. Requires `--features release-tools` deps (clap_complete);
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

# (shell, output filename) pairs Homebrew installs from.
SHELLS=(
    "zsh:_git-same"
    "bash:git-same.bash"
    "fish:git-same.fish"
)

for entry in "${SHELLS[@]}"; do
    SHELL_NAME="${entry%%:*}"
    OUT_NAME="${entry##*:}"
    OUT_PATH="$OUT_DIR/$OUT_NAME"
    echo "==> $SHELL_NAME -> $OUT_PATH"
    cargo run \
        --release \
        --features release-tools \
        --bin gen-completions \
        ${TARGET_FLAG[@]+"${TARGET_FLAG[@]}"} \
        -- "$SHELL_NAME" \
        > "$OUT_PATH"
done

echo "==> Done. Completions in $OUT_DIR"
