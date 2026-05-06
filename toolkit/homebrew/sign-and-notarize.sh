#!/usr/bin/env bash
# Sign + notarize a built git-same binary, emit a release tarball + .sha256.
#
# Usage:
#   sign-and-notarize.sh BINARY_PATH TARGET VERSION OUT_DIR
#
# TARGET is the full Rust target triple (e.g. aarch64-apple-darwin).
# Only darwin targets are accepted; the script signs and notarizes via Apple.
#
# Required env vars (CI provides via GitHub Secrets):
#   APPLE_DEVELOPER_CERTIFICATE_P12       Base64 .p12 (Developer ID Application)
#   APPLE_DEVELOPER_CERTIFICATE_PASSWORD  Password for the .p12
#   APPLE_SIGNING_IDENTITY                Identity name without "Developer ID Application:" prefix
#   APPLE_ID                              Apple ID email
#   APPLE_TEAM_ID                         10-char team ID
#   APPLE_APP_SPECIFIC_PASSWORD           App-specific password for notarytool
#   APPLE_KEYCHAIN_PASSWORD               Random password for the temp build keychain
#
# Output:
#   $OUT_DIR/git-same-${VERSION}-${TARGET}.tar.gz
#   $OUT_DIR/git-same-${VERSION}-${TARGET}.tar.gz.sha256
#
# Stapling is intentionally skipped: stapler staple only works on bundles/.pkg/.dmg.
# Gatekeeper resolves the notarization ticket online on first launch for bare binaries.

set -euo pipefail

if [ $# -ne 4 ]; then
    echo "Usage: $0 BINARY_PATH TARGET VERSION OUT_DIR" >&2
    exit 2
fi

BINARY_PATH="$1"
TARGET="$2"
VERSION="$3"
OUT_DIR="$4"

case "$TARGET" in
    x86_64-apple-darwin|aarch64-apple-darwin) ;;
    *) echo "ERROR: TARGET must be a darwin Rust triple, got '$TARGET'" >&2; exit 2 ;;
esac

if ! [[ "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "ERROR: VERSION must be strict semver (e.g. 3.0.0), got '$VERSION'" >&2
    exit 2
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found: $BINARY_PATH" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

for var in APPLE_DEVELOPER_CERTIFICATE_P12 APPLE_DEVELOPER_CERTIFICATE_PASSWORD \
           APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID \
           APPLE_APP_SPECIFIC_PASSWORD APPLE_KEYCHAIN_PASSWORD; do
    if [ -z "${!var:-}" ]; then
        echo "ERROR: required env var $var is not set" >&2
        exit 1
    fi
done

KEYCHAIN_NAME="git-same-build-$$.keychain"
KEYCHAIN_PATH="$HOME/Library/Keychains/${KEYCHAIN_NAME}-db"
CERT_FILE="$(mktemp -t git-same-cert.XXXXXX).p12"

cleanup() {
    rm -f "$CERT_FILE" || true
    if security list-keychains | grep -q "$KEYCHAIN_NAME"; then
        security delete-keychain "$KEYCHAIN_NAME" || true
    fi
    if [ -f "$KEYCHAIN_PATH" ]; then
        rm -f "$KEYCHAIN_PATH" || true
    fi
}
trap cleanup EXIT

echo "==> Creating temp keychain"
security create-keychain -p "$APPLE_KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security set-keychain-settings -lut 21600 "$KEYCHAIN_NAME"
security unlock-keychain -p "$APPLE_KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security list-keychains -d user -s "$KEYCHAIN_NAME" $(security list-keychains -d user | tr -d '"')

echo "==> Importing Developer ID Application certificate"
echo "$APPLE_DEVELOPER_CERTIFICATE_P12" | base64 --decode > "$CERT_FILE"
security import "$CERT_FILE" \
    -k "$KEYCHAIN_NAME" \
    -P "$APPLE_DEVELOPER_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign \
    -T /usr/bin/security
security set-key-partition-list \
    -S apple-tool:,apple:,codesign: \
    -s -k "$APPLE_KEYCHAIN_PASSWORD" \
    "$KEYCHAIN_NAME" >/dev/null

echo "==> Signing binary"
/usr/bin/codesign \
    --force \
    --options runtime \
    --timestamp \
    --sign "Developer ID Application: $APPLE_SIGNING_IDENTITY" \
    "$BINARY_PATH"

echo "==> Verifying signature"
/usr/bin/codesign --verify --verbose=4 "$BINARY_PATH"

TARBALL_NAME="git-same-${VERSION}-${TARGET}.tar.gz"
TARBALL_PATH="$OUT_DIR/$TARBALL_NAME"

echo "==> Creating tarball $TARBALL_NAME"
tar -czf "$TARBALL_PATH" \
    -C "$(dirname "$BINARY_PATH")" \
    "$(basename "$BINARY_PATH")"

echo "==> Submitting tarball to notarytool"
xcrun notarytool submit "$TARBALL_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --wait \
    --timeout 600

echo "==> Verifying Gatekeeper acceptance"
spctl --assess --type execute --verbose "$BINARY_PATH"

echo "==> Computing SHA256"
SHA_FILE="$TARBALL_PATH.sha256"
shasum -a 256 "$TARBALL_PATH" | awk '{print $1}' > "$SHA_FILE"

echo "==> Done"
echo "    tarball:  $TARBALL_PATH"
echo "    sha256:   $(cat "$SHA_FILE")"
