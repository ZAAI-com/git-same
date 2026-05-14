#!/usr/bin/env bash
# Render the git-same-cli Homebrew Formula from the checked-in template.
#
# Cross-platform: signed macOS tarballs + Linux tarballs.
#
# Usage:
#   render-formula.sh VERSION --url URL_PREFIX \
#                     --sha-macos-arm   <hex> --sha-macos-intel <hex> \
#                     --sha-linux-arm   <hex> --sha-linux-intel <hex> \
#                     [--out PATH]
#
# Reads:    toolkit/homebrew/formula-cli.rb.tmpl
# Writes:   stdout, or --out PATH
#
# Used both locally (for `brew style` + `brew install` testing against a draft
# release) and from the S3 publishing workflow.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

VERSION=""
URL=""
SHA_MAC_ARM=""
SHA_MAC_INTEL=""
SHA_LIN_ARM=""
SHA_LIN_INTEL=""
OUT=""

usage() {
    cat <<EOF >&2
Usage:
  $0 VERSION --url URL_PREFIX \\
       --sha-macos-arm <hex> --sha-macos-intel <hex> \\
       --sha-linux-arm <hex> --sha-linux-intel <hex> [--out PATH]

  VERSION              Strict semver, no leading zeros, no v prefix (e.g. 3.0.1)
  --url                URL prefix (no trailing slash)
  --sha-*              SHA256 of the tarball (64 hex chars)
EOF
}

require_flag_value() {
    local flag="$1"
    if [ "$#" -lt 2 ] || [[ "$2" == --* ]]; then
        echo "ERROR: ${flag} requires a value" >&2
        usage
        exit 2
    fi
}

while [ $# -gt 0 ]; do
    case "$1" in
        --url)               require_flag_value "$@"; URL="$2"; shift 2 ;;
        --sha-macos-arm)     require_flag_value "$@"; SHA_MAC_ARM="$2"; shift 2 ;;
        --sha-macos-intel)   require_flag_value "$@"; SHA_MAC_INTEL="$2"; shift 2 ;;
        --sha-linux-arm)     require_flag_value "$@"; SHA_LIN_ARM="$2"; shift 2 ;;
        --sha-linux-intel)   require_flag_value "$@"; SHA_LIN_INTEL="$2"; shift 2 ;;
        --out)               require_flag_value "$@"; OUT="$2"; shift 2 ;;
        -h|--help)           usage; exit 0 ;;
        --*)                 echo "ERROR: unknown flag $1" >&2; usage; exit 2 ;;
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

require_sha() {
    local label="$1" value="$2"
    if ! [[ "$value" =~ ^[0-9a-f]{64}$ ]]; then
        echo "ERROR: $label must be 64 lowercase hex chars" >&2; exit 2
    fi
}

TEMPLATE="$SCRIPT_DIR/formula-cli.rb.tmpl"
if [ -z "$URL" ]; then
    echo "ERROR: --url is required" >&2; exit 2
fi
require_sha --sha-macos-arm   "$SHA_MAC_ARM"
require_sha --sha-macos-intel "$SHA_MAC_INTEL"
require_sha --sha-linux-arm   "$SHA_LIN_ARM"
require_sha --sha-linux-intel "$SHA_LIN_INTEL"

if [ ! -r "$TEMPLATE" ]; then
    echo "ERROR: template not readable: $TEMPLATE" >&2
    exit 1
fi

RENDERED="$(sed \
    -e "s|VERSION_PLACEHOLDER|${VERSION}|g" \
    -e "s|URL_PLACEHOLDER|${URL}|g" \
    -e "s|SHA_LINUX_X86_64_PLACEHOLDER|${SHA_LIN_INTEL}|g" \
    -e "s|SHA_LINUX_AARCH64_PLACEHOLDER|${SHA_LIN_ARM}|g" \
    -e "s|SHA_MACOS_X86_64_PLACEHOLDER|${SHA_MAC_INTEL}|g" \
    -e "s|SHA_MACOS_AARCH64_PLACEHOLDER|${SHA_MAC_ARM}|g" \
    "$TEMPLATE")"

if [ -n "$OUT" ]; then
    printf '%s\n' "$RENDERED" > "$OUT"
else
    printf '%s\n' "$RENDERED"
fi
