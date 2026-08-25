# Changelog

All notable changes to this project will be documented in this file.

## [1.6.1] - 2026-08-25

### Added
- **Storage & Cache Management**: Added a dedicated disk storage inspector and cleaner in Settings (under Safety & Backup) to view real-time space used by Temporary Downloads (`PalModManager_Downloads`) and the Local Mod Library (`mods-library`), with 1-click cache purge and folder quick-access.
- **Installed Status Badge in Mod Details**: Added an active `Installed (vX.Y.Z)` badge to the Discovery mod details view when inspecting a mod that is already installed in your manager.
- **Complete Storage Localization**: Full translation coverage across all 6 supported languages (English, Spanish, Portuguese, Simplified Chinese, Japanese, and Korean) for all storage management controls and toast notifications.

### Changed
- **Direct CDN HTTP Download Stream Handling**: Extended the native download pipeline to support direct HTTP/HTTPS CDN endpoints alongside `nxm://` protocol URLs for 1-click Premium downloads.
- **Windows File Lock Release on Downloads**: Added immediate file flushing and handle release upon download completion to prevent OS file write locks during extraction and inspection.
- **Optimized Update Modal Dimensions**: Expanded and balanced the modal width to comfortably fit update notes and config diff tools without visual clutter.

### Fixed
- **Mod Update Hanging on Analysis**: Fixed a runtime `ReferenceError` during version comparison that caused the installation modal to hang indefinitely on "Analyzing mod structure..." during updates.
- **App Startup Self-Termination**: Resolved a duplicate single-instance plugin initialization that was falsely detecting active instances and causing unexpected app closures.
- **Update Modal Preview Card Squishing**: Fixed flexbox shrink behavior on the left-side preview card, preserving its unconstrained layout and image proportions when config merge options are displayed.
- **Discovery Modal Auto-Dismiss on Download**: Ensured the Discovery details view cleanly dismisses when triggering an in-app download and opening the installer.

---

## [1.6.0] - 2026-08-24

### Added
- **Nexus Mods Discovery Tab**: Built-in mod browser for Palworld mods with live search, advanced filtering (Categories, AI Assisted / Quality of Life tags, Adult Content toggle, Time Range), sorting (Trending, Most Endorsed, Most Downloaded, Newest, Recently Updated), and 1-click in-app installation.
- **Rich Mod Details Modal**: Complete mod inspection view featuring BBCode-to-HTML description formatting with spoiler tags and video embeds, categorized files list (Main, Update, Optional, and collapsible Old/Archived files), virus scan verification badges (Verified, Manual, Quarantine), and dedicated changelog tabs.
- **Full Nexus Mods OAuth 2.0 (SSO) Integration**: 1-click authentication with avatar display, secure AES-GCM encrypted token storage, and persistent profile statistics (Endorsements, Tracked mods, and Authored mods).
- **Social Mod Actions & Author Detection**: Endorse, track/untrack, and access Posts/Bugs directly from the app, featuring automatic mod author detection (`👑 Author`) to prevent accidental self-endorsement errors.
- **Native NXM Download Handler (`nxm://`)**: Direct 1-click downloads from Nexus Mods with a real-time download queue tray, transfer speed tracking, and automated multi-component installation.
- **Protocol Association Settings**: In-app switch to register and toggle Windows `nxm://` protocol handling for Palworld.
- **Lua Config Smart Merging (`.lua`)**: Extended the smart configuration merge engine to support `.lua` configs (such as `config.lua` and `settings.lua`), automatically preserving user keybindings and custom settings during mod updates.
- **Expanded Localization**: Full translation coverage across all 6 supported languages (English, Spanish, Japanese, Korean, Portuguese, and Simplified Chinese) for discovery filters, scan states, and dialogs.

### Changed
- **Multi-Component Mod Navigation**: Quick-access component buttons and context menu actions to open specific mod subdirectories directly (UE4SS, PalSchema, Pak, LogicMods).
- **Archive Extraction & Folder Hierarchy Sanitizer**: Improved root detection to prevent flattening internal folders that share the mod's name (e.g. `Scripts\HUDLocator`).
- **Profile-Aware Workshop Tab**: Dynamic Steam Workshop subtab visibility adapting automatically to active profile dependency configurations.

### Fixed
- **PalSchema Load-Order Subfolder Preservation**: Fixed zip extraction routing to preserve numerical load order prefixes (e.g. `000_PassiveTraitExtraction`) without injecting duplicate wrapper folders.
- **UE4SS Nested Path Duplicate Prevention**: Resolved path normalization bug on full game directory zips that caused duplicate `Binaries\Win64\ue4ss\Mods\` nesting.
- **Database Inspector Security & Display**: Masked sensitive OAuth credentials (`accessToken`, `refreshToken`) on screen and enhanced object formatting for nested configuration settings.
- **UI & Theme Polishing**: Fixed contrast, scrollbar visibility, and alignment across Light and Dark themes in the discovery and details views.

---

### 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

### 📦 Installation Instructions

You can choose between the portable version or the full installer:

*   **Portable Version:** Download `palmodmanager.exe`. You can place it in any folder and run it directly without installation.
*   **Installer Version:** Download `PalModManager_1.6.1_x64-setup.exe` and follow the setup wizard to install the application on your system.
