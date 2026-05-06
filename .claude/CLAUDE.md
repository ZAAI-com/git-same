# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Author:** Manuel from Eggenfelden.

## Build & Test Commands

```bash
cargo build                        # Debug build
cargo build --release              # Optimized release build (LTO, stripped)
cargo test                         # Run all tests
cargo test <test_name>             # Run a single test by name
cargo test --test integration_test # Run only integration tests
cargo fmt -- --check               # Check formatting
cargo clippy -- -D warnings        # Lint (zero warnings enforced)
```

Logging is controlled via `GISA_LOG` env var (e.g., `GISA_LOG=debug cargo run -- sync`).

## Architecture

Git-Same is a Rust CLI + TUI tool that discovers GitHub org/repo structures and mirrors them locally with parallel cloning and syncing.

**Binary aliases:** `git-same`, `gitsame`, `gitsa`, `gisa` — all point to `src/main.rs`.

**Dual mode:** Running with a subcommand (`gisa sync`) uses the CLI path. Running without a subcommand (`gisa`) launches the interactive TUI.

**CLI flow:** CLI parsing (`src/cli.rs`) → `main.rs` routes to command handler → handler orchestrates modules.

**Commands:** `init`, `setup`, `sync`, `status`, `scan`, `workspace {list,default}`, `reset`.

### Core modules

- **`app/`** — Top-level entry points: `app/cli/` runs the CLI subcommand path, `app/tui/` boots the interactive TUI. `main.rs` dispatches to one or the other based on whether a subcommand was given
- **`commands/`** — Per-subcommand handlers (`init`, `setup`, `sync_cmd`, `status`, `scan`, `reset`, `workspace`) plus shared `support/` helpers
- **`workflows/`** — Cross-cutting orchestration shared by CLI and TUI: `sync_workspace` (discover + clone + fetch/pull) and `status_scan` (walk local repos, collect git status)
- **`auth/`** — `gh_cli.rs` obtains GitHub API tokens via `gh auth token`. `ssh.rs` exposes low-level SSH probing primitives (`SshProbeResult`, `parse_ssh_probe_output`) used by clone-time diagnostics
- **`config/`** — TOML config parser. Default: `~/.config/git-same/config.toml`. Top-level keys: `workspaces`, `default_workspace`, plus `[clone]` and `[filters]` sections
- **`discovery.rs`** — `DiscoveryOrchestrator` coordinates repo discovery via providers, applies filters, builds `ActionPlan` (what to clone vs sync)
- **`operations/clone.rs`** — `CloneManager` handles concurrent cloning (configurable 1–32, default 4)
- **`operations/sync.rs`** — `SyncManager` handles fetch/pull with concurrency. Detects repos with uncommitted changes and optionally skips them
- **`provider/`** — Trait-based provider abstraction (`Provider` trait in `traits.rs`). GitHub implementation in `github/client.rs` with pagination. Mock provider in `mock.rs` for testing
- **`git/`** — `GitOperations` trait (`traits.rs`) with `ShellGit` implementation (`shell.rs`) that shells out to `git` commands
- **`cache/`** — `discovery.rs` provides `DiscoveryCache` (TTL-based validity, persisted at `<workspace-root>/.git-same/cache.json`); `sync_history.rs` records sync runs at `<workspace-root>/.git-same/sync-history.json`
- **`domain/`** — Domain primitives, currently `repo_path_template.rs` for resolving `{org}/{repo}` style structures
- **`infra/storage/`** — Storage abstractions for workspace-local persistence
- **`setup/`** — Setup wizard state machine, shared between the CLI `setup` command and the TUI workspace-setup screen
- **`errors/`** — Custom error hierarchy: `AppError`, `GitError`, `ProviderError` with `suggested_action()` methods
- **`output/`** — `printer.rs` for verbosity-aware text output; `progress/` holds the `indicatif` progress bars (`CloneProgressBar`, `SyncProgressBar`, `DiscoveryProgressBar`)
- **`types/repo.rs`** — Core data types: `Repo`, `Org`, `ActionPlan`, `OpResult`, `OpSummary`
- **`checks.rs`** — System/runtime checks (presence of `git`, `gh`, auth status, SSH access via `check_ssh_github_access`)
- **`banner.rs`** — CLI banner rendering

### TUI module (`src/tui/`, feature-gated behind `tui`)

Elm architecture: `app.rs` = Model, `screens/` = View, `handler.rs` = Update.

- **`app.rs`** — `App` struct holds all TUI state. `Screen` enum: `WorkspaceSetup`, `Workspaces`, `Dashboard`, `Sync`, `Settings`
- **`handler.rs`** — Keyboard input handlers per screen + `handle_backend_message` for async results
- **`backend.rs`** — Spawns Tokio tasks for async operations (sync, status scan), sends `BackendMessage` variants via unbounded channels
- **`event.rs`** — `AppEvent` (terminal input, backend messages, ticks) and `BackendMessage` enum
- **`screens/`** — Stateless render functions per screen (dashboard, workspace, settings, etc.)
- **`widgets/`** — Shared widgets (status bar, spinner)

### Key patterns

- **Trait-based abstractions:** `GitOperations`, `Provider`, progress traits — enables mocking in tests
- **Concurrency:** Tokio tasks with `Arc<dyn Trait>` for sharing progress reporters across tasks
- **Error handling:** `thiserror` for typed errors + `anyhow` for propagation. Custom `Result` type alias in `errors/`
- **Channel-based TUI updates:** Backend operations send `BackendMessage` through `mpsc::UnboundedSender<AppEvent>`, processed by the TUI event loop
- **Arrow-only navigation:** All directional movement uses arrow keys only (`←` `↑` `↓` `→`). No vim-style `j`/`k`/`h`/`l` letter navigation. Display hints use `[←] [↑] [↓] [→] Move`.

## Formatting

`rustfmt.toml`: `max_width = 100`, `tab_spaces = 4`, edition 2021.

## Testing

**Convention:** Colocated test files using `#[path]` attribute. Every source file `foo.rs` has a companion `foo_tests.rs` in the same directory.

In the source file:
```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

The test file contains `use super::*;` and all `#[test]` / `#[tokio::test]` functions.

**Do not** write inline `#[cfg(test)] mod tests { ... }` blocks — always use separate `_tests.rs` files.

**Integration tests** remain in `tests/integration_test.rs`.

## CI/CD Workflows

All workflows are `workflow_dispatch` (manual trigger) in `.github/workflows/`:

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `S1-Test-CI.yml` | fmt, clippy, test, build dry-run, coverage, audit | Manual dispatch |
| `S2-Release-GitHub.yml` | Full CI + cross-compile 6 targets + GitHub Release | Manual dispatch (select tag) |
| `S3-Publish-Homebrew.yml` | Update Homebrew tap formula | Manual dispatch (select tag) |
| `S4-Publish-Crates.yml` | `cargo publish` to crates.io | Manual dispatch (select tag) |

S2 runs all S1 jobs (test, coverage, audit) as gates before building release artifacts.

## Specs & Docs

End-user docs in `docs/README.md`. Contributor/build-from-source docs in `docs/DEVELOPMENT.md`. Sync-screen design notes in `docs/Sync-Screen.md`. In-flight design plans live under `docs/plans/` and the workspace-local `.context/plans/`.
