@echo off
setlocal
cd /d "%~dp0\.."

echo === PalModManager - Dev Mode ===
call pnpm tauri dev
if %ERRORLEVEL% neq 0 (
    echo [Fallback] Attempting dev mode with npx tauri...
    call npx tauri dev
)
pause
