# Reliable macOS monitor installation, startup, and recovery

Status: implemented in 3.1.2. This records the decisions that are not obvious from the code.

## Shape

One controller, `git_same_core::macos::monitor_agent`, serves the Homebrew cask installer, the CLI, and the Tauri app. Monitoring runs as a separate helper copy under a per-user LaunchAgent (`com.zaai.git-same.monitor`), so it survives closing or moving `Git-Same.app`. No root daemon.

## Decisions and why

- **Runtime lock outside `macos::`.** `monitor::runtime_guard` is cross-platform because the double-monitor bug (a second monitor unlinks the first one's socket and both write `status.json`) exists on Linux too. The lock is taken before anything is written; the socket is still bound only after the first scan, so "lock held, socket absent" means `starting` and the Finder wire format is unchanged.
- **A PID is never proof.** A recorded PID is trusted only when the runtime lock is held and the process start identity matches. `status.json`'s `daemon_pid` is display data; it stays in the file because the Swift reader requires it.
- **Config reload instead of restart.** Automatic recovery leaves a healthy monitor alone (PID unchanged), so it cannot deliver "a workspace was registered". The monitor reloads config on `REFRESH_ALL` and when the file changes before a full scan; registry-changing commands are "ensure, then refresh".
- **Exit-code contract of the managed helper.** `KeepAlive = { SuccessfulExit = false }` restarts every unsuccessful exit. Disabled monitoring, a malformed config, a redirected environment, or another running monitor therefore exit 0 after one log line; a missing config means defaults (an idle monitor), which is what makes a fresh cask install work before any configuration exists.
- **Stop is persistent twice.** `monitor.autostart = false` in the config and `launchctl disable`. Automatic paths and the cask installer never call `launchctl enable`; only explicit Start and Restart do. `gisa reset` of the global config stops first, so the launchd half survives the config deletion.
- **Settings saves are merges.** The app used to rebuild the whole config from the form plus defaults, which would have reset `autostart` on every save (and did reset `[ui] custom_folder_icon`). Saves are now `toml_edit` merges of only the keys the form models, under the control lock, and the write DTO has no `autostart`.
- **Source selection looks only at the resolved executable path.** No `PATH` or `~/.cargo/bin` search: whatever is recorded gets executed by launchd at every login. App bundles are recognized by `Info.plist`, Homebrew formula kegs map to the stable `opt` path, `homebrew_cask` ownership is written only by `--install-agent`, a different caller never replaces a healthy helper, and automatic recovery refuses binaries inside a cargo `target/` directory.
- **Isolation.** `UserContext::resolve` is the only reader of the environment and refuses uid 0, `--config`, `GIT_SAME_CONFIG_DIR`, and a `HOME` that is not the passwd home. CLI hooks live only in `run_command` and the TUI entry, because handler unit tests call handlers in-process with a fake `HOME`.
- **Cask.** `installer script:` is unsandboxed (the sandboxed `*_steps` cannot reach launchd) and executes before `app` is moved, hence the staged executable, the explicit final app path, and the retained `git-same-service-tool` for uninstall. `uninstall launchctl:` and `delete:` are gone: they run before the script, probe with sudo, and would bypass the owner check. Uninstall and upgrade always run the previously installed cask's stanza, so the first upgrade from 3.1.1 still runs the old one; the installer copes with a plist that was deleted or points into the app bundle.
- **Packaging protocol.** `monitor --agent-protocol-version` prints `1` with no side effects. S3 refuses to render the cask against a release whose bundled CLI does not answer it.

## Not verified by automated tests

Real launchd and Homebrew behavior, signed-helper access to the app-group container, and whether macOS asks for Full Disk Access again for the relocated helper. See section 8 of `toolkit/packaging/release-checklist.md`; run it only in a disposable macOS account or VM.
