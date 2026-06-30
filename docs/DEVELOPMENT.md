# Development & Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/zaai-com/git-same).

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

## Building from source

```bash
git clone https://github.com/zaai-com/git-same
cd git-same

# Development build (whole workspace)
cargo build --workspace

# Release build (optimized, stripped, with LTO)
cargo build --release --workspace
```

The repository is a Cargo workspace with two member crates: `git-same-core` (engine library, `crates/git-same-core/`) and `git-same` (the CLI binary + TUI, `crates/git-same-cli/` on disk). The release binary is output at the workspace level: `target/release/git-same` (or `target/debug/git-same`). Alias symlinks are created by the install scripts, not by Cargo.

## Running the macOS App in development

The Tauri-based desktop app lives at `crates/git-same-app/`. You need [pnpm](https://pnpm.io/) and the [`tauri-cli`](https://v2.tauri.app/reference/cli/) (`cargo install tauri-cli --version "^2.0"`).

```bash
# Install frontend dependencies
pnpm --dir crates/git-same-app/ui install

# Start the dev server (Vite + Rust backend with hot reload)
cargo tauri dev --manifest-path crates/git-same-app/Cargo.toml
```

The window opens with the workspace dashboard, reading from `~/.config/git-same/config.toml`. The app subscribes to the monitor's `status.json`, so updates from `git-same sync` (run in another terminal) appear live.

## Running tests

```bash
# Run all tests across the workspace
cargo test --workspace

# Run with all features enabled
cargo test --workspace --all-features

# Run tests for a single crate
cargo test -p git-same-core
cargo test -p git-same

# Run tests that require GitHub authentication
cargo test --workspace -- --ignored

# Run with verbose output
cargo test --workspace -- --nocapture
```

## Test file organization

Unit tests use colocated test files. Each `foo.rs` has a companion `foo_tests.rs` in the same directory, linked via `#[path]` attribute. Integration tests live in `crates/git-same-cli/tests/`.

## Linting and formatting

```bash
# Lint the whole workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Installing locally

```bash
# Install the CLI from source to ~/.cargo/bin/
cargo install --path crates/git-same-cli
```

This installs the `git-same` binary. Install via Homebrew to get all aliases automatically. Make sure `~/.cargo/bin` is in your `$PATH`.

## Rebuilding

```bash
# Incremental rebuild
cargo build --release

# Clean rebuild
cargo clean && cargo build --release
```

## Uninstalling

```bash
# Remove binaries
cargo uninstall git-same

# Remove config and cache
rm -rf ~/.config/git-same/

# Workspace-local cache/history live under each workspace:
# <workspace-root>/.git-same/cache.json
# <workspace-root>/.git-same/sync-history.json
```
