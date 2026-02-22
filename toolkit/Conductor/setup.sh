#!/bin/bash
# Git-Same (Gisa CLI) Setup Script
# Checks prerequisites and builds the project

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
    echo "The CLI can still work with GITHUB_TOKEN environment variable."
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

# Build the project
echo "--- Building Git-Same ---"
echo "Running: cargo build --release"
echo ""
cargo build --release

echo ""
echo "--- Verifying Binaries ---"
BINARIES=("git-same" "gitsame" "gitsa" "gisa")
ALL_OK=true
for bin in "${BINARIES[@]}"; do
    if [ -f "target/release/$bin" ]; then
        echo "  [OK] $bin"
    else
        echo "  [MISSING] $bin"
        ALL_OK=false
    fi
done

if [ "$ALL_OK" = false ]; then
    echo ""
    echo "WARNING: Some binaries are missing."
fi

echo ""
echo "--- Running Tests ---"
echo "Running: cargo test"
echo ""
cargo test 2>&1 || echo "Note: Some tests may require GitHub authentication"

echo ""
echo "========================================"
echo "  Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Run the prototype:  ./toolkit/Conductor/run.sh"
echo "  2. Or manually install (Option 1): cargo install --path ."
echo "  3. Then run:"
echo "     gisa --help"
echo "     gisa init"
echo "     gisa clone ~/github --dry-run"
echo "  4. Remove installed binaries: ./toolkit/Conductor/archive.sh"
echo ""
