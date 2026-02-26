# Move Workspace Config to Sync Folder

## Status: Proposal (not yet implemented)

## Problem

Workspace configs live in `~/.config/git-same/<name>/` — a location disconnected from the repos they manage. This creates several friction points:

- **Requires auto-generated names** — The `<name>` directory (e.g., `github-repos`) is an artifact of this storage model. Users never chose it and gain nothing from it.
- **Not portable** — Moving `~/repos` to another machine loses the config. You'd need to re-run setup.
- **Two locations to back up** — Config in `~/.config`, data in `~/repos`.
- **Not self-describing** — No way to tell a folder is a gisa sync target by looking at it.

## Proposed Design

Move workspace config into the sync folder itself:

```text
~/repos/                               ~/.config/git-same/
├── .git-same/                         └── config.toml  (global only)
│   ├── config.toml                        ├── structure = "{org}/{repo}"
│   └── cache.json                         ├── concurrency = 8
├── org1/repo1/.git/                       └── sync_mode = "fetch"
└── org2/repo3/.git/
```

### Key changes

1. **Workspace config moves to `{base_path}/.git-same/config.toml`**
2. **Cache moves to `{base_path}/.git-same/cache.json`**
3. **Global config stays at `~/.config/git-same/config.toml`** — holds defaults + a registry of known workspace paths
4. **`default_workspace` becomes a path** — e.g., `default_workspace = "~/repos"` instead of `default_workspace = "github-repos"`
5. **Workspace discovery** — scan registered paths in global config, verify `.git-same/config.toml` exists

### Global config changes

```toml
# ~/.config/git-same/config.toml
structure = "{org}/{repo}"
concurrency = 8
sync_mode = "fetch"

# Default workspace (by path)
default_workspace = "~/repos"

# Known workspace paths (for discovery)
workspaces = [
    "~/repos",
    "~/work/code",
]
```

### Migration strategy

1. On first run after update, detect old-format configs in `~/.config/git-same/<name>/`
2. Move each workspace config into its `base_path/.git-same/`
3. Update global config with `workspaces = [...]` array
4. Remove old workspace directories from `~/.config/git-same/`
5. Print a migration summary

### What this eliminates

- `WorkspaceConfig.name` field (no longer needed — path IS the identity)
- `WorkspaceManager::name_from_path()` / `unique_name()`
- The entire `~/.config/git-same/<name>/` directory structure
- `SetupState.workspace_name` / `name_editing`

### What this enables

- `gisa setup` in any directory drops config right there
- Moving a sync folder to another machine preserves the config
- `base_path` becomes the sole workspace identifier everywhere

## Edge Cases to Handle

| Case | Resolution |
|------|-----------|
| Sync folder on read-only mount | Fall back to `~/.config` location, warn |
| Sync folder is itself a git repo | Add `.git-same/` to `.gitignore` automatically |
| Org or repo named `.git-same` | Use a more unique name like `.gisa/` |
| User deletes `.git-same/` | Workspace disappears from registry; `gisa setup` re-creates |
| Two workspaces with same base_path | Not possible — path is unique identity |

## Files to Modify

- `src/config/workspace_manager.rs` — Complete rewrite of discovery/save/load
- `src/config/workspace.rs` — Remove `name` field, update constructors
- `src/config/parser.rs` — Add `workspaces` array, change `default_workspace` to path
- `src/setup/handler.rs` — Save to `base_path/.git-same/` instead of `~/.config`
- `src/setup/state.rs` — Remove `workspace_name` / `name_editing`
- `src/cache.rs` — Update cache path resolution
- `src/commands/workspace.rs` — Rewrite for path-based operations
- `src/tui/handler.rs` — Update workspace matching
- `src/tui/app.rs` — Update workspace loading

## Estimated Scope

Medium-large refactor. Migration logic is the riskiest part — must handle partial migrations, permission errors, and rollback. Consider feature-flagging the new storage format during development.
