# Plan: Remove [[providers]] from Global Config

## Context

Previous work simplified auth to gh-cli only and restructured providers (Steps 1–5, all done). The remaining task: remove `[[providers]]` from the global user config entirely. Confirmed that `config.providers: Vec<ProviderEntry>` is never used at runtime — all sync/clone operations use `WorkspaceConfig.provider` (workspace-level). The global config should contain only: `concurrency`, `sync_mode`, `structure`, `default_workspace`, `[clone]`, `[filters]`. `ProviderEntry` / `AuthMethod` remain as internal types used by workspace config and the provider factory.

---

## Step 1: `src/config/parser.rs`

- Remove `providers: Vec<ProviderEntry>` field and `#[serde(default = "default_providers")]` annotation from `Config`
- Remove `default_providers()` function
- Remove `use super::provider_config::ProviderEntry;` import (no longer needed here)
- Remove provider validation block from `Config::validate()` (the `for (i, provider)` loop and the empty-providers check)
- Remove `enabled_providers()` method
- Remove the `[[providers]]` section from `Config::default_toml()` (lines ~260–265)
- Remove `ProviderEntry` from `Config::default()` (it's in the providers field)

---

## Step 2: `src/config/mod.rs`

- Update the doc comment example to remove `[[providers]]`
- Keep `AuthMethod` and `ProviderEntry` in `pub use provider_config::{...}` — required because `WorkspaceProvider.auth: AuthMethod` is a `pub` field and `to_provider_entry()` returns `ProviderEntry`. Removing them from `mod.rs` while `provider_config` is a private module would cause a `E0446` compile error (public field/method using a type that is unreachable outside the module).

---

## Step 3: `src/lib.rs` prelude

- Remove `AuthMethod` and `ProviderEntry` from the prelude re-exports in `src/lib.rs:73` — they remain accessible as `crate::config::AuthMethod` / `crate::config::ProviderEntry` but are no longer advertised at the top-level API surface

---

## Step 4: `src/config/parser_tests.rs`

- `test_default_config` (line 12): remove `assert_eq!(config.providers.len(), 1)`
- `test_load_full_config` (lines 41–43): remove `[[providers]]` section from the test TOML string (TOML parses fine without it)
- Remove `test_load_multi_provider_config` entirely (lines 58–75)
- Remove `test_validation_rejects_empty_providers` entirely (lines 104–113)
- Remove `test_enabled_providers_filter` entirely (lines 131–152)
- `test_parse_config_with_default_workspace` (lines 165–167): remove `[[providers]]` from content
- `test_parse_config_without_default_workspace` (lines 175–177): remove `[[providers]]` from content
- `test_save_default_workspace_to_replace_without_sync_mode` (lines 241–244): remove `[[providers]]` from content

> Note: serde ignores unknown TOML keys by default, so existing user config files with `[[providers]]` will continue to load without error — the section is silently ignored.

---

## Files Summary

| File | Change |
|------|--------|
| `src/config/parser.rs` | Remove `providers` field, `default_providers()`, empty-providers validation, `enabled_providers()`, `[[providers]]` from default TOML |
| `src/config/mod.rs` | Remove `AuthMethod`/`ProviderEntry` from public exports; update doc example |
| `src/lib.rs` | Remove `AuthMethod`/`ProviderEntry` from prelude |
| `src/config/parser_tests.rs` | Remove provider-related assertions and tests |

---

## Verification

1. `cargo fmt -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
