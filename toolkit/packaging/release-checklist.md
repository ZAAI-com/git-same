# git-same release checklist

Pre-flight steps for cutting a new release. Each major step maps to one of the
manual `workflow_dispatch` workflows under `.github/workflows/`.

## 1. Local prep

- [ ] Working tree clean on `main`, all PRs merged.
- [ ] Bump `version` in `Cargo.toml` (and confirm `Cargo.lock` regenerates clean: `cargo build`).
- [ ] Update `CHANGELOG` / release notes draft if applicable.
- [ ] Smoke-render the Homebrew artifacts locally:
  ```sh
  bash toolkit/homebrew/render-cask.sh    3.X.Y --sha-arm <64x0> --sha-intel <64x0>
  bash toolkit/homebrew/render-formula.sh 3.X.Y --url https://example --sha-macos-arm <64x0> --sha-macos-intel <64x0> --sha-linux-arm <64x0> --sha-linux-intel <64x0>
  ```

## 2. S1 (test CI)

- [ ] Run **S1 — Test CI** on `main`. fmt / clippy / test / coverage / audit must all be green.

## 3. Tag

- [ ] `git tag <version>` (strict semver, no `v` prefix, no leading zeros).
- [ ] `git push origin <version>`.

## 4. S2 (release build)

- [ ] Run **S2 — Release GitHub** against the tag.
- [ ] Verify the four release tarballs are uploaded:
  - `git-same-<v>-x86_64-unknown-linux-gnu.tar.gz`
  - `git-same-<v>-aarch64-unknown-linux-gnu.tar.gz`
  - `git-same-<v>-x86_64-apple-darwin.tar.gz`
  - `git-same-<v>-aarch64-apple-darwin.tar.gz`
- [ ] Verify the macOS tarballs are notarized:
  ```sh
  curl -sSL <url> | tar -xz && spctl --assess --type execute --verbose ./git-same
  ```
- [ ] Verify each tarball's contents match `toolkit/packaging/tarball-manifest.txt` (the workflow gates on this; spot-check anyway).

## 5. S3 (publish Homebrew)

- [ ] Run **S3 — Publish Homebrew** with the tag.
- [ ] `verify-tap.sh` step must pass (gates the tap push).
- [ ] Confirm the commit landed on `zaai-com/homebrew-tap`.

## 6. S4 (publish to crates.io)

- [ ] Run **S4 — Publish Crates** with the tag.
- [ ] Confirm crate is live at https://crates.io/crates/git-same.

## 7. Post-release smoke

- [ ] On a clean Mac (arm64): `brew install --cask zaai-com/tap/git-same`. Run `git-same --version`, `gisa workspace --help`, `man git-same`, and tab-complete `gisa <Tab>`.
- [ ] On a clean Mac (x86_64): same as above.
- [ ] On Linux (Docker is fine): `brew install zaai-com/tap/git-same-cli`. Same checks (sans `man` if unavailable).
- [ ] `cargo install git-same` succeeds.
