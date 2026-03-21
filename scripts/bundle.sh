#!/usr/bin/env bash
# scripts/bundle.sh - Package the FSKit extension as .appex inside a host .app
#
# Usage: ./scripts/bundle.sh [--install]
#
# Creates:
#   build/mNFTS.app/Contents/Extensions/com.mnfts.extension.appex/
#
# With --install, also registers the extension with LaunchServices.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$ROOT_DIR/build"
SWIFT_BUILD="$ROOT_DIR/swift/.build/release"

APP_BUNDLE="$BUILD_DIR/mNFTS.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APPEX_BUNDLE="$APP_CONTENTS/Extensions/com.mnfts.extension.appex"
APPEX_CONTENTS="$APPEX_BUNDLE/Contents"

echo "=== mNFTS Extension Bundler ==="

# Step 1: Build everything
echo "Building Rust library..."
cd "$ROOT_DIR"
cargo build --release

echo "Building Swift targets..."
cd "$ROOT_DIR/swift"
swift build -c release

# Step 2: Create bundle structure
echo "Creating bundle structure..."
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_CONTENTS/MacOS"
mkdir -p "$APPEX_CONTENTS/MacOS"

# Step 3: Copy binaries
cp "$SWIFT_BUILD/mnfts" "$APP_CONTENTS/MacOS/mNFTS"
cp "$SWIFT_BUILD/MNFTSExtension" "$APPEX_CONTENTS/MacOS/MNFTSExtension"

# Step 4: Copy Info.plists
# Extension Info.plist
cp "$ROOT_DIR/swift/Sources/MNFTSExtension/Info.plist" "$APPEX_CONTENTS/Info.plist"

# Host app Info.plist
cat > "$APP_CONTENTS/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.mnfts.app</string>
    <key>CFBundleName</key>
    <string>mNFTS</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>mNFTS</string>
    <key>LSMinimumSystemVersion</key>
    <string>15.4</string>
    <key>NSSystemExtensionUsageDescriptionKey</key>
    <string>mNFTS needs permission to mount NTFS filesystems.</string>
</dict>
</plist>
PLIST

# Step 5: Ad-hoc code sign
echo "Code signing (ad-hoc)..."

# Sign extension first (inner before outer)
codesign --force --sign "-" --timestamp=none \
    --entitlements "$SCRIPT_DIR/extension.entitlements" \
    --generate-entitlement-der \
    "$APPEX_BUNDLE"

# Sign host app
codesign --force --sign "-" --timestamp=none \
    --entitlements "$SCRIPT_DIR/host.entitlements" \
    --generate-entitlement-der \
    "$APP_BUNDLE"

echo "Bundle created at: $APP_BUNDLE"
echo ""

# Step 6: Optionally install
if [[ "${1:-}" == "--install" ]]; then
    echo "Registering with LaunchServices..."
    LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Versions/Current/Frameworks/LaunchServices.framework/Versions/Current/Support/lsregister"
    "$LSREGISTER" -f -R -trusted "$APP_BUNDLE"

    echo ""
    echo "Extension registered. Enable it in:"
    echo "  System Settings > General > Login Items & Extensions > File System Extensions"
    echo ""
    echo "Verify with:"
    echo "  pluginkit --match --all-versions -vv --identifier com.mnfts.extension"
    echo ""
    echo "Mount an NTFS volume with:"
    echo "  mount -F -t mntfs /dev/diskNsN /Volumes/MyNTFS"
else
    echo ""
    echo "To install, run: $0 --install"
    echo "Or manually: open $APP_BUNDLE"
fi

echo "=== Done ==="
