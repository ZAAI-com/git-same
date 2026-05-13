#!/usr/bin/env bash
# Sign, verify, notarize, and staple a git-same macOS app bundle and,
# optionally, its DMG.
#
# Usage:
#   sign-app-bundle.sh APP_PATH [--dmg DMG_PATH] [--skip-app] [--skip-notarization]

set -euo pipefail

APP_PATH=""
DMG_PATH=""
SKIP_APP=0
SKIP_NOTARIZATION=0

usage() {
    cat <<EOF >&2
Usage: $0 APP_PATH [--dmg DMG_PATH] [--skip-app] [--skip-notarization]

Required env vars:
  APPLE_DEVELOPER_CERTIFICATE_P12
  APPLE_DEVELOPER_CERTIFICATE_PASSWORD
  APPLE_SIGNING_IDENTITY
  APPLE_ID
  APPLE_TEAM_ID
  APPLE_APP_SPECIFIC_PASSWORD
  APPLE_KEYCHAIN_PASSWORD
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --dmg) DMG_PATH="${2:-}"; shift 2 ;;
        --skip-app) SKIP_APP=1; shift ;;
        --skip-notarization) SKIP_NOTARIZATION=1; shift ;;
        -h|--help) usage; exit 0 ;;
        --*) echo "ERROR: unknown flag $1" >&2; usage; exit 2 ;;
        *)
            if [ -z "$APP_PATH" ]; then
                APP_PATH="$1"; shift
            else
                echo "ERROR: unexpected positional arg $1" >&2; usage; exit 2
            fi
            ;;
    esac
done

if [ -z "$APP_PATH" ]; then
    echo "ERROR: APP_PATH is required" >&2; usage; exit 2
fi
if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: app bundle not found: $APP_PATH" >&2; exit 1
fi
if [ -n "$DMG_PATH" ] && [ ! -f "$DMG_PATH" ]; then
    echo "ERROR: DMG not found: $DMG_PATH" >&2; exit 1
fi

for var in APPLE_DEVELOPER_CERTIFICATE_P12 APPLE_DEVELOPER_CERTIFICATE_PASSWORD \
           APPLE_SIGNING_IDENTITY APPLE_ID APPLE_TEAM_ID \
           APPLE_APP_SPECIFIC_PASSWORD APPLE_KEYCHAIN_PASSWORD; do
    if [ -z "${!var:-}" ]; then
        echo "ERROR: required env var $var is not set" >&2
        exit 1
    fi
done

APP_ABS="$(cd "$(dirname "$APP_PATH")" && pwd)/$(basename "$APP_PATH")"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
IDENTITY="Developer ID Application: ${APPLE_SIGNING_IDENTITY}"
KEYCHAIN_NAME="git-same-app-build-$$.keychain"
KEYCHAIN_PATH="$HOME/Library/Keychains/${KEYCHAIN_NAME}-db"
CERT_DIR="$(mktemp -d -t git-same-app-cert.XXXXXX)"
NOTARY_DIR="$(mktemp -d -t git-same-app-notary.XXXXXX)"
CERT_FILE="$CERT_DIR/cert.p12"

cleanup() {
    rm -rf "$CERT_DIR" "$NOTARY_DIR" || true
    if security list-keychains | grep -q "$KEYCHAIN_NAME"; then
        security delete-keychain "$KEYCHAIN_NAME" || true
    fi
    rm -f "$KEYCHAIN_PATH" || true
}
trap cleanup EXIT

echo "==> Creating temp keychain"
security create-keychain -p "$APPLE_KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security set-keychain-settings -lut 21600 "$KEYCHAIN_NAME"
security unlock-keychain -p "$APPLE_KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security list-keychains -d user -s "$KEYCHAIN_NAME" "$(security list-keychains -d user | tr -d '"')"

echo "==> Importing Developer ID certificate"
echo "$APPLE_DEVELOPER_CERTIFICATE_P12" | base64 -D > "$CERT_FILE"
security import "$CERT_FILE" \
    -k "$KEYCHAIN_NAME" \
    -P "$APPLE_DEVELOPER_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign \
    -T /usr/bin/security
security set-key-partition-list \
    -S apple-tool:,apple:,codesign: \
    -s -k "$APPLE_KEYCHAIN_PASSWORD" \
    "$KEYCHAIN_NAME" >/dev/null

