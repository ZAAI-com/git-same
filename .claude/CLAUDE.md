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

**Commands:** `init`, `setup`, `sync`, `status`, `workspace {list,default}`, `reset`.

### Core modules

- **`auth/`** — GitHub CLI (`gh`) authentication only (`gh auth token`), with SSH clone support
- **`config/`** — TOML config parser. Default location: `~/.config/git-same/config.toml`. Sections: `[clone]`, `[filters]`, `[[providers]]`
- **`discovery/`** — `DiscoveryOrchestrator` coordinates repo discovery via providers, applies filters, builds `ActionPlan` (what to clone vs sync)
- **`operations/clone/`** — `CloneManager` handles concurrent cloning (configurable 1–32, default 4)
- **`operations/sync/`** — `SyncManager` handles fetch/pull with concurrency. Detects repos with uncommitted changes and optionally skips them
- **`provider/`** — Trait-based provider abstraction (`Provider` trait in `traits.rs`). GitHub implementation in `github/client.rs` with pagination. Mock provider in `mock.rs` for testing
- **`git/`** — `GitOperations` trait (`traits.rs`) with `ShellGit` implementation (`shell.rs`) that shells out to `git` commands
- **`cache/`** — `DiscoveryCache` with TTL-based validity at `~/.cache/git-same/`
- **`errors/`** — Custom error hierarchy: `AppError`, `GitError`, `ProviderError` with `suggested_action()` methods
- **`output/`** — Verbosity levels and `indicatif` progress bars (`CloneProgressBar`, `SyncProgressBar`, `DiscoveryProgressBar`)
- **`types/repo.rs`** — Core data types: `Repo`, `Org`, `ActionPlan`, `OpResult`, `OpSummary`

### TUI module (`src/tui/`, feature-gated behind `tui`)

Elm architecture: `app.rs` = Model, `screens/` = View, `handler.rs` = Update.

- **`app.rs`** — `App` struct holds all TUI state. `Screen` enum: `InitCheck`, `SetupWizard`, `Workspace`, `Dashboard`, `Progress`, `Settings`
- **`handler.rs`** — Keyboard input handlers per screen + `handle_backend_message` for async results
- **`backend.rs`** — Spawns Tokio tasks for async operations (sync, status scan), sends `BackendMessage` variants via unbounded channels
- **`event.rs`** — `AppEvent` (terminal input, backend messages, ticks) and `BackendMessage` enum
- **`screens/`** — Stateless render functions per screen (dashboard, workspace, settings, etc.)
- **`widgets/`** — Shared widgets (status bar, spinner)
- **`setup/`** — Setup wizard state machine (shared between CLI `setup` command and TUI `SetupWizard` screen)

### Recent workspace-screen updates

- Sidebar navigation now supports `←`/`→` in addition to `↑`/`↓` and `Tab` (see `handle_workspace_key`).
- Workspace actions changed: `[f]` opens workspace folder; `[o]` is no longer bound.
- Default workspace behavior is now set-only from `[d]`: pressing `[d]` on the current default does not clear it.
- Workspace detail view is grouped into sections (`Identity`, `Paths`, `Sync`, `Account`) with wrapped org rendering for narrow widths.
- Last sync display now shows relative time and, when parseable RFC3339 exists, an absolute timestamp line.
- Added focused tests in `src/tui/handler.rs` and `src/tui/screens/workspace.rs` for key handling and workspace detail formatting helpers.

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

Design specifications live in `docs/specs/` (S1–S5). Internal documentation in `.context/GIT-SAME-DOCUMENTATION.md`.
