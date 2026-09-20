#!/usr/bin/env bash
# Verify that the Tauri host, the FinderSync extension, and the signed
# standalone CLI declare matching `com.apple.security.application-groups`
# entries. Only that key is compared: the extension also needs app-sandbox,
# which the host and the CLI must not have.
#
# A typo here silently splits the runtime container: the monitor writes to
# one group and the extension reads from another, and badges stop rendering
# without an obvious error. CI must catch this before signing.
#
# macOS-only (uses /usr/bin/plutil). Run from S2's app-DMG build job before the
# bundle build, and from S1's macOS Tauri build job so PRs catch drift early.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

HOST="$ROOT/crates/git-same-app/entitlements.plist"
EXT="$ROOT/macos/GitSameBadges/GitSameBadges.entitlements"
CLI="$ROOT/toolkit/packaging/darwin-entitlements.plist"
RELEASE_WORKFLOW="$ROOT/.github/workflows/S2-Release-GitHub.yml"

for f in "$HOST" "$EXT" "$CLI"; do
    if [ ! -r "$f" ]; then
        echo "ERROR: entitlements file not readable: $f" >&2
        exit 1
    fi
done

extract_groups() {
    # PlistBuddy treats `:` as path separators (not `.`), so dotted keys like
    # `com.apple.security.application-groups` work as a single segment.
    # `-x` emits XML; we pull the inner <string> elements and sort them so
    # the comparison is order-insensitive.
    local xml
    if ! xml="$(/usr/libexec/PlistBuddy -x \
        -c "Print :com.apple.security.application-groups" "$1" 2>/dev/null)"; then
        echo "ERROR: $1 declares no com.apple.security.application-groups" >&2
        exit 1
    fi
    printf '%s\n' "$xml" | grep -oE '<string>[^<]*' | sed 's/<string>//' | sort
}

HOST_GROUPS="$(extract_groups "$HOST")"
EXT_GROUPS="$(extract_groups "$EXT")"
CLI_GROUPS="$(extract_groups "$CLI")"

if [ "$HOST_GROUPS" != "$EXT_GROUPS" ] || [ "$HOST_GROUPS" != "$CLI_GROUPS" ]; then
    echo "ERROR: application-groups mismatch between host, extension, and CLI." >&2
    echo "  host ($HOST):" >&2
    echo "    $HOST_GROUPS" >&2
    echo "  ext  ($EXT):" >&2
    echo "    $EXT_GROUPS" >&2
    echo "  cli  ($CLI):" >&2
    echo "    $CLI_GROUPS" >&2
    echo "" >&2
    echo "These lists must be identical. A mismatch silently splits the" >&2
    echo "runtime app-group container and breaks Finder badge rendering." >&2
    exit 1
fi

# The entitlement only reaches the CLI if the release workflow passes it.
if ! grep -q -- "darwin-entitlements.plist" "$RELEASE_WORKFLOW"; then
    echo "ERROR: $RELEASE_WORKFLOW no longer signs the CLI with darwin-entitlements.plist" >&2
    exit 1
fi

echo "OK: application-groups match across host, extension, and CLI"
echo "    $HOST_GROUPS"
