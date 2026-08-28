# Changelog

All notable changes to this project will be documented in this file.

## [1.7.0] - 2026-08-28

### Added
- **Save Health Doctor & World Hub**: Complete diagnostic and repair suite for Palworld savegames under the **Scan** tab.
  - Automatically discovers world saves across Steam, Xbox / Game Pass (WinGDK), and Linux Proton.
  - Validates GVAS binary integrity and detects orphaned mod references causing startup crashes.
  - 1-click automatic backup and save sanitizer to safely rescue corrupted or crashing saves.
  - Backup snapshot history with side-by-side diff comparison, vanilla detection, and 1-click restore.
  - Interactive inspection for `WorldOption.sav` multipliers, player roster, and storage analytics.
- **Deep Pak & UAsset Package Inspector**: Pure-Rust inspection of `.pak` archives (`repak`) and Unreal Engine binary assets (`unreal_asset`).
  - Explore virtual file trees directly in the Detail Panel and Mod Installer with instant search.
  - Deep `.uasset` / `.uexp` inspector modal displaying export classes, import dependencies, and name table tokens.
  - Added DirectDraw Surface (DDS) texture metadata inspection card with dimensions, mip levels, and DXGI format.
- **Nexus Mods Creator Badge & Metrics**:
  - Integrated `✔ Mod Author` badge in profile header for creators.
  - Real-time aggregation of total accumulated downloads across all authored mods.
- **Config Editor Enhancements**: Smart auto-indentation on `Enter`, live cursor position (`Ln X, Col Y`), folder state persistence, and unsaved changes indicator (`●`).
- **Expanded Syntax Highlighting**: Added native syntax highlighting for `.ini`, `.cfg`, `.toml`, `.yaml`, `.xml`, `.py`, and `.md` files.
- **Automated Pak Compatibility Patch Engine**: Detects colliding asset nodes across `.pak` mods and generates merged priority compatibility patches (`zzz_Patch_*.pak`) with 1 click.
- **Xbox Game Pass & PC WinGDK Compatibility**: Full detection and 1-click `.utoc` / `.ucas` table-of-contents generation (`retoc`) for Game Pass mod support.
- **Altermatic / Dynamic Mod Alteration Engine**: Framework integration for runtime alteration manifests and rule-based asset overrides.
- **Full 6-Language Localization**: Complete translation coverage across English, Spanish, Portuguese, Simplified Chinese, Japanese, and Korean.

### Changed
- **Comprehensive Codebase Modularization & Refactoring**: Restructured and granularized large monolithic files across frontend and backend into dedicated, focused domain modules to significantly improve code readability, testability, and long-term maintainability.
- **Real-Time Settings Reactivity**: Mod visibility and behavior settings (including "Hide UE4SS native mods") now apply and re-render the view instantly without requiring an app reload.
- **Optimized Mod Installer Layout**: Compacted and balanced the preview card layout and config diff view in single-mod installation wizards.
- **Decoupled Syntax Highlighting Engine**: Rendered editor tokens with `requestAnimationFrame` and line-count gutter caching for fluid, lag-free typing.
- **Standardized Backend Observability**: Added structured logging for mod installations, game launching, library syncs, and profile switching.

### Fixed
- **Filesystem Watcher Stability**: Paused filesystem watchers during profile switches to prevent event storms and duplicate disk scans.
- **Window Focus Throttling**: Added a 60-second throttle to window focus checks to eliminate redundant rescans.
- **Database Write Hygiene**: Prevented redundant database disk writes when scanned mod data is unchanged.
- **UI Toast Z-Index Layering**: Fixed toast notifications displaying behind active modal dialogs.
- **Cleaned Obsolete Translations**: Audited and eliminated duplicate translation keys and lingering legacy references across all locale files.

---

### 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.7.0_x64-setup.exe` and follow the setup wizard to install the application on your system.
