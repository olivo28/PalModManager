# Changelog

All notable changes to this project will be documented in this file.

## [1.6.2] - 2026-08-25

### Added
- **Universal DNS-over-HTTPS (DoH) Image Fallback**: Embedded a resilient multi-stage DoH proxy (`System DNS → Cloudflare 1.1.1.1 → Google 8.8.8.8`) across all app views (Discover, Installed Mods, Library, Detail Panel, Installer, and Nexus Profile) to transparently bypass ISP/firewall image blocking on Nexus CDNs (`staticdelivery.nexusmods.com`), accompanied by disk-backed WebP caching.
- **Network & Sources Settings Tab**: Added a dedicated Network & Sources tab featuring DNS resolver selection (Auto, System, Cloudflare, Google), real-time cache size inspection and 1-click purge, and verified transparency links to official dependency repositories (Okaetsu's UE4SS and PalSchema).
- **Library Installation Filter & Multi-Criteria Sorting**: Added filtering by installation status (All Mods, Installed Only, Not Installed, Updates Available) and flexible sorting (Name A-Z, Installed First, Not Installed First, Date Added) to both Local and Workshop library views.
- **Native 7-Zip & Multi-Codec Decompression Engine**: Embedded pure-Rust `sevenz-rust` and extended zip codec support (LZMA, LZMA2, Bzip2, Zstd) to read and extract `.7z` and `.zip` archives directly without relying on external system tools (`tar.exe`), eliminating `LZMA codec is unsupported` errors on Windows 10/11.
- **Official PalSchema Visual Branding**: Integrated official `{ p }` iconography designed and provided by Okaetsu.

### Fixed
- **External Links in Settings**: Fixed the "View Source ↗" buttons in Settings failing silently by properly routing URLs through the cross-platform native browser opener (`open_url`).
- **Discovery Lightbox ESC Key Handling**: Added keyboard shortcut support (`Escape`) to instantly dismiss the full-screen image zoom viewer (Lightbox) and details modal.
- **Nexus OAuth Auto-Refresh & 401 Expiration Recovery**: Resolved an issue where leaving PMM open for extended periods resulted in 401/402 unauthorized errors in the Discover tab by automatically refreshing tokens on-demand and providing anonymous fallback for public mod queries.
- **Bundled Asset Resolution & Image Flicker Loop**: Fixed an issue where missing bundled static assets in portable builds triggered rapid recursive `onerror` reload loops, causing flickering and broken placeholder icons in Discover and Library mod cards.
- **PalSchema Data Mod Detection Priority**: Improved classification heuristics for PalSchema data mods containing internal definition folders.

---

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
*   **Installer Version:** Download `PalModManager_1.6.2_x64-setup.exe` and follow the setup wizard to install the application on your system.
