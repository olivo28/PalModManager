# Changelog

All notable changes to this project will be documented in this file.

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
*   **Installer Version:** Download `PalModManager_1.7.0_x64-setup.exe` and follow the setup wizard to install the application on your system.
