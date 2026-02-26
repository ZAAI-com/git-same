# Sync Screen Reference

This document is the source-level reference for how the TUI Sync screen works today.
It covers:

- state machine (`Discovering` -> `Running` -> `Finished`)
- backend event/message order
- per-state UI anatomy
- key bindings by state
- persistence and side effects
- notable implementation caveats

## Scope and Source of Truth

Primary implementation files:

- `src/tui/app.rs`
- `src/tui/event.rs`
- `src/tui/backend.rs`
- `src/tui/handler.rs`
- `src/tui/screens/sync.rs`
- `src/tui/screens/dashboard.rs`
- `src/tui/screens/settings.rs`

## High-Level Flow

From Dashboard, pressing `s` does one of two things:

1. If Sync context already exists (discovering/running/finished state or existing sync logs), it opens the Sync screen.
2. Otherwise, it starts a new Sync operation.

When a new operation starts, flow is:

1. UI enters `Discovering("Starting Sync...")`.
2. Backend discovers repos and builds an action plan.
3. UI receives `OperationStarted` and enters `Running`.
4. Clone/sync progress messages stream in.
5. UI receives `OperationComplete` and enters `Finished`.
6. A status scan is auto-triggered to refresh Dashboard repo data.

## State Model

`OperationState` variants in `src/tui/app.rs`:

- `Idle`
- `Discovering { operation, message }`
- `Running { ...metrics and internals... }`
- `Finished { ...summary and final metrics... }`

### Discovering Fields

- `operation`: currently Sync for this screen
- `message`: human-readable phase text (for example `Found 3 organizations`, `Discovering: my-org`)

### Running Fields

- `operation`
- `total`
- `completed`
- `failed`
- `skipped`
- `current_repo`
- `with_updates`
- `cloned`
- `synced`
- `to_clone`
- `to_sync`
- `total_new_commits`
- `started_at`
- `active_repos` (for worker slot line)
- `throughput_samples`
- `last_sample_completed`

### Finished Fields

- `operation`
- `summary` (`success`, `failed`, `skipped`)
- `with_updates`
- `cloned`
- `synced`
- `total_new_commits`
- `duration_secs`

## Backend Message Protocol

Messages come through `BackendMessage` in `src/tui/event.rs`.

Main Sync-related variants:

- `OrgsDiscovered(usize)`
- `OrgStarted(String)`
- `OrgComplete(String, usize)`
- `DiscoveryComplete(Vec<OwnedRepo>)`
- `DiscoveryError(String)`
- `OperationStarted { operation, total, to_clone, to_sync }`
- `RepoStarted { repo_name }`
- `RepoProgress { repo_name, success, skipped, message, had_updates, is_clone, new_commits, skip_reason }`
- `OperationComplete(OpSummary)`
- `OperationError(String)`
- `RepoCommitLog { repo_name, commits }`

## Exact Event Sequence (Typical Run)

### Pre-backend local transition

Dashboard start logic sets:

- `operation_state = Discovering("Starting Sync...")`
- clears running log
- resets animation tick counter
- navigates to Sync screen

### Discovery phase

Backend may emit, in order:

1. `OrgsDiscovered(count)` (optional)
2. `OrgStarted(name)` (0..N)
3. `OrgComplete(name, count)` (0..N)
4. `DiscoveryComplete(repos)` (success path)

Error path:

- `DiscoveryError(msg)` or `OperationError(msg)` and operation returns to `Idle` with `error_message`.

### Branch after discovery

If `repos.is_empty()`:

- backend emits `OperationComplete(OpSummary::new())`
- no `OperationStarted`, no repo progress stream

If repos exist:

1. backend emits `OperationStarted { total, to_clone, to_sync }`
2. per repo, concurrent/interleaved:
   - `RepoStarted { repo_name }`
   - `RepoProgress { ... }`
3. backend emits final `OperationComplete(combined_summary)`

### Post-complete side effects

On `OperationComplete`, handler:

1. extracts running metrics (updates/cloned/synced/commit count/duration)
2. writes `last_synced` on active workspace
3. persists workspace via `WorkspaceManager::save`
4. appends sync history entry
5. caps history in memory to 50
6. persists history via `SyncHistoryManager`
7. auto-starts status scan operation
8. sets default post-filter:
   - `Updated` if any updates/clones
   - else `All`
9. sets state to `Finished { ... }`

## UI Anatomy by State

`src/tui/screens/sync.rs` has two layout modes: running-layout and finished-layout.

### Discovering UI

Discovering reuses the running-layout skeleton with discovery-specific values:

- title uses `message`
- progress bar label is `Discovering...` with ratio `0`
- log panel still visible
- status hint: `Esc: Minimize  q: Quit`

### Running UI

Running layout sections, top to bottom:

1. animated banner
2. title (`Syncing Repositories`)
3. main progress gauge (`completed/total`)
4. enriched counters line:
   - `Updated`
   - `Current` (derived)
   - `Cloned`
   - optional `Failed`
   - optional `Skipped`
   - current repo name
5. throughput line:
   - elapsed
   - repos/sec
   - ETA (when enough data)
   - sparkline from throughput samples
6. phase indicator line:
   - clone bar (`cloned/to_clone`)
   - sync bar (`synced/to_sync`)
7. active worker slots (`[1] repo-a  [2] repo-b ...`)
8. running log list (color coded by prefix)
9. status bar hint (`Esc`, arrow scrolling, quit hint)

### Finished UI (normal)

If not empty-state, sections are:

1. banner
2. title (`Sync Complete`)
3. progress bar (`Done`)
4. summary boxes:
   - `Updated`
   - `Failed` (if failures exist) otherwise `Current`
   - `Cloned`
   - `Skipped`
