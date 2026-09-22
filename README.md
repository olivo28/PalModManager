# 🎮 PalModManager (PMM)

> **The Unified Palworld Mod Management, Game-Data Inspection & Development Suite**
>
> A high-performance desktop application for managing, installing, inspecting, configuring, and developing Palworld mods across Steam and Xbox Game Pass PC environments.

[![Version](https://img.shields.io/badge/version-1.7.2-blue)](https://github.com/olivo28/PalModManager/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/olivo28/PalModManager)
[![Palworld](https://img.shields.io/badge/game-Palworld-orange)](https://www.palworldgame.com/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-orange)](https://tauri.app/)

---

## 🌟 What is PalModManager?

**PalModManager (PMM)** is a specialized desktop suite built specifically for the Palworld modding ecosystem.

Rather than acting solely as a conventional mod installer, PMM unites three foundational pillars of the modding lifecycle into a single, cohesive desktop interface:

1. **🎮 Mod Management** — End-to-end mod lifecycle management: 1-click Nexus OAuth SSO downloads, multi-format smart installation, isolated profile sandboxes, non-destructive UE4SS and PalSchema load order sequencing, and dedicated Steam Workshop integration.
2. **🔍 Game-Data Inspection & Diagnostics** — Deep asset analysis: in-app `.pak` and `.uasset` exploration, GPU-accelerated texture previewing, multi-layer conflict detection, hotkey conflict resolution, and binary GVAS save file repair.
3. **🛠️ Native Mod Development & Tooling** — In-place binary tweaking of cooked properties and DataTables within `.pak` archives, a full Monaco IDE with Palworld reflection and PalSchema autocompletion, USMAP reflection exploration, C++ SDK synchronization, and dual-platform mod packaging.

Whether you are a player curating a conflict-free loadout or a mod author authoring complex scripts and schemas, PMM provides the speed, precision, and diagnostic depth required to mod Palworld with confidence.

---

## 🎮 Platform Ecosystem

PMM is engineered to adapt dynamically to your gaming environment:

| Platform | Support Tier | Architecture & Capabilities |
|---|---|---|
| **Steam** | 🟢 **Primary** | Full end-to-end integration: native Steam Workshop management with per-profile state, real-time Steam Build ID tracking, Master Manifest synchronization, Dev Resources auto-detection, zero-admin NTFS junctions, and direct `nxm://` protocol handling. |
| **Xbox Game Pass PC** | 🟢 **Supported** | Dedicated WinGDK path normalization, IoStore `.utoc`/`.ucas` manifest generation, and UE4SS GDK mode. Steam Workshop controls are automatically disabled to prevent invalid environment states. |
| **Linux / Steam Deck** | 🟡 **Proton** | Native desktop build with full save discovery, asset inspection, Monaco editor, and packaging capabilities. Environment flags for Wayland and WebKitGTK rendering are provided out of the box. |

---

## ✨ Architectural Highlights

| | |
|---|---|
| 📦 **Smart Multi-Format Installer** (.zip, .7z, .rar, Hybrid, Altermatic) | 👤 **Isolated Profiles** with sub-50ms switching and virtual folders |
| 🔄 **Nexus Mods OAuth SSO** & live in-app catalog browser | 🧩 **Lifecycle Dependency Management** for UE4SS & PalSchema |
| 🔀 **Non-Destructive Load Orders** (`mods.txt` & zero-admin NTFS junctions) | 🔬 **Pak & UAsset Explorer** with GPU-accelerated texture decoding |
| 🛠️ **In-Place Pak Tweaker** & cooked binary DataTable editor | 🧬 **Live Blueprint CDO Comparative Diff** against vanilla defaults |
| ⚔️ **Multi-Layer Conflict Scanner** (Paks, DataTables, Lua hooks & hotkeys) | 💾 **Save Doctor & World Hub** with GVAS binary integrity validation |
| 📝 **Full Monaco IDE** with EmmyLua & PalSchema IntelliSense | 🏗️ **Dual-Platform Mod Packer** with embedded routing manifests |
| 🗄️ **USMAP & C++ SDK Manager** with cloud manifest synchronization | 🌐 **Universal 6-Language Localization** across all UI surfaces |

---

# 🎮 Pillar 1: Mod Management

PMM provides a robust, fail-safe environment for discovering, installing, ordering, and maintaining Palworld mods.

### 🌐 Nexus Mods Integration
- **OAuth 2.0 SSO Authentication**: Secure 1-click browser login with persistent sessions and automatic token refresh. Displays your profile tier, avatar, tracked mods, and author status.
- **In-App Discovery Catalog**: Search and browse the entire Palworld Nexus library. Filter across 13 official categories, multi-select tags with Include/Exclude logic, and search by title, description, or author.
- **Rich Mod Details**: In-app BBCode and HTML renderer, interactive image lightbox with zoom and pan, changelogs, virus scan safety badges, and categorized file downloads.
- **Direct Downloads & NXM Protocol**: Full `nxm://` protocol registration lets you trigger downloads from your browser directly into PMM. A sliding-window throughput calculator displays real-time transfer speeds (MB/s) and dynamic ETAs before handing archives off to the installer.
- **Social Actions**: Endorse, track, and navigate directly to mod bug trackers and community discussions. Built-in author detection prevents accidental self-endorsements.

### 📦 Smart Multi-Format Mod Installer
- **Broad Archive Support**: Seamlessly extracts `.zip`, `.7z`, and `.rar` archives, including complex multi-codec and hybrid packages.
- **Heuristic Packaging Unpacker**: Normalizes disorganized mod folder structures, automatically bypassing nested wrapper directories such as `(STEAM)`, `(XBOX)`, `Win64`, `WinGDK`, or `UE4SS mods folder`.
- **7-Way Mod Type Auto-Detection**: Inspects incoming file signatures and routes files automatically to their exact game destinations:
  - **UE4SS Mods**: Lua scripts, DLL binaries, and configuration assets to `ue4ss/Mods/`.
  - **PalSchema Mods**: JSON/JSONC data schema overrides to `palschema/mods/`.
  - **Pak Mods**: Cooked binary assets to `Pal/Content/Paks/~mods/`.
  - **LogicMods**: Scripted `.pak` bundles routed to `Pal/Content/Paks/LogicMods/`.
  - **Hybrid Mods**: Multi-target archives containing combinations of Lua, PalSchema, and Pak assets.
  - **Altermatic Replacers**: Runtime mesh and texture replacer mods utilizing `SwapJSON/` configurations.
  - **UniPalUI**: Standalone UI frameworks and widget extensions.
- **Interactive File Preview Tree**: Inspect archive contents, verify projected file destinations, select individual sub-components, and resolve multi-variation installers before writing a single byte to disk.
- **Batch Drag & Drop**: Queue and install dozens of archives simultaneously with atomic progress tracking.

### 🔀 Dual UE4SS & PalSchema Load Order
- **Dual UE4SS Activation Modes**: Switch seamlessly between simple `enabled.txt` toggles and direct `mods.txt` sequencing. The parser is completely non-destructive, preserving developer comments, category headers, and manual load orders.
- **PalSchema NTFS Zero-Admin Junctions**: PalSchema lacks native sorting logic. PMM solves this transparently on Windows using NTFS junction points:
  ```
  PalSchema/
  └── Storage/         ← Physical mod directories (isolated & untouched)
      ├── ModA/
      └── ModB/

  mods/                ← Zero-padded junction aliases (strictly ordered)
  ├── 001_ModA   →   Storage/ModA
  └── 002_ModB   →   Storage/ModB
  ```
  *Requires zero Administrator rights or UAC elevation.*
- **Side-by-Side Visual Management**: Drag and drop mod entries across synchronized dual panels to balance script and schema execution order simultaneously.

### 🧩 Full Lifecycle Dependency Management
- **Automatic Environment Auditing**: Instantly detects installation status, version strings, and deployment modes (Standard, Steam Workshop, Xbox WinGDK) for core frameworks.
- **1-Click Official Installation**: Fetches and installs verified releases of **UE4SS** and **PalSchema** directly from GitHub without disturbing existing user modifications.
- **Dependency Rollback Vault**: Keeps archived versions of dependency frameworks to allow instant rollbacks whenever an experimental release introduces instability.
- **Sidecar Tracking Manifests**: Maintains `ue4ss.pmm.json` and `palschema.pmm.json` metadata to prevent orphaned files during framework updates.

### 👤 Profile Manager & Virtual Mod Folders
- **Instant In-Place Switching**: Switch between completely distinct mod loadouts (`Singleplayer`, `Multiplayer`, `Hardcore`, `Development`) in under 50ms without copying large asset files.
- **Isolated State & Load Orders**: Each profile maintains its own activation list, load order sequence, and mod configurations.
- **Virtual Mod Folders**: Organize extensive mod lists into custom visual folders with breadcrumb navigation, batch toggles, and contextual actions.
- **Profile Packs (`.pmmprofile`)**: Export an entire curated setup—including mods, configs, and dependency specifications—into a single distributable bundle for friends or server communities.

### 🎮 Steam Workshop Integration
- **Strict Steam Environment Gating**: Workshop management is automatically enabled on Steam installations and cleanly disabled on Xbox Game Pass PC to prevent runtime file corruption.
- **Real Version Detection**: Parses inner `Info.json` manifests to report true mod versions rather than generic workshop item IDs.
- **Per-Profile Activation**: Enable, disable, or assign Workshop mods to specific profiles independently.
- **Safe Uninstallation Protection**: Removing a Workshop mod from a profile cleans up its game-level deployment without deleting the underlying Steam Workshop subscription or local download cache.

### 📚 Profile-Aware Mod Library
- **Centralized Archive**: Retains all downloaded mod archives and multiple version revisions for one-click rollback.
- **Profile Awareness**: The library detects whether a mod is active in the current profile. If installed only in another profile, it displays as `Not Installed` with a 1-click `Install to Profile` action.
- **Original Archive Integrity**: Preserves original archive filenames and metadata tokens to ensure accurate historical lineage.

---

# 🔍 Pillar 2: Inspection & Compatibility

PMM includes a comprehensive diagnostic and inspection engine designed to catch mod collisions, syntax errors, and game-breaking overrides before you launch the game.

### 🔬 Pak & UAsset Explorer
- **Inline Node Exploration**: Open and browse cooked `.pak` archives directly without running external extraction tools.
- **Category Filter Chips**: Filter archive contents by asset category (`Blueprints`, `Textures`, `Materials`, `DataTables`, `Audio`, `Other`) with live element counts.
- **Deep UAsset Metadata**: Inspect cooked `.uasset` and `.uexp` binaries across dedicated tabs:
  - **Overview**: Asset summary, class path, and cooked flags.
  - **Exports & Imports**: Full table of exported objects and external package references.
  - **Name Map**: Decoded Unreal FName dictionary table.
  - **USMAP Schema**: Matched engine struct schema and property layout.
- **Detached Window Popout**: Pop any asset viewer into a standalone floating window for multi-monitor workflows.

### 🖼️ GPU Texture Inspector
- **Hardware-Accelerated Decoding**: Renders Unreal Engine texture formats (BC1/DXT1, BC3/DXT5, BC5, BC7, and uncompressed RGBA) directly on the GPU.
- **Channel Isolation**: Toggle Red, Green, Blue, and Alpha channels individually to inspect normal maps, roughness channels, and opacity masks.
- **Asset Metadata & Export**: View texture dimensions, mipmap levels, and surface formats, with 1-click export to standard PNG images.

### ⚔️ Multi-Layer Conflict Scanner
- **Pak Asset Node Collisions**: Scans all active `.pak` archives to identify overlapping file paths and cooked assets, predicting which mod will override assets based on load priority.
- **PalSchema DataTable Collisions**: Identifies competing schema mods modifying the same DataTable rows (e.g., conflicting Pal capture rates or weapon damage stats).
- **Lua Engine Hook Collisions**: Detects multiple UE4SS Lua scripts intercepting the same internal engine function (`NotifyOnNewObject`, `RegisterHook`).
- **Lua Hotkey Scanner**: Analyzes active scripts for keybinding declarations (`RegisterKeyBind`). Identifies overlapping keys and features a **1-Click Quick Rebind** utility to reassign conflicting hotkeys to free function keys (F1–F12).
- **Recursive Variable Resolution**: Recursively traces configuration variables across script files (e.g., mapping `Config.MenuKey` back to its root definition in `settings.lua`) to allow direct inline editing.

### 🛠️ 1-Click Compatibility Patch Engine
- When conflicting `.pak` mods are detected, PMM can automatically synthesize an override patch archive:
  ```
  zzz_PMM_Patch_MergedAssets.pak
  ```
  This resolves file collisions dynamically, allowing conflicting mods to coexist without manual repackaging.

### ⚠️ Crash Risk Diagnostics
- **Critical Asset Overwrite Warnings**: Flags mods that overwrite core vanilla Blueprints and UI widgets known to cause fatal crashes when Palworld updates:
  - Character & Camera: `BP_PalPlayerCharacter`, `BP_PalPlayerState`, `BP_PlayerCamera`
  - Core UI Widgets: `WBP_TitleMenu`, `WBP_EscMenu`, `WBP_WorldMap`
  - Camp & World Systems: `BP_PalBaseCampModel`, `BP_PalBox`
- **Triage Badges**: Categorizes risks as `Critical`, `High`, or `Moderate` with 1-click disable actions.

### 🔍 Engine Schema Hook Validation
- Validates UE4SS `RegisterHook` parameters against live USMAP engine reflection data.
- Accurately distinguishes between native C++ classes and dynamic Blueprint asset classes:
  - `✅ Valid` — Verified C++ engine class and function target.
  - `🔷 Dynamic Blueprint` — Valid Blueprint asset resolved through asset registry.
  - `❌ Missing Class` — Engine class not found in current game build.
  - `⚠️ Missing Function` — Class exists, but target function signature does not match.

### 💾 Save Doctor & World Hub
- **Multi-Platform Save Discovery**: Automatically locates world saves across Steam, Xbox Game Pass PC (WinGDK), and Linux Proton wineprefixes.
- **GVAS Binary Integrity Validation**: Validates the structural health of `Level.sav` and player save files. Scans for corrupted byte offsets and orphaned asset class references left behind by uninstalled mods.
- **Snapshot History & Progression Diff**: Compare save backups side-by-side with metrics for world day progression, player levels, Paldeck completion counts, and file size deltas.
- **WorldOption Inspector**: View and adjust world difficulty parameters and multiplier settings across 5 distinct categories.
- **Base Camp Decay Guard**: Detects when building deterioration is enabled (`BuildObjectDeteriorationDamageRate > 0`) alongside base expansion mods, alerting you to structure decay risks outside standard camp boundaries.

---

# 🛠️ Pillar 3: Mod Development & Tooling

PMM introduces a comprehensive suite of native mod-development utilities, bridging the gap between playing mods and building them.

### 🔬 In-Place Pak Tweaker & Binary Asset Editor
Directly modify cooked Unreal Engine properties and DataTable rows inside packed `.pak` archives without external tools:
- **Pristine Safety Backups (`.original.bak`)**: Automatically creates an untouched backup before applying any binary modification, guaranteeing 1-click rollback at any time.
- **Interactive DataTable Grid Viewer**: View item parameters, drop tables, and Pal attributes in a clean, searchable table with inline cell editing (`✏️`).
- **Live Blueprint CDO Comparative Diff**: A 5-column comparative matrix displaying Property Name, Data Type, Vanilla Default, Mod Value, and a 1-click reset to vanilla—revealing precisely what a mod changes relative to the base game.
- **3-Way Noise Filter**: Switch between viewing `All Properties`, `Only Modified Deltas`, or `Conflicting Properties` to isolate edits instantly.

### 🌐 Palworld Development Resources Hub
A centralized dashboard providing modders with instant access to essential reverse-engineering and development resources:
- **Live Environment Metadata**: Real-time display of current Palworld game version, Steam Build ID, and UE4SS runtime state.
- **Master Manifest Integration**: Automatically queries and syncs with official remote manifests to resolve game dumps and mapping assets without recompiling the application.
- **1-Click Resource Synchronization**: Download, index, and update C++ SDKs, JMAP dump files, and USMAP reflection mappings with dynamic progress tracking.

### 📝 Monaco Code Editor & IntelliSense
A fully integrated, multi-buffer Monaco IDE tailored for Palworld mod development:
- **Multi-Buffer Workspace**: Edit multiple Lua, JSON, and JSONC files simultaneously with full unsaved-change preservation across tab switches.
- **Integrated File Tree**: Create, rename, delete, and organize files and directories directly within the editor sidebar.
- **Unsaved Changes Protection**: Floating revert controls and a multi-file safety modal protect pending edits when switching views or changing active profiles.
- **JSONC Comment Preservation**: Full JSON with Comments support—comments are preserved during save cycles while payload data is validated against active schemas.

### 💡 EmmyLua & PalSchema IntelliSense
- **20,000+ Engine Classes**: Autocompletion engine powered by Palworld C++ reflection and EmmyLua type definitions.
- **Blueprint Reflection Autocomplete**: Autocompletes dynamic Blueprint classes, components, and event signatures.
- **PalSchema DataTable IntelliSense**: Provides autocompletion and hover documentation across 423 game DataTables and 149,000+ rows, complete with parameter hints and return types.
- **Diagnostics & QuickFix**: Real-time syntax validation, missing symbol diagnostics, and 1-click QuickFix corrections.

### 🗄️ USMAP Reflection Explorer
- **Interactive Engine Schema Explorer**: Browse all cooked Palworld engine classes, structs, enums, properties, and FNames directly from the Database tab.
- **Inheritance Hierarchy Breadcrumbs**: Visual class hierarchy navigation (e.g., `PalPlayerCharacter → PalCharacter → Character → Pawn → Actor → Object`).
- **Memory Layout & Offsets**: Detailed property offset tables, type definitions, and inner struct dimensions for native reverse engineering.

### 🧩 C++ SDK Manager
- Visual indexing and version management for Palworld C++ SDK headers.
- Import local SDK dumps or synchronize pre-built SDK packages directly from the Dev Resources Hub.

### 🏗️ Mod Packer & Builder
A visual packaging studio for mod authors:
- **Hierarchical Staging Workspace**: Drag and drop mod files into a visual tree and configure target deployment paths.
- **Manifest Generation (`modinfo.pmm.json`)**: Automatically embeds routing metadata into the packaged archive for zero-configuration, heuristic-free installation on user machines.
- **Dual-Platform Packaging**: Compiles platform-optimized archives simultaneously:
  ```
  ModName_Steam_Win64.zip
  ModName_Xbox_WinGDK.zip
  ```
- **Vortex / Root Preset**: Generates game-relative directory trees compatible with Vortex and manual installation methods.

---

# 🧩 Pillar 4: Configuration & Data

PMM ensures you have complete control over mod configurations, application state, and personal ergonomics.

### ⚙️ Config Editor & Dynamic Form Views
- Edit mod configuration files using the Monaco code editor or intuitive, auto-generated form interfaces with toggles, sliders, and color pickers.
- Supports `config.lua`, `settings.lua`, `config.json`, and `config.jsonc`.

### 🔄 Smart Config Archiving & Merge
- When updating a mod, PMM automatically archives existing user configurations before deploying new files.
- **Granular Config Review Dialog**: Inspect a side-by-side diff between your customized configuration and the incoming update. Cherry-pick individual keys or restore entire files with a single click.

### 🗄️ Database Grid Inspector
- Inspect and manage internal PMM database entities (Mods, Profiles, Settings) directly.
- Features strict JSON validation, entity cloning, and automatic masking of sensitive credentials (e.g., Nexus API tokens).

### 📌 Mod Notes & Ergonomic Reminders
- Attach personal notes, load order reminders, and custom keybinding cheatsheets directly to any mod card.
- Notes are preserved across updates and stored within profile metadata.

### 🛡️ Strict Profile Isolation
- Configuration changes made in one profile are strictly sandboxed, preventing setting leakage or corrupted states across different play styles.

---

# ⚙️ Pillar 5: Platform & Infrastructure

PalModManager is engineered on top of a modern, resilient desktop architecture designed for raw performance and low resource consumption.

### ⚡ PMM-Core Reactive Framework
- **Zero-VDOM Architecture**: Utilizes direct, compile-time typed DOM scope accessors (`mainDom`, `editorDom`, `detailDom`, etc.) eliminating virtual DOM overhead.
- **Decoupled Event Mesh (`bus`)**: Subsystems communicate asynchronously via a strongly typed event mesh (`bus.emit` / `bus.on`), preventing circular dependencies and guaranteeing responsive UI rendering.
- **Declarative State Projections**: Automatic UI synchronization via reactive binders connected to the centralized application state store.

### 🌐 Universal 6-Language Localization
Every user-facing string—including UI controls, dialogs, error prompts, toasts, and tooltips—is 100% localized across all 6 supported languages:

| Language | Locale Code | Coverage |
|---|---|---|
| 🇺🇸 **English** | `en` | 100% |
| 🇪🇸 **Spanish** | `es` | 100% |
| 🇧🇷 **Portuguese** | `pt` | 100% |
| 🇨🇳 **Simplified Chinese** | `zh-CN` | 100% |
| 🇯🇵 **Japanese** | `ja` | 100% |
| 🇰🇷 **Korean** | `ko` | 100% |

### 🔒 Safety Vaults & Rollback Systems
- **Dependency Rollback Vault**: Preserves previous versions of UE4SS and PalSchema for one-click restoration.
- **Pristine Asset Backups**: Maintains untouched `.original.bak` copies for all in-place binary edits.
- **Automated World Save Protection**: Creates redundant save backups before performing any structural repair or salvage operation.

---

# 🏗️ Technical Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       PalModManager Desktop                             │
├─────────────────────────────────────────────────────────────────────────┤
│                     Frontend (TypeScript & Vite)                        │
│                                                                         │
│  PMM-Core Scopes       Reactive Event Mesh (bus)       Monaco Editor    │
│  Vanilla CSS Themes    6-Language i18n Engine          Virtual Folders  │
├─────────────────────────────────────────────────────────────────────────┤
│                             Tauri v2 IPC                                │
├─────────────────────────────────────────────────────────────────────────┤
│                       Backend Core (Rust)                               │
│                                                                         │
│  Smart Unpacker        Pak & UAsset Inspector     USMAP / SDK Indexer   │
│  Nexus OAuth / NXM     In-Place Pak Tweaker       Conflict & Hotkey Scan│
│  Profile Engine        GVAS Save Doctor           Patch Generator       │
│  Workshop Manager      Config Merge Engine        Mod Packer & Builder  │
└─────────────────────────────────────────────────────────────────────────┘
```

- **Frontend**: TypeScript, Vite 8 (Rolldown), Monaco Editor, Vanilla CSS, PMM-Core Framework.
- **Backend**: Rust, Tauri v2 — modular domain command crates for installation, inspection, extraction, reflection, and network communication.
- **Storage**: Local JSON database with isolated credential vault and portable directory redirection.

---

# 🧪 Development & Testing

PMM includes a comprehensive suite of automated tests covering archive extraction, configuration merging, USMAP parsing, and mod lifecycle journeys:

```bash
pnpm dev              # Launch frontend development server
pnpm tauri dev        # Launch complete desktop application in dev mode
pnpm build            # Build frontend production bundle
pnpm tauri build      # Compile desktop release binaries
pnpm check            # TypeScript type checking
pnpm test             # Run Rust backend integration tests
pnpm test:verbose     # Run Rust tests with verbose serialized output
pnpm clean            # Clean build artifacts and debug caches
pnpm dumps:check      # Verify Palworld development dump status
pnpm dumps:sync       # Synchronize UE4SS dumps and development assets
```

---

# 🚀 Installation & Setup

## Windows

Download the latest release from **[GitHub Releases](https://github.com/olivo28/PalModManager/releases)**.

- **Setup Wizard (Recommended)**: Download and run `PalModManager_1.7.2_x64-setup.exe`.
- **Portable Edition**: Download `palmodmanager.exe` and run it from any folder—no system installation required.

### Quick Start
1. Launch PMM and open **Settings** (⚙).
2. Select your Palworld installation folder. PMM will automatically detect your platform (Steam or Xbox Game Pass PC).
3. Connect your Nexus Mods account with 1-click OAuth SSO.
4. Drag and drop mod archives directly into PMM, or download mods directly with 1-click NXM links.

## Linux / Steam Deck (SteamOS)

PMM runs natively on Linux. If you experience WebKitGTK or driver-related launch issues:

```bash
# Force X11 backend for Wayland environments
GDK_BACKEND=x11 ./palmodmanager

# Disable DMA-BUF renderer for Nvidia/Intel graphics issues
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager

# Combined launch command (recommended for Steam Deck desktop mode)
GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
```

---

# 🔐 Security, Backups & Antivirus

### Independent Backups
While PMM automatically creates safety backups before performing binary modifications, dependency updates, and save repairs, we strongly encourage maintaining independent backups of your game directories:
```
Pal/Content/Paks
Pal/Binaries/Win64/ue4ss    (Steam)
Pal/Binaries/WinGDK/ue4ss   (Xbox)
```

### Antivirus False Positives
PalModManager is 100% open-source, safe, and free of malware. Because PMM is an independent community project without an expensive corporate digital signing certificate, some antivirus engines may flag it with generic heuristic alerts (e.g., `Trojan:Win32/Wacatac.B!ml` or `Heur.Boring.1`).

**Why this occurs**: Machine learning heuristics automatically flag unsigned applications that extract files, modify `.pak` archives, and manage game binaries within protected directories like `steamapps/common`.

**How to resolve**:
1. Verify you downloaded PMM from the official [GitHub Releases](https://github.com/olivo28/PalModManager/releases) or [Nexus Mods](https://www.nexusmods.com/palworld/mods/4549).
2. On the Windows SmartScreen dialog, click **"More Info"** → **"Run Anyway"**.
3. If necessary, add `palmodmanager.exe` to your antivirus whitelist.

---

# 🐛 Troubleshooting

Before submitting a bug report, please verify:
1. Your Palworld path is correctly configured in Settings.
2. Required frameworks (UE4SS or PalSchema) are installed and enabled.
3. You have checked the Conflict Scanner for incompatible mod combinations.
4. Testing in a clean profile helps determine if an issue is mod-specific.

When reporting an issue, please include:
```
PMM Version: 1.7.2
Platform: Steam / Xbox Game Pass PC
Game Version / Steam Build ID:
Active Profile Mode:
Steps to Reproduce:
Expected vs. Actual Behavior:
Relevant logs from Settings > Logs
```

---

# 🤝 Contributing & Feedback

Contributions, mod compatibility feedback, and bug reports are warmly welcomed!

- Fork the repository and create a focused feature branch.
- Ensure all changes pass `pnpm check` and `pnpm test`.
- Adhere to the established PMM-Core framework patterns and universal 6-language localization rules.

---

# 💬 Community & Support

- **Discord**: [Join the Community](https://discord.gg/AHTDAUwm77) — Get help, discuss features, or report bugs.
- **GitHub**: [github.com/olivo28/PalModManager](https://github.com/olivo28/PalModManager)
- **Nexus Mods**: [PalModManager on Nexus Mods](https://www.nexusmods.com/palworld/mods/4549)
- **Contact**: Discord user **olivo28** (Nexus Mods Discord, Palworld Modding Community, PalSchema)

### ☕ Support the Project

If you love PalModManager and want to support its continuous development, maintenance, and future updates:

[![Ko-fi](https://img.shields.io/badge/Support%20on-Ko--fi-ff5e5b?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/olivo28)

*(Prefer **Binance Pay**? Contact me directly on Discord (`olivo28`) for the QR code!)*

---

# 📜 License

PalModManager is distributed under the **[MIT License](LICENSE)**.

---

## ❤️ Credits & Acknowledgments

Crafted with dedication by **Olivo28**.

- **HalRiveria** — Dedicated community tester and bug reporter across multiple release cycles. Crucial in diagnosing load order edge cases, mod update regressions, and batch installer stability.
- **Valdacil** — Invaluable real-world feedback, bug reports, and suggestions regarding hybrid mod routing, filename tracking, and installer workflows.
- **Palworld Modding Community** — Thank you to all mod authors, reverse engineers, and players whose daily feedback and testing continue to push PalModManager forward.

---

*PalModManager — Manage your mods, inspect your game data, and build for Palworld.*
