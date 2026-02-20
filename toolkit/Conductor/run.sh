#!/bin/bash
# Git-Same (Gisa CLI) Run Script
# Runs the prototype and demonstrates features

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

GISA="./target/release/gisa"
CONFIG_FILE="$HOME/.config/git-same/config.toml"
TEST_DIR="${1:-/tmp/gisa-prototype-test}"

# Check if binary exists, build if not
if [ ! -f "$GISA" ]; then
    echo "Binary not found. Running setup first..."
    "$SCRIPT_DIR/setup.sh"
    echo ""
fi

echo "========================================"
echo "  Git-Same (Gisa CLI) Prototype"
echo "========================================"
echo ""

# Show version
echo "--- Version ---"
$GISA --version
echo ""

# Show help
echo "--- Available Commands ---"
$GISA --help
echo ""

# Initialize config if not exists
echo "--- Configuration ---"
if [ -f "$CONFIG_FILE" ]; then
    echo "Config file exists: $CONFIG_FILE"
else
    echo "Initializing configuration..."
    $GISA init
    echo "Config created: $CONFIG_FILE"
fi
echo ""

# Show config contents
echo "--- Config Contents ---"
if [ -f "$CONFIG_FILE" ]; then
    cat "$CONFIG_FILE"
fi
echo ""

# Dry run clone
echo "========================================"
echo "  Running Dry-Run Clone"
echo "========================================"
echo ""
echo "Test directory: $TEST_DIR"
echo "Command: $GISA clone $TEST_DIR --dry-run -v"
echo ""

$GISA clone "$TEST_DIR" --dry-run -v 2>&1 || {
    echo ""
    echo "Note: If you see authentication errors, make sure you have:"
    echo "  1. GitHub CLI authenticated: gh auth login"
    echo "  2. Or GITHUB_TOKEN environment variable set"
}

echo ""
echo "========================================"
echo "  Feature Test Commands"
echo "========================================"
echo ""
echo "Try these commands to test features:"
echo ""
echo "  # Clone (dry-run first to preview)"
echo "  $GISA clone $TEST_DIR --dry-run"
echo ""
echo "  # Clone with filters"
echo "  $GISA clone $TEST_DIR --org YOUR_ORG --depth 1"
echo ""
echo "  # Check status"
echo "  $GISA status $TEST_DIR"
echo "  $GISA status $TEST_DIR --dirty"
echo "  $GISA status $TEST_DIR --detailed"
echo ""
echo "  # Fetch updates"
echo "  $GISA fetch $TEST_DIR --dry-run"
echo "  $GISA fetch $TEST_DIR"
echo ""
echo "  # Pull updates"
echo "  $GISA pull $TEST_DIR --dry-run"
echo ""
echo "  # Shell completions"
echo "  $GISA completions bash"
echo "  $GISA completions zsh"
echo "  $GISA completions fish"
echo ""
echo "  # Verbose and JSON output"
echo "  $GISA -v clone $TEST_DIR --dry-run"
echo "  $GISA --json status $TEST_DIR"
echo ""
