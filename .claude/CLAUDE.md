# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                        # Debug build
cargo build --release              # Optimized release build (LTO, stripped)
cargo test                         # Run all tests (207 unit + 16 integration + 8 doc)
cargo test <test_name>             # Run a single test by name
cargo test --test integration_test # Run only integration tests
cargo fmt -- --check               # Check formatting
cargo clippy -- -D warnings        # Lint (zero warnings enforced)
```

Logging is controlled via `GISA_LOG` env var (e.g., `GISA_LOG=debug cargo run -- clone`).

## Architecture

Git-Same is a Rust CLI that discovers GitHub org/repo structures and mirrors them locally with parallel cloning and syncing.

**Binary aliases:** `git-same`, `gitsame`, `gitsa`, `gisa` — all point to `src/main.rs`.

**Command flow:** CLI parsing (`src/cli.rs`) → `main.rs` routes to command handler → handler orchestrates modules.

### Core modules

- **`auth/`** — Multi-strategy auth: GitHub CLI (`gh`) → env token (`GITHUB_TOKEN`) → config token, with SSH support
- **`config/`** — TOML config parser. Default location: `~/.config/git-same/config.toml`. Sections: `[clone]`, `[filters]`, `[[providers]]`
- **`discovery/`** — `DiscoveryOrchestrator` coordinates repo discovery via providers, applies filters, builds `ActionPlan` (what to clone vs sync)
- **`clone/parallel.rs`** — `CloneManager` handles concurrent cloning (configurable 1–32, default 4)
- **`sync/manager.rs`** — `SyncManager` handles fetch/pull with concurrency. Detects dirty repos and optionally skips them
- **`provider/`** — Trait-based provider abstraction (`Provider` trait in `traits.rs`). GitHub implementation in `github/client.rs` with pagination. Mock provider in `mock.rs` for testing
- **`git/`** — `GitOperations` trait (`traits.rs`) with `ShellGit` implementation (`shell.rs`) that shells out to `git` commands
- **`cache/`** — `DiscoveryCache` with TTL-based validity at `~/.cache/git-same/`
- **`errors/`** — Custom error hierarchy: `AppError`, `GitError`, `ProviderError` with `suggested_action()` methods
- **`output/`** — Verbosity levels and `indicatif` progress bars (`CloneProgressBar`, `SyncProgressBar`, `DiscoveryProgressBar`)
- **`types/repo.rs`** — Core data types: `Repo`, `Org`, `ActionPlan`, `OpResult`, `OpSummary`

### Key patterns

- **Trait-based abstractions:** `GitOperations`, `Provider`, progress traits — enables mocking in tests
- **Concurrency:** Tokio tasks with `Arc<dyn Trait>` for sharing progress reporters across tasks
- **Error handling:** `thiserror` for typed errors + `anyhow` for propagation. Custom `Result` type alias in `errors/`

## Formatting

`rustfmt.toml`: `max_width = 100`, `tab_spaces = 4`, edition 2021.

## Specs & Docs

Design specifications live in `docs/specs/` (S1–S5). Internal documentation in `.context/GIT-SAME-DOCUMENTATION.md`.