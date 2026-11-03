#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT_DIR/dist/3printer-linux"

cd "$ROOT_DIR"
echo "Building release binary..."
cargo build --release

mkdir -p "$APP_DIR"
cp target/release/3printer "$APP_DIR/3printer"
cp assets/app-icon.png "$APP_DIR/app-icon.png"
chmod +x "$APP_DIR/3printer"

cat > "$APP_DIR/3printer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=3printer
Comment=3D printing workspace
Exec=$APP_DIR/3printer
Icon=$APP_DIR/app-icon.png
Terminal=false
Categories=Graphics;Utility;
EOF

echo "Created $APP_DIR"
echo "Double-click $APP_DIR/3printer.desktop to launch without a terminal."
