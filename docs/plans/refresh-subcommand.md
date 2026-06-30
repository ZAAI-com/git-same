# Plan — New `gisa refresh` subcommand

## Context

Users occasionally want to force an immediate `status.json` rewrite without running a full sync (e.g. after manually deleting a repo, or when debugging badge issues). The socket protocol already supports this via `REFRESH_ALL` and `REFRESH /path`; we just need a user-facing surface. It's also a natural place to diagnose "is the daemon running?" because the command either succeeds (daemon alive) or prints a helpful error (daemon down, start with `gisa daemon`).

## Files to create

- `src/commands/refresh.rs` — new handler, mirroring `src/commands/status.rs:11` shape.
- `src/commands/refresh_tests.rs` — colocated tests per CLAUDE.md convention.

## Files to modify

- `src/cli.rs` — add a `Refresh(RefreshArgs)` variant to the `Command` enum (around line 124 where `Status` is defined), plus a `RefreshArgs` struct. Likely flags:
  - `--path <DIR>`: optional single-path refresh (routes to `REFRESH /path` instead of `REFRESH_ALL`).
  - No others for the MVP.
- `src/commands/mod.rs:29-68` — add `Command::Refresh(args) => refresh::run(args, &config, output).await` arm in the match block (around line 65 where `Status` is routed). Also declare `pub mod refresh;` near the other modules.
- `run.sh` — append a line to the cheat sheet, consistent with commit `33d0c7a` that added daemon/scan/TUI commands.

## Handler shape

```rust
pub async fn run(args: &RefreshArgs, _config: &Config, output: &Output) -> Result<()> {
    use crate::ipc::{IpcConfig, UnixSocketClient};
    let cfg = IpcConfig::default_path()?;
    let client = UnixSocketClient::new(cfg.socket_path());
    let response = match args.path.as_deref() {
        Some(p) => client.refresh(p).await,
        None => client.refresh_all().await,
    };
    match response {
        Ok(_) => { output.success("Daemon refreshed"); Ok(()) }
        Err(e) => { output.error("Daemon not reachable. Start it with `gisa daemon`."); Err(e) }
    }
}
```

Note: unlike the post-sync/post-reset nudges, `gisa refresh` is user-initiated, so a daemon-down state SHOULD return a clear error (not silent). That is the one meaningful behavior difference.

## Windows / non-unix

`UnixSocketClient` is `#[cfg(unix)]`. Gate the handler similarly; on non-unix print a short "refresh is unix-only for now" message and return `Ok(())`. `src/ipc/mod.rs:1-8` notes Windows named-pipe support is planned but not shipped.

## Verification

1. `gisa daemon` in a terminal.
2. `gisa refresh` → "Daemon refreshed" printed; `status.json` mtime bumps.
3. `gisa refresh --path /path/to/org` → same, targeted.
4. Kill daemon, `gisa refresh` → clear error, non-zero exit.
5. `cargo test` includes new `refresh_tests.rs`.
6. Manual TUI regression pass: no screen references `refresh` yet, so no TUI changes needed.

## Out of scope

- Auto-starting the daemon if it's down (needs a separate decision about launchd/systemd integration).
- A `--watch` mode.
