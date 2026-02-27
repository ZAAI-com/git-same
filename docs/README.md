# Git-Same

Mirror GitHub structure /orgs/repos/ to local file system

[![Crates.io](https://img.shields.io/crates/v/git-same.svg)](https://crates.io/crates/git-same)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/zaai-com/git-same/actions/workflows/S1-Test-CI.yml/badge.svg)](https://github.com/zaai-com/git-same/actions/workflows/S1-Test-CI.yml)

## Features

- **Interactive TUI**: Full terminal UI with dashboard, workspace management, and live progress
- **Multi-Provider Support**: Works with GitHub and GitHub Enterprise (GitLab and Bitbucket planned)
- **Parallel Operations**: Clones and syncs repositories concurrently
- **Smart Filtering**: Filter by archived status, forks, organizations
- **Incremental Sync**: Only fetches/pulls what has changed
- **Progress Reporting**: Beautiful progress bars and status updates
- **Multiple Aliases**: Install once, use with your preferred command name

## Installation

### From crates.io

```bash
cargo install git-same
```

### GitHub Releases

Download pre-built binaries from [GitHub Releases](https://github.com/zaai-com/git-same/releases) for Linux (x86_64, ARM64), macOS (x86_64, Apple Silicon), and Windows (x86_64, ARM64).

### From source

```bash
git clone https://github.com/zaai-com/git-same
cd git-same
cargo install --path .
```

### Homebrew

```bash
brew install zaai-com/tap/git-same
```

## Available Commands

The tool can be invoked using any of these names:

| Command    | Description                    |
|------------|--------------------------------|
| `git-same` | Primary binary                 |
| `gitsame`  | No-hyphen alias (symlink)      |
| `gitsa`    | Short alias (symlink)          |
| `gisa`     | Shortest alias (symlink)       |
| `git same` | Git subcommand (requires git-same in PATH) |

> **Install method differences:** Homebrew (`brew install zaai-com/tap/git-same`) installs all aliases automatically. `cargo install git-same` installs only the primary binary — run `toolkit/Conductor/run.sh` to create alias symlinks. The canonical alias list lives in `toolkit/packaging/binary-aliases.txt`.

## Quick Start

### Interactive (TUI)

Run `gisa` with no arguments to launch the interactive terminal UI:

```bash
gisa
```

The TUI provides a dashboard with workspace management, sync operations, repo status, and settings — all accessible via keyboard shortcuts.

### CLI

```bash
# 1. Initialize configuration
gisa init

# 2. Set up a workspace (interactive wizard)
gisa setup

# 3. Sync repositories (discover, clone new, fetch/pull existing)
gisa sync

# 4. Check repository status
gisa status
gisa status --uncommitted
gisa status --behind
```

## Authentication

Git-Same uses GitHub CLI (`gh`) for authentication:

```bash
# Install GitHub CLI
brew install gh  # macOS
# or: sudo apt install gh  # Ubuntu

# Authenticate
gh auth login

# Git-Same will now use your gh credentials
gisa sync
```

## Configuration

Edit `~/.config/git-same/config.toml` to customize behavior:

```toml
# Directory structure: {org}/{repo} or {provider}/{org}/{repo}
structure = "{org}/{repo}"

# Number of concurrent clone/sync operations
concurrency = 4

# Default sync mode: fetch or pull
sync_mode = "fetch"

[clone]
# Clone depth (0 = full history)
depth = 0

# Default branch to clone (empty = provider's default)
branch = ""

# Recursively clone submodules
recurse_submodules = false

[filters]
# Include archived repositories
include_archived = false

# Include forked repositories
include_forks = false

# Filter by organizations (empty = all)
orgs = []

# Default provider (GitHub.com)
[[providers]]
kind = "github"
auth = "gh-cli"
prefer_ssh = true
enabled = true
```

`base_path` is workspace-specific (`WorkspaceConfig.base_path`) and is set during
`gisa setup` (or via workspace config files), not in the global `Config`.

### Multi-Provider Setup

```toml
# GitHub.com
[[providers]]
kind = "github"
auth = "gh-cli"
prefer_ssh = true
enabled = true

# GitHub Enterprise
[[providers]]
kind = "github-enterprise"
name = "Work GitHub"
api_url = "https://github.company.com/api/v3"
auth = "gh-cli"
prefer_ssh = true
enabled = true
base_path = "~/work/code"
```

Authenticate GitHub Enterprise once with:

```bash
gh auth login --hostname github.company.com
```

## Commands

### `init`

Initialize git-same configuration:

```bash
gisa init [-p <config-path>] [-f | --force]
```

Creates a config file at `~/.config/git-same/config.toml` with sensible defaults.

### `setup`

Configure a workspace (interactive wizard):

```bash
gisa setup [--name <NAME>]
```

Walks through provider selection, authentication, org filters, and base path.

### `sync`

Sync repositories — discover, clone new, fetch/pull existing:

```bash
gisa sync [OPTIONS]

Options:
  -w, --workspace <NAME>      Workspace to sync
      --pull                  Use pull instead of fetch for existing repos
  -n, --dry-run               Show what would be done
  -c, --concurrency <N>       Number of parallel operations (1-32)
      --refresh               Force re-discovery (ignore cache)
      --no-skip-uncommitted         Don't skip repos with uncommitted changes
```

### `status`

Show status of local repositories:

```bash
gisa status [OPTIONS]

Options:
  -w, --workspace <NAME>      Workspace to check
  -o, --org <ORG>...          Filter by organization (repeatable)
  -d, --uncommitted                 Show only repositories with uncommitted changes
  -b, --behind                Show only repositories behind upstream
      --detailed              Show detailed status information
```

### `workspace`

Manage workspaces:

```bash
gisa workspace list              # List configured workspaces
gisa workspace default [NAME]    # Set default workspace
gisa workspace default --clear   # Clear default workspace
```

### `reset`

Remove all config, workspaces, and cache:

```bash
gisa reset [-f | --force]
```

## TUI Mode

Running `gisa` without a subcommand launches the interactive terminal UI.

### Screens

| Screen | Purpose | Key bindings |
|--------|---------|-------------|
| **Dashboard** | Overview with stats, quick actions | `s`: Sync, `t`: Status, `w`: Workspaces, `?`: Settings |
| **Workspace Selector** | Pick active workspace | `[←] [↑] [↓] [→]`: Move, `Enter`: Select, `d`: Set default, `n`: New |
| **Init Check** | System requirements check | `Enter`: Check, `c`: Create config, `s`: Setup |
| **Setup Wizard** | Interactive workspace configuration | Step-by-step prompts |
| **Command Picker** | Choose operation to run | `Enter`: Run |
| **Progress** | Live sync progress with per-repo updates | `Esc`: Back when complete |
| **Repo Status** | Table of local repos with git status | `[←] [↑] [↓] [→]`: Move, `/`: Filter, `D`: Uncommitted, `B`: Behind, `r`: Refresh |
| **Org Browser** | Browse discovered repos by organization | `[←] [↑] [↓] [→]`: Move |
| **Settings** | View workspace settings | `Esc`: Back |

## Examples

### Sync all repositories in default workspace

```bash
gisa sync
```

### Sync with pull mode for a specific workspace

```bash
gisa sync --workspace work --pull
```

### Check which repositories have uncommitted changes

```bash
gisa status --uncommitted
```

### Dry run to see what would be synced

```bash
gisa sync --dry-run
```

## Development

### Building from source

```bash
git clone https://github.com/zaai-com/git-same
cd git-same

# Development build
cargo build

# Release build (optimized, stripped, with LTO)
cargo build --release
```

The binary is output to `target/release/git-same` (or `target/debug/git-same`). Alias symlinks are created by the install scripts, not by Cargo.

### Running tests

```bash
# Run all tests
cargo test

# Run with all features enabled
cargo test --all-features

# Run tests that require GitHub authentication
cargo test -- --ignored

# Run with verbose output
cargo test -- --nocapture
```

### Test file organization

Unit tests use colocated test files — each `foo.rs` has a companion `foo_tests.rs` in the same directory, linked via `#[path]` attribute. Integration tests live in `tests/`.

### Linting and formatting

```bash
# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

### Installing locally

```bash
# Install from source to ~/.cargo/bin/
cargo install --path .
```

This installs the `git-same` binary. To also create the alias symlinks (`gitsame`, `gitsa`, `gisa`), run `toolkit/Conductor/run.sh` or install via Homebrew. Make sure `~/.cargo/bin` is in your `$PATH`.

### Rebuilding

```bash
# Incremental rebuild
cargo build --release

# Clean rebuild
cargo clean && cargo build --release
```

### Uninstalling

```bash
# Remove binaries
cargo uninstall git-same

# Remove config and cache
rm -rf ~/.config/git-same/
rm -rf ~/.cache/git-same/
```

## License

MIT License - see [LICENSE](../LICENSE) for details

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/zaai-com/git-same).

## Roadmap

- [x] GitHub support
- [x] Parallel cloning
- [x] Smart filtering
- [x] Progress bars
- [x] Interactive TUI mode
- [x] Workspace management
- [ ] GitLab support
- [ ] Bitbucket support
- [ ] Repo groups
- [ ] Web dashboard
