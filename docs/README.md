# Git-Same

Mirror GitHub org/repo structure locally - supports multiple providers

[![Crates.io](https://img.shields.io/crates/v/git-same.svg)](https://crates.io/crates/git-same)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/zaai-com/git-same/workflows/CI/badge.svg)](https://github.com/zaai-com/git-same/actions)

## Features

- **Multi-Provider Support**: Works with GitHub, GitHub Enterprise, GitLab, and Bitbucket
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

### From source

```bash
git clone https://github.com/zaai-com/git-same
cd git-same
cargo install --path .
```

### Homebrew (coming soon)

```bash
brew install git-same
```

## Available Commands

The tool can be invoked using any of these names (all installed by default):

- `git-same` - Main command
- `gitsame` - No hyphen variant
- `gitsa` - Short form
- `gisa` - Shortest variant
- `git same` - Git subcommand (requires git-same in PATH)

## Quick Start

### 1. Initialize configuration

```bash
git-same init
```

This creates a config file at `~/.config/git-same/config.toml` with sensible defaults.

### 2. Clone all repositories

```bash
# Dry run first to see what would be cloned
git-same clone ~/github --dry-run

# Clone for real
git-same clone ~/github
```

### 3. Keep repositories in sync

```bash
# Fetch updates (doesn't modify working tree)
git-same fetch ~/github

# Pull updates (modifies working tree)
git-same pull ~/github
```

### 4. Check repository status

```bash
# Show status of all repositories
git-same status ~/github

# Show only dirty repositories
git-same status ~/github --dirty

# Show only repositories behind upstream
git-same status ~/github --behind
```

## Authentication

Git-Same uses GitHub CLI (`gh`) for authentication by default:

```bash
# Install GitHub CLI
brew install gh  # macOS
# or: sudo apt install gh  # Ubuntu

# Authenticate
gh auth login

# Git-Same will now use your gh credentials
git-same clone ~/github
```

Alternatively, use a personal access token:

```bash
export GITHUB_TOKEN=ghp_your_token_here
git-same clone ~/github
```

## Configuration

Edit `~/.config/git-same/config.toml` to customize behavior:

```toml
# Base directory for cloning (can be overridden per-provider)
base_path = "~/code"

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
auth = "env"
token_env = "WORK_GITHUB_TOKEN"
prefer_ssh = true
enabled = true
base_path = "~/work/code"
```

## Commands

### `init`

Initialize git-same configuration:

```bash
git-same init [--path <config-path>] [--force]
```

### `clone`

Clone all discovered repositories:

```bash
git-same clone <base-path> [OPTIONS]

Options:
  --org <ORG>...              Filter by organization
  --include-archived          Include archived repositories
  --include-forks             Include forked repositories
  --dry-run                   Show what would be cloned
  --concurrency <N>           Number of parallel clones
  --depth <N>                 Clone depth (0 = full)
  --branch <BRANCH>           Clone specific branch
  --recurse-submodules        Clone submodules recursively
  --https                     Use HTTPS instead of SSH
  --no-cache                  Skip cache, always discover
  --refresh                   Force refresh from API
```

### `fetch`

Fetch updates for all repositories:

```bash
git-same fetch <base-path> [OPTIONS]

Options:
  --org <ORG>...              Filter by organization
  --skip-dirty                Skip repositories with uncommitted changes
  --dry-run                   Show what would be fetched
  --concurrency <N>           Number of parallel fetches
```

### `pull`

Pull updates for all repositories:

```bash
git-same pull <base-path> [OPTIONS]

Options:
  --org <ORG>...              Filter by organization
  --skip-dirty                Skip repositories with uncommitted changes
  --dry-run                   Show what would be pulled
  --concurrency <N>           Number of parallel pulls
```

### `status`

Show status of local repositories:

```bash
git-same status <base-path> [OPTIONS]

Options:
  --org <ORG>...              Filter by organization
  --dirty                     Show only dirty repositories
  --behind                    Show only repositories behind upstream
  --detailed                  Show detailed status information
```

### `completions`

Generate shell completions:

```bash
git-same completions <SHELL>

Shells: bash, zsh, fish, powershell, elvish
```

#### Installation

**Bash:**
```bash
git-same completions bash > ~/.local/share/bash-completion/completions/git-same
```

**Zsh:**
```bash
git-same completions zsh > ~/.zfunc/_git-same
```

**Fish:**
```bash
git-same completions fish > ~/.config/fish/completions/git-same.fish
```

## Examples

### Clone all repositories from specific orgs

```bash
git-same clone ~/github --org octocat --org github
```

### Clone with shallow depth for faster initial clone

```bash
git-same clone ~/github --depth 1
```

### Fetch updates for specific organization

```bash
git-same fetch ~/github --org mycompany
```

### Check which repositories have uncommitted changes

```bash
git-same status ~/github --dirty
```

### Use HTTPS instead of SSH

```bash
git-same clone ~/github --https
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

Binaries are output to `target/release/` (or `target/debug/`): `git-same`, `gitsame`, `gitsa`, `gisa`.

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

This installs all 4 binary aliases (`git-same`, `gitsame`, `gitsa`, `gisa`). Make sure `~/.cargo/bin` is in your `$PATH`.

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

MIT License - see [LICENSE](LICENSE) for details

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/zaai-com/git-same).

## Roadmap

- [x] GitHub support
- [x] Parallel cloning
- [x] Smart filtering
- [x] Progress bars
- [x] Shell completions
- [ ] GitLab support
- [ ] Bitbucket support
- [ ] Interactive mode
- [ ] Repo groups
- [ ] Web dashboard
