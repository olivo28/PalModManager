@echo off
setlocal
cd /d "%~dp0\.."

echo === PalModManager - Windows Release Build ===
call pnpm tauri build
if %ERRORLEVEL% neq 0 (
    echo [Fallback] Attempting build with npx tauri...
    call npx tauri build
)

echo.
echo Build completed. Packages in: src-tauri\target\release\
pause
