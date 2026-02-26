# Plan: Alias Source of Truth + Homebrew OOTB Guarantee

**Status:** Proposed  
**Impact:** Packaging reliability and drift prevention across installers

## Goals

1. Ensure Homebrew installs all supported command names out of the box:
   - `git-same` (primary)
   - `gitsame`
   - `gitsa`
   - `gisa`
2. Define aliases in one source of truth to avoid drift across workflows, scripts, and docs.
3. Keep the design portable so MacPorts can reuse the same alias data later.

## Non-Goals

- Implement full MacPorts publishing in this phase.
- Change runtime behavior based on executable name (`argv[0]`).
- Add new aliases beyond the current supported set.

## Current Problems

- Alias names are duplicated in multiple places (`Cargo.toml`, docs, local scripts, workflows).
- Homebrew formula generation currently installs and tests only `git-same`.
- There is no automated drift check to ensure all package surfaces expose the same aliases.

## Proposed Design

### 1. Add a single alias manifest

Create a new file:

`packaging/binary-aliases.txt`

Format:
- Line 1 = primary binary name.
- Remaining lines = aliases.

Example:

```text
git-same
gitsame
gitsa
gisa
```

Why this format:
- Easy to parse in bash, GitHub Actions, and other tooling without extra dependencies.
- Easy to consume later from MacPorts update scripts.

### 2. Homebrew formula generation consumes the manifest

Update `.github/workflows/S3-Publish-Homebrew.yml`:

- Read primary name + alias list from `packaging/binary-aliases.txt`.
- Keep `bin.install ... => "git-same"` for primary executable.
- Generate one `bin.install_symlink` line per alias.
- Extend `test do` to validate all command names:
  - `git-same --version`
  - `gitsame --version`
  - `gitsa --version`
  - `gisa --version`

This turns alias support into a release-gated check instead of documentation-only behavior.

### 3. Local Conductor scripts consume the manifest

Update:
- `toolkit/Conductor/run.sh`
- `toolkit/Conductor/archive.sh`

Behavior:
- Parse alias manifest.
- Create symlinks after `cargo install --path .` in `run.sh`.
- Remove all aliases from manifest in `archive.sh`.

This avoids hardcoding alias names in multiple script arrays.

### 4. Add drift-check script for CI

Add a small script, e.g.:

`toolkit/packaging/verify-binary-aliases.sh`

Checks:
- Manifest exists, non-empty, unique values.
- Primary command appears first.
- Homebrew workflow template/test includes all aliases (or generated output contains them).
- README alias section is consistent with manifest.

Wire it into S1 test workflow so drift fails early.

### 5. MacPorts readiness path (future phase)

Add a helper renderer script now (or in follow-up), e.g.:

`toolkit/packaging/render-aliases.sh --target macports`

Output snippet example:

```tcl
ln -s git-same ${destroot}${prefix}/bin/gitsame
ln -s git-same ${destroot}${prefix}/bin/gitsa
ln -s git-same ${destroot}${prefix}/bin/gisa
```

When MacPorts work starts, this snippet can be consumed in `post-destroot` with no alias list duplication.

## Files to Modify

| File | Change |
|------|--------|
| `packaging/binary-aliases.txt` | New source-of-truth alias manifest |
| `.github/workflows/S3-Publish-Homebrew.yml` | Generate install symlinks + alias tests from manifest |
| `toolkit/Conductor/run.sh` | Create aliases from manifest after install |
| `toolkit/Conductor/archive.sh` | Remove aliases from manifest during cleanup |
| `toolkit/packaging/verify-binary-aliases.sh` | New drift guard script |
| `.github/workflows/S1-Test-CI.yml` | Execute drift check script |
| `docs/README.md` | Clarify alias behavior by install method |
| `toolkit/packaging/render-aliases.sh` *(optional now, recommended)* | Generate MacPorts/Homebrew alias snippets |

## Validation

### Automated

Run:

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
./toolkit/packaging/verify-binary-aliases.sh
```

### Packaging smoke checks

Homebrew formula check (release flow):
- Trigger `S3-Publish-Homebrew` with a known release tag.
- Confirm generated formula contains all `bin.install_symlink` lines.
- Confirm formula `test do` passes for all aliases.

Local install check:

```bash
toolkit/Conductor/run.sh
for cmd in git-same gitsame gitsa gisa; do "$cmd" --version; done
```

## Rollout Strategy

1. Add manifest + script consumers (`run.sh`, `archive.sh`).
2. Add Homebrew generation + formula tests from manifest.
3. Add drift-check script + S1 CI gate.
4. Add MacPorts renderer helper (or track as immediate follow-up issue).

## Risks and Mitigations

- Risk: Manifest parser bugs in shell scripts.
  - Mitigation: Keep plain line-based format and add parser unit/smoke checks.
- Risk: Homebrew formula template drift.
  - Mitigation: CI drift check inspects generated formula content.
- Risk: Future installer adds aliases manually and diverges.
  - Mitigation: Require all packaging workflows/scripts to read from manifest only.

## Success Criteria

- Homebrew install exposes all four commands on first install.
- Alias list changes require editing one file only.
- CI fails on alias drift before release workflows are run.
- MacPorts onboarding only needs package-specific glue, not alias-list redesign.
