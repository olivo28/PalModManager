# 🎮 PalModManager (PMM)

> **The Ultimate Next-Gen Mod Manager & Config Editor for Palworld**  
> Built with **Tauri v2**, **Rust**, **TypeScript**, and **Vite** for blazing-fast performance and ultra-low memory footprint.

---

## 🌟 Overview

**PalModManager (PMM)** is a modern, lightweight, and feature-rich desktop application designed to easily manage, install, configure, and edit mods for **Palworld** on **Steam** and **Xbox Game Pass (PC)**.

Whether you're installing complex UE4SS mods, PalSchema mods, Pak mods, or LogicMods, PalModManager automatically detects structure, installs dependencies, handles updates, and provides a built-in interactive JSON code editor for mod configuration files.

---

## ✨ Core Features

### 📦 Smart Multi-Type Mod Installer & Dynamic Previews
- **Auto-Detection**: Recognizes mod structures automatically (`UE4SS`, `PalSchema`, `Pak`, `LogicMods`, `Hybrid`).
- **Interactive File Preview Tree**: Open a collapsible tree viewer in both single and batch mod installers to inspect ZIP contents and installation targets in real-time before deploying.
- **Smart Heuristic Unpacking**: PMM uses a custom heuristic layout analyzer. No matter how deeply nested or disorganized a ZIP file is packaged, PMM dynamically flattens and reorganizes it into the correct directories.
- **Bypass Platform Wrappers**: Skip platform-specific folder tags like `(STEAM)`, `(XBOX)`, `Win64`, `WinGDK` and helper directories (like `"UE4SS mods folder"`) seamlessly during unpacking.
- **Batch Installation**: Drag and drop single or multiple ZIP, `.rar`, or `.7z` files to preview mod details, check versions, and install in bulk.
- **Pak Destination Selector**: Choose between standard `~mods` and `LogicMods` targets for Paks in both single/batch installers and the mod details tab.
- **Installed Version Comparison**: Displays your currently installed version side-by-side with the ZIP file version when updating existing mods.

### 🔀 Dynamic UE4SS Load Order Manager
- **Sidebar Load Tab**: Manage mod loading sequences interactively with a drag-and-drop ordering interface.
- **State Transition Sync**: Automatically manages turning on/off the load order setting, transitioning configuration states between `enabled.txt` and `mods.txt` dynamically based on your preferences.
- **Robust Cleanups**: Deleting a mod removes all instances and references from `mods.txt` automatically.

### 🛠️ Dedicated Mod Packer & Builder (Build Tab)
- **Visual Projects Hub**: Create, rename, delete, and stashing mod packaging projects as folders.
- **Route Manifesting (`modinfo.pmm.json`)**: Package your mods with custom target routing, versioning, author tags, and Nexus IDs. Saves layout manifests into a dedicated `modinfo.pmm.json` file inside the ZIP.
- **Staging Tree Drag & Drop**: Drag and drop files to stashed workspace nodes to re-arrange their destination paths inside the archive.
- **Pre-installed Mod Metadata Scan**: Scanned pre-installed mods automatically parse `modinfo.pmm.json` if available to retrieve rich metadata (Version, Author, Description, Nexus ID) without manual input.

### 🔍 Mod Conflict & Compatibility Scanner (Scan Tab)
- **Engine Hook & Table Collision Detection**: Analyzes enabled mods to identify table row overlaps (PalSchema overrides) and hook overlaps (multiple mods hooking the same engine function in Lua).
- **Side-by-Side Split View**: Renders Lua hook conflicts on the left and PalSchema table overlaps on the right.
- **Lua Hotkeys Manager**: Scan active Lua scripts for keybind configurations (`RegisterKeyBind`), display them in an interactive table, highlight conflicts, and inline edit the key combinations.

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
- **Database Grid Inspector (DB Tab)**: Advanced database inspector to view and edit Mods, Profiles, and Settings tables with raw JSON record validation.

---

## 💻 Tech Stack

- **Frontend**: HTML5, Vanilla CSS (Modern Dark Mode with Glassmorphism), TypeScript, Vite
- **Backend**: Rust, Tauri v2
- **Storage**: Encrypted JSON database for persistent profiles and mod settings

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
- [pnpm` or `npm`
- [Rust & Cargo](https://rustup.rs/) (latest stable)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/olivo28/PalModManager.git
cd PalModManager

# Install frontend dependencies
npm install

# Run in Development Mode
npm run tauri dev

# Build Production Release (.exe & Installer)
npm run tauri build
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

## 📄 License

Distributed under the MIT License. See `LICENSE` for details.

---

Developed with ❤️ by **Olivo28**
