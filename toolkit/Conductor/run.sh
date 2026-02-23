#!/bin/bash
# Git-Same Run Script
# Installs binaries and shows available commands

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

CARGO_BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
GS_COMMAND="$CARGO_BIN_DIR/git-same"
TEST_DIR="${1:-/tmp/gisa-prototype-test}"

# Always install to ensure all binaries are up to date
echo "Installing with: cargo install --path ."
cargo install --path .
echo ""

if [ ! -x "$GS_COMMAND" ]; then
    echo "ERROR: git-same installation failed."
    exit 1
fi

# Warn if git-same is also installed elsewhere (e.g. Homebrew)
RED='\033[0;31m'
NC='\033[0m'
OTHER_PATHS=$(which -a git-same 2>/dev/null | grep -v "$CARGO_BIN_DIR" || true)
if [ -n "$OTHER_PATHS" ]; then
    echo -e "${RED}WARNING: git-same found in another location:${NC}"
    echo -e "${RED}  $OTHER_PATHS${NC}"
    echo -e "${RED}  This may shadow the version installed by this script.${NC}"
    echo -e "${RED}  Consider uninstalling it to avoid version conflicts.${NC}"
    echo ""
fi

echo "========================================"
echo "  Feature Test Commands"
echo "========================================"
echo ""
echo "Try these commands to test features:"
echo ""
echo "  # Clone (dry-run first to preview)"
echo "  $GS_COMMAND clone $TEST_DIR --dry-run"
echo ""
echo "  # Clone with filters"
echo "  $GS_COMMAND clone $TEST_DIR --org YOUR_ORG --depth 1"
echo ""
echo "  # Check status"
echo "  $GS_COMMAND status $TEST_DIR"
echo "  $GS_COMMAND status $TEST_DIR --dirty"
echo "  $GS_COMMAND status $TEST_DIR --detailed"
echo ""
echo "  # Fetch updates"
echo "  $GS_COMMAND fetch $TEST_DIR --dry-run"
echo "  $GS_COMMAND fetch $TEST_DIR"
echo ""
echo "  # Pull updates"
echo "  $GS_COMMAND pull $TEST_DIR --dry-run"
echo ""
echo "  # Shell completions"
echo "  $GS_COMMAND completions bash"
echo "  $GS_COMMAND completions zsh"
echo "  $GS_COMMAND completions fish"
echo ""
echo "  # Verbose and JSON output"
echo "  $GS_COMMAND -v clone $TEST_DIR --dry-run"
echo "  $GS_COMMAND --json status $TEST_DIR"
echo ""
