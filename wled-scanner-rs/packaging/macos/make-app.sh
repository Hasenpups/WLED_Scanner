#!/bin/sh
# Builds "WLED Scanner.app" (with icon) in target/release/.
# Usage: packaging/macos/make-app.sh [--universal]
set -eu
cd "$(dirname "$0")/../.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)

if [ "${1:-}" = "--universal" ]; then
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    BIN=target/release/wled-scanner-universal
    mkdir -p target/release
    lipo -create -output "$BIN" \
        target/aarch64-apple-darwin/release/wled-scanner \
        target/x86_64-apple-darwin/release/wled-scanner
else
    cargo build --release
    BIN=target/release/wled-scanner
fi

APP="target/release/WLED Scanner.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BIN" "$APP/Contents/MacOS/wled-scanner"
cp assets/icon.icns "$APP/Contents/Resources/icon.icns"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>WLED Scanner</string>
    <key>CFBundleDisplayName</key><string>WLED Scanner</string>
    <key>CFBundleIdentifier</key><string>de.stefanriese.wled-scanner</string>
    <key>CFBundleExecutable</key><string>wled-scanner</string>
    <key>CFBundleIconFile</key><string>icon.icns</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$VERSION</string>
    <key>CFBundleVersion</key><string>$VERSION</string>
    <key>LSMinimumSystemVersion</key><string>10.13</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
echo "Created $APP"
