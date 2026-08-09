# Changelog

All notable changes to this project will be documented in this file.

## [1.4.1] - 2026-08-09

### Added
- **Direct Editor Code Navigation**: Added a `Code` action button next to hotkeys in the scanner view. Clicking it immediately opens the built-in Config Editor, switches to the target mod and script, and scrolls/focuses directly to the registration line.
- **Segregated Self-Conflicts (Internal Duplicates)**: Modified the conflict scanner to distinguish between conflicts between different mods and internal duplicates within a single mod. Self-conflicts are now grouped in a dedicated collapsible warning panel.

### Improvements & Corrections
- **Excluded Dynamic Variables from Hotkey Conflicts**: Prevented false hotkey warning conflicts on dynamic keybind configurations that reference variables (e.g., config lookups) instead of static keys.
- **Robust JSONC/JSON Parser**: Enhanced JSON parsing to automatically strip UTF-8 Byte Order Marks (BOM) and ignore empty/whitespace-only files, resolving generic parser warnings (e.g., `expected value at line 1 column 1`).
- **Zip Folder Traversal & Batch Leaf Filtering**: Excluded non-folder leaves (such as `install.bat`) from mod root selection heuristics and corrected nested archive wrapper traversal.
- **Unwrap Panic Prevention in Installer**: Resolved a Tokio thread panic in the ZIP extractor when processing empty paths or entries without valid filenames.
- **Smart Mod Name & Nexus ID Extraction Heuristics**: Re-engineered name-cleaning algorithms in the backend and frontend to scan from right-to-left. This allows correct identification of Nexus Mod IDs at the end of the filename (even if they are single digits like `1`), while ignoring prefix numbers added by users (e.g., `1 - ModName`).
- **Companion Mod Registry Cleanup**: Ensured companion/extra UE4SS subfolders are properly removed from `mods.txt` when a primary mod is deactivated or uninstalled.
- **Expanded Backup & Restore Routines**: Added logic to target `LogicMods` in backups and preserve custom companion folder structures during deactivation/restoration.
- **Vastly Optimized Config Searches**: Implemented a 300 match cap on search results within the configuration editor to prevent UI freezing on generic search strings.
- **Source Zip Persistence**: Ensured source archives are copied to the library during manifest-based mod installations.

## [1.4.0] - 2026-08-08

### Features
- **Exclusive PalSchema Load Order Manager (Unique Feature! - Windows Only)**: Enables full control over PalSchema mods loading sequence through an innovative folder-redirection system using **NTFS Junctions** without requiring administrator/UAC privileges. Physical directories are kept isolated in a `/Storage` folder (preventing double-loading from the game engine) while creating sorted zero-padded junctions (`001_`, `002_`, etc.) under `/mods`. 
- **Dynamic UE4SS Load Order Manager (Experimental)**: Added a new **Load** tab inside the sidebar to manage mod loading sequences interactively with drag-and-drop support.
- **Side-by-Side Dual Load Order Panels**: Redesigned the "Load" tab to show two independent columns side-by-side with separate scrollbars, allowing you to organize UE4SS and PalSchema mods concurrently with full visual clarity.

### Redesigned Installer
Replaced the legacy file-heuristic installer with a robust, metadata-first packaging parser. It reads the local package manifest (`modinfo.pmm.json` / `.pmm.json`) to accurately extract mod metadata (version, author, description, and Nexus Mod ID) and maps complex multi-directory routes (such as hybrid files deploying concurrently to UE4SS and PalSchema) flawlessly.

### Added
- **Interactive File Preview Tree**: Added a collapsible file tree viewer in both single and batch installers to inspect ZIP contents and installation targets before deploying.
- **State Transition Sync**: Automatically manages turning on/off the load order setting, transitioning configuration states between `enabled.txt` and `mods.txt` dynamically.

### Improvements & Corrections
- **Fixed Scrollbars in Scanner Views**: Added scrolling capability to the conflict scanner and hotkeys manager panels.
- **Refined Folder-Name and Platform Wrapper Heuristics**: Prevents directory names from being overwritten by mod manager names (fixing flat and DLL-only mods), and skips platform wrapper directories like `(STEAM)` and `(XBOX)`.
- **Intelligent Packaging Manifest (`modinfo.pmm.json`)**: Mod Packer now supports saving custom Nexus IDs and outputs route manifests into a dedicated `modinfo.pmm.json` file inside the ZIP.
- **Pre-installed Mod Metadata Scan**: Scanned pre-installed mods now parse `modinfo.pmm.json` if available to retrieve rich metadata (Version, Author, Description, Nexus ID) automatically.
- **Robust modclean-up**: Deleting a mod now cleans up all instances of its folder name from `mods.txt` to avoid trace entries.
- **Prevented Duplicate Sidecar Metadata**: Sidecar metadata generation is now limited to `.pak` files, avoiding duplicate `.pmm.json` files for `.ucas`/`.utoc` files.
- **Enhanced Hybrid Mod Support**: Corrected restoration routing for hybrid mod companion directories and verified load order updates match their physical folders.
- **Installer Layout Consolidation**: Consolidated Pak targets in batch installs to simplify folder management.

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.4.1_x64-setup.exe` and follow the setup wizard to install the application on your system.
