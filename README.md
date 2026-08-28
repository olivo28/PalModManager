# 🎮 PalModManager (PMM)

> **The Ultimate Next-Gen Mod Manager & Config Editor for Palworld**  
> Built with **Tauri v2**, **Rust**, **TypeScript**, and **Vite** for blazing-fast performance and ultra-low memory footprint.

---

## 🌟 Overview

**PalModManager (PMM)** is a modern, lightweight, and feature-rich desktop application designed to easily manage, install, configure, and edit mods for **Palworld** on **Steam** and **Xbox Game Pass (PC)**.

Whether you're installing complex UE4SS mods, PalSchema mods, Pak mods, or LogicMods, PalModManager automatically detects structure, installs dependencies, handles updates, and provides a built-in interactive JSON code editor for mod configuration files.

---

## ✨ Core Features

### 🌐 Full Nexus Mods SSO & Discovery Browser (Discover Tab)
- **1-Click OAuth 2.0 SSO Authentication**: Securely log into Nexus Mods with a single click. Displays your profile avatar, tier status, and persistent metrics (*Endorsements, Tracked Mods, Published Mods*).
- **Interactive Mod Discovery Hub**: Browse and search the entire Nexus Mods Palworld catalog natively within the application.
- **Advanced Filtering**: Filter by 13 official Palworld categories (*Gameplay, Pals, Characters, Visuals, Scripts, etc.*), interactive Tag multi-select (*Includes / Excludes* with quick remove chips), search parameters (*Title, Description, Author, Uploader*), language checkboxes (*with Hide Translations*), and adult content toggles.
- **Rich Mod Details & Media Lightbox**: View formatted mod descriptions with BBCode/HTML parsing, image galleries with mouse-wheel zoom and drag-to-pan lightbox, changelogs, virus scan safety badges (*Verified, Manual, Quarantine*), and categorized downloadable files (*Main, Updates, Optionals, Archived*).
- **Social Actions & Author Recognition**: Endorse, track/untrack, and access Community/Bugs with automatic author detection (`👑 Author`) to protect against self-endorsement errors.
- **Native `nxm://` Protocol Integration**: Associate PalModManager with the `nxm://` protocol for direct 1-click browser downloads, complete with a live download queue tray and automated installation.

### 📦 Smart Multi-Type Mod Installer & Dynamic Previews
- **Auto-Detection**: Recognizes mod structures automatically (`UE4SS`, `PalSchema`, `Pak`, `LogicMods`, `Hybrid`).
- **Interactive File Preview Tree**: Open a collapsible tree viewer in both single and batch mod installers to inspect ZIP contents and installation targets in real-time before deploying.
- **Smart Heuristic Unpacking**: PMM uses a custom heuristic layout analyzer. No matter how deeply nested or disorganized a ZIP file is packaged, PMM dynamically flattens and reorganizes it into the correct directories.
- **Bypass Platform Wrappers**: Skip platform-specific folder tags like `(STEAM)`, `(XBOX)`, `Win64`, `WinGDK` and helper directories (like `"UE4SS mods folder"`) seamlessly during unpacking.
- **Batch Installation**: Drag and drop single or multiple ZIP, `.rar`, or `.7z` files to preview mod details, check versions, and install in bulk.
- **Pak Destination Selector**: Choose between standard `~mods` and `LogicMods` targets for Paks in both single/batch installers and the mod details tab.
- **Installed Version Comparison**: Displays your currently installed version side-by-side with the ZIP file version when updating existing mods.
- **Smart Lua & JSON Config Merging**: Automatically snapshot and merge custom configurations (`config.lua`, `settings.lua`, `config.jsonc`, `settings.json`) across updates, preserving your custom keybindings and settings.

### 🔀 Dynamic UE4SS Load Order Manager
- **Sidebar Load Tab**: Manage mod loading sequences interactively with a drag-and-drop ordering interface.
- **State Transition Sync**: Automatically manages turning on/off the load order setting, transitioning configuration states between `enabled.txt` and `mods.txt` dynamically based on your preferences.
- **Robust Cleanups**: Deleting a mod removes all instances and references from `mods.txt` automatically.

### 🔗 Exclusive PalSchema Load Order Manager (Unique Feature! - Windows Only)
- **Innovative Folder Redirection**: PalSchema natively loads folders without any sorting logic, making load order impossible. PalModManager introduces a unique, zero-admin folder-redirection system.
- **NTFS Junction Points**: Keeps the physical mod directories isolated in `PalSchema/Storage/` and creates zero-padded NTFS Junctions (e.g. `001_ModName`, `002_ModName`) inside `/mods`. Junctions do *not* require Administrator/UAC permissions.
- **Dual Side-by-Side Panels**: Manage both UE4SS and PalSchema load orders simultaneously in a redone side-by-side flex layout inside the sidebar's **Load** tab, featuring independent scrollable panels.

