#!/usr/bin/env bash
# Render a git-same Homebrew Formula from one of the checked-in templates.
#
# Two kinds:
#   --kind cli   Renders formula-cli.rb.tmpl  (cross-platform, signed macOS
#                tarballs + Linux tarballs)
#   --kind shim  Renders formula-shim.rb.tmpl (deprecation shim that points
#                at the cli formula)
#
# Usage (cli):
#   render-formula.sh VERSION --kind cli --url URL_PREFIX \
#                     --sha-macos-arm   <hex> --sha-macos-intel <hex> \
#                     --sha-linux-arm   <hex> --sha-linux-intel <hex> \
#                     [--out PATH]
#
# Usage (shim):
#   render-formula.sh VERSION --kind shim --deprecation-date YYYY-MM-DD \
#                     --src-sha <hex> [--out PATH]
#
# Reads:    toolkit/homebrew/formula-cli.rb.tmpl  or  formula-shim.rb.tmpl
# Writes:   stdout, or --out PATH
#
# Used both locally (for `brew style` + `brew install` testing against a draft
# release) and from the S3 publishing workflow.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

VERSION=""
KIND=""
URL=""
SHA_MAC_ARM=""
SHA_MAC_INTEL=""
SHA_LIN_ARM=""
SHA_LIN_INTEL=""
DEPRECATION_DATE=""
SRC_SHA=""
OUT=""

usage() {
    cat <<EOF >&2
Usage:
  $0 VERSION --kind cli --url URL_PREFIX \\
       --sha-macos-arm <hex> --sha-macos-intel <hex> \\
       --sha-linux-arm <hex> --sha-linux-intel <hex> [--out PATH]

  $0 VERSION --kind shim --deprecation-date YYYY-MM-DD \\
       --src-sha <hex> [--out PATH]

  VERSION              Strict semver, no leading zeros, no v prefix (e.g. 3.0.0)
  --url                URL prefix for cli kind (no trailing slash)
  --sha-*              SHA256 of the tarball (64 hex chars)
  --deprecation-date   ISO date when the shim begins emitting a warning
  --src-sha            SHA256 of the GitHub source archive tarball
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --kind)              KIND="${2:-}"; shift 2 ;;
        --url)               URL="${2:-}"; shift 2 ;;
        --sha-macos-arm)     SHA_MAC_ARM="${2:-}"; shift 2 ;;
        --sha-macos-intel)   SHA_MAC_INTEL="${2:-}"; shift 2 ;;
        --sha-linux-arm)     SHA_LIN_ARM="${2:-}"; shift 2 ;;
        --sha-linux-intel)   SHA_LIN_INTEL="${2:-}"; shift 2 ;;
        --deprecation-date)  DEPRECATION_DATE="${2:-}"; shift 2 ;;
        --src-sha)           SRC_SHA="${2:-}"; shift 2 ;;
        --out)               OUT="${2:-}"; shift 2 ;;
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
    echo "ERROR: VERSION must be strict semver (e.g. 3.0.0), got '$VERSION'" >&2
    exit 2
fi

require_sha() {
    local label="$1" value="$2"
    if ! [[ "$value" =~ ^[0-9a-f]{64}$ ]]; then
        echo "ERROR: $label must be 64 lowercase hex chars" >&2; exit 2
    fi
}

case "$KIND" in
    cli)
        TEMPLATE="$SCRIPT_DIR/formula-cli.rb.tmpl"
        if [ -z "$URL" ]; then
            echo "ERROR: --url is required for --kind cli" >&2; exit 2
        fi
        require_sha --sha-macos-arm   "$SHA_MAC_ARM"
        require_sha --sha-macos-intel "$SHA_MAC_INTEL"
        require_sha --sha-linux-arm   "$SHA_LIN_ARM"
        require_sha --sha-linux-intel "$SHA_LIN_INTEL"
        RENDERED="$(sed \
            -e "s|VERSION_PLACEHOLDER|${VERSION}|g" \
            -e "s|URL_PLACEHOLDER|${URL}|g" \
            -e "s|SHA_LINUX_X86_64_PLACEHOLDER|${SHA_LIN_INTEL}|g" \
            -e "s|SHA_LINUX_AARCH64_PLACEHOLDER|${SHA_LIN_ARM}|g" \
            -e "s|SHA_MACOS_X86_64_PLACEHOLDER|${SHA_MAC_INTEL}|g" \
            -e "s|SHA_MACOS_AARCH64_PLACEHOLDER|${SHA_MAC_ARM}|g" \
            "$TEMPLATE")"
        ;;
    shim)
        TEMPLATE="$SCRIPT_DIR/formula-shim.rb.tmpl"
        if [ -z "$DEPRECATION_DATE" ]; then
            echo "ERROR: --deprecation-date is required for --kind shim" >&2; exit 2
        fi
        if ! [[ "$DEPRECATION_DATE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
            echo "ERROR: --deprecation-date must be ISO date YYYY-MM-DD" >&2; exit 2
        fi
        require_sha --src-sha "$SRC_SHA"
        RENDERED="$(sed \
            -e "s|VERSION_PLACEHOLDER|${VERSION}|g" \
            -e "s|DEPRECATION_DATE_PLACEHOLDER|${DEPRECATION_DATE}|g" \
            -e "s|SOURCE_TARBALL_SHA_PLACEHOLDER|${SRC_SHA}|g" \
            "$TEMPLATE")"
        ;;
    "")
        echo "ERROR: --kind is required (cli|shim)" >&2; usage; exit 2 ;;
    *)
        echo "ERROR: --kind must be cli or shim, got '$KIND'" >&2; exit 2 ;;
esac

if [ ! -r "$TEMPLATE" ]; then
    echo "ERROR: template not readable: $TEMPLATE" >&2
    exit 1
fi

if [ -n "$OUT" ]; then
    printf '%s\n' "$RENDERED" > "$OUT"
else
    printf '%s\n' "$RENDERED"
fi
