#!/usr/bin/env bash
# Render the git-same Homebrew Cask from the checked-in template.
#
# Usage:
#   render-cask.sh VERSION --sha-arm <hex> --sha-intel <hex> [--out PATH]
#
# Reads:    toolkit/homebrew/cask.rb.tmpl
# Writes:   stdout, or --out PATH
#
# Used both locally (for brew style + brew install --cask testing against a
# draft release) and from the S3 publishing workflow.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE="$SCRIPT_DIR/cask.rb.tmpl"

if [ ! -r "$TEMPLATE" ]; then
    echo "ERROR: Cask template not readable: $TEMPLATE" >&2
    exit 1
fi

VERSION=""
SHA_ARM=""
SHA_INTEL=""
OUT=""

usage() {
    cat <<EOF >&2
Usage: $0 VERSION --sha-arm <hex> --sha-intel <hex> [--out PATH]

  VERSION       Strict semver, no leading zeros, no v prefix (e.g. 3.0.1)
  --sha-arm     SHA256 of the aarch64 tarball (64 hex chars)
  --sha-intel   SHA256 of the x86_64 tarball (64 hex chars)
  --out PATH    Write to PATH instead of stdout
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --sha-arm)   SHA_ARM="${2:-}"; shift 2 ;;
        --sha-intel) SHA_INTEL="${2:-}"; shift 2 ;;
        --out)       OUT="${2:-}"; shift 2 ;;
        -h|--help)   usage; exit 0 ;;
        --*)         echo "ERROR: unknown flag $1" >&2; usage; exit 2 ;;
        *)
            if [ -z "$VERSION" ]; then
                VERSION="$1"; shift
            else
                echo "ERROR: unexpected positional arg $1" >&2; usage; exit 2
            fi
            ;;
    esac
done

if [ -z "$VERSION" ]; then
    echo "ERROR: VERSION is required" >&2; usage; exit 2
fi
if ! [[ "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "ERROR: VERSION must be strict semver (e.g. 3.0.1), got '$VERSION'" >&2
    exit 2
fi
if ! [[ "$SHA_ARM" =~ ^[0-9a-f]{64}$ ]]; then
    echo "ERROR: --sha-arm must be 64 lowercase hex chars" >&2; exit 2
fi
if ! [[ "$SHA_INTEL" =~ ^[0-9a-f]{64}$ ]]; then
    echo "ERROR: --sha-intel must be 64 lowercase hex chars" >&2; exit 2
fi

RENDERED="$(sed \
    -e "s|VERSION_PLACEHOLDER|${VERSION}|g" \
    -e "s|SHA_AARCH64_PLACEHOLDER|${SHA_ARM}|g" \
    -e "s|SHA_X86_64_PLACEHOLDER|${SHA_INTEL}|g" \
    "$TEMPLATE")"

if [ -n "$OUT" ]; then
    printf '%s\n' "$RENDERED" > "$OUT"
else
    printf '%s\n' "$RENDERED"
fi
