#!/bin/bash
# Git-Same Run Script
# Installs binaries and shows available commands

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
ALIAS_FILE="$PROJECT_DIR/toolkit/packaging/binary-aliases.txt"
PRIMARY_BIN=$(head -n1 "$ALIAS_FILE")
GS_COMMAND="$CARGO_BIN_DIR/$PRIMARY_BIN"

# Install primary binary
echo "Installing with: cargo install --path . --force"
cargo install --path . --force
echo ""

if [ ! -x "$CARGO_BIN_DIR/$PRIMARY_BIN" ]; then
    echo "ERROR: $PRIMARY_BIN installation failed."
    exit 1
fi

# Create alias symlinks from manifest (skip line 1 = primary)
tail -n +2 "$ALIAS_FILE" | while read -r alias; do
    [ -z "$alias" ] && continue
    # Replace stale standalone alias binaries with a symlink to the primary binary.
    if [ -e "$CARGO_BIN_DIR/$alias" ] && [ ! -L "$CARGO_BIN_DIR/$alias" ]; then
        rm -f "$CARGO_BIN_DIR/$alias"
    fi
    ln -sf "$CARGO_BIN_DIR/$PRIMARY_BIN" "$CARGO_BIN_DIR/$alias"
    echo "  Symlinked: $alias -> $PRIMARY_BIN"
done
echo ""

# Warn if gisa is also installed elsewhere (e.g. Homebrew)
RED='\033[0;31m'
NC='\033[0m'
OTHER_PATHS=$(which -a gisa 2>/dev/null | grep -v "$CARGO_BIN_DIR" || true)
if [ -n "$OTHER_PATHS" ]; then
    echo -e "${RED}WARNING: gisa found in another location:${NC}"
    echo -e "${RED}  $OTHER_PATHS${NC}"
    echo -e "${RED}  This may shadow the version installed by this script.${NC}"
    echo -e "${RED}  Consider uninstalling it to avoid version conflicts.${NC}"
    echo ""
fi

echo "========================================"
echo "  Gisa Commands"
echo "========================================"
echo ""
echo "Getting started:"
echo ""
echo "  $GS_COMMAND init                              # Create config file"
echo "  $GS_COMMAND setup                             # Interactive workspace wizard"
echo ""
echo "Sync repos (discover + clone new + fetch existing):"
echo ""
echo "  $GS_COMMAND sync --dry-run                    # Preview what would happen"
echo "  $GS_COMMAND sync                              # Run sync (fetch mode)"
echo "  $GS_COMMAND sync --pull                       # Sync with pull instead of fetch"
echo "  $GS_COMMAND sync --workspace github           # Sync specific workspace"
echo "  $GS_COMMAND sync --concurrency 8              # Control parallelism"
echo ""
echo "Status:"
echo ""
echo "  $GS_COMMAND status                            # Show all repo status"
echo "  $GS_COMMAND status --uncommitted              # Only repos with changes"
echo "  $GS_COMMAND status --detailed                 # Full detail per repo"
echo ""
echo "Workspace management:"
echo ""
echo "  $GS_COMMAND workspace list                    # List configured workspaces"
echo "  $GS_COMMAND workspace default my-ws           # Set default workspace"
echo "  $GS_COMMAND workspace default                 # Show current default"
echo ""
echo "Reset / cleanup:"
echo ""
echo "  $GS_COMMAND reset                             # Interactive cleanup"
echo "  $GS_COMMAND reset --force                     # Force remove everything"
echo ""
echo "Verbose and JSON output:"
echo ""
echo "  $GS_COMMAND -v sync --dry-run"
echo "  $GS_COMMAND --json status"
echo ""
