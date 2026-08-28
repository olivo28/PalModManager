#!/bin/bash
set -e

# Always navigate to repository root regardless of where the script was invoked from
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "=== PalModManager - Linux Release Build ==="

if command -v pnpm &> /dev/null; then
    pnpm tauri build
else
    echo "[Fallback] Using npx tauri build..."
    npx tauri build
fi

echo ""
echo "Build completed. AppImage / Debian package in: src-tauri/target/release/bundle/"
