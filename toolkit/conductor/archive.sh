#!/bin/bash
# Git-Same (Gisa CLI) Archive Script
# Removes cargo-installed binaries from ~/.cargo/bin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

PACKAGE_NAME="git-same"
CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
ALIAS_FILE="$PROJECT_DIR/toolkit/packaging/binary-aliases.txt"
if [ -f "$ALIAS_FILE" ]; then
    BINARIES=()
    while IFS= read -r line || [ -n "$line" ]; do
        line="${line%%#*}"
        line="${line#"${line%%[![:space:]]*}"}"
        line="${line%"${line##*[![:space:]]}"}"
        [ -n "$line" ] && BINARIES+=("$line")
    done < "$ALIAS_FILE"
    if [ ${#BINARIES[@]} -eq 0 ]; then
        echo "WARNING: $ALIAS_FILE contains no aliases, falling back to hardcoded list."
        BINARIES=("git-same" "gitsame" "gitsa" "gisa")
    fi
else
    echo "WARNING: $ALIAS_FILE not found, falling back to hardcoded list."
    BINARIES=("git-same" "gitsame" "gitsa" "gisa")
fi

echo "========================================"
echo "  Git-Same (Gisa CLI) Archive"
echo "========================================"
echo ""
echo "Project directory: $PROJECT_DIR"
echo "Cargo bin directory: $CARGO_BIN_DIR"
echo ""

if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found."
    exit 1
fi

# Owner-scoped monitor cleanup. run.sh suppresses automatic monitor management,
# so this only matters after a deliberate GIT_SAME_DEV_ALLOW_MONITOR_AUTOSTART=1
# session. Never uninstall a monitor this worktree does not own: that would
# stop and remove the developer's real background monitor.
echo "--- Monitor Cleanup (owner-scoped) ---"
INSTALL_RECORD="$HOME/Library/Application Support/com.zaai.git-same/monitor/install.json"
MANAGED_HELPER="$HOME/Library/Application Support/com.zaai.git-same/monitor/git-same"
if [ -f "$INSTALL_RECORD" ] && grep -qF "\"$PROJECT_DIR/" "$INSTALL_RECORD"; then
    if [ -x "$MANAGED_HELPER" ] && "$MANAGED_HELPER" --quiet monitor --uninstall; then
        echo "  [REMOVED] background monitor installed from this worktree"
        echo "  Monitoring is now stopped. Re-enable it from your real install:"
        echo "    gisa monitor --start"
    else
        echo "  WARNING: monitor was installed from this worktree but could not be removed."
        echo "  Run 'gisa monitor --uninstall' from an installed Git-Same."
    fi
else
    echo "  No monitor owned by this worktree. Skipping."
fi
echo ""

echo "--- Uninstalling Cargo Package ---"
if cargo uninstall "$PACKAGE_NAME"; then
    echo "Removed package: $PACKAGE_NAME"
else
    echo "Package '$PACKAGE_NAME' is not currently installed. Continuing cleanup..."
fi
echo ""

echo "--- Removing Leftover Binaries ---"
FOUND_LEFTOVERS=false
for bin in "${BINARIES[@]}"; do
    path="$CARGO_BIN_DIR/$bin"
    # -L also catches aliases left dangling after the worktree was removed.
    if [ -f "$path" ] || [ -L "$path" ]; then
        rm -f "$path"
        echo "  [REMOVED] $path"
        FOUND_LEFTOVERS=true
    fi
done

if [ "$FOUND_LEFTOVERS" = false ]; then
    echo "  No leftover binaries found."
fi

echo ""
echo "========================================"
echo "  Archive Complete"
echo "========================================"
echo ""
