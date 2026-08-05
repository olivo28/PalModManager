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
- **Top-Right Toast Alerts**: Relocated toast notifications to the top-right corner to prevent overlapping with packer footer controls, adding a smooth slide-down animation and border shadows.
- **Warning Toast Alert Level**: Added warning toasts featuring warning-tailored dark/orange aesthetics and union type support in TypeScript.