5. performance line:
   - total repos
   - duration
   - repos/sec
   - optional total new commits
   - optional cloned count
6. filterable log (selectable rows, optional inline commit details)
7. status bar (filter keys/history/enter/esc/quit)

### Finished UI (empty-state)

If:

- `with_updates == 0`
- `cloned == 0`
- no failed entries

Then it renders:

- message: `Everything up to date`
- subtext: `N repositories synced, no changes found`
- performance line
- simplified status bar hint

### Sync History Overlay

When finished and `show_sync_history == true`, a centered overlay appears on top:

- list of recent runs (reverse chronological)
- each row includes time, repo count, changes summary, duration
- max overlay height is capped

## Log Data Model and Rendering

Structured entries are stored in `app.sync_log_entries` as `SyncLogEntry`:

- `repo_name`
- `status` (`Success`, `Updated`, `Cloned`, `Failed`, `Skipped`)
- `message`
- `had_updates`
- `is_clone`
- `new_commits`
- `path` (computed from workspace structure template)

Legacy plain lines are also stored in `app.log_lines` for running log rendering.

### Status Prefix Mapping

- `Failed` -> `[!!]` (red)
- `Skipped` -> `[--]` (dark gray)
- `Cloned` -> `[++]` (cyan)
- `Updated` -> `[**]` (yellow)
- `Success` -> `[ok]` (green)

## Keymap

Global handling lives in `src/tui/handler.rs`; Sync-local keys in `src/tui/screens/sync.rs`.

### Global keys (all screens including Sync)

- `Ctrl+C`: immediate quit
- `q`: two-step quit (`q` then `q`)
- `Esc`:
  - if Sync row is expanded, collapse expansion first
  - otherwise back/minimize
  - if Sync has empty screen stack, force to Dashboard

### Sync keys while Discovering/Running

- `Up` / `Down`: scroll running log
- `Esc`: minimize/back

### Sync keys while Finished

- `Up` / `Down`:
  - move selected row in filterable log
  - in changelog mode, scroll changelog timeline
- `Enter`: expand/collapse selected repo and fetch/show commits
- `a`: filter `All`
- `u`: filter `Updated`
- `f`: filter `Failed`
- `x`: filter `Skipped`
- `c`: filter `Changelog` and batch-fetch commits for updated repos
- `h`: toggle sync history overlay
- `Esc`: back/minimize

## Filters and Views

`LogFilter` modes:

- `All`
- `Updated` (includes updated and cloned entries)
- `Failed`
- `Skipped`
- `Changelog`

`Changelog` mode:

1. collects all entries with `had_updates`
2. spawns one async commit fetch per repo
3. shows loading state until fetched count reaches total
4. renders grouped timeline:
   - colored repo header
   - commit lines beneath
   - total commits in title

## Throughput, ETA, and Sampling

Event loop tick rate: `100ms`.

During Sync screen active operation:

- `tick_count` increments on each tick
- every 10 ticks (1 second), sample is appended:
  - `delta = completed - last_sample_completed`
- samples capped at `MAX_THROUGHPUT_SAMPLES` (240)

Render usage:

- elapsed and average repos/sec from completed/time
- ETA shown only if there is non-zero sample data and adequate rate
- sparkline rendered from recent sample values

## Dashboard Integration

Dashboard bottom line reflects Sync state:

- Discovering: `Sync discovering: ...`
- Running: percentage, completed/total, repos/sec, ETA, workers active/limit
- Finished: `Last Sync` summary (repos, updated, failed, duration)
- Idle with last sync timestamp: formatted last synced line

Dashboard `s` key behavior:

- starts sync only when there is no active/previous sync context
- otherwise opens Sync screen

## Settings Integration

Settings screen has `m` toggle for fetch/pull mode:

- `app.sync_pull = false` -> fetch mode
- `app.sync_pull = true` -> pull mode

Sync backend reads this flag and passes `pull: pull_mode` into `prepare_sync_workspace`.

## Error and Empty Paths

- no selected workspace -> `OperationError("No workspace selected...")`
- discovery/preparation failure -> `OperationError(...)`
- operation errors set state to `Idle` and set `error_message`
- zero discovered repos short-circuits directly to `Finished` via empty summary

## Implementation Caveats (Current Behavior)

1. TUI sync always uses `skip_uncommitted: true` in backend request.
2. TUI sync uses `execute_prepared_sync(..., false, ...)`, so dry-run is not currently wired through this path.
3. Sync screen status bar text uses single `q` wording, but actual quit logic is global double-press `q` (`qq`) unless `Ctrl+C` is used.
4. `RepoProgress.skip_reason` exists in message payload but is not currently consumed in Sync-screen render logic.

## Compact Sequence Diagram

```text
Dashboard [s]
  -> local: Discovering("Starting Sync..."), open Sync screen
  -> spawn backend sync task

Backend discovery
  -> OrgsDiscovered?
  -> OrgStarted/OrgComplete*
  -> DiscoveryComplete(repos)
  -> if repos.empty: OperationComplete(empty) -> Finished
  -> else OperationStarted(total,to_clone,to_sync) -> Running
  -> RepoStarted/RepoProgress* (concurrent, interleaved)
  -> OperationComplete(summary) -> Finished

Handler on complete
  -> persist last_synced + sync history
  -> set default finished filter
  -> auto-spawn status scan
  -> StatusResults

Finished extras
  [Enter] -> spawn_commit_fetch -> RepoCommitLog
  [c]     -> spawn_changelog_fetch* -> RepoCommitLog* (unordered arrival)
```
