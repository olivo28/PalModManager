# Changelog

All notable changes to this project will be documented in this file.

## [1.3.1] - 2026-08-07

### Added
- **Remember Window Size**: The manager now remembers and restores your window size, position, and maximized state across sessions using Tauri's window events and AppSettings.
- **Batch Pak Destination Selection & Technical Overrides**: Choose between `~mods` and `LogicMods` folder targets for Pak/LogicMods/Hybrid mods in both the batch installation modal and the mod details "Technical" tab. Mod updates will now properly preserve their target folder layout.
- **Lua Hotkeys Manager**: Added a new "Hotkeys" sub-tab inside the **Scan** workspace, allowing players to scan all active Lua script files for `RegisterKeyBind` calls, display them in an interactive table, highlight collision conflicts (when two mods share the same hotkey), and inline edit the key combinations.
- **Ignore Specific Updates**: Omit specific update versions for individual mods. Clicking "Ignore" on an available update banner will cache the version and hide notifications for it. Users can easily revert this by clicking "Unignore" in the mod details version panel.
- **Open All Nexus Update Pages**: Added a dynamic "🌐 Open Updates" button in the toolbar that automatically shows up when one or more mods have available updates. Clicking it opens all corresponding NexusMods pages in your default browser at once for convenient manual downloading.
- **Interactive Dependency Prompts**: When installing a mod with missing dependencies (like UE4SS or PalSchema), the manager automatically prompts to download and install them. Conversely, attempting to uninstall a core dependency now warns the user if there are active mods that depend on it.
- **Quick-Access Folder Navigation**: Added an "Open Folder" group to the main workspace background right-click context menu, allowing players to instantly open active UE4SS Mods, PalSchema Mods, and Pak folders in Explorer.
- **Toolbar UI Scaling**: Added a dedicated range slider under the Settings panel to customize the visual scale of all main top toolbars (mods, library, config editor, build packer, database inspector) from 80% to 180% with live slider previews and automatic persistence.

### Fixed
- **Native Mod Filtering**: Added the `adapters` mod to the internal core dependencies registry. It will now be properly hidden from the main list when the "Hide Native/Core Mods" settings checkbox is active.
- **Editor Sidebar Sorting**: Fixed the configuration tree list inside the **Edit** workspace panel to sort all active mods alphabetically within their respective type groupings (UE4SS, PalSchema, and Hybrid) instead of arbitrary filesystem order.
- **New Profile Disabled Mods Isolation**: Fixed a bug where switching profiles/syncing state erroneously matched disabled mods from other profiles (causing them to auto-load). Also fixed a bug where stale copies of disabled mods in the profile's active backup directories caused them to be physically restored and reactivated on profile switch. The manager now isolates disabled mods correctly and cleans up stale backup files dynamically.
- **Library Hot-Reload on Profile Switch**: Fixed a bug requiring an app restart to load the library in a newly created or switched profile. Switching profiles now automatically triggers a library refresh.
- **Selective Library Deletion**: Corrected the delete action in the Mod Library to only delete the specific zip version selected, rather than purging all archived versions of that mod by deleting its entire directory.
- **Installer Rename Integration**: Fixed a bug where custom names supplied during installation were not used to define the directory name on disk (`safe_folder_name`), causing split-mod packages sharing the same inner folder name to overwrite each other. The installer now correctly respects custom names on disk and generates unique mod IDs.
- **Nested Script Enrouting (True Sixth Party)**: Fixed a bug where files nested inside directories like `Scripts/adapters/` were extracted directly to the root mod folder. The UE4SS root detector now correctly climbs up the folder tree to determine the true grandparent mod root.
- **Same Nexus ID Sub-package Overwrites**: Fixed a detection bug where installing multiple options from the same Nexus mod page (sharing a Nexus ID) mistakenly registered as an update collision and overwrote the previously installed folder. Folder-based mods now require exact folder name matches to trigger updates.

## [1.3.0] - 2026-08-05

### Added
- **Build & Package Tool (Mod Packer)**: Added a new **Build** workspace tab inside the manager, allowing creators to organize, structure, and bundle mod files into clean, compliant archive formats.
  - **Visual Projects Hub (Dashboard)**: A dedicated grid workspace representing stashed mod packaging projects as folders (📁). Supports creating, double-click loading, renaming, and deleting stashes.
  - **Auto-Scanning Directory Stashing**: Persists source directories and file paths along with target overrides instead of flat file snapshots. Re-scans active local directories automatically upon load, picking up new files seamlessly.
  - **Staging Tree Drag & Drop**: Visual List View and Collapsible Tree View of staged files, allowing files to be dragged and dropped into parent folder nodes to re-arrange their destination paths inside the archive.
  - **Auto-Structuring Heuristics**: Automatically organizes loose files into compliant folder layouts based on Mod Type (UE4SS, PalSchema, Pak, LogicMods, Hybrid) and name (e.g. mapping Lua to `Mods/<mod_name>/Scripts/` and Pak to `Pal/Content/Paks/~mods/`).
  - **Standardized Metadata Generator**: Automatically generates a standard `modinfo.json` metadata file containing Name, Version, Author, and Description.
  - **Multi-Format Archiving**: Compresses the final workspace into clean, correctly structured `.zip` (natively in Rust), `.7z`, or `.rar` (utilizing system CLI tools) files.
- **Mod Conflict & Compatibility Scanner**: Added a new **Scan** tab that analyzes enabled mods to identify table collisions (PalSchema row overrides) and hook overlaps (multiple mods hooking the same engine function in Lua).
  - **Side-by-Side Split View**: Renders Lua hook conflicts on the left and PalSchema table/row overlaps on the right as collapsible cards for clear, sequential visibility matching loading orders.
  - **Active Mod Registries Content Summary**: Added a collapsible bottom registry indexing all active row edits and engine hook signatures per mod, with dynamic type badges (UE4SS, PalSchema, Hybrid), and supporting 2-column grid rendering.
  - **Configuration & Asset Exclusions**: Excludes asset-only mods (`.pak` / `.ucas` / `.utoc` files) and internal manager metadata (`.pmm.json`) from the scanning registry.
  - **Dynamic Hybrid Path Resolution**: Automatically scans and associates both physical deployment folders (the main UE4SS directory and the PalSchema/mods/ subdirectory) for Hybrid mods, resolving content correctly.
- **Internal Database Inspector & Editor**: Added a new **DB** workspace tab for advanced users to inspect and debug the live application state.
  - **Dynamic Tabbed Data Grid**: Supports browsing active tables (Mods, Profiles, and Settings) via an interactive grid layout.
  - **Live Raw JSON Editor**: Edit selected database records with inline validation to ensure proper format before saving.
  - **Re-scan Integration**: Prompting confirmation modal after saving edits to trigger a full mod directory re-scan to immediately reflect changes.
- **Top-Right Toast Alerts**: Relocated toast notifications to the top-right corner to prevent overlapping with packer footer controls, adding a smooth slide-down animation and border shadows.


---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.3.1_x64-setup.exe` and follow the setup wizard to install the application on your system.

