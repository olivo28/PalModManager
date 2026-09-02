# Changelog

All notable changes to this project will be documented in this file.

## [1.7.1] - 2026-09-02

### Added
- **Monaco Code Editor & IntelliSense**: Full-featured in-app editor with Palworld C++ reflection autocomplete, live game Blueprints autocompletion (20,921 classes extracted directly from .pak), PalSchema DataTables & Row autocompletion (423 tables, 149k rows), live diagnostics dock, 1-click QuickFixes, and in-editor file/folder creation.
- **Custom Mod Notes**: Save personal notes, guides, and keybindings directly on any mod card via the detail panel and context menu.
- **Dual UE4SS Activation Mode**: Choose between standard folder isolation (`enabled.txt`) and direct `mods.txt` control, with clean category formatting.
- **USMAP Reflection Explorer**: Browse and inspect Palworld engine structs, classes, enums, and properties directly in the Database Inspector (`🗄 DB`).
- **C++ SDK Manager**: Manage and index Palworld SDK headers in Settings with 1-click cloud sync and local folder import.
- **Conflict Mitigation & Hotkey Tools**: 1-click disable/edit actions on conflict cards and intelligent hotkey collision detection with automatic rebind to free keys.
- **Mod Packer Enhancements**: Hierarchical tree-table file list view, Vortex folder preset, and 1-click dual packaging for Steam and Xbox.
- **Full 6-Language Localization**: Complete translation coverage across all new tools, editor features, and menus.

### Changed
- **Editor Canvas**: Clean full-height editor workspace with native keyboard shortcuts (`Ctrl + S`, `Shift + Alt + F`) and zero-latency mod switching.
- **Database & USMAP Performance**: Vastly faster search response times and fixed scrollable layouts.
- **Non-Destructive `mods.txt` Handling**: Preserves custom comments, headers, and manual load orders cleanly.
- **Codebase Optimization**: Thorough internal modularization for faster loading and long-term stability.

### Fixed
- **Precise Line Navigation**: Clicking "Edit" or "Preview" in diagnostics now jumps straight to the exact script line with a brief highlight.
- **Nexus Details Modal**: Fixed unresponsiveness when clicking "Update Mod" on installed mods.
- **Engine Reflection & Hook Diagnostics**: Improved C++ method parsing and eliminated false-positive warnings on Blueprint assets.
- **Steam Workshop & Profile Stability**: Fixed 0-mod counts, toggle sync in `PalModSettings.ini`, Workshop PalSchema detection, and cross-profile state preservation.
- **Archive & Routing Handlers**: Fixed installation errors for `.7z`/`.rar` files and corrected misrouted files for Hybrid and Altermatic mods.
- **UI & Translations**: Deduplicated component folder buttons, cleaned up bulk selection labels, and fixed missing translations.

---

## [1.7.0] - 2026-08-31

Thanks to the low volume of bug reports this past week, I had the dedicated time to tackle several things that were on my todo list and finish implementing them! Many of them are now implemented, though some have not been 100% field-tested yet. However, the core foundation for many new and very useful features is now ready.

### Added
- **Full Altermatic & UniPalUI Framework Support**: Complete out-of-the-box support for Altermatic runtime replacer mods and UniPalUI, automatically routing `.pak` files to `~mods/` and configs to `SwapJSON/`, managing full physical enable/disable lifecycles per profile, and keeping `_LoadList.json` synchronized automatically.
- **PMM World Backups Vault**: Manage and restore PalModManager world backups with 1-click safety restoration.
- **Co-op Profile Packs**: Export and import complete mod setups with dependencies and configs in a single `.zip` / `.pmmprofile`.
- **Config Merge & Backup Diff**: Visual side-by-side file comparison with smart merge that preserves custom settings during updates.
- **Dependency Version Vault**: Archive and rollback UE4SS and PalSchema versions with 1-click, preserving mod configs.
- **PMM-Core Engine**: Ultra-fast reactive framework with typed DOM elements and instant UI response times.
- **Download Speed & ETA**: Live download speed (`⚡ MB/s`) and remaining time in the Nexus queue.
- **Save Health Doctor**: Detect and clean orphaned mod classes to fix loading crashes, with world settings inspector.
- **Deep Pak & UAsset Inspector**: Pure-Rust inspection of `.pak` files, `.uasset` / `.uexp`, and binary assets.
- **GPU Texture Inspector**: View mod textures and images directly inside the app with channel filters and PNG export.
- **Library Mod Rollback**: Keep older downloaded mod versions to rollback updates anytime with 1-click.
- **Nexus Creator Badges**: Added Mod Author badge and total accumulated download counters.
- **Enhanced Code Editor**: Auto-indentation, line/column counter, and syntax highlighting for various file formats.
- **Unreal Engine Schema Mappings (.usmap)**: Pure-Rust USMAP v4 parser and dynamic GitHub synchronization engine for deep cooked asset inspection and DataTable decoding with offline Steam ACF build detection.
- **Manual Compatibility Patch Builder (Beta)**: Resolve overlapping `.pak` asset collisions by selecting winning assets and compiling unified priority patches (`zzz_PMM_Patch_*_P.pak`) with native `repak` and `retoc`. *(Experimental testing; compatibility depends on mod structure and companions)*.
- **Conflict & Collision Scanner**: Multi-layer detection of Pak asset collisions, PalSchema table collisions, and Lua hook collisions.
- **Xbox Game Pass (IoStore) Support**: Built-in IoStore (`.utoc`/`.ucas`) conversion and detection.
- **Virtual Folder Startup Modes**: Configurable folder behavior on startup (Expanded, Collapsed, Remember).

### Changed
- **Zero-Freeze Async Profile Switching**: Instantaneous profile switching powered by zero-copy NTFS hard-link fast paths and background async execution without UI locking.
- **Faster Startup**: Streamlined initialization and eliminated background rescan lag.
- **Modular Codebase**: Decoupled monolithic files into clean, maintainable domain modules.
- **Instant Settings**: UI toggles now apply immediately without reloading.
- **Streamlined UI**: More compact layouts for installer preview cards and savegame world lists.
- **Structured Logging**: Standardized dual-layer frontend and backend logging.

### Fixed
- **Watcher Storms**: Paused filesystem watchers during profile switches to avoid duplicate rescans.
- **UE4SS Update Checks**: Fixed asset timestamp detection for GitHub releases with updated binaries.
- **Modal Layering**: Fixed toast alerts appearing behind open modals.
- **Database Hygiene**: Prevented redundant database disk writes when metadata is unchanged.
- **Translation Keys**: Audited and fixed missing and duplicate translation keys.

---

### 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.7.1_x64-setup.exe` and follow the setup wizard to install the application on your system.
