#!/bin/bash
# Git-Same Run Script
# Installs binaries and shows available commands

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
ALIAS_FILE="$PROJECT_DIR/toolkit/packaging/binary-aliases.txt"
if [ ! -r "$ALIAS_FILE" ]; then
    echo "ERROR: Alias manifest not found or unreadable: $ALIAS_FILE"
    exit 1
fi

BINARIES=()
while IFS= read -r line || [ -n "$line" ]; do
    line="${line%%#*}"
    line="${line#"${line%%[![:space:]]*}"}"
    line="${line%"${line##*[![:space:]]}"}"
    [ -n "$line" ] && BINARIES+=("$line")
done < "$ALIAS_FILE"

if [ ${#BINARIES[@]} -eq 0 ]; then
    echo "ERROR: Alias manifest contains no aliases: $ALIAS_FILE"
    exit 1
fi

PRIMARY_BIN="${BINARIES[0]}"
GS_COMMAND="$CARGO_BIN_DIR/$PRIMARY_BIN"

# Install primary binary
echo "Installing with: cargo install --path crates/git-same-cli --force"
cargo install --path crates/git-same-cli --force
echo ""

if [ ! -x "$CARGO_BIN_DIR/$PRIMARY_BIN" ]; then
    echo "ERROR: $PRIMARY_BIN installation failed."
    exit 1
fi

# Create alias symlinks from manifest (skip primary)
for alias in "${BINARIES[@]:1}"; do
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
echo "Interactive TUI:"
echo ""
echo "  $GS_COMMAND                                   # Launch TUI (no subcommand)"
echo ""
echo "Sync repos (discover + clone new + fetch existing):"
echo ""
echo "  $GS_COMMAND sync --dry-run                    # Preview what would happen"
echo "  $GS_COMMAND sync                              # Run sync (fetch mode)"
echo "  $GS_COMMAND sync --pull                       # Sync with pull instead of fetch"
echo "  $GS_COMMAND sync --workspace github           # Sync specific workspace"
echo "  $GS_COMMAND sync --concurrency 8              # Control parallelism"
echo "  $GS_COMMAND sync --refresh                    # Ignore cache, re-discover repos"
echo "  $GS_COMMAND sync --no-skip-uncommitted        # Don't skip dirty repos"
echo ""
echo "Status:"
echo ""
echo "  $GS_COMMAND status                            # Show all repo status"
echo "  $GS_COMMAND status --uncommitted              # Only repos with changes"
echo "  $GS_COMMAND status --behind                   # Only repos behind upstream"
echo "  $GS_COMMAND status --detailed                 # Full detail per repo"
echo "  $GS_COMMAND status --org my-org               # Filter to one org (repeatable)"
echo ""
echo "Workspace management:"
echo ""
echo "  $GS_COMMAND workspace list                    # List configured workspaces"
echo "  $GS_COMMAND workspace default                 # Show current default"
echo "  $GS_COMMAND workspace default my-ws           # Set default workspace"
echo "  $GS_COMMAND workspace default --clear         # Clear the default"
echo ""
echo "Scan for unregistered workspaces:"
echo ""
echo "  $GS_COMMAND scan                              # Scan current directory"
echo "  $GS_COMMAND scan ~/projects                   # Scan a specific directory"
echo "  $GS_COMMAND scan --depth 3                    # Limit search depth"
echo "  $GS_COMMAND scan ~/projects --register        # Auto-register found workspaces"
echo ""
echo "Finder extension daemon (macOS):"
echo ""
echo "  $GS_COMMAND daemon                            # Start daemon (daemonizes)"
echo "  $GS_COMMAND daemon --foreground               # Run in foreground (debug)"
echo "  $GS_COMMAND daemon --interval 60              # Poll every 60 seconds"
echo "  $GS_COMMAND daemon --status                   # Check if daemon is running"
echo "  $GS_COMMAND daemon --stop                     # Stop a running daemon"
echo "  $GS_COMMAND refresh                           # Force immediate status.json rewrite"
echo "  $GS_COMMAND refresh --path ~/work/org         # Refresh a single folder"
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

# Launch the Tauri desktop app in dev mode
TAURI_CLI="$PROJECT_DIR/crates/git-same-app/ui/node_modules/.bin/tauri"
if [ ! -x "$TAURI_CLI" ]; then
    echo "ERROR: Tauri CLI not found at $TAURI_CLI"
    echo "Run ./toolkit/conductor/setup.sh first to install frontend dependencies."
    exit 1
fi

echo "========================================"
echo "  Launching Tauri app (dev mode)"
echo "========================================"
echo ""
cd "$PROJECT_DIR/crates/git-same-app"
exec "$TAURI_CLI" dev
