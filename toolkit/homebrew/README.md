# toolkit/homebrew

Templates and helper scripts that render the git-same Homebrew tap entries
(`zaai-com/homebrew-tap`). Used by `S3-Publish-Homebrew.yml` and runnable
locally for pre-publish smoke tests.

## What gets published

| Source template | Rendered to (on the tap) | Audience |
|---|---|---|
| `cask.rb.tmpl`         | `Casks/git-same.rb`        | macOS users (signed + notarized tarball, GUI-friendly) |
| `formula-cli.rb.tmpl`  | `Formula/git-same-cli.rb`  | Linux users + headless macOS |

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
  also `brew install --cask` end-to-end (downloads the real tarballs from the
  release; only run after the release exists).

## Local pre-publish smoke

After the GitHub release exists for the new version:

```sh
VERSION=3.0.2
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
