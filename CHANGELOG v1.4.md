# Changelog

All notable changes to this project will be documented in this file.

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
*   **Installer Version:** Download `PalModManager_1.4.0_x64-setup.exe` and follow the setup wizard to install the application on your system.
