#!/bin/sh
# Builds WLED Scanner and installs the binary, icon and desktop entry.
# Usage: packaging/linux/install.sh [PREFIX]   (default PREFIX: ~/.local)
set -eu
cd "$(dirname "$0")/../.."
PREFIX="${1:-$HOME/.local}"

cargo build --release
install -Dm755 target/release/wled-scanner "$PREFIX/bin/wled-scanner"
install -Dm644 assets/icon.png "$PREFIX/share/icons/hicolor/256x256/apps/wled-scanner.png"
install -Dm644 assets/icon-512.png "$PREFIX/share/icons/hicolor/512x512/apps/wled-scanner.png"
install -Dm644 packaging/linux/wled-scanner.desktop "$PREFIX/share/applications/wled-scanner.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" || true
echo "Installed to $PREFIX"
