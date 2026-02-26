# Optimize Binary Aliases

**Status:** Proposed
**Impact:** ~4x faster release link stage

## Problem

The release build produces 4 identical binaries (`git-same`, `gitsame`, `gitsa`, `gisa`), all compiled from `src/main.rs` with no behavioral differences. Combined with the release profile (`lto = true`, `codegen-units = 1`), each binary triggers a full LTO link pass — the most expensive build step. This roughly quadruples link time.

## Current State

- `Cargo.toml` defines 4 `[[bin]]` entries all pointing to `src/main.rs`
- `src/main.rs` does not inspect `argv[0]` — all binaries behave identically
- Integration tests only reference `git-same`
- Homebrew formula already installs only `git-same`
- GitHub Release artifacts are single binaries per platform

## Proposed Solution

Replace the 4 `[[bin]]` entries with a single `git-same` binary and create aliases via symlinks or documentation depending on the install method.

### Cargo.toml

Remove 3 duplicate `[[bin]]` sections, keeping only:

```toml
[[bin]]
name = "git-same"
path = "src/main.rs"
```

### Homebrew (S3-Publish-Homebrew.yml)

Add symlinks in the formula's `install` method:

```ruby
bin.install_symlink "git-same" => "gitsame"
bin.install_symlink "git-same" => "gitsa"
bin.install_symlink "git-same" => "gisa"
```

### cargo install / GitHub Releases

Document that users can create shell aliases:

```bash
alias gitsame="git-same"
alias gitsa="git-same"
alias gisa="git-same"
```

Or symlinks:

```bash
for alias in gitsame gitsa gisa; do
    ln -sf "$(which git-same)" "$(dirname $(which git-same))/$alias"
done
```

### toolkit/Conductor/run.sh

Add symlink creation after `cargo install --path .`:

```bash
for alias in gitsame gitsa gisa; do
    ln -sf "$HOME/.cargo/bin/git-same" "$HOME/.cargo/bin/$alias"
done
```

## Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Remove 3 duplicate `[[bin]]` entries |
| `toolkit/Conductor/run.sh` | Add symlink creation after install |
| `.github/workflows/S3-Publish-Homebrew.yml` | Add `bin.install_symlink` lines |
| `docs/README.md` | Document alias setup for manual installs |

## No Changes Needed

- `src/main.rs` — no binary-name awareness
- `src/cli.rs` — display name hardcoded to `git-same`, completions generate as `gisa` (works via symlink)
- `tests/integration_test.rs` — already only references `git-same`
- `.github/workflows/S2-Release-GitHub.yml` — already builds single artifact per platform

## Trade-offs

- **Pro:** ~4x faster link stage in release builds
- **Pro:** Smaller build output (1 binary instead of 4)
- **Con:** `cargo install git-same` no longer auto-installs all 4 aliases
- **Con:** Users need to manually set up aliases or symlinks (unless using Homebrew)
