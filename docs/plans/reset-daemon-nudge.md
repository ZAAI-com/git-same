# Plan — Nudge the daemon after `gisa reset`

## Context

`gisa reset` removes git-same config, workspace metadata, and cached discovery data (cloned repos stay on disk). Per `src/cli.rs:142-145` help text, this does not delete repos, but it does invalidate everything the daemon currently believes about workspaces. Without a nudge, the daemon keeps serving stale `status.json` until its next poll, so Finder keeps painting "R" badges for workspaces the user just wiped.

Same root-cause shape as the sync case (commit `6ae60ff`): state changes on disk, daemon doesn't know.

## Files to modify

- `src/commands/reset.rs:49` — `pub async fn run(args: &ResetArgs, output: &Output) -> Result<()>`. Add the nudge immediately before the final `Ok(())` at line 74, after `execute_reset()` returns successfully.

## Reuse

- Same helper shape already added to `sync_cmd.rs` in commit `6ae60ff` (`nudge_daemon_refresh` private async fn wrapping `UnixSocketClient::refresh_all`). Either duplicate the 12 lines into `reset.rs` or lift it to `src/commands/mod.rs` as a `pub(super)` helper. Duplicating is fine for now — two callers isn't premature abstraction.

## Gating

- Fire the nudge only on the real-work path. Reset has no `--dry-run` flag today, only `--force` (skip confirmation). Fire after `execute_reset()` succeeds regardless of `--force`.

## Verification

1. Start `gisa daemon` in the background and populate `status.json` via `gisa sync`.
2. Run `gisa reset --force`.
3. In Finder, the previously-badged workspace folders should lose their badges within a second (daemon re-scans and writes an updated `status.json`).
4. Kill the daemon, repeat — reset must still succeed silently.
5. `cargo test && cargo clippy -- -D warnings && cargo fmt -- --check`.

## Out of scope

- Any change to what `reset` actually deletes.
- Adding a `--dry-run` flag.
