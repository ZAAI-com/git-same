# Apple Code Signing & Notarization for macOS Releases

## Context

The macOS binaries (`git-same-macos-x86_64`, `git-same-macos-aarch64`) built by the S2 release workflow are currently unsigned. Since we have an Apple Developer account, we can sign and notarize the binaries for users who download them directly.

### Who is affected by unsigned binaries?

| Distribution channel | Gatekeeper warning? | Signing needed? |
|---|---|---|
| `brew install zaai-com/tap/git-same` | **No** — Homebrew strips quarantine | No |
| Direct download from GitHub Releases (browser) | **Yes** — browser sets quarantine xattr | Yes |
| `curl` / `wget` from GitHub Releases | **No** — no quarantine xattr set | No |

Our S3 pipeline publishes a Formula (bare CLI binary + SHA256), not a Cask (.app). Homebrew's `curl`-based download does not set `com.apple.quarantine`, so **Homebrew users will not see Gatekeeper warnings regardless of signing**. Signing benefits users who download binaries directly from the GitHub Releases page via a browser.

## Workflow Changes

**Single file modified:** `.github/workflows/S2-Release-GitHub.yml`

Add 3 new steps to the `build-release-assets` job, conditional on `runner.os == 'macOS'`, inserted between the existing "Rename binary" and "Upload artifact" steps:

### Step 1: Import signing certificate
- Decode the base64 `.p12` certificate from secrets
- Create a temporary keychain (`build.keychain`), import the cert
- Use `security set-key-partition-list` to allow non-interactive `codesign` access

### Step 2: Sign the binary
```bash
codesign \
  --sign "Developer ID Application: $APPLE_SIGNING_IDENTITY" \
  --options runtime \
  --timestamp \
  --force \
  ${{ matrix.asset_name }}
codesign --verify --verbose=4 ${{ matrix.asset_name }}
```
- `--options runtime` enables Hardened Runtime (required for notarization)
- `--timestamp` embeds a secure timestamp (required for notarization)
- No entitlements file needed — a plain CLI tool has no special capability requirements

### Step 3: Notarize the binary
```bash
zip ${{ matrix.asset_name }}.zip ${{ matrix.asset_name }}
xcrun notarytool submit ${{ matrix.asset_name }}.zip \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_SPECIFIC_PASSWORD" \
  --wait --timeout 300
rm ${{ matrix.asset_name }}.zip
```
- Stapling is not possible for bare binaries (only `.app`/`.pkg`/`.dmg`), but macOS does an online Gatekeeper check on first run which resolves the notarization ticket automatically

### Step 4: Cleanup keychain
- Delete the temporary keychain and restore defaults
- Runs even if previous steps fail (uses `if: always()`)

## Pre-requisite: Create the Developer ID Application Certificate

Before configuring the workflow, you need a "Developer ID Application" certificate (this is the type used for signing software distributed outside the App Store).

1. **Open Xcode** → Settings → Accounts → select your Apple ID → Manage Certificates
2. Click **"+"** → select **"Developer ID Application"** → Create
3. Alternatively via [developer.apple.com/account/resources/certificates](https://developer.apple.com/account/resources/certificates): create a new certificate of type "Developer ID Application", upload a CSR generated via Keychain Access
4. **Export the `.p12`:** Open Keychain Access → find "Developer ID Application: Your Name" → right-click → Export → save as `.p12` with a strong password
5. **Base64-encode it:**
   ```bash
   base64 -i DeveloperIDApplication.p12 | pbcopy
   ```
6. **Create an app-specific password:** Go to [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords → Generate one labeled "git-same notarization"
7. **Find your Team ID:** Go to [developer.apple.com/account](https://developer.apple.com/account) → Membership Details → Team ID (10-character alphanumeric)

## Required GitHub Secrets

Configure these in your repo settings (Settings → Secrets and variables → Actions):

| Secret | Description |
|--------|-------------|
| `APPLE_DEVELOPER_CERTIFICATE_P12` | Base64-encoded `.p12` from step 5 above |
| `APPLE_DEVELOPER_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12` in step 4 |
| `APPLE_SIGNING_IDENTITY` | Full identity string, e.g. `Your Name (TEAMID)` — visible in Keychain Access |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_TEAM_ID` | 10-character Team ID from step 7 |
| `APPLE_APP_SPECIFIC_PASSWORD` | App-specific password from step 6 |

## What Does NOT Change

- Linux and Windows build matrix entries are unaffected (steps are gated on `runner.os == 'macOS'`)
- No new files created (no entitlements plist needed)
- S3 Homebrew workflow unchanged — it already downloads the release binaries, which will now be signed
- `Cargo.toml` release profile (`strip = true`, `lto = true`) is fully compatible with signing

## Verification

1. After implementation, trigger S2 on a tag push
2. Download the macOS artifacts and verify locally:
   ```bash
   codesign --verify --verbose=4 git-same-macos-aarch64
   spctl --assess --type execute --verbose git-same-macos-aarch64
   ```
3. Install via Homebrew tap and confirm no Gatekeeper warnings appear
