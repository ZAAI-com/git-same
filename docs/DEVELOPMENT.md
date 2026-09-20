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

[monitor]
# Start the background monitor automatically (macOS). `gisa monitor --stop`
# sets this to false and `gisa monitor --start` back to true; edit it through
# those commands rather than by hand.
autostart = true
# Seconds between full rescans. `gisa monitor --interval N` overrides it for
# a foreground run. A running monitor picks up changes without a restart.
fullscan_interval_secs = 30
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

The repository is a Cargo workspace with three member crates: `git-same-core` (engine library, `crates/git-same-core/`), `git-same` (the CLI binary + TUI, `crates/git-same-cli/` on disk), and `git-same-app` (the Tauri desktop app, `crates/git-same-app/`). The release binary is output at the workspace level: `target/release/git-same` (or `target/debug/git-same`). Alias symlinks are created by the install scripts, not by Cargo.

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

## Developing without touching your own monitor

Routine tests can never reach launchd: the lifecycle controller runs against a scripted fake, explicit controls refuse a redirected `HOME`, `GIT_SAME_CONFIG_DIR`, or `--config`, CLI integration tests set `GIT_SAME_DISABLE_MONITOR_AUTOSTART=1`, and automatic hooks live only in the command dispatcher, which no unit test calls. Automatic recovery also refuses to install a binary from a cargo `target/` directory.

`toolkit/conductor/run.sh` exports `GIT_SAME_DISABLE_MONITOR_AUTOSTART=1`, so the dev app never manages your real monitor. To exercise the lifecycle deliberately, set `GIT_SAME_DEV_ALLOW_MONITOR_AUTOSTART=1` or run `gisa monitor --start` from the dev build; `archive.sh` then removes only a monitor that was installed from that worktree. Real launchd and Homebrew scenarios belong in a disposable macOS account or VM (see `toolkit/packaging/release-checklist.md`, section 8).

## Uninstalling

On macOS, remove the background monitor first. Package managers do not do it for you: `cargo uninstall` (or `brew uninstall git-same-cli`) only deletes the CLI, while the monitor is an independent helper copy that keeps running.

```bash
# macOS: stop and remove the background monitor (keeps repos, config, logs)
gisa monitor --uninstall

# Remove binaries
cargo uninstall git-same

# Remove config and cache
rm -rf ~/.config/git-same/

# Workspace-local cache/history live under each workspace:
# <workspace-root>/.git-same/cache.json
# <workspace-root>/.git-same/sync-history.json
```
