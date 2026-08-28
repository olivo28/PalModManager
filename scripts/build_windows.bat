@echo off
echo === PalModManager - Release Build ===
npx tauri build
echo.
echo Build completed. EXE in: src-tauri\target\release\bundle\msi\
