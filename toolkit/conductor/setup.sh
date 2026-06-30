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

# Check Node.js (required for Tauri app frontend)
echo "--- Checking Node.js ---"
if ! command -v node &> /dev/null; then
    echo "ERROR: Node.js not found."
    echo "Install with: brew install node"
    echo "Or via nvm: https://github.com/nvm-sh/nvm"
    exit 1
fi
echo "node: $(node --version)"
echo ""

# Enable pnpm via Corepack
echo "--- Enabling pnpm (Corepack) ---"
if ! command -v corepack &> /dev/null; then
    echo "ERROR: Corepack not found. Requires Node.js 16.10+."
    echo "Reinstall Node or run: npm install -g corepack"
    exit 1
fi
corepack enable pnpm
echo "pnpm: $(corepack pnpm --version)"
echo ""

# Install Tauri app frontend dependencies
echo "--- Installing Tauri app frontend dependencies ---"
UI_DIR="$PROJECT_DIR/crates/git-same-app/ui"
if ! corepack pnpm --dir "$UI_DIR" install --frozen-lockfile; then
    echo "WARNING: --frozen-lockfile failed, retrying without it."
    corepack pnpm --dir "$UI_DIR" install
fi
echo ""

# Sanity-check Tauri CLI
echo "--- Checking Tauri CLI ---"
TAURI_CLI="$UI_DIR/node_modules/.bin/tauri"
if [ ! -x "$TAURI_CLI" ] || ! "$TAURI_CLI" --version &> /dev/null; then
    echo "ERROR: Tauri CLI not runnable at $TAURI_CLI"
    echo "Re-run: corepack pnpm --dir $UI_DIR install"
    exit 1
fi
echo "tauri: $("$TAURI_CLI" --version)"
echo ""

# Reset Rust state, then warm the workspace target/ cache so the next
# run.sh is a tight incremental build instead of a full cold compile.
echo "--- Cleaning Build Cache ---"
cargo clean
echo ""
echo "--- Updating Dependencies ---"
cargo update
echo ""
echo "--- Pre-building workspace (debug) to warm target/ cache ---"
cargo build --workspace
echo ""

echo "========================================"
echo "  Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Run:  ./toolkit/conductor/run.sh"
echo "     (installs the CLI/TUI and launches the Tauri app in dev mode)"
echo "  2. Or manually install: cargo install --path crates/git-same-cli --force"
echo "     (then refresh aliases via ./toolkit/conductor/run.sh)"
echo ""
