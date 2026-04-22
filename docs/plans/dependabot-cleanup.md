# Plan — Clear Dependabot alerts (3 low-severity)

## Context

Pushing `6ae60ff` surfaced `GitHub found 3 vulnerabilities on ZAAI-com/git-same's default branch (3 low)`. Raw data from `gh api repos/ZAAI-com/git-same/dependabot/alerts`:

| # | Package | Vulnerable range | Patched | Severity |
|---|---|---|---|---|
| 9  | `rand`           | `>= 0.7.0, < 0.9.3`      | `0.9.3`    | low |
| 10 | `rustls-webpki`  | `>= 0.101.0, < 0.103.12` | `0.103.12` | low |
| 11 | `rustls-webpki`  | `>= 0.101.0, < 0.103.12` | `0.103.12` | low |

All three are in `Cargo.lock` only (no source change required). On the `C/Finder-Icons` branch, `Cargo.lock` already shows `rustls-webpki 0.103.12` and `rand 0.9.4` as the primary versions — those alerts may already be resolved on the default branch as of the most recent `Update Cargo.lock` commit (`d576e63`), and GitHub just hasn't re-scanned. However, `Cargo.lock` also still contains a stale `rand 0.8.6` entry (lines 1879-1886) that `cargo tree` cannot trace back to any consumer — a likely cruft entry that a fresh `cargo update` should prune.

## Steps

1. On `main` (NOT a Finder branch), run `cargo update` to let Cargo recompute the lockfile. Confirm `rand 0.8.6` drops out. If it doesn't, `cargo tree --target all --all-features -i -p rand@0.8.6` + `grep` Cargo.lock backward to find the consumer — may need a targeted `cargo update -p rand@0.8.6` or a Cargo.toml bump for the intermediate crate.
2. Run `cargo audit` locally (already part of `S1-Test-CI.yml`). Should report zero advisories after step 1.
3. `cargo test && cargo clippy -- -D warnings && cargo fmt -- --check`.
4. Commit as `Bump Cargo.lock to drop vulnerable rand and rustls-webpki versions`. Push to `main` via a small PR (do NOT piggyback on another feature branch — these are independent changes).
5. Wait for Dependabot to re-scan (usually within minutes after push). Confirm the three open alerts auto-close. If any stay open, manually dismiss with a reason via `gh api -X PATCH repos/ZAAI-com/git-same/dependabot/alerts/<n> -f state=dismissed -f dismissed_reason=fix_started` (only if truly a false positive).

## Verification

- `cargo audit` clean locally.
- GitHub Dependabot dashboard shows 0 open alerts.
- Next `S1-Test-CI` workflow run is green on the audit job.

## Risk / roll-back

- `cargo update` can pull in minor-bumped transitive deps with behavior changes. After updating, run the full test suite and do a manual smoke test of `gisa sync` against a small real workspace before merging. Roll back by `git restore Cargo.lock`.

## Out of scope

- Upgrading to `rustls-webpki 0.104.x` (still alpha per advisory; don't chase pre-releases).
- Adding `cargo deny` or stricter audit gating.
