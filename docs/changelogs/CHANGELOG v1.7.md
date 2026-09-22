# Changelog

All notable changes to this project will be documented in this file.

## [1.7.2] - 2026-09-22

### Added
- **Interactive FOMOD Installation Wizard**: Step-by-step modular assistant with image previews, MO2-style choice memory for updates/reinstalls (`🟢 PREVIOUSLY SELECTED`), retroactive choice deduction from disk, multi-encoding support (UTF-16LE/BE, UTF-8 BOM), and tri-source consensus versioning.
- **TOS-Compliant Profile Sharing (`.pmmprofile`)**: Lightweight, copyright-safe profile export sharing only mod metadata, load orders, and user config deltas without redistributing third-party binaries. Includes a missing mods resolver with direct download links and an auto-applying floating helper.
- **Steam Workshop Bridge ("Artificial Workshop Mods")**: Register local and Nexus mods directly into Palworld's native in-game Steam Workshop menu (`Mods/PalModSettings.ini`).
- **Direct Nexus Mods Integration**: Endorse, track, and manage mods directly from the app with instant local caching and new Tracked/Endorsed library filter tabs.
- **Save Health Doctor & GVAS Deep Scanner**: Structural savegame validation against compiled engine catalogs to detect corrupted parameters, orphaned bases, and missing asset references, with atomic world repair and 1-click backup restore.
- **Real-Time UE4SS Log Viewer**: Live streaming log viewer with severity filtering (`info`, `warning`, `error`, `crash`, `mod`), search, line numbers, and direct navigation into the code editor.
- **Native Dynamic UI Scaling (100% Zero-Zoom)**: Fluid proportional scaling (`--ui-scale`) across all views, dialogs, and Monaco editor with Settings slider (85%–140%) and universal hotkeys (`Ctrl +`, `Ctrl -`, `Ctrl 0`).
- **Conflict Scanner ⇄ Monaco Editor Bridge**: Scan diagnostics display directly in Monaco's Problems panel, and in-editor fixes automatically resolve scanner warnings in real-time with non-rotating immutable backups (`.bak`).
- **Headless Worker Process (`--pmm-worker`)**: Dedicated background worker for heavy tasks like deep save scans and sanitization, keeping the interface completely fluid.
- **Reinstall Mod Context Menu Action**: 1-click reinstall from the local library to easily re-trigger FOMOD wizards or restore pristine mod states.

### Changed
- **Clean-Slate Update Pipeline**: Automatically snapshots user configurations and purges obsolete or renamed files from disk when updating directory-based mods (UE4SS and PalSchema), preventing duplicate schema conflicts.
- **Dynamic Zero-Recompile Resource Resolution**: Engine catalogs and mappings now resolve dynamically from manifests and disk discovery without hardcoded build IDs.
- **Linux AppImage Optimization**: Recompressed Linux AppImages with Zstandard (zstd) and removed redundant multimedia frameworks for a substantially lighter bundle.
- **Enhanced Scrollbars**: Widened vertical scrollbars with high-contrast semi-transparent thumb styling across dark and light themes.
- **Active Mods Launch Logging**: Launching Palworld logs an exhaustive, numbered list of active mods, versions, and types into `palmodmanager.log` and console.

