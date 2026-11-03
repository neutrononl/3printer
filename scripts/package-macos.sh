#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="3printer.app"
APP_DIR="$ROOT_DIR/dist/$APP_NAME"
ICONSET_DIR="$(mktemp -d "${TMPDIR:-/tmp}/3printer-icon.XXXXXX")"

cleanup() {
	rm -rf "$ICONSET_DIR"
}
trap cleanup EXIT

cd "$ROOT_DIR"

echo "Building release binary..."
cargo build --release

echo "Preparing app icon..."
mkdir -p "$ICONSET_DIR/AppIcon.iconset"
for size in 16 32 128 256 512; do
	sips -z "$size" "$size" assets/app-icon.png --out "$ICONSET_DIR/AppIcon.iconset/icon_${size}x${size}.png" >/dev/null
	double=$((size * 2))
	sips -z "$double" "$double" assets/app-icon.png --out "$ICONSET_DIR/AppIcon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET_DIR/AppIcon.iconset" -o "$ICONSET_DIR/AppIcon.icns"

echo "Assembling $APP_DIR..."
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp target/release/3printer "$APP_DIR/Contents/MacOS/3printer"
cp Info.plist "$APP_DIR/Contents/Info.plist"
cp "$ICONSET_DIR/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
chmod +x "$APP_DIR/Contents/MacOS/3printer"

echo "Created $APP_DIR"
echo "Open it with: open '$APP_DIR'"
