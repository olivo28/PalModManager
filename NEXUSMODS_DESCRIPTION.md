# PalModManager (PMM) - Next-Gen Mod Manager & Config Editor for Palworld

---
Latest version: 1.6.2

Changelog: You can find the latest update notes in the pinned comments section or check the full history on [GitHub](https://github.com/olivo28/PalModManager).

---

## Description
**PalModManager** is a modern, high-performance desktop application built specifically for **Palworld** players and modders on **Steam** and **Xbox Game Pass (PC)**.

Unlike generic mod managers, PalModManager is tailor-made for Palworld: it natively handles all mod structures (**UE4SS**, **PalSchema**, **Pak**, **LogicMods**, and **Hybrids**), features a full **Nexus Mods OAuth 2.0 SSO & in-app Discovery browser**, provides conflict detection, and includes a full-featured built-in JSON configuration code editor.

Built with **Tauri v2** and **Rust**, PalModManager launches instantly and uses minimal RAM and CPU.

---

## Installation instructions
1. Download **PalModManager** from the Files section (choose Portable EXE or Installer).
2. Run **PalModManager.exe**.
3. Open **Settings** (⚙ icon) and select your Palworld installation directory.
4. Log into your Nexus Mods account with 1-click SSO, browse the **Discover** tab, or drag & drop any mod *.zip* or *.rar* file directly into the app window to install!

---

## Main features
- **Full Nexus Mods SSO & Profile Suite**: 1-click OAuth 2.0 authentication. Displays your Nexus avatar, tier, and live metrics (*Endorsements, Tracked Mods, Published Mods*).
- **In-App Mod Discovery Browser (Discover Tab)**: Browse, search, filter, and install Palworld mods directly from Nexus Mods with 1 click without leaving the app.
- **Advanced Discovery Filtering**: Filter across all 13 official Palworld categories (*Gameplay, Pals, Characters, Visuals, etc.*), interactive Tag multi-selectors (*Includes / Excludes* with quick chip removal), search parameters (*Title, Description, Author, Uploader*), language checkboxes, and adult content filters.
- **Rich Mod Details & Media Lightbox**: View formatted mod descriptions with BBCode/HTML parsing, screenshot gallery with mouse-wheel zoom and drag-to-pan lightbox, changelogs, virus scan safety badges (*Verified, Manual, Quarantine*), and categorized downloadable files (*Main, Updates, Optionals, Archived*).
- **Native `nxm://` Protocol Integration**: Associate PalModManager with the `nxm://` protocol for direct 1-click browser downloads from NexusMods.com, complete with a live download queue tray and automated installation.
- **Social Actions & Author Recognition**: Endorse, track/untrack, and access Community/Bugs with automatic author detection (`👑 Author`) to protect against self-endorsement errors.
- **Smart Lua & JSON Config Merging**: Automatically snapshot and merge custom configurations (`config.lua`, `settings.lua`, `config.jsonc`, `settings.json`) across mod updates, preserving your custom keybindings and settings.
- **Ultra-Fast & Lightweight**: Built on Rust and Tauri v2 for instantaneous response times and zero bloat.
- **Smart Mod Auto-Detection**: Automatically identifies and categorizes UE4SS, PalSchema, Pak, and LogicMods.
- **Smart Heuristic Zip Unpacker**: Never worry about how mod files are structured. Generic managers fail when mod authors zip their files with custom structures or nested folders. PalModManager implements a custom layout sanitizer that automatically parses, flattens, and reorganizes any zipped layout into clean game-ready directories before installation.
- **Interactive File Preview Tree**: Open a collapsible tree viewer in both single and batch mod installers to inspect ZIP contents and installation targets in real-time before deploying.
- **Dynamic UE4SS Load Order Manager**: Manage mod loading sequences interactively with a drag-and-drop ordering interface in the sidebar, syncing state transitions between `enabled.txt` and `mods.txt` dynamically.
- **Exclusive PalSchema Load Order Manager (Unique Feature! - Windows Only)**: Control the load order of PalSchema mods! PalSchema natively reads files without any sorting logic. PMM solves this by isolating mods in a `/Storage` directory and creating zero-padded NTFS Junctions (e.g. `001_ModName`, `002_ModName`) inside `/mods` that do *not* require Administrator/UAC permissions.
- **Automated Dependency Management**: Detects missing core dependencies (UE4SS or PalSchema) and offers one-click automatic installation using official GitHub releases.
- **Batch & Drag-and-Drop Installation**: Drop single or multiple zip files to preview mod details, check versions, and install in bulk.
- **Installed Version Warnings**: The upgrade assistant explicitly warns and displays your currently installed version when dragging files to update existing mods.
- **Mod Library**: Maintain a centralized library of downloaded mods complete with auto-fetched NexusMods thumbnails, authors, and summary metadata.
- **Reactive Profile Manager (File Explorer style)**: Create and switch between isolated mod profiles (e.g. Singleplayer, Multiplayer, Hardcore) with instant physical folder deployment. Shows virtual folders as folder cards side-by-side with ungrouped mods, supporting double-click navigation and a right-click custom folder options menu.
- **ID Stability System**: Uses the **Nexus Mod ID** or **Sanitized descriptive names** to identify mods (no random UUIDs), ensuring upgrades and profile matching are 100% stable.
- **Built-in Code Editor**: Edit mod json and jsonc configuration files directly within the application with syntax highlighting, auto-formatting, search (Ctrl+F), and safety checks preventing loss of changes when switching profiles.
- **Settings & Database Inspector**: Advanced database inspector to view and edit Mods, Profiles, and Settings tables with raw JSON record validation, secure credential masking, custom app storage redirection/migration, and toolbar UI scaling (80% to 180%).
- **Modern Dark Mode**: Sleek glassmorphism UI designed for maximum usability and aesthetic clarity.

---

## 🐧 Linux Native Support & Troubleshooting
PalModManager features native compatibility for Linux systems. If you run the Linux version and experience crashes on startup or a blank screen, execute the app using these parameters:

- **Force X11 Backend** (resolves crashes related to Wayland windowing layers in WebKitGTK):
  ```bash
  GDK_BACKEND=x11 ./palmodmanager
  ```
- **Disable DMABUF Rendering** (resolves blank/invisible interface issues caused by graphics driver/Nvidia rendering conflicts):
  ```bash
  WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
  ```
- **Combined execution**:
  ```bash
  GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
  ```

---

## ⚠️ Important Safety & Backup Recommendation
> **Before using PalModManager (or installing mods in general), we strongly recommend manually backing up your game's mod directories.**

While **PalModManager automatically handles and backs up UE4SS/PalSchema/Paks files**, it is always best practice to create a copy of the following folders in your Palworld directory:
- 📁 *Pal/Content/Paks*
- 📁 *Pal/Binaries/Win64/ue4ss* (Steam) or *Pal/Binaries/WinGDK/ue4ss* (Xbox GDK)

---

## 🛡️ Anti-Virus & False Positives Note
This application is built using **Rust + Tauri** for maximum efficiency and security.

Because this is an independent community project without a commercial Code Signing Certificate, some antivirus engines (like Windows Defender or SecureAge) may trigger a **false positive** (e.g., *Trojan:Win32/Wacatac.B!ml* or similar flags).
- **Why this happens**: Heuristic AI and machine learning analysis (*!ml*) frequently flags unsigned executables that perform folder and file system modifications (such as injecting or switching files in your Steam Palworld directories).
- **Is it safe?**: Absolutely. The program is completely clean. If you download it from the official page, there is no threat. 
- **Workaround**: If Windows Defender blocks the manager, click *"More Info"* -> *"Run Anyway"* on the SmartScreen prompt, or add *palmodmanager.exe* to your antivirus exclusions list.

---

## Requirements
- **Game**: Palworld (Steam or PC Game Pass).
- **OS**: Windows 10 / Windows 11 (64-bit) or Linux (Ubuntu, Debian, SteamOS, etc.).
- **Optional Dependencies**:
  - UE4SS (PalModManager can install this for you automatically).
  - PalSchema (PalModManager can install this for you automatically).

---

## 🙏 Credits & Acknowledgements
- **HalRiveria** — Community tester and dedicated bug reporter. Has consistently tracked down and reported hard-to-catch edge cases across multiple releases — including mod update regressions, Force Load Order installation bugs, batch installer failures, and more. His feedback has directly shaped the stability and polish of PalModManager. Thank you!

---

## 💬 Community & Support
Need help, want to report a bug, or suggest a new feature? Join our official Discord community:
* **Discord Community:** [Join Discord Server (AHTDAUwm77)](https://discord.gg/AHTDAUwm77)

---

## Github
[PalModManager](https://github.com/olivo28/PalModManager)