sign_app() {
    echo "==> Signing app bundle inside-out"
    # The helper runs the monitor LaunchAgent and reads/writes the
    # app-group container at ~/Library/Group Containers/<APP_GROUP_ID>/.
    # Without application-groups here macOS treats every group-container
    # access as cross-app and shows a TCC AppData prompt attributed to
    # "Git-Same.app".
    /usr/bin/codesign --force --options runtime --timestamp \
        --sign "$IDENTITY" \
        --entitlements "$ROOT/crates/git-same-app/entitlements.plist" \
        "$APP_ABS/Contents/Helpers/git-same"

    if [ -d "$APP_ABS/Contents/PlugIns/GitSameBadges.appex" ]; then
        /usr/bin/codesign --force --options runtime --timestamp \
            --sign "$IDENTITY" \
            --entitlements "$ROOT/macos/GitSameBadges/GitSameBadges.entitlements" \
            "$APP_ABS/Contents/PlugIns/GitSameBadges.appex"
    fi

    /usr/bin/codesign --force --options runtime --timestamp \
        --sign "$IDENTITY" \
        --entitlements "$ROOT/crates/git-same-app/entitlements.plist" \
        "$APP_ABS/Contents/MacOS/git-same-app"

    /usr/bin/codesign --force --options runtime --timestamp \
        --sign "$IDENTITY" \
        --entitlements "$ROOT/crates/git-same-app/entitlements.plist" \
        "$APP_ABS"

    /usr/bin/codesign --verify --deep --strict --verbose=2 "$APP_ABS"
    /usr/sbin/spctl --assess --type execute --verbose "$APP_ABS"

    verify_helper_entitlements
}

verify_helper_entitlements() {
    local helper="$APP_ABS/Contents/Helpers/git-same"
    local expected_group
    expected_group="$(/usr/libexec/PlistBuddy -c \
        "Print :com.apple.security.application-groups:0" \
        "$ROOT/crates/git-same-app/entitlements.plist")"
    local actual
    actual="$(/usr/bin/codesign -d --entitlements - "$helper" 2>&1)"
    if ! printf '%s' "$actual" | grep -q "$expected_group"; then
        echo "ERROR: helper binary is missing the application-groups entitlement." >&2
        echo "  binary: $helper" >&2
        echo "  expected group: $expected_group" >&2
        echo "  codesign output:" >&2
        printf '%s\n' "$actual" | sed 's/^/    /' >&2
        echo "" >&2
        echo "Without this entitlement the monitor LaunchAgent triggers" >&2
        echo "a recurring \"would like to access data from other apps\" TCC" >&2
        echo "prompt whenever it touches the group container." >&2
        exit 1
    fi
    echo "OK: helper signed with application-groups=$expected_group"
}

notarize_app() {
    local zip_path="$NOTARY_DIR/$(basename "$APP_ABS").zip"
    echo "==> Zipping app for notarization"
    /usr/bin/ditto -c -k --keepParent "$APP_ABS" "$zip_path"
    echo "==> Submitting app to notarytool"
    xcrun notarytool submit "$zip_path" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_SPECIFIC_PASSWORD" \
        --wait \
        --timeout 1200
    xcrun stapler staple "$APP_ABS"
    xcrun stapler validate "$APP_ABS"
}

sign_dmg() {
    echo "==> Signing DMG"
    /usr/bin/codesign --force --timestamp --sign "$IDENTITY" "$DMG_PATH"
    if [ "$SKIP_NOTARIZATION" -eq 0 ]; then
        echo "==> Submitting DMG to notarytool"
        xcrun notarytool submit "$DMG_PATH" \
            --apple-id "$APPLE_ID" \
            --team-id "$APPLE_TEAM_ID" \
            --password "$APPLE_APP_SPECIFIC_PASSWORD" \
            --wait \
            --timeout 1200
        xcrun stapler staple "$DMG_PATH"
    fi
}

if [ "$SKIP_APP" -eq 0 ]; then
    sign_app
    if [ "$SKIP_NOTARIZATION" -eq 0 ]; then
        notarize_app
    fi
fi

if [ -n "$DMG_PATH" ]; then
    sign_dmg
fi
