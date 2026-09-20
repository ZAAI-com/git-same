# toolkit/homebrew

Templates and helper scripts that render the git-same Homebrew tap entries
(`zaai-com/homebrew-tap`). Used by `S3-Publish-Homebrew.yml` and runnable
locally for pre-publish smoke tests.

S3 publishes two tap entries from one GitHub release: the `git-same` cask from
signed and notarized macOS DMG assets, and the `git-same-cli` formula from CLI
tarballs for Linux and headless macOS.

## What gets published

| Source template | Rendered to (on the tap) | Audience |
|---|---|---|
| `cask.rb.tmpl`         | `Casks/git-same.rb`        | macOS users (signed + notarized DMG, GUI + Finder badges) |
| `formula-cli.rb.tmpl`  | `Formula/git-same-cli.rb`  | Linux users + headless macOS CLI tarballs |

## Decision tree for users

- **macOS GUI / casual install** → cask: `brew install --cask zaai-com/tap/git-same`
- **macOS headless / shell scripts** → formula: `brew install zaai-com/tap/git-same-cli`
- **Linux** → formula: `brew install zaai-com/tap/git-same-cli`

## Scripts

- `render-cask.sh VERSION --sha-arm <hex> --sha-intel <hex> [--out PATH]`
  Renders `cask.rb.tmpl` → stdout or PATH.

- `render-formula.sh VERSION --url URL_PREFIX --sha-macos-arm <hex> --sha-macos-intel <hex> --sha-linux-arm <hex> --sha-linux-intel <hex> [--out PATH]`
  Renders `formula-cli.rb.tmpl` → stdout or PATH.

- `verify-tap.sh --cask cask.rb --formula-cli formula-cli.rb [--install-smoke]`
  Stages rendered files into a throwaway local tap, then runs `brew style`
  and `brew audit --strict --online` against each. Pass `--install-smoke` to
  also `brew install --cask` end-to-end (downloads the real DMG from the
  release; only run after the release exists).

## Local pre-publish smoke

After the GitHub release exists for the new version:

```sh
VERSION=3.1.2
URL_PREFIX="https://github.com/zaai-com/git-same/releases/download/${VERSION}"

# Compute SHAs for the four CLI tarballs used by the formula
for target in aarch64-apple-darwin x86_64-apple-darwin \
              aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
    curl -sSL "${URL_PREFIX}/git-same-${VERSION}-${target}.tar.gz" \
        | shasum -a 256 | awk '{print $1}' | tee "/tmp/sha-${target}.txt"
done

# Compute SHAs for the two macOS DMGs used by the cask
for arch in aarch64 x86_64; do
    curl -sSL "${URL_PREFIX}/git-same-${VERSION}-${arch}.dmg" \
        | shasum -a 256 | awk '{print $1}' | tee "/tmp/sha-${arch}.dmg.txt"
done

# Render
bash toolkit/homebrew/render-cask.sh "$VERSION" \
    --sha-arm   "$(cat /tmp/sha-aarch64.dmg.txt)" \
    --sha-intel "$(cat /tmp/sha-x86_64.dmg.txt)" \
    --out /tmp/cask.rb

bash toolkit/homebrew/render-formula.sh "$VERSION" \
    --url "$URL_PREFIX" \
    --sha-macos-arm   "$(cat /tmp/sha-aarch64-apple-darwin.txt)" \
    --sha-macos-intel "$(cat /tmp/sha-x86_64-apple-darwin.txt)" \
    --sha-linux-arm   "$(cat /tmp/sha-aarch64-unknown-linux-gnu.txt)" \
    --sha-linux-intel "$(cat /tmp/sha-x86_64-unknown-linux-gnu.txt)" \
    --out /tmp/formula-cli.rb

# Verify
bash toolkit/homebrew/verify-tap.sh \
    --cask /tmp/cask.rb \
    --formula-cli /tmp/formula-cli.rb \
    --install-smoke
```

## How the cask installs the monitor

The cask uses `installer script:` to run the bundled CLI from the staged app:

```
Git-Same.app/Contents/Helpers/git-same monitor --install-agent \
    --app-path <appdir>/Git-Same.app \
    --installer-copy <staged_path>/git-same-service-tool
```

- It runs outside the cask sandbox (it has to reach launchd) and before Homebrew moves the app, even though `brew style` requires the stanza to be written after `app`.
- It copies a separate helper to `~/Library/Application Support/com.zaai.git-same/monitor/`, writes the LaunchAgent, and starts monitoring only when it is enabled. It never calls `launchctl enable`, so `gisa monitor --stop` survives upgrades.
- It first retains a copy of itself as `git-same-service-tool` in the Caskroom version directory. `uninstall script:` runs that copy with `--remove-agent`, which removes only a monitor owned by this cask and leaves the start/stop preference alone.
- There is deliberately no `uninstall launchctl:` or `delete:`: both run before the script, probe with sudo, and would bypass the owner check.
- `brew style` and `brew audit` passing says nothing about whether the monitor installs. S3 checks that the released CLI answers `monitor --agent-protocol-version` with the protocol this template needs, and the release checklist has the real acceptance matrix.
- Recovery when the Caskroom version directory was deleted by hand: `brew uninstall --force --cask git-same`, then `gisa monitor --uninstall`.
