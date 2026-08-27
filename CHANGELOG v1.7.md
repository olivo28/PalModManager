# Changelog

All notable changes to this project will be documented in this file.

## [1.7.0] - 2026-08-26

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
- **Automated Pak Compatibility Patch Engine**: Detects colliding asset nodes across `.pak` mods and generates merged priority compatibility patches (`zzz_Patch_*.pak`) with 1 click.
- **Xbox Game Pass & PC WinGDK Compatibility**: Full detection and 1-click `.utoc` / `.ucas` table-of-contents generation (`retoc`) for Game Pass mod support.
- **Altermatic / Dynamic Mod Alteration Engine**: Framework integration for runtime alteration manifests and rule-based asset overrides.
- **Full 6-Language Localization**: Complete translation coverage across English, Spanish, Portuguese, Simplified Chinese, Japanese, and Korean.

### Changed
- Mod Installer and Detail Panel now allow direct inspection and exploration of `.pak` contents and binary asset metadata.

### Fixed
- Fixed toast notifications displaying behind modal overlays.

---

### 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.7.0_x64-setup.exe` and follow the setup wizard to install the application on your system.

