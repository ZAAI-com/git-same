# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Author:** Manuel from Eggenfelden.

## Build & Test Commands

```bash
cargo build --workspace                                # Debug build
cargo build --release --workspace                      # Optimized release (LTO, stripped)
cargo test --workspace                                 # Run all tests
cargo test -p git-same-core                            # Tests for the engine crate only
cargo test -p git-same                                 # Tests for the CLI crate only
cargo test --workspace <test_name>                     # Run a single test by name
cargo test -p git-same --test integration_test         # Run only integration tests
cargo fmt --all -- --check                             # Check formatting
cargo clippy --workspace --all-targets --all-features -- -D warnings   # Lint
```

Logging is controlled via `GISA_LOG` env var (e.g., `GISA_LOG=debug cargo run -p git-same -- sync`).

## Architecture

Git-Same is a Rust CLI + TUI tool that discovers GitHub org/repo structures and mirrors them locally with parallel cloning and syncing.

**Workspace layout:** the project is a Cargo workspace with two member crates:

- `git-same-core` (`crates/git-same-core/`): the engine library. No UI dependencies (no clap, ratatui, crossterm). Holds discovery, clone/sync, IPC, status scanning, and shared types.
- `git-same` (lives at `crates/git-same-cli/` on disk; the directory name and package name intentionally diverge so `cargo install git-same` keeps working as it has since pre-3.x): the CLI binary + TUI. Depends on `git-same-core`. Owns clap parsing, the TUI screens, the setup wizard, and command handlers. The produced binary is named `git-same` (per `[[bin]]` name) so installer aliases (`gisa`, `gitsa`, `gitsame`) and `target/release/git-same` are unchanged from the pre-split layout.

**Binary aliases:** `git-same`, `gitsame`, `gitsa`, `gisa`: all resolve to the binary built from `crates/git-same-cli/src/main.rs`.

**Dual mode:** Running with a subcommand (`gisa sync`) uses the CLI path. Running without a subcommand (`gisa`) launches the interactive TUI.

**macOS host strategy:** The Tauri host (`crates/git-same-app/`, Svelte + TypeScript + Vite) is the sole macOS GUI. It is built and notarized in S2 and ships via the cask. The earlier SwiftUI host (`macos/GitSameSwiftApp/`) was removed in Phase C (commit `7fd2ae0`) once the Tauri scaffold took over; do not resurrect it without explicit approval. The only remaining macOS Swift target is the FinderSync badge extension at `macos/GitSameBadges/`.

**CLI flow:** CLI parsing (`crates/git-same-cli/src/cli.rs`) → `main.rs` routes to command handler → handler orchestrates engine modules from `git-same-core`.

**Commands:** `init`, `setup`, `sync`, `status`, `scan`, `workspace {list,default}`, `reset`, `monitor` (alias: `daemon`), `refresh`.

**Why `monitor` is a CLI subcommand and not solely a Tauri-host responsibility:** the LaunchAgent invokes `gisa monitor --foreground`, non-cask installs (`cargo install`, the homebrew formula) ship only the binary, `--status` / `--stop` are the supported debugging surface, and a future Linux file-manager extension would talk to the same `gisa monitor` over the same Unix socket. The CLI handler is a thin shim (~140 lines); the loop itself lives in `git-same-core::monitor`.

### Engine modules (`crates/git-same-core/src/`)

