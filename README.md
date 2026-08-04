# 🎮 PalModManager (PMM)

> **The Ultimate Next-Gen Mod Manager & Editor for Palworld**  
> Built with **Tauri v2**, **Rust**, **TypeScript**, and **Vite** for blazing-fast performance and ultra-low memory footprint.

---

## 🌟 Overview

**PalModManager (PMM)** is a modern, lightweight, and feature-rich desktop application designed to easily manage, install, configure, and edit mods for **Palworld** on **Steam** and **Xbox Game Pass (PC)**.

Whether you're installing complex UE4SS mods, PalSchema mods, Pak mods, or LogicMods, PalModManager automatically detects structure, installs dependencies, handles updates, and provides a built-in interactive JSON code editor for mod configuration files.

---

## ✨ Core Features

### 📦 Smart Multi-Type Mod Installer
- **Auto-Detection**: Recognizes mod structures automatically (`UE4SS`, `PalSchema`, `Pak`, `LogicMods`).
- **Smart Heuristic Unpacking**: Other managers break if mod files are not zipped in a strict directory structure. PalModManager uses a custom heuristic layout analyzer: no matter how poorly or deeply nested a mod author packed their zip file, PMM dynamically extracts, flattens, and reorganizes it into the correct directory structure before applying it to the game.
- **Drag & Drop**: Simply drop `.zip` or `.rar` files directly onto the app window.
- **Batch Installation**: Preview and install multiple mods simultaneously with clash/conflict detection.
- **Dependency Warnings**: Alerts you if a mod requires **UE4SS** or **PalSchema** before installation and lets you install them in one click.
- **Installed Version Comparison**: Displays your currently installed version side-by-side with the zip file version when updating existing mods.


### 📚 Dedicated Mod Library
- **Central Repository**: Store all your downloaded mods in your local library.
- **Rich Metadata & Nexus Integration**: Auto-fetches thumbnails, author information, descriptions, and mod version numbers directly from **NexusMods**.
- **One-Click Deployment**: Deploy mods from your library into active game profiles with live preview modals.
- **Marquee Drag Selection**: Select multiple mods with rectangle drag-selection, Shift/Ctrl clicks, or checkboxes.
- **ID Stability Integration**: Uses **Nexus Mod ID** or **Sanitized Descriptive Names** as unique IDs (never unstable random UUIDs), securing profile matching, upgrades, and exports across sessions.

### 👤 Profile Manager
- Create and switch between multiple isolated, reactive mod profiles (e.g., *Vanilla*, *Hardcore*, *Multiplayer*, *Singleplayer*).
- **Reactive Real-time Synchronization**: Mods of the profile are restored and backed up dynamically.
- **Automatic ID Migration**: Automatically repairs and cleans legacy UUID structures in your saved profiles on app startup to match stable IDs.

### ✎ Built-in Code & Schema Editor
- **Interactive Syntax Highlighting**: Edit JSON config files (`metadata.json`, `.jsonc`, `.json`) directly inside the manager.
- **Fast Search & Formatting**: Includes built-in `Ctrl+F` search, auto-formatting, and keyboard shortcuts (`Ctrl+S`, `Ctrl+Z`, `Ctrl+Y`).
- **Safety Change Checks**: Prompts you to save or discard unsaved changes before switching profiles, preventing work loss.
- **Smart UI Reset**: Instantly clears active files and paths upon changing profiles to keep the workspace clean.

### 🔄 NexusMods Integration & Updates
- **Update Checks & Metadata Sync**: Automatically check for new mod releases and sync metadata/thumbnails directly from NexusMods.

---

## 💻 Tech Stack

- **Frontend**: HTML5, Vanilla CSS (Modern Dark Mode with Glassmorphism), TypeScript, Vite
- **Backend**: Rust, Tauri v2
- **Storage**: Encrypted SQLite database for persistent profiles and mod settings

---

## 🚀 Installation & Usage

### Standard Installer
1. Download the latest `PalModManager_Setup.exe` (or `.msi`) from the [Releases](https://github.com/olivo28/PalModManager/releases) tab.
2. Run the installer and launch PalModManager.
3. On first launch, open **Settings** (⚙) and select your Palworld installation directory.

### Portable / Standalone Executable
If you prefer not to install anything:
- Download `palmodmanager.exe` from the Release section (or locate `src-tauri/target/release/palmodmanager.exe` after building).
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
- [pnpm](https://pnpm.io/) or `npm`
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
- **Installer Setup**: `src-tauri/target/release/bundle/nsis/` or `bundle/msi/`

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

## 📄 License


Distributed under the MIT License. See `LICENSE` for details.


---

Developed with ❤️ by **Olivo28**
