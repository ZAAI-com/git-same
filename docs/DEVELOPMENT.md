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

# Development build
cargo build

# Release build (optimized, stripped, with LTO)
cargo build --release
```

The binary is output to `target/release/git-same` (or `target/debug/git-same`). Alias symlinks are created by the install scripts, not by Cargo.

## Running tests

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

## Test file organization

Unit tests use colocated test files. Each `foo.rs` has a companion `foo_tests.rs` in the same directory, linked via `#[path]` attribute. Integration tests live in `tests/`.

## Linting and formatting

```bash
# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Installing locally

```bash
# Install from source to ~/.cargo/bin/
cargo install --path .
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