- **`auth/`**: `gh_cli.rs` obtains GitHub API tokens via `gh auth token`. `ssh.rs` exposes low-level SSH probing primitives (`SshProbeResult`, `parse_ssh_probe_output`) used by clone-time diagnostics
- **`workflows/`**: Cross-cutting orchestration: `sync_workspace` (discover + clone + fetch/pull) and `status_scan` (walk local repos, collect git status)
- **`monitor/`**: Long-running monitor loop (periodic scan + Unix-socket server) used by `gisa monitor` and reusable by host apps like the Tauri GUI
- **`config/`**: TOML config parser. Default: `~/.config/git-same/config.toml`. Top-level keys: `workspaces`, `default_workspace`, plus `[clone]` and `[filters]` sections
- **`discovery.rs`**: `DiscoveryOrchestrator` coordinates repo discovery via providers, applies filters, builds `ActionPlan` (what to clone vs sync)
- **`operations/clone.rs`**: `CloneManager` handles concurrent cloning (configurable 1–32, default 4)
- **`operations/sync.rs`**: `SyncManager` handles fetch/pull with concurrency. Detects repos with uncommitted changes and optionally skips them
- **`provider/`**: Trait-based provider abstraction (`Provider` trait in `traits.rs`). GitHub implementation in `github/client.rs` with pagination. Mock provider in `mock.rs` for testing
- **`git/`**: `GitOperations` trait (`traits.rs`) with `ShellGit` implementation (`shell.rs`) that shells out to `git` commands
- **`cache/`**: `discovery.rs` provides `DiscoveryCache` (TTL-based validity, persisted at `<workspace-root>/.git-same/cache.json`); `sync_history.rs` records sync runs at `<workspace-root>/.git-same/sync-history.json`
- **`domain/`**: Domain primitives, currently `repo_path_template.rs` for resolving `{org}/{repo}` style structures
- **`infra/storage/`**: Storage abstractions for workspace-local persistence
- **`ipc/`**: Monitor ↔ Finder-extension interface (`status_file.rs`, `unix_socket.rs`)
- **`api/`**: Higher-level service helpers built on top of git/provider/config (e.g. `RepoScanService`)
- **`errors/`**: Custom error hierarchy: `AppError`, `GitError`, `ProviderError` with `suggested_action()` methods
- **`output/`**: `printer.rs` for verbosity-aware text output; `progress/` holds the `indicatif` progress bars (`CloneProgressBar`, `SyncProgressBar`, `DiscoveryProgressBar`)
- **`types/`**: Core data types: `Repo`, `Org`, `ActionPlan`, `OpResult`, `OpSummary`, plus `RepoEntry`/`SyncHistoryEntry` (lifted out of the TUI in B0.1)
- **`checks.rs`**: System/runtime checks (presence of `git`, `gh`, auth status, SSH access via `check_ssh_github_access`)

### CLI / TUI modules (`crates/git-same-cli/src/`)

- **`app/`**: Top-level entry points: `app/cli/` runs the CLI subcommand path, `app/tui/` boots the interactive TUI. `main.rs` dispatches to one or the other based on whether a subcommand was given
- **`commands/`**: Per-subcommand handlers (`init`, `setup`, `sync_cmd`, `status`, `scan`, `reset`, `workspace`, `monitor`, `refresh`) plus shared `support/` helpers
- **`setup/`**: Setup wizard state machine + ratatui rendering, shared between the CLI `setup` command and the TUI workspace-setup screen (gated by the `tui` feature)
- **`tui/`**: Ratatui-based TUI (gated by the `tui` feature)
- **`cli.rs`**: clap derive types
- **`banner.rs`**: CLI banner rendering
- **`bin/gen_completions.rs`, `bin/gen_manpage.rs`**: Release-only helpers gated by the `release-tools` feature

### TUI module (`crates/git-same-cli/src/tui/`, feature-gated behind `tui`)

Elm architecture: `app.rs` = Model, `screens/` = View, `handler.rs` = Update.

- **`app.rs`**: `App` struct holds all TUI state. `Screen` enum: `WorkspaceSetup`, `Workspaces`, `Dashboard`, `Sync`, `Settings`
- **`handler.rs`**: Keyboard input handlers per screen + `handle_backend_message` for async results
- **`backend.rs`**: Spawns Tokio tasks for async operations (sync, status scan), sends `BackendMessage` variants via unbounded channels
- **`event.rs`**: `AppEvent` (terminal input, backend messages, ticks) and `BackendMessage` enum
- **`screens/`**: Stateless render functions per screen (dashboard, workspace, settings, etc.)
- **`widgets/`**: Shared widgets (status bar, spinner)

### Key patterns

- **Trait-based abstractions:** `GitOperations`, `Provider`, progress traits: enables mocking in tests
- **Concurrency:** Tokio tasks with `Arc<dyn Trait>` for sharing progress reporters across tasks
- **Error handling:** `thiserror` for typed errors + `anyhow` for propagation. Custom `Result` type alias in `errors/`
- **Channel-based TUI updates:** Backend operations send `BackendMessage` through `mpsc::UnboundedSender<AppEvent>`, processed by the TUI event loop
- **Arrow-only navigation:** All directional movement uses arrow keys only (`←` `↑` `↓` `→`). No vim-style `j`/`k`/`h`/`l` letter navigation. Display hints use `[←] [↑] [↓] [→] Move`.

## FinderSync extension gotchas (macOS)

Three non-obvious traps in `macos/GitSameBadges/`. Each one silently breaks badges with no error log: the extension self-check still shows green.

1. **Boot-volume alias paths.** macOS auto-creates `/Volumes/<boot-volume-name>` as a symlink to `/`. Finder presents home-folder URLs with that prefix (`/Volumes/Manuel-SSD-4TB/Users/m/...`) and gates `requestBadgeIdentifier` on the URL matching an entry in `directoryURLs`. `Principal.updateMonitoredDirectories()` must register both the canonical and the alias-prefixed form of every watched root, otherwise the callback fires for nothing.

