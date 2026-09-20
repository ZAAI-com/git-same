# git-same release checklist

Pre-flight steps for cutting a new release. Each major step maps to one of the
manual `workflow_dispatch` workflows under `.github/workflows/`.

## 1. Local prep

- [ ] Working tree clean on `main`, all PRs merged.
- [ ] Bump `version` in `Cargo.toml` (and confirm `Cargo.lock` regenerates clean: `cargo build`).
- [ ] Bump `CFBundleShortVersionString` and `CFBundleVersion` in `macos/GitSameBadges/Info.plist` to the same version (hand-maintained; S1 gates the match, `plutil -lint` does not).
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
- [ ] Verify the macOS tarballs are signed with the app group (a bare Mach-O cannot be assessed by `spctl`, which reports "does not seem to be an app" even when correct):
  ```sh
  curl -sSL <url> | tar -xz && codesign --verify --strict ./git-same \
    && codesign -d --entitlements - ./git-same | grep group.57KL6Y7V32.com.zaai.git-same
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

## 8. Monitor acceptance (signed release, disposable macOS account or VM only)

Never run these against a developer's own login: they install, start, stop, and remove the real LaunchAgent. `brew reinstall` of the cask needs a terminal with sudo access.

After every step check `gisa monitor --status`, `launchctl print gui/$(id -u)/com.zaai.git-same.monitor`, and that exactly one `git-same monitor` process exists.

| Scenario | Required result |
|---|---|
| Fresh signed cask install | Monitor active without opening the app |
| Upgrade from the 3.1.1 cask | Legacy cleanup, new helper running, no second monitor |
| Upgrade between new versions | Preference preserved, helper updated |
| `brew upgrade` of the cask | Homebrew runs the *old* cask's uninstall stanza first, so the helper, plist, and `install.json` are removed and recreated: expect a new monitor PID and a new `install.json`. Required result: monitoring is `running` again once the upgrade returns, the helper reports the new version, and badges refresh within one scan. A brief badge gap during the swap is expected, not a defect. See `docs/plans/monitor-continuity-across-cask-upgrades.md` for the deferred fix |
| Reinstall while enabled | Exactly one monitor |
| Reinstall after `gisa monitor --stop` | Helper updated, no monitor started |
| Custom `--appdir` | `install.json` owner and source paths point at the custom location |
| App deleted before `brew uninstall` | Retained service tool removes the monitor |
| Helper deleted before `brew uninstall` | Removal still completes |
| Caskroom version directory deleted before uninstall | Uninstall fails clearly; `brew uninstall --force --cask git-same` then `gisa monitor --uninstall` recover |
| App placement or linking failure | brew reports failure; helper stays inspectable and removable |
| Logout/login and full restart | Enabled monitor returns |
| Stop, then restart the Mac | Monitor stays stopped |
| Direct DMG app launch (no cask) | Monitor installs and starts on first launch |
| Formula CLI first eligible use (`gisa sync`) | Standalone managed installation works |
| Cargo CLI first eligible use | Works, subject to normal macOS permissions |
| App and CLI started simultaneously | One process, one status writer |
| Helper with no config file | Runs idle, writes an empty status, no respawn loop |
| Helper with a malformed config | Exits 0 once, file untouched, one line in `~/Library/Logs/git-same/monitor.err.log` |
| Headless (SSH-only) install | State `deferred`, exit 0, starts at next GUI login |
| 321-repository workspace | `starting` for the whole first scan, then `running`, no restarts |
| Signed helper outside the app bundle | Writes the app-group container; Finder badges render; note whether macOS asks for Full Disk Access again (the helper path changed, approvals are not carried over) |
| Roll back to the 3.1.1 cask | Old cask restores an app-bundle LaunchAgent; the next app or CLI run repairs it to the managed helper |
| Dev `toolkit/conductor/run.sh` with a cask monitor present | plist, PID, and `install.json` byte-identical afterwards |