### Fixed
- **Same-Nexus-ID Mod Conflicts**: Fixed false conflict detection blocking sequential installation and re-enabling of multi-file mods sharing the same Nexus page ID (e.g. Renamon body variants) by verifying physical disk identity.
- **Orphaned Config Files on Update / Clean Slate**: Previous ghost configs are cleanly purged from disk and snapshots when a newer release drops bundled config files (e.g. SmartPause 1.3.14).
- **PalSchema Internal Files Misclassified as Configs**: Excluded internal `.json` and `.jsonc` blueprint and table files inside PalSchema directories (`blueprints`, `tables`, `schemas`, `templates`) from config archiving and diff alerts.
- **Cross-Platform Linux & SteamOS Stability**: Replaced Windows drive letter backslash globs in asset scope with universal scope, eliminating startup crashes on Linux (Issue #12), fixed non-Windows game launching via official `steam://run/1623730` URI, and decoupled AppImage `LD_LIBRARY_PATH`.
- **Xbox Game Pass Nested `Content\` Directory**: Canonical game root discovery automatically handles Microsoft Store wrapper layouts (`Palworld\Content\Pal\...`).
- **Mod Library Rollback & Metadata Retention**: Prevented Nexus API latest version from overwriting user-selected rollback versions, and prevented empty string overrides from wiping thumbnails, authors, and descriptions.
- **PalSchema Profile Sync**: Fixed `PalSchema ✕` false-uninstalled status when switching profiles via vault-aware sync and live runtime checks.
- **UE4SS Log Viewer Error Truncation**: Large log files (>2000 lines) no longer discard early startup errors or crash entries during truncation.
- **Monaco Editor Ghost "Unsaved Changes"**: Keyed file buffers by `modId::filePath` and purged buffers on mod uninstall, eliminating bogus unsaved changes alerts.
- **Floating Download Tray Positioning**: Reanchored the NXM download tray to the bottom-left, keeping Monaco editor Save and Revert buttons fully accessible.
- **OAuth CSRF Security**: Hardened Nexus OAuth callback with mandatory strict CSRF `state` parameter verification.
- **Safe Hook Detection**: Enhanced hook scanner to recognize defensive Lua wrappers (`safeRegisterHook`, `safeNotifyOnNewObject`) without false positives.

---

## [1.7.1] - 2026-09-09

### Added
- **In-Place Pak Tweaker & Binary Asset Editor**: Directly inspect and modify cooked Unreal Engine properties and DataTable cells inside packed `.pak` mod archives without external tools.
  - **Pristine Safety Backups**: Automatically creates an untouched `.original.bak` copy before the first modification, guaranteeing 1-click rollback anytime.
  - **Interactive DataTable Grid Viewer**: Dedicated tab displaying item stats, drop rates, and pal parameters in a clean searchable table with inline cell editing (`✏️`).
  - **Live Blueprint CDO Comparative Diff**: 5-column table displaying Property Name, Type, Vanilla Default, Mod Value, and 1-click reset to vanilla, showing exactly what a mod changes compared to the unmodded game.
  - **3-Way Noise Filter Toolbar**: Switch instantly between `[ 🎯 Editable Tweaks ]`, `[ ⚡ All Raw Deltas ]`, and `[ 📋 All Properties ]` to focus on what matters.
- **Dedicated Engine Development Resources Hub**: Independent `🛠️ Dev Resources` settings tab with an obsidian Master Manifest banner showing Steam Build ID, game version, and resource counts. Provides a unified 9-resource dashboard for Unreal Mappings (`.usmap`), C++ SDK headers (`Pal.hpp`), property mappings (`.jmap`), and automated dump tools.
- **Per-Profile Mod Library Isolation**: The Local Library now strictly respects your active profile. Mods installed in other profiles cleanly display as "Not Installed" with an active "Install to Profile" button.
- **Steam Workshop Integration Enhancements**:
  - Workshop library cards now show genuine mod version numbers from `Info.json` instead of "unknown", eliminating spurious update warnings.
  - Workshop cards display per-profile installation state with dedicated "Install to Profile" and "Remove from Profile" (🗑️) buttons.
  - Safe removal guards Steam Workshop directories: removing a workshop mod detaches it from the active profile and unlinks runtime junctions without deleting the subscribed files from Steam.
  - Steam Workshop support is strictly restricted to Steam installations (cleanly disabled on Xbox / Game Pass).
- **Monaco Code Editor & IntelliSense**: Full-featured in-app code editor with Palworld C++ reflection autocomplete, live game Blueprints autocompletion (>20,000 classes extracted directly from game paks), PalSchema DataTables & Row autocompletion (423 tables, 149k rows), live diagnostics dock, 1-click QuickFixes, and in-editor file/folder creation.
- **Custom Mod Notes**: Write and save personal notes, load order reminders, and custom keybindings directly on any mod card via the detail panel and context menu.
- **Dual UE4SS Activation Mode**: Choose between standard folder isolation (`enabled.txt`) and direct `mods.txt` control, with clean category formatting.
- **USMAP Reflection Explorer**: Browse and inspect Palworld engine structs, classes, enums, and properties directly in the Database Inspector (`🗄 DB`).
- **C++ SDK Manager**: Manage and index Palworld SDK headers in Settings with 1-click cloud sync and local folder import.
- **Conflict Mitigation & Hotkey Tools**: 1-click disable/edit actions on conflict cards and intelligent hotkey collision detection with automatic rebind recommendations.
- **Mod Packer Enhancements**: Hierarchical tree-table file list view, Vortex folder preset, and 1-click dual packaging for Steam and Xbox.
- **Full 6-Language Localization**: Complete translation coverage across all new tools, editor features, and menus (English, Spanish, Portuguese, Simplified Chinese, Japanese, Korean).

### Changed
- **Editor Canvas & Performance**: Clean full-height editor workspace with native keyboard shortcuts (`Ctrl + S`, `Shift + Alt + F`) and zero-latency mod switching.
- **Database & USMAP Performance**: Significantly faster search response times, lower memory overhead, and smooth scrollable layouts.
- **Non-Destructive `mods.txt` Handling**: Preserves custom comments, headers, and manual load orders cleanly during profile sync.
- **Codebase Optimization & Architecture**: Modularized core subsystems to keep all source files lean, testable, and under strict line count limits.

### Fixed
- **Precise Line Navigation**: Clicking "Edit" or "Preview" in diagnostics now jumps straight to the exact script line with a brief highlight.
- **Nexus Details Modal**: Fixed unresponsiveness when clicking "Update Mod" on installed mods.
- **Engine Reflection & Hook Diagnostics**: Improved C++ method parsing and eliminated false-positive warnings on Blueprint assets.
- **Steam Workshop & Profile Stability**: Fixed 0-mod counts, toggle sync in `PalModSettings.ini`, Workshop PalSchema detection, and cross-profile state preservation.
- **Archive & Routing Handlers**: Fixed installation errors for `.7z`/`.rar` files and corrected misrouted files for Hybrid and Altermatic mods.
- **Workshop Auto-Add Rescan**: Prevented background mod scanners from automatically re-adding detached Workshop mods back into active profiles.
- **UI & Translations**: Cleaned up bulk selection labels, deduplicated folder buttons, and resolved missing translation keys.

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
*   **Installer Version:** Download `PalModManager_1.7.2_x64-setup.exe` and follow the setup wizard to install the application on your system.
