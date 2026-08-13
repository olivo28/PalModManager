@echo off
echo Packaging PalModManager source code (respecting .gitignore)...

:: Remove old zip if it exists
if exist PalModManager_Source.zip (
    del PalModManager_Source.zip
)

:: Use git ls-files to get all tracked and untracked (but not ignored) files,
:: and pipe them to tar to build the zip file.
git ls-files -co --exclude-standard | tar -acf PalModManager_Source.zip -T -

if %ERRORLEVEL% equ 0 (
    echo.
    echo Successfully created PalModManager_Source.zip!
) else (
    echo.
    echo Error: Failed to create zip file.
)

pause
