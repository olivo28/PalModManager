# Changelog

All notable changes to this project will be documented in this file.

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
- **Warning Toast Alert Level**: Added warning toasts featuring warning-tailored dark/orange aesthetics and union type support in TypeScript.
