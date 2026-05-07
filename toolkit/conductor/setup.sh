#!/bin/bash
# Git-Same Setup Script
# Checks prerequisites

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_DIR"

echo "========================================"
echo "  Git-Same (Gisa CLI) Setup"
echo "========================================"
echo ""
echo "Project directory: $PROJECT_DIR"
echo ""

# Check Rust toolchain
echo "--- Checking Rust Toolchain ---"
if ! command -v rustc &> /dev/null; then
    echo "ERROR: Rust not found."
    echo "Install from: https://rustup.rs/"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"
echo ""

# Check GitHub CLI
echo "--- Checking GitHub CLI ---"
if ! command -v gh &> /dev/null; then
    echo "WARNING: GitHub CLI (gh) not found."
    echo "Install with: brew install gh"
    echo "Git-Same requires GitHub CLI for authentication."
    echo "Install gh first, then run: gh auth login"
    echo ""
else
    echo "gh: $(gh --version | head -1)"
    echo ""
    echo "GitHub CLI authentication status:"
    if gh auth status 2>&1; then
        echo ""
    else
        echo ""
        echo "WARNING: GitHub CLI not authenticated."
        echo "Run: gh auth login"
        echo ""
    fi
fi

# Check Git
echo "--- Checking Git ---"
if ! command -v git &> /dev/null; then
    echo "ERROR: Git not found."
    exit 1
fi
echo "git: $(git --version)"
echo ""

# Clean build artifacts and update dependencies
echo "--- Cleaning Build Cache ---"
cargo clean
echo ""
echo "--- Updating Dependencies ---"
cargo update
echo ""

echo "========================================"
echo "  Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Run:  ./toolkit/conductor/run.sh"
echo "  2. Or manually install: cargo install --path crates/git-same-cli --force"
echo "     (then refresh aliases via ./toolkit/conductor/run.sh)"
echo ""