### 🛠️ Dedicated Mod Packer & Builder (Build Tab)
- **Visual Projects Hub**: Create, rename, delete, and stash mod packaging projects as folders.
- **Route Manifesting (`modinfo.pmm.json`)**: Package your mods with custom target routing, versioning, author tags, and Nexus IDs. Saves layout manifests into a dedicated `modinfo.pmm.json` file inside the ZIP.
- **Staging Tree Drag & Drop**: Drag and drop files to stashed workspace nodes to re-arrange their destination paths inside the archive.
- **Pre-installed Mod Metadata Scan**: Scanned pre-installed mods automatically parse `modinfo.pmm.json` if available to retrieve rich metadata (Version, Author, Description, Nexus ID) without manual input.

### 🔍 Mod Conflict, Keybinds, Pak Inspector & Save Doctor (Scan Tab)
- **Deep Pak & UAsset Package Inspector**: Pure-Rust virtual file tree inspection for `.pak` archives (`repak`) and Unreal Engine binary assets (`unreal_asset`), displaying export classes, dependencies, and embedded name tokens.
- **Automated Pak Compatibility Patch Engine**: Detects colliding asset nodes across `.pak` mods and generates merged priority compatibility patches (`zzz_Patch_*.pak`) with 1 click.
- **Engine Hook & Table Collision Detection**: Analyzes enabled mods to identify table row overlaps (PalSchema overrides) and hook overlaps (multiple mods hooking the same engine function in Lua).
- **Lua Hotkeys Manager**: Scan active Lua scripts for keybind configurations (`RegisterKeyBind`), display them in an interactive table, highlight conflicts, and inline edit the key combinations.
- **Save Health Doctor & World Hub**:
  - **Auto-Discovery & Zero-Latency Hub**: Instant tab switching with automatic savegame discovery across Steam, PC Game Pass (WinGDK), and Linux Proton.
  - **Save Integrity & Orphaned Mod Sanitizer**: Validates GVAS binary integrity, scans `Level.sav` for orphaned asset classes left behind by uninstalled mods, and rescues crashing saves with 1-click automatic backup.
  - **Auto-Backup Snapshot History & Diff Inspector**: Side-by-side comparison modal with live size deltas, in-game day progression, player level differences, RAM payload size, pure vanilla detection, and 1-click restore.
  - **WorldOption.sav Multipliers & Player Roster**: Inspects gameplay multipliers across 5 categories, player character saves (UID, level, captured Pals, Paldeck unlocks), and storage analytics.

### ✎ Config & Schema Editor (Edit Tab)
- **Interactive Syntax Highlighting**: Edit JSON config files (`metadata.json`, `.jsonc`, `.json`) directly inside the manager.
- **Line Numbering Gutter**: Line number gutter synchronized with editor scrolling.
- **Safety Change Checks**: Prompts you to save or discard unsaved changes before switching profiles, preventing work loss.
- **JSONC File Support**: Comments in `.jsonc` files are preserved on save while validation checks are performed on stripped versions.

### 📚 Profile Manager & Mod Library
- **Isolated Profiles**: Set up separate mod sets for single-player, co-op, or vanilla play. Switch profiles with real-time physical directory synchronization.
- **Profile Duplication & Purging**: Quickly clone or wipe profile lists, backup directories, and active game deployments.
- **Virtual Mod Folders (Explorer-Style)**: Organize mods in directories with double-click navigation, breadcrumb bars, and context menus (Rename, Delete, Toggle mods).
- **Nexus Integration**: Auto-fetches thumbnails, downloads, endorsements, and descriptions. Includes "Open Updates" to open all Nexus pages for mods with available updates at once, and option to ignore specific update versions.

### ⚙️ Settings & Database Inspector
- **Custom Data Storage Redirection**: Redirection of app storage files (profiles, library, backups) to custom folders or portable executable directories with automated migration.
- **Tauri Window Persistence**: Remembers and restores window size, position, and maximized state across runs.
- **Toolbar UI Scaling**: Resize main workspace toolbars from 80% to 180% via settings range slider.
- **Database Grid Inspector (DB Tab)**: Advanced database inspector to view and edit Mods, Profiles, and Settings tables with raw JSON record validation and secure credential masking.

