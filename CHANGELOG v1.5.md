# Changelog

All notable changes to this project will be documented in this file.

## [1.5.1] - 2026-08-17

This hotfix addresses several issues reported in v1.5.0, improves general stability, and introduces a workaround for the notorious Steam Workshop update delay bug. Instead of having to manually unsubscribe and re-subscribe to mods, PMM now queries the Steam Web API to check mod update timestamps online and lets you trigger Steam's file verification directly to force pending downloads (safely locking the Play button for 2 minutes while Steam finishes in the background).

### Added
- **Multilingual Support (i18n)**: Integrated 6 native languages (English, Español, Português, 简体中文, 日本語, 한국어) with auto-detection and live switching.
- **Multi-Card Drag & Drop**: Select and move multiple mod cards simultaneously into folders or root.
- **About & Legal Modal**: Added info modal accessible from sidebar logo with license, credits, and terms.
- **Official App Branding**: Integrated official PMM logo across the app, taskbar, and tray.
- **Automatic Safety Backup on Launch**: PMM automatically creates a timestamped safety backup of game dependencies (UE4SS / PalSchema binaries and configs) upon launch to prevent file corruption.
- **Pre-Launch Dependency Check**: Game launch verifies UE4SS and PalSchema integrity before starting.
- **Disabled Hybrid Mods Support**: Persistent state tracking for disabled hybrid mods across sessions.
- **Steam Workshop Updates & Verification**: Online timestamp checks and direct Steam file validation trigger to force pending mod downloads.
- **Separated Library Context Menus**: Distinct actions and update checkers for Local Library and Workshop tabs.

### Changed
- **Pointer Drag & Drop**: Overhauled drag mechanics using pointer captures to prevent browser drop conflicts.
- **Installer Layout**: Compacted mode toggle labels and responsive sizing in installation dialogs.
- **Profile Cloning**: Full duplication including disabled mod states and configuration snapshots.

### Fixed
- **Library Install Freeze**: Fixed DOM reference error when installing mods from local library.
- **PalSchema Force Load Order**: Resolved duplicate junction creation and leftover NTFS junction cleanup.
- **Storage Cleanup**: Cleaned up residual storage directories when removing PalSchema or Hybrid mods.
- **Workshop Dependency Deployment**: Fixed file overwriting for updated Workshop dependencies in game folders.
- **General Polishing**: Fixed dark window startup flash, profile isolation leaks, and duplicate toasts.

---

## [1.5.0] - 2026-08-13

### 🔄 Real-Time Reactive Filesystem & Live Editor
- **Native Filesystem Watcher**: High-performance watcher (`notify`) monitoring all mod folders (`Mods`, `Paks`, `PalSchema`, `NativeMods`, `mods-library`).
- **Live Auto-Reloading Config Editor**: Automatically reloads files in real time when modified externally (e.g. in VS Code or Notepad) and updates syntax highlighting.
- **Dynamic File Tree Refresh**: File creations, deletions, and renames outside PMM immediately update the editor's file tree.

### 🎮 Steam Workshop Support & Live Alerts
- **Workshop Integration**: Native scanning, management, and `PalModSettings.ini` integration for Steam Workshop mods (`NativeMods`).
- **New Mod Alerts & Badges**: Live notifications and animated `✨ NEW` badges displayed on newly subscribed mods during their first 10 minutes.
- **Auto-Update Notifications**: Background watcher alerts you whenever Steam downloads updates for subscribed mods.
- **Safe Workshop Updates**: One-click update action with smart config merging to preserve custom settings.

### 📦 Revamped Mod Library & Multi-Version Selector
- **Unified Mod Cards**: Multiple versions of the same mod are now consolidated into a single card with an interactive version dropdown (`v1.4`, `v1.3`, `v1.2`).
- **Dynamic Actions & Badges**: Instant button transitions (`Install`, `Update`, `Rollback`, `Reinstall`) and live status badges based on selected version.
- **Selective Deletion**: `✕` removes only the chosen version archive without deleting others.

### ⚡ Local Library Update Cross-Referencing
- **Local Update Badges**: The Mods tab now detects if a newer version exists in your local library and shows the `▲ Update (vX.X)` badge.
- **One-Click Local Update**: Right-click `⚡ Update Mod (vX.X)` to deploy local archives instantly without re-downloading.

### 🛠️ UI, Installer & Safety Improvements
- **Dependency Install Confirmation**: Added safety prompts before downloading/updating UE4SS or PalSchema to prevent accidental clicks.
- **Persistent Mode Switcher**: Added an interactive `[ Update ] | [ Install as New ]` toggle in the install modal.
- **Codebase Modularization**: Extensive architecture refactor splitting frontend views and backend commands into modular domain subsystems for faster load times and stability.
- **Sync Fixes**: Instant library status refresh on install and fixed selection desync in bulk operations.

---

### 🔜 Coming Soon
- **Altermatic Support**: Native integration for Altermatic (and its skin pack ecosystem) is actively being planned. PMM will handle `_LoadList.json` management automatically on install, uninstall, enable/disable, and profile switches. For users who prefer the original workflow, PMM will also support running the `.bat` directly from a built-in console modal — your choice.

---

### 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.5.1_x64-setup.exe` and follow the setup wizard to install the application on your system.
