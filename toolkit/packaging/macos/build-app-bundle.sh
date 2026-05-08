#!/usr/bin/env bash
# Build the git-same macOS app bundle and DMG.

set -euo pipefail

ROOT="${WORKSPACE_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
VERSION="${VERSION:-}"
ARCH="${ARCH:-}"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT/dist/macos}"
INCLUDE_FINDER_EXTENSION="${INCLUDE_FINDER_EXTENSION:-0}"
SKIP_SIGNING="${SKIP_SIGNING:-0}"
SKIP_NOTARIZATION="${SKIP_NOTARIZATION:-0}"

usage() {
    cat <<EOF >&2
Required env vars:
  VERSION                  Strict semver, e.g. 3.1.0
  ARCH                     aarch64 or x86_64

Optional env vars:
  WORKSPACE_ROOT           Repo root (default: auto-detected)
  OUTPUT_DIR               Artifact output directory (default: dist/macos)
  INCLUDE_FINDER_EXTENSION 1 to embed GitSameBadges.appex, 0 for D-App
  SKIP_SIGNING             1 to build unsigned app/dmg for local smoke tests
  SKIP_NOTARIZATION        1 to sign without notarytool/stapler
EOF
}

if [ -z "$VERSION" ] || [ -z "$ARCH" ]; then
    usage
    exit 2
fi
if ! [[ "$VERSION" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "ERROR: VERSION must be strict semver, got '$VERSION'" >&2
    exit 2
fi
case "$ARCH" in
    aarch64|x86_64) ;;
    *) echo "ERROR: ARCH must be aarch64 or x86_64, got '$ARCH'" >&2; exit 2 ;;
esac

TARGET="${ARCH}-apple-darwin"
BUILD_ROOT="$OUTPUT_DIR/build-${ARCH}"
APP="$OUTPUT_DIR/git-same.app"
DMG="$OUTPUT_DIR/git-same-${VERSION}-${ARCH}.dmg"
SIGN_SCRIPT="$ROOT/toolkit/packaging/macos/sign-app-bundle.sh"

mkdir -p "$OUTPUT_DIR"
rm -rf "$BUILD_ROOT" "$APP" "$DMG"
mkdir -p "$BUILD_ROOT"

echo "==> Building CLI ($TARGET)"
( cd "$ROOT" && cargo build --release --target "$TARGET" -p git-same )

echo "==> Installing frontend dependencies"
if command -v corepack >/dev/null 2>&1; then
    corepack enable pnpm
fi
PNPM=(pnpm)
if ! command -v pnpm >/dev/null 2>&1; then
    PNPM=(corepack pnpm)
fi
( cd "$ROOT/crates/git-same-app" && "${PNPM[@]}" --dir ui install --frozen-lockfile )

echo "==> Building Tauri app binary ($TARGET, no bundle)"
TAURI_CLI="$ROOT/crates/git-same-app/ui/node_modules/.bin/tauri"
if [ ! -x "$TAURI_CLI" ]; then
    echo "ERROR: Tauri CLI not found at $TAURI_CLI" >&2
    exit 1
fi
( cd "$ROOT/crates/git-same-app" && "$TAURI_CLI" build --target "$TARGET" --no-bundle )

echo "==> Assembling app bundle"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Helpers" "$APP/Contents/Resources" "$APP/Contents/PlugIns"
cp "$ROOT/target/$TARGET/release/git-same-app" "$APP/Contents/MacOS/git-same-app"
cp "$ROOT/target/$TARGET/release/git-same" "$APP/Contents/Helpers/git-same"
cp "$ROOT/macos/com.zaai.git-same.monitor.plist" "$APP/Contents/Resources/com.zaai.git-same.monitor.plist"
cp "$ROOT/crates/git-same-app/icons/icon.icns" "$APP/Contents/Resources/icons.icns"
chmod +x "$APP/Contents/MacOS/git-same-app" "$APP/Contents/Helpers/git-same"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>git-same-app</string>
  <key>CFBundleIconFile</key><string>icons.icns</string>
  <key>CFBundleIdentifier</key><string>com.zaai.git-same</string>
  <key>CFBundleName</key><string>Git-Same</string>
  <key>CFBundleDisplayName</key><string>Git-Same</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

if [ "$INCLUDE_FINDER_EXTENSION" = "1" ]; then
    echo "==> Building FinderSync extension"
    xcodebuild \
        -project "$ROOT/macos/GitSameBadges.xcodeproj" \
        -scheme GitSameBadges \
        -configuration Release \
        -destination "generic/platform=macOS" \
        SYMROOT="$BUILD_ROOT/xcode-products" \
        OBJROOT="$BUILD_ROOT/xcode-obj" \
        CODE_SIGNING_ALLOWED=NO \
        CODE_SIGNING_REQUIRED=NO \
        build

    APPEX="$BUILD_ROOT/xcode-products/Release/GitSameBadges.appex"
    if [ ! -d "$APPEX" ]; then
        echo "ERROR: FinderSync extension product not found at $APPEX" >&2
        exit 1
    fi
    cp -R "$APPEX" "$APP/Contents/PlugIns/"
fi

if [ "$SKIP_SIGNING" != "1" ]; then
    SIGN_ARGS=()
    if [ "$SKIP_NOTARIZATION" = "1" ]; then
        SIGN_ARGS+=(--skip-notarization)
    fi
    bash "$SIGN_SCRIPT" "$APP" "${SIGN_ARGS[@]}"
fi

echo "==> Creating DMG"
if command -v create-dmg >/dev/null 2>&1; then
    create-dmg \
        --volname "Git-Same ${VERSION}" \
        --window-size 540 380 \
        --icon-size 100 \
        --icon "git-same.app" 140 190 \
        --app-drop-link 400 190 \
        "$DMG" \
        "$APP"
else
    DMG_ROOT="$BUILD_ROOT/dmg-root"
    mkdir -p "$DMG_ROOT"
    cp -R "$APP" "$DMG_ROOT/"
    ln -s /Applications "$DMG_ROOT/Applications"
    hdiutil create -volname "Git-Same ${VERSION}" -srcfolder "$DMG_ROOT" -ov -format UDZO "$DMG"
fi

if [ "$SKIP_SIGNING" != "1" ]; then
    SIGN_ARGS=(--skip-app --dmg "$DMG")
    if [ "$SKIP_NOTARIZATION" = "1" ]; then
        SIGN_ARGS+=(--skip-notarization)
    fi
    bash "$SIGN_SCRIPT" "$APP" "${SIGN_ARGS[@]}"
fi

shasum -a 256 "$DMG" | awk '{print $1}' > "$DMG.sha256"

echo "==> Done"
echo "    app: $APP"
echo "    dmg: $DMG"
echo "    sha256: $(cat "$DMG.sha256")"
