# Plan: Remove `[[providers]]` From Global Config

## Goal
Remove provider definitions from the global config file (`~/.config/git-same/config.toml`) and keep provider configuration workspace-scoped.  
After this change, global config should only contain:
- `concurrency`
- `sync_mode`
- `structure`
- `default_workspace`
- `[clone]`
- `[filters]`

## Scope Decision
- Treat this as a **breaking library API change** (CLI behavior remains aligned with current workspace-based flow).
- Runtime already uses workspace provider config for setup/sync operations.

## Implementation Steps

### 1. Confirm release/API scope
- Mark this work as breaking for crate consumers because `AuthMethod`/`ProviderEntry` are currently part of public interfaces.
- Audit affected surfaces:
  - `src/lib.rs`
  - `src/config/mod.rs`
  - `src/config/workspace.rs`
  - `src/auth/mod.rs`
  - `src/provider/mod.rs`

### 2. Remove global `[[providers]]` schema from parser
Update `src/config/parser.rs`:
- Remove `providers: Vec<ProviderEntry>` from `Config`.
- Remove `default_providers()` helper.
- Remove provider-specific validation (empty-check and per-provider loop).
- Remove `enabled_providers()` method.
- Remove `[[providers]]` block from `Config::default_toml()`.
- Remove unused import of `ProviderEntry`.

### 3. Redesign workspace/provider bridge API
Update `src/config/workspace.rs`:
- Remove `to_provider_entry()` adapter from `WorkspaceProvider`.
- Add direct helpers required by auth/provider code paths (for example, API URL/name helpers), so runtime no longer depends on `ProviderEntry`.

### 4. Remove `AuthMethod` from public workspace model
- Since auth is gh-cli-only, remove `auth` from `WorkspaceProvider` and related serialization/tests.
- Keep workspace provider fields that are still user-specific (`kind`, `api_url`, `prefer_ssh`).
- Update `src/config/provider_config.rs` as needed so legacy type usage is minimized or internalized.

### 5. Update auth/provider entrypoints to use workspace provider type
Update:
- `src/auth/mod.rs`
- `src/provider/mod.rs`

Actions:
- Replace function signatures that currently accept `ProviderEntry`.
- Preserve existing behavior for host extraction and enterprise URL handling.

### 6. Migrate all runtime call sites
Update:
- `src/workflows/sync_workspace.rs`
- `src/setup/handler.rs`

Actions:
- Pass `WorkspaceProvider` directly to auth/provider layers.
- Remove intermediate conversion calls.

### 7. Remove public re-exports for legacy provider config types
Update:
- `src/config/mod.rs`
- `src/lib.rs`
- `src/lib_tests.rs`

Actions:
- Remove prelude/config re-exports of `AuthMethod` and `ProviderEntry`.
- Adjust prelude tests to validate remaining public API.

### 8. Update parser tests for new global schema
Update `src/config/parser_tests.rs`:
- Remove provider-related assertions/tests.
- Remove `[[providers]]` snippets where no longer necessary.
- Add backward-compat test: config containing legacy `[[providers]]` still parses and is ignored.

### 9. Update workspace/auth/provider tests
Update relevant tests to new interfaces and structs:
- `src/config/workspace_tests.rs`
- `src/provider/mod_tests.rs`
- `src/auth/mod_tests.rs`
- `src/workflows/sync_workspace_tests.rs`

### 10. Update docs and examples
Update:
- `docs/README.md`
- `.context/GIT-SAME-DOCUMENTATION.md` (if maintained in parallel)

Actions:
- Remove global `[[providers]]` examples.
- Document provider configuration as workspace-scoped.

### 11. Validation
Run:
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo test`

Manual smoke checks:
- `gisa init`
- `gisa setup`
- `gisa sync --dry-run`
- Verify legacy config with `[[providers]]` still loads without failure.

### 12. Delivery strategy (recommended)
Split into 3 commits:
1. Parser/schema cleanup + parser tests
2. API redesign + runtime call-site migration
3. Docs + remaining test updates

