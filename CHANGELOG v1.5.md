# Changelog

All notable changes to this project will be documented in this file.

> Hey everyone! First off, sorry for the quiet stretch these past few days — I wasn't sitting still, but most of the work was happening behind the scenes. Had to do a lot of research and testing to figure out how to implement Steam Workshop the right way, and along the way it became clear the codebase needed a proper cleanup too (a lot of it had grown into one big monolithic file, which wasn't great). So between those two things, visible updates were slow. On the bright side, the foundation is now a lot cleaner and more solid. Also — **Altermatic support is being planned next**, already reached out to the developer to make sure it gets done right. :)
>
> ⚠️ **For users updating to v1.5:** It is recommended to do a **Profile Clear** before using this version. Sorry about this — several internal changes were made to how profiles handle Workshop mods, and an old profile state could cause unexpected behavior. You can do this from the profile menu inside PMM.
>
> 🧪 **A heads up:** all internal testing was done with a relatively small number of mods. If you're running a large setup, you may encounter edge cases I haven't hit yet — if anything feels off, please report it in the Discord so I can get it sorted.

## [1.5.1] - 2026-08-14

### Added
- **Dependency Update Confirmation**: Added interactive confirmation prompts when newer versions of UE4SS or PalSchema are detected on GitHub, preventing silent background overwrites.
- **Disabled Hybrid Mods Scanning**: Added scanning support for `disabled_mods/hybrid/` to preserve disabled hybrid mods across manager restarts.

### Changed
- **Full Profile Cloning**: Cloning a profile now replicates all data including disabled mods, settings, and snapshots.
- **Atomic Mod Updating**: Mod updates now stage new files and validate installation before removing previous versions to prevent data loss.

### Fixed
- **PalSchema FLO Synchronization**: Fixed NTFS junction and `Storage/` directory handling when enabling or disabling PalSchema mods under Force Load Order (FLO).
- **Mod Name Prefix Cleanup**: Automatically strips internal numeric prefixes (`001_`) from folder names to prevent duplicate entries and scan collisions.
- **Profile Load Order Isolation**: Fixed circular overwrites when switching profiles by strictly backing up and restoring `mods.txt.snapshot` per profile.
- **Update-While-Disabled Routing**: Updating a disabled mod now correctly routes files back to `disabled_mods/` and clears lingering `mods.txt` entries.

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
