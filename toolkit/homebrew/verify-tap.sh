#!/usr/bin/env bash
# Stage rendered cask + formula files into a throwaway local tap, then run
# `brew style --strict` and `brew audit --strict --online` against each. Exits
# non-zero on any audit/style failure so it can gate the real tap push in S3.
#
# Usage:
#   verify-tap.sh --cask cask.rb --formula-cli formula-cli.rb \
#                 --formula-shim formula-shim.rb [--install-smoke]
#
# Optional --install-smoke does `brew install --cask` against the temp tap to
# confirm the artifacts download + install end-to-end. Skipped by default
# because notarization checks need network egress and ~minutes per arch.
#
# Requires `brew` on PATH. Designed to run on macos-latest in CI and locally
# on a developer Mac.

set -euo pipefail

CASK=""
FORMULA_CLI=""
FORMULA_SHIM=""
INSTALL_SMOKE=0
ONLINE=1

usage() {
    cat <<EOF >&2
Usage: $0 --cask cask.rb --formula-cli formula-cli.rb --formula-shim formula-shim.rb [--install-smoke] [--offline]

  --offline       Skip the URL/livecheck audit. Useful for local dry runs before
                  the GitHub Release is uploaded; CI should leave this off.
  --install-smoke Run \`brew install --cask\` end-to-end. Pulls real tarballs.
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --cask)          CASK="${2:-}"; shift 2 ;;
        --formula-cli)   FORMULA_CLI="${2:-}"; shift 2 ;;
        --formula-shim)  FORMULA_SHIM="${2:-}"; shift 2 ;;
        --install-smoke) INSTALL_SMOKE=1; shift ;;
        --offline)       ONLINE=0; shift ;;
        -h|--help)       usage; exit 0 ;;
        *)               echo "ERROR: unknown arg $1" >&2; usage; exit 2 ;;
    esac
done

if [ -z "$CASK" ];         then echo "ERROR: --cask is required" >&2;         usage; exit 2; fi
if [ -z "$FORMULA_CLI" ];  then echo "ERROR: --formula-cli is required" >&2;  usage; exit 2; fi
if [ -z "$FORMULA_SHIM" ]; then echo "ERROR: --formula-shim is required" >&2; usage; exit 2; fi
for f in "$CASK" "$FORMULA_CLI" "$FORMULA_SHIM"; do
    [ -f "$f" ] || { echo "ERROR: file not found: $f" >&2; exit 1; }
done

if ! command -v brew >/dev/null 2>&1; then
    echo "ERROR: brew not on PATH" >&2; exit 1
fi

TAP_NAME="local/git-same-verify"
TAP_PATH="$(brew --repository)/Library/Taps/local/homebrew-git-same-verify"

cleanup() {
    if [ -d "$TAP_PATH" ]; then
        brew untap "$TAP_NAME" >/dev/null 2>&1 || rm -rf "$TAP_PATH"
    fi
}
trap cleanup EXIT

echo "==> Creating throwaway tap at $TAP_PATH"
mkdir -p "$TAP_PATH/Formula" "$TAP_PATH/Casks"
cp "$FORMULA_CLI"  "$TAP_PATH/Formula/git-same-cli.rb"
cp "$FORMULA_SHIM" "$TAP_PATH/Formula/git-same.rb"
cp "$CASK"         "$TAP_PATH/Casks/git-same.rb"

# brew tap-new normally requires a git repo; we mimic enough for tap commands
# to work by initializing one.
( cd "$TAP_PATH" && git init -q && git add . && git -c user.email=verify@local -c user.name=verify commit -q -m init )

echo "==> brew style (cask + formulae)"
brew style --strict "$TAP_PATH"

AUDIT_FLAGS=(--strict)
if [ "$ONLINE" -eq 1 ]; then
    AUDIT_FLAGS+=(--online)
fi

echo "==> brew audit (cask)"
brew audit "${AUDIT_FLAGS[@]}" --cask "$TAP_NAME/git-same"

echo "==> brew audit (formula-cli)"
brew audit "${AUDIT_FLAGS[@]}" --formula "$TAP_NAME/git-same-cli"

echo "==> brew audit (formula-shim)"
brew audit "${AUDIT_FLAGS[@]}" --formula "$TAP_NAME/git-same"

if [ "$INSTALL_SMOKE" -eq 1 ]; then
    echo "==> brew install --cask (smoke)"
    brew install --cask "$TAP_NAME/git-same"
    "$(brew --prefix)/bin/git-same" --version
    brew uninstall --cask "$TAP_NAME/git-same"
fi

echo "==> All tap verification checks passed."
