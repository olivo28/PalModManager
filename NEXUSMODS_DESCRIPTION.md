# PalModManager (PMM) - Next-Gen Mod Manager & Editor for Palworld

---

## Description
**PalModManager** is a modern, high-performance desktop application built specifically for **Palworld** players and modders on **Steam** and **Xbox Game Pass (PC)**. 

Unlike generic mod managers, PalModManager is tailor-made for Palworld: it natively handles all mod structures (**UE4SS**, **PalSchema**, **Pak**, and **LogicMods**), provides conflict detection, and includes a full-featured built-in JSON configuration code editor.

Built with **Tauri v2** and **Rust**, PalModManager launches instantly and uses minimal RAM/CPU.

---

## Installation instructions
1. Download **PalModManager** from the Main Files section on NexusMods.
2. Extract the contents or run `PalModManager_Setup.exe` (a portable standalone `.exe` version is also available).
3. Open **PalModManager**.
4. Open **Settings** (⚙ icon) and select your **Palworld** installation directory (Steam or Game Pass).
5. Drag & drop any mod `.zip` or `.rar` file into the manager window to install!

---

## Main features
- ⚡ **Ultra-Fast & Lightweight**: Built on Rust and Tauri v2 for instantaneous response times and zero bloat.
- 📦 **Smart Mod Auto-Detection**: Automatically identifies and categorizes **UE4SS**, **PalSchema**, **Pak**, and **LogicMods**.
- 🧹 **Smart Heuristic Zip Unpacker**: Never worry about how mod files are structured. Generic managers fail when mod authors zip their files with custom structures or nested folders. PalModManager implements a custom layout sanitizer that automatically parses, flattens, and reorganizes any zipped layout into clean game-ready directories before installation.
- 🛠️ **Automated Dependency Management**: Detects missing core dependencies (**UE4SS** or **PalSchema**) and offers one-click automatic installation.
- 📥 **Batch & Drag-and-Drop Installation**: Drop single or multiple zip files to preview mod details, check versions, and install in bulk.

- 🔄 **Installed Version Warnings**: The upgrade assistant explicitly warns and displays your currently installed version when dragging files to update existing mods.
- 📚 **Mod Library**: Maintain a centralized library of downloaded mods complete with auto-fetched NexusMods thumbnails, authors, and summary metadata.
- 👤 **Reactive Profile Manager**: Create and switch between isolated mod profiles (e.g. *Singleplayer*, *Multiplayer*, *Hardcore*) with instant physical folder deployment. Integrates automatic ID migration to cleanly upgrade old profiles into stable IDs.
- 🆔 **ID Stability System**: Uses the **Nexus Mod ID** or **Sanitized descriptive names** to identify mods (no random UUIDs), ensuring upgrades and profile matching are 100% stable.
- ✍️ **Built-in Code Editor**: Edit mod json and jsonc configuration files directly within the application with syntax highlighting, auto-formatting, search (Ctrl+F), and safety checks preventing loss of changes when switching profiles.
- 🔄 **NexusMods API Sync**: Check for mod updates automatically and sync Nexus data using your personal API Key.
- 🌙 **Modern Dark Mode**: Sleek glassmorphism UI designed for maximum usability and aesthetic clarity.

---

## Roadmap / Planned features
- 📥 **Direct Mod Downloader**: Download mods directly into the manager using the NexusMods API (supports premium high-speed & free download rate-limits).
- 🔍 **In-App Nexus Mod Browser**: Search, filter, and explore Palworld mods on NexusMods directly from the app interface.


---

## ⚠️ Important Safety & Backup Recommendation

> **Before using PalModManager (or installing mods in general), we strongly recommend manually backing up your game's mod directories.**

While **PalModManager automatically handles and backs up PalSchema files**, it is always best practice to create a copy of the following folders in your Palworld directory:
- 📁 `Pal/Content/Paks`
- 📁 `Pal/Binaries/Win64/ue4ss` *(Steam)* or `Pal/Binaries/WinGDK/ue4ss` *(Xbox GDK)*

---

## 🛡️ Anti-Virus & False Positives Note

This application is built using **Rust + Tauri** for maximum efficiency and security.

Because this is an independent community project without a commercial Code Signing Certificate, some antivirus engines (like Windows Defender or SecureAge) may trigger a **false positive** (e.g., `Trojan:Win32/Wacatac.B!ml` or similar flags).

- **Why this happens**: Heuristic AI and machine learning analysis (`!ml`) frequently flags unsigned executables that perform folder and file system modifications (such as injecting or switching files in your Steam Palworld directories).
- **Is it safe?** Absolutely. The program is completely clean. If you download it from the official page, there is no threat. 
- **Workaround**: If Windows Defender blocks the manager, click *"More Info"* -> *"Run Anyway"* on the SmartScreen prompt, or add `palmodmanager.exe` to your antivirus exclusions list.

---

## Requirements
- **Game**: Palworld (Steam or PC Game Pass).
- **Operating System**: Windows 10 or Windows 11 (64-bit).
- **Optional**: 
  - [UE4SS](https://github.com/Okaetsu/RE-UE4SS) (PalModManager can install this for you automatically).
  - PalSchema (PalModManager can install this for you automatically).