### ⚡ Internal PMM-Core Reactive Framework & Telemetry
- **Zero-Virtual DOM Performance**: Custom internal reactive engine (`src/framework/`) providing compile-time type safety across all 11 UI domain scopes with instant, micro-second execution.
- **Strongly-Typed Event Mesh**: Pub/sub event bus (`bus`) decoupling cross-subsystem interactions (packers, installers, scanners, profiles) without cyclic dependencies.
- **Live Download Velocity & ETA Telemetry**: Real-time sliding-window throughput calculator rendering dynamic download speed (`⚡ MB/s`) and estimated completion time (`⏳ ETA`) badges in the NXM download tray.
- **Cross-Platform Cache Hygiene**: Integrated fast cache cleaner script (`pnpm clean`) to maintain a lean developer and runtime footprint.

---

## 💻 Tech Stack

- **Frontend**: HTML5, Vanilla CSS (Modern Glassmorphism Dark Mode), TypeScript, **PMM-Core Framework**, Vite 8 (Rolldown)
- **Backend**: Rust, Tauri v2
- **Storage**: JSON database with secure credential isolation for persistent profiles and mod settings

---

## 🚀 Installation & Usage

### Standard Installer
1. Download the latest `PalModManager_Setup.exe` (or `.msi`) from the [Releases](https://github.com/olivo28/PalModManager/releases) tab.
2. Run the installer and launch PalModManager.
3. On first launch, open **Settings** (⚙) and select your Palworld installation directory.

### Portable / Standalone Executable
If you prefer not to install anything:
- Download `palmodmanager.exe` from the Release section.
- Place `palmodmanager.exe` anywhere on your PC and run it directly!

### 🐧 Linux Troubleshooting
If you are running PalModManager on Linux (native compiled or AppImage) and encounter crashes on startup or a blank screen:
- **Force X11 Backend** (resolves crashes related to Wayland windowing layers in WebKitGTK):
  ```bash
  GDK_BACKEND=x11 ./palmodmanager
  ```
- **Disable DMABUF Rendering** (resolves blank/invisible interface issues caused by graphics driver incompatibilities, common with Nvidia/Intel drivers):
  ```bash
  WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
  ```
- **Combined execution**:
  ```bash
  GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
  ```

---

## 🛠️ Building from Source

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+ recommended)
- [pnpm](https://pnpm.io/)
- [Rust & Cargo](https://rustup.rs/) (latest stable)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/olivo28/PalModManager.git
cd PalModManager

# Install frontend dependencies
pnpm install

# Run in Development Mode
pnpm tauri dev

# Build Production Release (.exe & Installer)
pnpm tauri build
```

The compiled binaries will be generated at:
- **Standalone Portable `.exe`**: `src-tauri/target/release/palmodmanager.exe`
- **Installer Setup**: `src-tauri/target/release/bundle/nsis/`

---

## ⚠️ Important Safety & Backup Recommendation

> **Before using PalModManager (or installing mods in general), we strongly recommend manually backing up your game's mod directories.**

While **PalModManager automatically handles and backs up PalSchema files**, it is always best practice to create a copy of the following folders in your Palworld directory:
- 📁 `Pal/Content/Paks`
- 📁 `Pal/Binaries/Win64/ue4ss` *(Steam)* or `Pal/Binaries/WinGDK/ue4ss` *(Xbox)*

---

## 🛡️ Anti-Virus & False Positives Note

This application is built using **Rust + Tauri** for maximum efficiency and security.

Because this is an independent community project without a commercial Code Signing Certificate, some antivirus engines (like Windows Defender or SecureAge) may trigger a **false positive** (e.g., `Trojan:Win32/Wacatac.B!ml` or similar flags).

- **Why this happens**: Heuristic AI and machine learning analysis (`!ml`) frequently flags unsigned executables that interact with other file systems (such as injecting or modifying files in your Steam Palworld directories).
- **Is it safe?** Absolutely. The program is completely clean. If you download it from the official page, there is no threat. 
- **Workaround**: If Windows Defender blocks the manager, click *"More Info"* -> *"Run Anyway"* on the SmartScreen prompt, or add `palmodmanager.exe` to your antivirus exclusions list.

---

## Contact me

If you want to contact me, you can do it in Discord with my tag: **olivo28**
I'm in the Nexus Mods discord, Palworld Modding Community and PalSchema

---

## 🙏 Credits & Acknowledgements

- **HalRiveria** — Community tester and dedicated bug reporter. Has consistently tracked down and reported hard-to-catch edge cases across multiple releases — including mod update regressions, Force Load Order installation bugs, batch installer failures, and more. His feedback has directly shaped the stability and polish of PalModManager. Thank you!

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for details.

---

Developed with ❤️ by **Olivo28**
Special thanks to **HalRiveria** for extensive testing and bug reporting.
