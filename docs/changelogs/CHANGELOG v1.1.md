# Changelog

All notable changes to this project will be documented in this file.

---

## [1.1.1] - 2026-07-31

### Fixed
- **Smart Structure-Based Mod Routing**: Implemented dynamic folder signature discovery (detecting `.lua` scripts/`Scripts` folder for UE4SS components, and `.json` files/`pals`/`enums`/`translations`/`raw` folders for PalSchema components) to enroute files to their correct destination directories. This fixes installation for mods wrapped inside custom or non-standard parent directories (e.g. `UE4SS mods folder`).
- **Fixed UI Install Modal Hang**: Fixed a silent Javascript TypeError where checking dependencies crashed the installation promise when files list was not passed, leaving the install button disabled and status text stuck.
- **Network Fetching Timeout**: Added a 10-second timeout to the Nexus API graphql request client to prevent potential infinite connection hangs.

---

## [1.1.0] - 2026-07-31

### Added
- **Single App Instance Enforcement**: Added Tauri single-instance plugin to prevent opening the application multiple times. Starting a new instance now restores, focuses, and unminimizes the currently running app window.
- **Steam / Xbox GDK Platform Detection**: Automatically detects whether Palworld is installed on Steam or Xbox (WinGDK) and resolves target paths accordingly.
- **Priority-Sequenced Dependency Installer**: Added automatic download and installation of required core dependencies inside the mod installer modal. The dependencies are installed in sequence of priority (UE4SS is always installed first, followed by PalSchema).
- **Auto-Installation during Backup Restore**: Restoring a backup ZIP containing UE4SS or PalSchema mods now checks if those dependencies are missing, prompting the user to install them automatically before performing the restore.
- **Mod Backup & Restore**: Added "Backup" and "Restore Backup" buttons. Backups are exported as structured ZIP files categorized under `UE4SS/`, `PalSchema/`, `Paks/`, and `LogicMods/` with metadata filenames (e.g. `PMM_#Mods_YYYY-MM-DD_Backup.zip`).
- **Pre-installed Mod Auto-Import**: Automatically detects and registers pre-existing mods on the first startup or scan, adding them directly into the active profile.

### Fixed
- **Mod Toggling for Optionals & Addons**: Resolved issues where installing a main mod and an optional/addon file from the same mod caused conflicts (only letting you toggle one of them). You can now toggle both main and optional files correctly.
- **Multi-Path Hybrid Mod Extraction**: Fixed support for mod archives containing folders meant for multiple destinations (e.g., packages having both a `.pak` file and a `PalSchema` folder inside).
- **Xbox GDK Path Installation Fix**: Resolved the bug where reinstalling UE4SS on Game Pass versions erroneously targeted Steam-like paths (`Pal/Binaries/Win64`) instead of Xbox-specific paths (`Pal/Binaries/WinGDK/ue4ss`).
- **Profile Synchronization & Persistence**: Fixed a bug where scanning mods did not update or write changes to the active profile's `profile.json` file. Scans now immediately write profile settings to disk.
- **Real-Time Badge Updates**: Fixed dependency check casing bugs to ensure UI toolbar badges (`UE4SS` / `PalSchema`) immediately turn green after successful installation.

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.1.1_x64-setup.exe` and follow the setup wizard to install the application on your system.
