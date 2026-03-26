# Git-Same

Mirror your GitHub org structure to the local filesystem — parallel clone, incremental sync, TUI dashboard.

[![Crates.io](https://img.shields.io/crates/v/git-same.svg)](https://crates.io/crates/git-same)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/zaai-com/git-same/actions/workflows/S1-Test-CI.yml/badge.svg)](https://github.com/zaai-com/git-same/actions/workflows/S1-Test-CI.yml)
[![Homebrew](https://img.shields.io/badge/homebrew-zaai--com%2Ftap-blue)](https://github.com/zaai-com/homebrew-tap)

## What It Does

```
                GitHub                                           Local
┌─────────────────────────────────────────┐       ┌─────────────────────────────────────────┐
│  github.com/zaai-com/git-same           │ ───── │  ~/GitHub/zaai-com/git-same/            │
│  github.com/zaai-com/GreenHub           │ ───── │  ~/GitHub/zaai-com/GreenHub/            │
│  github.com/zaai-com/ZAAI-AgentBox      │ ───── │  ~/GitHub/zaai-com/ZAAI-AgentBox/       │
│  github.com/zaai-com/MyDomains          │ ───── │  ~/GitHub/zaai-com/MyDomains/           │
│                                         │       │                                         │
│  github.com/zaai-agents/...             │ ───── │  ~/GitHub/zaai-agents/.../              │
│                                         │       │                                         │
│  github.com/Manuel-Forks/chatml         │ ───── │  ~/GitHub/Manuel-Forks/chatml/          │
│  github.com/Manuel-Forks/netfluss       │ ───── │  ~/GitHub/Manuel-Forks/netfluss/        │
│  github.com/Manuel-Forks/gstack         │ ───── │  ~/GitHub/Manuel-Forks/gstack/          │
└─────────────────────────────────────────┘       └─────────────────────────────────────────┘

                  git-same sync  ←  one command, parallel, incremental
```

One command discovers every repo across your GitHub orgs and mirrors them locally — cloning new repos in parallel, fetching updates for existing ones, and skipping repos with uncommitted changes.

## Screenshots

<!-- TODO: Add screenshots — see docs/assets/ for filenames -->

| | |
|---|---|
| ![TUI Dashboard](assets/tui-dashboard.png) | ![TUI Sync Progress](assets/tui-sync-progress.png) |
| TUI Dashboard — stats, repo table, quick actions | Sync Progress — live progress, worker slots, throughput |

![CLI Sync](assets/cli-sync.png)

## Quick Start

### Interactive (TUI)

```bash
git-same
```

Launches the full terminal UI with dashboard, sync, status, and workspace management — all via keyboard shortcuts.

### CLI

```bash
git-same init          # 1. Create config
git-same setup         # 2. Configure workspace (interactive wizard)
git-same sync          # 3. Clone new repos, fetch/pull existing
git-same status        # 4. Check repo status across orgs
```

## Installation

### Homebrew

```bash
brew install zaai-com/tap/git-same
```

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

## Aliases

Git-Same installs multiple binary names so you can use whichever you prefer:

| Command    | Description                                    |
|------------|------------------------------------------------|
| `git-same` | Primary binary (always available)              |
| `gitsame`  | No-hyphen alias (symlink)                      |
| `gitsa`    | Short alias (symlink)                          |
| `gisa`     | Shortest alias (symlink)                       |
| `git same` | Git subcommand (requires git-same in PATH)     |

> **Install method differences:** Homebrew (`brew install zaai-com/tap/git-same`) installs all aliases automatically. `cargo install git-same` installs only the primary binary. The canonical alias list lives in `toolkit/packaging/binary-aliases.txt`.

All examples in this README use `git-same`, but any alias works interchangeably.

## Commands

| Command | Description |
|---------|-------------|
| `git-same init` | Create config file with sensible defaults |
| `git-same setup` | Interactive wizard to configure a workspace |
| `git-same sync` | Discover, clone new, fetch/pull existing repos |
| `git-same status` | Show git status across all local repos |
| `git-same workspace` | List workspaces, set default |
| `git-same reset` | Remove all config, workspaces, and cache |
| `git-same scan` | Discover repos without cloning or syncing |

### `init`

Initialize git-same configuration:

```bash
git-same init [-p <config-path>] [-f | --force]
```

Creates a config file at `~/.config/git-same/config.toml` with sensible defaults.

### `setup`

Configure a workspace (interactive wizard):

```bash
git-same setup [--name <NAME>]
```

Walks through provider selection, authentication, org filters, and base path.

### `sync`

Sync repositories — discover, clone new, fetch/pull existing:

```bash
git-same sync [OPTIONS]

Options:
  -w, --workspace <WORKSPACE> Workspace to sync (path or unique folder name)
      --pull                  Use pull instead of fetch for existing repos
  -n, --dry-run               Show what would be done
  -c, --concurrency <N>       Number of parallel operations (1-32)
      --refresh               Force re-discovery (ignore cache)
      --no-skip-uncommitted         Don't skip repos with uncommitted changes
```

### `status`

Show status of local repositories:

```bash
git-same status [OPTIONS]

Options:
  -w, --workspace <WORKSPACE> Workspace to check (path or unique folder name)
  -o, --org <ORG>...          Filter by organization (repeatable)
  -d, --uncommitted                 Show only repositories with uncommitted changes
  -b, --behind                Show only repositories behind upstream
      --detailed              Show detailed status information
```

### `workspace`

Manage workspaces:

```bash
git-same workspace list              # List configured workspaces
git-same workspace default [WORKSPACE] # Set default workspace (path or unique folder name)
git-same workspace default --clear   # Clear default workspace
```

### `reset`

Remove all config, workspaces, and cache:

```bash
git-same reset [-f | --force]
```

## TUI Mode

Running `git-same` without a subcommand launches the interactive terminal UI.

<!-- TODO: Add TUI workspace screenshot -->
<!-- ![TUI Workspaces](assets/tui-workspaces.png) -->

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

## Authentication

Git-Same uses GitHub CLI (`gh`) for authentication:

```bash
# Install GitHub CLI
brew install gh  # macOS
# or: sudo apt install gh  # Ubuntu

# Authenticate
gh auth login

# Git-Same will now use your gh credentials
git-same sync
```

For GitHub Enterprise, configure the workspace provider:

```toml
[provider]
kind = "github-enterprise"
api_url = "https://github.company.com/api/v3"
prefer_ssh = true
```

Authenticate GitHub Enterprise once with:

```bash
gh auth login --hostname github.company.com
```

## Configuration

Global behavior is configured in `~/.config/git-same/config.toml`:

```toml
# Directory structure: {org}/{repo} or {provider}/{org}/{repo}
structure = "{org}/{repo}"

# Number of concurrent clone/sync operations
concurrency = 4

# Default sync mode: fetch or pull
sync_mode = "fetch"

# Optional default workspace root path
# default_workspace = "~/Git-Same/GitHub"

# Registered workspace root paths
# workspaces = ["~/Git-Same/GitHub"]

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
```

Provider and workspace-specific settings are stored inside each workspace at
`<workspace-root>/.git-same/config.toml`:

```toml
username = "my-user"
orgs = ["my-org"]

[provider]
kind = "github"
prefer_ssh = true
```

## Examples

### Sync all repositories in default workspace

```bash
git-same sync
```

### Sync with pull mode for a specific workspace

```bash
git-same sync --workspace work --pull
```

### Check which repositories have uncommitted changes

```bash
git-same status --uncommitted
```

### Dry run to see what would be synced

```bash
git-same sync --dry-run
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

This installs the `git-same` binary. Install via Homebrew to get all aliases automatically. Make sure `~/.cargo/bin` is in your `$PATH`.

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

# Workspace-local cache/history live under each workspace:
# <workspace-root>/.git-same/cache.json
# <workspace-root>/.git-same/sync-history.json
```

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/zaai-com/git-same).

## License

MIT License - see [LICENSE](../LICENSE) for details

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

---

## Diagram Option A — Flat rows with tree

```
GitHub                                                       <<== Sync ==>>  /users/m/engineering/same-github/
                                                                             │
https://github.com/manuelgruber                              <<==  Org ==>>  ├── manuelgruber/
https://github.com/manuelgruber/ManuelGruber                 <<== Repo ==>>  │   ├── ManuelGruber/
https://github.com/manuelgruber/manuelgruber.github.io       <<== Repo ==>>  │   ├── manuelgruber.github.io/
https://github.com/manuelgruber/dotfiles                     <<== Repo ==>>  │   └── dotfiles/
                                                                             │
https://github.com/ZAAI-com                                  <<==  Org ==>>  ├── ZAAI-com/
https://github.com/ZAAI-com/PowerNight                       <<== Repo ==>>  │   ├── PowerNight/
https://github.com/ZAAI-com/Clean-Autofill                   <<== Repo ==>>  │   ├── Clean-Autofill/
https://github.com/ZAAI-com/git-same                         <<== Repo ==>>  │   ├── git-same/
https://github.com/ZAAI-com/Jekyll-AEO                       <<== Repo ==>>  │   └── Jekyll-AEO/
                                                                             │
https://github.com/company1                                  <<==  Org ==>>  └── company1/
https://github.com/company1/example.ai                       <<== Repo ==>>      └── example.ai/

3 orgs · 10 repos                                                            3 dirs · 10 repos · all in sync
```

## Diagram Option B — Bordered table

```
┌──────────────────────────────────────────────────────────┬────────────────┬────────────────────────────────┐
│ GitHub                                                   │ <<== Sync ==>> │ /users/m/.../same-github/      │
├──────────────────────────────────────────────────────────┼────────────────┼────────────────────────────────┤
│ https://github.com/manuelgruber                          │ <<==  Org ==>> │ manuelgruber/                  │
│   /manuelgruber/ManuelGruber                             │ <<== Repo ==>> │   ManuelGruber/                │
│   /manuelgruber/manuelgruber.github.io                   │ <<== Repo ==>> │   manuelgruber.github.io/      │
│   /manuelgruber/dotfiles                                 │ <<== Repo ==>> │   dotfiles/                    │
├──────────────────────────────────────────────────────────┼────────────────┼────────────────────────────────┤
│ https://github.com/ZAAI-com                              │ <<==  Org ==>> │ ZAAI-com/                      │
│   /ZAAI-com/PowerNight                                   │ <<== Repo ==>> │   PowerNight/                  │
│   /ZAAI-com/Clean-Autofill                               │ <<== Repo ==>> │   Clean-Autofill/              │
│   /ZAAI-com/git-same                                     │ <<== Repo ==>> │   git-same/                    │
│   /ZAAI-com/Jekyll-AEO                                   │ <<== Repo ==>> │   Jekyll-AEO/                  │
├──────────────────────────────────────────────────────────┼────────────────┼────────────────────────────────┤
│ https://github.com/company1                              │ <<==  Org ==>> │ company1/                      │
│   /company1/example.ai                                   │ <<== Repo ==>> │   example.ai/                  │
├──────────────────────────────────────────────────────────┼────────────────┼────────────────────────────────┤
│ 3 orgs · 10 repos                                        │                │ 3 dirs · 10 repos              │
└──────────────────────────────────────────────────────────┴────────────────┴────────────────────────────────┘
```

## Diagram Option C — Double-line panels

```
╔══════════════════════════════════════════════════════════╗                ╔════════════════════════════════╗
║  GITHUB                                                  ║                ║  LOCAL FILESYSTEM              ║
║                                                          ║  <<== Sync ==>>║  /users/m/.../same-github/     ║
╠══════════════════════════════════════════════════════════╣                ╠════════════════════════════════╣
║                                                          ║                ║                                ║
║  https://github.com/manuelgruber                         ║  <<==  Org ==>>║  manuelgruber/                 ║
║    ├── /ManuelGruber                                     ║  <<== Repo ==>>║    ├── ManuelGruber/           ║
║    ├── /manuelgruber.github.io                           ║  <<== Repo ==>>║    ├── manuelgruber.github.io/ ║
║    └── /dotfiles                                         ║  <<== Repo ==>>║    └── dotfiles/               ║
║                                                          ║                ║                                ║
║  https://github.com/ZAAI-com                             ║  <<==  Org ==>>║  ZAAI-com/                     ║
║    ├── /PowerNight                                       ║  <<== Repo ==>>║    ├── PowerNight/             ║
║    ├── /Clean-Autofill                                   ║  <<== Repo ==>>║    ├── Clean-Autofill/         ║
║    ├── /git-same                                         ║  <<== Repo ==>>║    ├── git-same/               ║
║    └── /Jekyll-AEO                                       ║  <<== Repo ==>>║    └── Jekyll-AEO/             ║
║                                                          ║                ║                                ║
║  https://github.com/company1                             ║  <<==  Org ==>>║  company1/                     ║
║    └── /example.ai                                       ║  <<== Repo ==>>║    └── example.ai/             ║
║                                                          ║                ║                                ║
║  3 orgs · 10 repos                                       ║                ║  3 dirs · 10 repos · in sync   ║
╚══════════════════════════════════════════════════════════╝                ╚════════════════════════════════╝
```
