# Changelog

All notable changes to this project will be documented in this file.

## [1.2.1] - Hotfixes

### Added
- **Pak Destination Selector for Hybrid Mods**: Added a dropdown/selector in the install preview dialog to choose where `.pak` files inside Hybrid mods are installed (`~mods` or `LogicMods`).
- **Profile Purging (Clear Profile)**: Added a "Clear" button to the profile manager, allowing users to wipe all backing files, configurations, and mod lists from a profile (including the Default profile) without deleting the profile container itself.
- **Explorer-style Details List View**: Added an alternative compact Details List View resembling Windows File Explorer, featuring columns for Name, Status, Type, Version, Installed Path, Extra Files, and Date Installed, with interactive column header sorting.
- **Custom Data Storage Redirection & Migration**: Added support under Settings to choose where database, profiles, and library files are stored (Default AppData, Portable executable folder, or Custom directory) with automatic file migration.
- **Database Renaming and Auto-Migration**: Renamed backing database to `pmm_database.db` and added automatic transparent renaming/migration of legacy `mod-manager.db` files on launch.

### Changed
- **Profile Modal Layout Refinement**: Redesigned the Manage Profiles layout and increased the modal width to 480px to accommodate additional action buttons cleanly.

### Fixed
- **Dependency Warnings False Positive**: Resolved a bug where installing standard mods with metadata JSON files (such as `modinfo.json` in *Smart Breeding Planner*) would show a false "PalSchema is missing" warning.
- **PalSchema Detection Precision**: Restrained automatic PalSchema detection to JSON/JSONC configuration files located inside standard configuration folders (like `pals/`, `spawns/`, `items/`, etc.), preventing false positive type detections.
- **Search Clutter Fix**: Fixed a bug where virtual folders were still rendered in the grid when a search query was active, cluttering search results.
- **Duplicate Mod Cards on Update**: Fixed a bug where updating a Hybrid mod would cause its sub-component folders to be imported as duplicate standalone cards during background disk scans.
- **Duplicate Mod Warning on Pak Updates**: Resolved a bug where updating a `.pak` mod with Nexus suffix identifiers in the ZIP filename failed to detect that the mod was already installed.
- **Platform-Specific Folder Filtering**: Updated the installer to automatically strip platform wrapper folders (like `(STEAM)`, `(XBOX)`, `Win64`, `WinGDK`) and conditionally discard inactive platform files depending on the active game edition (Steam vs Game Pass/GDK).
- **C++ DLL Mod Support**: Improved UE4SS/Hybrid mod detection to recognize C++ mods that only package `.dll` files in a `dlls` folder without any `.lua` scripts.
- **Native UE4SS Mod Toggling**: Resolved a bug where enabling or disabling native UE4SS mods failed to update the `mods.txt` configuration file due to incorrect parent directory paths. The path resolution is now resolved dynamically and relatively to the mod's install folder across all game platforms.


---

## [1.2.0]

### Added
- **7z Archive Support**: Added support for installing mods packaged in the `.7z` format. Both drag-and-drop actions and manual file selection dialogs now natively support `.7z` archives.
- **Linux Compatibility Support**: Added native compilation support for Linux. Gated Windows-only FFI calls behind target conditional compilation and established persistent cross-platform data folders (`~/.local/share/PalModManager` on Linux).
- **Line Numbering Gutter**: Added a line number gutter to the left side of the configuration editor, synchronized with vertical scrolling.
- **Hybrid Mod Editor Navigation**: Added support for viewing, editing, and saving files from both the UE4SS and PalSchema directories within the configuration editor under separate root folders.
- **Profile Cloning / Duplication**: Added a "Clone" button to the profile manager dialog, enabling quick duplication of entire profiles along with their installed/enabled lists and physically backed-up mod files.
- **Virtual Mod Folders (File Explorer style)**: Added virtual folder cards to display directories side-by-side with ungrouped mods in the dashboard. Double-clicking folder cards enters the directory, showing its contents with a breadcrumb navigation bar, while double-clicking mod cards opens their details. Features a dedicated right-click context menu for folder cards to Enter, Rename, Delete, Enable/Disable all mods, and Check for updates.

### Changed
- **Mod Dashboard Interaction**: Modified click behaviors to mimic Windows File Explorer:
  - **Single Click**: Selects/deselects cards (supporting multi-selection, contextual menus, and drag-and-drop operations) without opening the detail panel.
  - **Double Click**: Opens the mod details side panel (for mods) or navigates into the folder directory (for virtual folders).

### Fixed
- **Hybrid Mod Editor Crash Fix**: Fixed a bug where opening the configuration editor for Hybrid mods containing binary `.pak` files in their extra directories would crash with "Error loading files". The backend now safely skips walking binary files.
- **Configuration Editor UX Improvements**:
  - Redesigned the search/find bar to float in the top-right corner, saving screen space and avoiding blocking bottom code lines.
  - Fixed a focus-stealing bug where typing in the find box moved the keyboard cursor back into the code.
  - Fixed selection and highlight overlapping artifacts by adding selection isolation to the highlight layer and correcting raw HTML tag replacement syntax in the search highlight parser.
  - Changed `.md` files to open in edit mode by default, adding a simple toggle next to the save button to switch to rendered HTML preview.
- **Game Detection simplification**: Replaced the unsafe Win32 FFI version checking code in `get_game_version` (which returned the Unreal Engine framework version `5.1.1` instead of the game's version) with a clean cross-platform existence check that returns `"Palworld"` upon successful detection.
- **Mod Update Workflow**: Fixed a bug where updating a mod would corrupt `.pak` file structures or ignore `Hybrid` mod layout separation. The update flow now correctly purges previous files, runs the mod-type specific installation rules, and preserves the active or disabled state.
- **Unified Mod Naming Heuristics**: Unified individual and batch mod installation naming behaviors. Both installation pathways now clean and propose the user-facing name based on the ZIP filename by default (with automatic Nexus metadata fallback if available), instead of pre-filling with the technical inner folder name (`rootFolder`).
- **JSONC File Support in Editor**: Fixed a bug where saving or formatting `.jsonc` files (JSON with comments) in the configuration editor would fail due to standard JSON parsing syntax errors. Comments are now dynamically stripped during syntax validation while keeping them completely intact when saving the file to disk.

