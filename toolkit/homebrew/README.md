# toolkit/homebrew

Templates and helper scripts that render the git-same Homebrew tap entries
(`zaai-com/homebrew-tap`). Used by `S3-Publish-Homebrew.yml` and runnable
locally for pre-publish smoke tests.

## What gets published

| Source template | Rendered to (on the tap) | Audience |
|---|---|---|
| `cask.rb.tmpl`         | `Casks/git-same.rb`        | macOS users (signed + notarized tarball, GUI-friendly) |
| `formula-cli.rb.tmpl`  | `Formula/git-same-cli.rb`  | Linux users + headless macOS |
| `formula-shim.rb.tmpl` | `Formula/git-same.rb`      | Existing users on the old name; depends on `git-same-cli`, prints a deprecation warning |

The shim is scheduled for removal at git-same 3.2 (see `formula-shim.rb.tmpl`).

## Decision tree for users

- **macOS GUI / casual install** → cask: `brew install --cask zaai-com/tap/git-same`
- **macOS headless / shell scripts** → formula: `brew install zaai-com/tap/git-same-cli`
- **Linux** → formula: `brew install zaai-com/tap/git-same-cli`
- **Already running `brew install zaai-com/tap/git-same`** → unchanged for now; the shim transparently installs `git-same-cli` and emits a one-time deprecation notice.

## Scripts

- `render-cask.sh VERSION --sha-arm <hex> --sha-intel <hex> [--out PATH]`
  Renders `cask.rb.tmpl` → stdout or PATH.

- `render-formula.sh VERSION --kind {cli|shim} ... [--out PATH]`
  Renders both formula templates. See `--help` for the full flag list per kind.

- `verify-tap.sh --cask cask.rb --formula-cli formula-cli.rb --formula-shim formula-shim.rb [--install-smoke]`
  Stages rendered files into a throwaway local tap, then runs `brew style --strict`
  and `brew audit --strict --online` against each. Pass `--install-smoke` to
  also `brew install --cask` end-to-end (downloads the real tarballs from the
  release; only run after the release exists).

## Local pre-publish smoke

After the GitHub release exists for the new version:

```sh
VERSION=3.0.0
URL_PREFIX="https://github.com/zaai-com/git-same/releases/download/${VERSION}"

# Compute SHAs for the four release tarballs
for arch in aarch64-apple-darwin x86_64-apple-darwin \
            aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
    curl -sSL "${URL_PREFIX}/git-same-${VERSION}-${arch}.tar.gz" \
        | shasum -a 256 | awk '{print $1}' | tee "/tmp/sha-${arch}.txt"
done

# Render
bash toolkit/homebrew/render-cask.sh "$VERSION" \
    --sha-arm   "$(cat /tmp/sha-aarch64-apple-darwin.txt)" \
    --sha-intel "$(cat /tmp/sha-x86_64-apple-darwin.txt)" \
    --out /tmp/cask.rb

bash toolkit/homebrew/render-formula.sh "$VERSION" --kind cli \
    --url "$URL_PREFIX" \
    --sha-macos-arm   "$(cat /tmp/sha-aarch64-apple-darwin.txt)" \
    --sha-macos-intel "$(cat /tmp/sha-x86_64-apple-darwin.txt)" \
    --sha-linux-arm   "$(cat /tmp/sha-aarch64-unknown-linux-gnu.txt)" \
    --sha-linux-intel "$(cat /tmp/sha-x86_64-unknown-linux-gnu.txt)" \
    --out /tmp/formula-cli.rb

# Compute the source archive SHA for the shim
SRC_SHA=$(curl -sSL "https://github.com/zaai-com/git-same/archive/refs/tags/${VERSION}.tar.gz" \
            | shasum -a 256 | awk '{print $1}')
bash toolkit/homebrew/render-formula.sh "$VERSION" --kind shim \
    --deprecation-date "$(date -u +%Y-%m-%d)" \
    --src-sha "$SRC_SHA" \
    --out /tmp/formula-shim.rb

# Verify
bash toolkit/homebrew/verify-tap.sh \
    --cask /tmp/cask.rb \
    --formula-cli /tmp/formula-cli.rb \
    --formula-shim /tmp/formula-shim.rb \
    --install-smoke
```