2. **macOS 26.4 sandbox rendering regression.** Both `NSImage.lockFocus()` and `NSImage(size:flipped:drawingHandler:)` produce empty/invalid pixel data when called inside a sandboxed FinderSync extension on 26.4. Symptom: Finder reserves the badge slot (folder icons shift) but no glyph renders. Workaround: build badges from SF Symbols (`NSImage(systemSymbolName:)` with palette `SymbolConfiguration`). SF Symbols are pre-rendered by macOS, no per-process drawing context required. Apple's own `r.circle.fill`/`o.circle.fill`/`u.circle.fill` are what `BadgeManager.symbolBadge` uses.

3. **Google Drive's FinderSync poisons the badge-rendering pipeline.** When `com.google.drivefs.finderhelper.findersync` is enabled, peer FinderSync extensions render no badge image even after Finder calls `setBadgeIdentifier`. Confirmed in this environment: badges only began appearing after the user disabled Google Drive in System Settings → Login Items & Extensions. Other peers (Keka, Synology, Dropbox) coexist fine. There is no code fix; document the workaround and surface it in the in-app self-check if you can.

`scan_roots` and `show_ambient`: defaults are `["~"]` / `false`. Never re-enable `show_ambient = true` with `~` in `scan_roots`: Finder refuses to call `requestBadgeIdentifier` on extensions whose `directoryURLs` contain the home folder (separate issue from the three above).

## Workspace folder branding (macOS)

The host paints a custom icon onto every workspace root via `NSWorkspace.setIcon` (wrapped in `crates/git-same-core/src/macos/folder_icon.rs`) so Finder shows it in the sidebar, column, list, icon, and Get Info views. A FinderSync extension can never replicate this; it only exposes corner badges. The icon is `crates/git-same-core/assets/workspace-folder.icns`, embedded via `include_bytes!` and regenerable via `bash toolkit/icons/build-workspace-folder-icns.sh`.

Lifecycle: painted by `core::setup::save_workspace` and `app::commands::save_workspace`, reapplied by the monitor (`monitor::run::reapply_workspace_folder_icons`) on every full scan if the `Icon\r` is missing, and stripped by `cli::commands::reset` and `app::commands::delete_workspace`. Opt out globally with `[ui] custom_folder_icon = false`.

**Finder Sidebar snapshot caveat.** `LSSharedFileList` captures a per-item icon bitmap into `~/Library/Application Support/com.apple.sharedfilelist/com.apple.LSSharedFileList.FavoriteItems.sfl3` at the moment the user drags a folder into Favorites. That snapshot is frozen: repainting the folder's `Icon\r` does **not** update the sidebar. The only refresh path is manual: right-click the stale sidebar item → Remove from Sidebar, then drag the folder back from a Finder window into Favorites. Don't waste time looking for a programmatic refresh API; the framework doesn't expose one, and the recommended workaround used by Synology / Dropbox is the same drag-and-drop.

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

**Do not** write inline `#[cfg(test)] mod tests { ... }` blocks; always use separate `_tests.rs` files.

**Integration tests** live in `crates/git-same-cli/tests/integration_test.rs`. They spawn the binary via `env!("CARGO_BIN_EXE_git-same")` (compile-time path), so they always run against the freshly built CLI binary at the workspace `target/`.

**Cross-crate test helpers:** `Repo::test()` in `git-same-core` is gated on `cfg(any(test, feature = "test-utils"))`. The CLI crate enables the `test-utils` feature in its `[dev-dependencies]` so its tests can call the helper without exposing it in production builds.

## CI/CD Workflows

All workflows are `workflow_dispatch` (manual trigger) in `.github/workflows/`:

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `S1-Test-CI.yml` | fmt, clippy, test, build dry-run, coverage, audit | Manual dispatch |
| `S2-Release-GitHub.yml` | Full CI + cross-compile 4 targets (per `toolkit/packaging/targets.txt`) + build/notarize the macOS app DMGs (aarch64, x86_64) + GitHub Release (all assets attached atomically) | Manual dispatch (select tag) |
| `S3-Publish-Homebrew.yml` | Download release tarballs and render `git-same-cli` formula + `git-same` cask templates into `zaai-com/homebrew-tap` | Manual dispatch (select tag) |
| `S4-Publish-Crates.yml` | Two-stage publish to crates.io: `git-same-core` → poll until indexed → `git-same` | Manual dispatch (select tag) |

S2 runs all S1 jobs (test, coverage, audit) as gates before building release artifacts.

## Specs & Docs

End-user docs in `docs/README.md`. Contributor/build-from-source docs in `docs/DEVELOPMENT.md`. Sync-screen design notes in `docs/Sync-Screen.md`. In-flight design plans live under `docs/plans/`.
