# Monitor continuity across cask upgrades

Status: **deferred to 4.0.0 / 5.0.0.** Decided during the 3.1.2 pre-release review. 3.1.2 ships
the honest description of today's behaviour (Option A); this document records the change that
would remove the restart, and what has to be true before it can be adopted.

## What happens today

`brew upgrade --cask git-same` does not replace the installation in place. Homebrew's
`Installer#uninstall_existing_cask` runs the **previously installed** cask's `uninstall` stanza
before staging the new one, and its `UPGRADE_REINSTALL_SKIP_DIRECTIVES` skips only `:signal`.
The `uninstall script:` in `toolkit/homebrew/cask.rb.tmpl` therefore always runs, so
`git-same monitor --remove-agent` deletes the helper, the LaunchAgent plist, and `install.json`.
The new cask's `installer script:` then runs `--install-agent`, which recreates all three.

The user-visible result of an upgrade:

- the monitor gets a **new PID**,
- `install.json` is **recreated**, stamped with the new version,
- Finder badges go stale for roughly the length of the swap, then refresh on the first scan,
- the start/stop preference survives, because it lives in `config.toml` and in `launchctl
  disable`, neither of which `--remove-agent` touches.

A consequence worth naming: the idempotent fast path
`MonitorAgentController::cask_install_is_current` (`crates/git-same-core/src/macos/monitor_agent/controller.rs`)
is **unreachable through `brew upgrade`**. It still serves repeated `--install-agent` calls from
other callers, but the "same version, nothing to do" case never fires during an upgrade.

Section 8 of `toolkit/packaging/release-checklist.md` describes exactly this, and the acceptance
row for `brew upgrade` asserts it rather than asserting an unchanged PID.

## The change (Option B)

Scope the uninstall directive so it runs only on a true uninstall, not on an upgrade, using the
`on_upgrade:` qualifier on the cask's `uninstall script:` stanza. The exact DSL spelling must be
checked against the installed Homebrew version at the time; do not copy it from here.

With removal skipped, the upgrade becomes: stage the new app, run `--install-agent`, and let
`cask_install_is_current` compare the retained helper against the staged one. Same version means
a genuine no-op, so the running monitor keeps its PID, plist, and `install.json` and badges never
pause. A different version takes the existing transactional replace path.

## Why it was not done for 3.1.2

1. **It depends on recent Homebrew behaviour.** Users on older Homebrew get whatever their
   version does, so the cask would likely need a Homebrew version floor, and the two paths would
   both have to be supported for a while.
2. **It makes the installer's own hash check load-bearing in production for the first time.** A
   new app paired with a still-running old helper is precisely the mismatch the packaging
   protocol probe exists to prevent. Under Option B, `cask_install_is_current` becomes the only
   thing standing between those two, with no fallback.
3. **It cannot be validated before the release candidate.** Real `brew upgrade` testing needs a
   signed build and a disposable Mac, which are already on the pending-acceptance list. Changing
   the uninstall path days before a release, with no way to exercise it, is the risky direction.

## Before adopting it

- Confirm the `on_upgrade:` spelling and semantics against the Homebrew version in use, and
  decide the minimum Homebrew version the cask requires.
- Run the full section 8 matrix on a disposable Mac, with two rows added: upgrade between two
  builds of the *same* version (expect a true no-op, PID unchanged) and upgrade between two
  *different* versions (expect a surgical replace).
- Add a row for the downgrade and the roll-back-to-an-older-cask paths, which currently rely on
  removal happening.

## Related: the packaging probe proves less than it looks like

Recorded here because it belongs to the same acceptance pass, and is **not** fixed in 3.1.2.

S3's compatibility gate runs `git-same monitor --agent-protocol-version`, which prints the
compile-time constant `PACKAGING_PROTOCOL_VERSION` before anything else runs. It proves the
binary is new enough to know about the private installer interface. It does **not** exercise
`--install-agent`, `install_for_cask`, or the retained-tool copy, so a regression in any of them
still leaves the probe printing `1` and the cask rendering happily.

`toolkit/homebrew/verify-tap.sh` has an `--install-smoke` mode that would actually install the
cask, and S3 does not pass it. That is deliberate: the installer script is unsandboxed and talks
to launchd, and a GitHub runner has no GUI session, so the monitor could only ever land as
`deferred` there. The smoke would add a new way for the publish job to fail while proving little.
Real installer coverage belongs in the section 8 acceptance pass on a disposable Mac.

If this is revisited, the useful version is a smoke test that asserts the version string and the
binary aliases, run somewhere a GUI session exists, not on the publish runner.
