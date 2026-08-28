#!/bin/bash
echo "=== PalModManager - Linux Release Build ==="
npx tauri build
echo ""
echo "Build completed. AppImage / Debian package in: src-tauri/target/release/bundle/"
