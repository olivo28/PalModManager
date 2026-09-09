# 🎮 PalModManager (PMM)

> **Palworld Mod Manager & Modding Toolkit**
>
> A desktop application for managing, installing, inspecting, configuring, and building Palworld mods across Steam and Xbox Game Pass PC environments.

[![Version](https://img.shields.io/badge/version-1.7.1-blue)](https://github.com/olivo28/PalModManager/releases)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/olivo28/PalModManager)
[![Palworld](https://img.shields.io/badge/game-Palworld-orange)](https://www.palworldgame.com/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-orange)](https://tauri.app/)

---

## 🌟 What is PalModManager?

**PalModManager (PMM)** is a native desktop application built around the Palworld modding ecosystem.

It is designed to cover the complete modding workflow:

- discover and download mods from Nexus Mods;
- install and update UE4SS, PalSchema, Pak, LogicMods, Hybrid, and Altermatic mods;
- manage UE4SS and PalSchema load order;
- manage Steam Workshop content;
- maintain isolated mod profiles;
- inspect Pak archives and Unreal Engine assets inline;
- detect mod conflicts and game update crash risks;
- inspect and repair Palworld save files;
- edit configuration files in a full Monaco code editor;
- build and package distributable mods;
- work with Palworld development resources — SDKs, UHT data, Lua types, JMAP, and USMAP mappings.

PMM is more than a traditional mod installer. It combines **mod management, game-data inspection, troubleshooting, and mod-development tooling** in a single application.

---

## 🎮 Platform Support

| Environment | Support | Notes |
|---|---|---|
| **Steam** | 🟢 Primary | Full mod-management workflow, UE4SS, PalSchema, Pak/IoStore tooling, profiles, saves, and Nexus integration |
| **Xbox Game Pass PC** | 🟢 Supported | WinGDK environment, platform-specific game paths, `.utoc`/`.ucas` generation |
| **Linux / Steam Proton** | 🟡 Partial | Save discovery and most tooling supported; NTFS junction features are Windows-only |

---

## ✨ Highlights

| | |
|---|---|
| 📦 Smart multi-format mod installation | 👤 Isolated mod profiles with instant switching |
| 🔄 Nexus Mods OAuth SSO & in-app discovery | 🧩 UE4SS & PalSchema dependency management |
| ⚔️ Multi-layer conflict & crash risk detection | 🔬 Pak / UAsset / DDS texture inspection |
| 🗄️ USMAP & Unreal reflection explorer | 💾 Save health doctor & world backup vault |
| 📝 Full Monaco editor with EmmyLua IntelliSense | 🏗️ Mod packer with dual-platform output |
| 🌐 6-language localization | 🎮 Steam Workshop integration |

---

# 🌐 Nexus Mods Integration

PMM integrates directly with Nexus Mods so that discovering and installing mods doesn't require switching between applications.

### Authentication

- OAuth 2.0 SSO — 1-click login with persistent session.
- Displays your Nexus avatar, tier, endorsements, tracked mods, and published mods.
- Secure credential handling with automatic token refresh.

### In-App Discovery Browser

- Search and browse the entire Palworld Nexus catalog.
- Filter by 13 official Palworld categories (Gameplay, Pals, Characters, Visuals, Scripts, etc.).
- Tag multi-selectors with Includes/Excludes and quick chip removal.
- Search by Title, Description, Author, or Uploader.
- Language toggles and adult content filters.

### Mod Details

- Rich mod descriptions with full BBCode/HTML rendering.
- Image gallery with mouse-wheel zoom and drag-to-pan lightbox.
- Changelogs, file lists, and virus scan safety badges (Verified / Manual / Quarantine).
- Categorized file downloads (Main, Updates, Optionals, Archived).

### Direct Downloads & NXM Protocol

- Native `nxm://` protocol — 1-click browser downloads piped directly into PMM.
- Queued download tray with live speed (⚡ MB/s) and ETA (⏳) via a real-time sliding-window throughput calculator.
- Automatic handoff to the installer on download completion.

### Social Actions

- Endorse, track/untrack mods, and access Community & Bug pages.
- Automatic `👑 Author` badge detection prevents accidental self-endorsements.

---

# 📦 Smart Mod Installer

PMM is designed to handle mods regardless of how their archives are packaged.

### Supported formats

`.zip`, `.7z`, `.rar` — including hybrid and multi-codec archives.

### Auto mod-type detection

PMM analyzes an archive and identifies structures for:

- **UE4SS** (Lua scripts, DLLs, `enabled.txt`)
- **PalSchema** (JSON/JSONC data schemas)
- **Pak** (`.pak` binary assets)
- **LogicMods** (`.pak` in `LogicMods/`)
- **Hybrid mods** (multiple types in a single archive)
- **Altermatic** (runtime replacer mods with `SwapJSON/` configs)
- **UniPalUI**

### Smart heuristic unpacker

PMM normalizes any ZIP structure before installation — no matter how nested or disorganized. Automatically bypasses common packaging wrappers:

```
(STEAM)   (XBOX)   Win64   WinGDK   UE4SS mods folder
```

### Installation features

- **Interactive file preview tree** — inspect ZIP contents and projected install targets before deploying.
- **Batch installation** — drag and drop multiple archives at once.
- **Installed version comparison** — side-by-side view of installed vs. incoming version when updating.
- **Pak destination selector** — choose `~mods` or `LogicMods` target per Pak.
- **Dependency warnings** — alerts for missing UE4SS or PalSchema before installing mods that require them.

### Configuration preservation

PMM archives and merges user configuration during mod updates instead of overwriting them:

```
config.lua    settings.lua
config.json   config.jsonc   settings.json
```

A **granular config review dialog** lets you toggle which config files or individual keys to restore before applying the merge.

---

# 🔀 UE4SS & PalSchema Load Order

## UE4SS

PMM provides a dedicated drag-and-drop load-order interface for UE4SS mods:

- Drag-and-drop reordering in the sidebar Load tab.
- Automatic state transitions between `enabled.txt` and `mods.txt`.
- Cleanly removes deleted mod entries from `mods.txt`.
- Profile-aware load order persistence.

## PalSchema *(Windows — unique feature)*

PalSchema has no native sorting logic. PMM solves this with **zero-admin NTFS Junction Points**:

```
PalSchema/
└── Storage/      ← physical mod directories (isolated)
    ├── ModA/
    └── ModB/

mods/             ← zero-padded junctions (ordered)
├── 001_ModA  →  Storage/ModA
└── 002_ModB  →  Storage/ModB
```

No Administrator or UAC permissions required. Side-by-side dual panels manage UE4SS and PalSchema load orders simultaneously.

---

# 🧩 Dependency Management

PMM provides full lifecycle management for UE4SS and PalSchema.

- **Auto-detection** — detects installation state, version, and install mode (Standard, Workshop, Xbox GDK).
- **1-click auto-install** — installs UE4SS and PalSchema from official GitHub releases without overwriting user mods.
- **Version vault** — archives dependency versions for rollback when a new release causes issues.
- **Sidecar manifests** — maintains `ue4ss.pmm.json` and `palschema.pmm.json` for precise tracking and safe reinstallation.
- **Directional SemVer comparison** — correctly handles custom and test builds (e.g. `"Palworld_ForPS066"`) by falling back to file modification dates when version strings are non-standard.

---

# 👤 Profile Manager

Create independent mod setups for different play styles:

```
Vanilla        Singleplayer      Multiplayer
Hardcore       Testing           Mod Development
```

Each profile maintains its own enabled/disabled mod state, load order, and deployment.

### Profile features

- Instant profile switching (< 50ms in-place switching when dependency mode matches).
- Isolated mod configurations per profile.
- Profile cloning, backup, export, and import.
- Stable mod IDs (Nexus ID or sanitized name — no random UUIDs).
- Cross-profile state preservation when switching.
- **Profile Packs** — export and share a complete mod setup (mods + config + dependencies) as a `.pmmprofile` package.

### Virtual Mod Folders

Explorer-style folder cards with:

- Double-click navigation.
- Breadcrumb bar.
- Right-click context menus (Rename, Toggle all, Check updates, Delete).

---

# 📚 Mod Library

Centralized archive of all your downloaded mods.

- Auto-fetched Nexus thumbnails, authors, descriptions, and update statuses.
- Retains multiple downloaded versions per mod for rollback.
- 1-click reinstallation and 1-click version rollback.
- Bulk selection and drag selection.
- Profile-aware deployment.

> PMM preserves the exact original archive filename (e.g. `Quality Of Life 4599 1 2026-07-30T23-44Z QXTyhgimX.zip`) so that rollback metadata is never lost.

### Steam Workshop

- Browse, enable, and update Steam Workshop mods directly from the Library tab.
- Full `Info.json` InstallRule routing for multi-target hybrid mods (Lua → `ue4ss/Mods/`, Paks → `~mods/`, PalSchema → `palschema/mods/`).
- Smart dependency reconciliation — Workshop mods requiring UE4SS or PalSchema show `✓ Managed by PMM` instead of false missing-dependency warnings.

---

# 🔍 Conflict Scanner & Diagnostics

PMM provides multi-layer conflict detection rather than treating every mod collision as a simple filename clash.

### Pak collisions

Detect overlapping asset nodes between `.pak` mods.

### 1-Click Compatibility Patch Engine

Detects collisions and auto-generates a merged priority patch:

```
zzz_PMM_Patch_*.pak
```

### PalSchema table collisions

Detect conflicting DataTable row overrides across enabled PalSchema mods.

### Lua hook collisions

Identify mods hooking the same engine function in UE4SS Lua scripts.

### Crash risk detection

Flags mods overwriting critical vanilla Blueprints and UI widgets that break on game updates:

- `BP_PalPlayerCharacter`, `BP_PalPlayerState`, `BP_PlayerCamera`
- `WBP_TitleMenu`, `WBP_EscMenu`, `WBP_WorldMap`
- `BP_PalBaseCampModel`, `BP_PalBox`

Risk levels: `critical`, `high`, `moderate` — with 1-click disable actions.

### Lua hotkeys manager

- Scans active Lua scripts for `RegisterKeyBind(...)` calls.
- Highlights conflicts when multiple mods use the same key.
- Inline key rebinding with recursive variable resolution (resolves `Config.OpenMenuKey` to its definition line).
- `⚡ Quick Rebind` auto-assigns a free function key (F1–F12).

### Engine schema hook validation

- USMAP-backed validation of `RegisterHook` targets.
- Distinguishes native C++ engine classes from dynamic Blueprint asset classes.
- Reports `✅ Valid`, `🔷 Dynamic Blueprint`, `❌ Missing Class`, `⚠️ Missing Function`.

### 1-click mitigation

Disable or jump directly to code from any conflict card.

---

# 💾 Save Doctor & World Hub

Dedicated save management and diagnostics for Palworld worlds.

### Save discovery

Auto-discovers saves across Steam, PC Game Pass (WinGDK), and Linux Proton.

### GVAS integrity validation

- Validates binary save integrity.
- Scans `Level.sav` for orphaned asset class references left by uninstalled mods.
- 1-click save rescue with automated backup before any destructive operation.

### Snapshot history & diff inspector

- Side-by-side backup comparison with size deltas.
- In-game day progression, player level differences, Paldeck counts.
- 1-click restore.

### WorldOption inspector

- Difficulty multipliers across 5 categories.
- Player character data: UID, level, captured Pals, storage analytics.

### Base camp decay alert

Warns when structure deterioration is enabled (`BuildObjectDeteriorationDamageRate > 0`) and base expansion mods are present, advising how to preserve outer buildings.

---

# 🖼️ GPU Texture Inspector

PMM can decode and preview supported Unreal texture formats directly inside the application:

- Texture preview with GPU-accelerated rendering.
- Channel filtering (RGBA).
- Dimensions, format name, mip count display.
- PNG export.

---

# ✏️ Code Editor & IntelliSense

PMM includes a full-featured **Monaco Editor** for configuration and mod-development workflows.

### Editor features

- Syntax highlighting for Lua, JSON, and JSONC.
- Line/column information and file tree navigation.
- Problems panel with diagnostics.
- QuickFix actions for supported issues.
- Search (Ctrl+F), formatting, auto-indentation.
- Native keyboard shortcuts.
- File and folder creation.
- JSONC support — comments preserved on save, validated on stripped content.
- Safety guards — prompts to save or discard before switching profiles.

### Pak & UAsset Explorer

Browse `.pak` archive contents inline without external tools:

- Live search input.
- Category filter chips (All / Blueprints / Textures / Materials / DataTables / Other) with live counts.
- Color-coded asset type pills and companion `Payload` badges.
- 1-click inspection of `.uasset`/`.uexp` files:
  - Tabs: Overview, GPU Texture Preview, Exports, Imports, Name Map, USMAP Schema.
  - `← Return to .pak` navigation and `⛶ Open in Window` popout.

### Smart Status Bar

- Automatically switches between **binary mode** (asset count + file size) and **text mode** (Ln/Col + UTF-8) based on the active file type.

### EmmyLua IntelliSense

- 1,700+ Palworld engine class definitions parsed from Palworld EmmyLua bindings.
- Tab-stop snippet parameters (e.g. `APalCharacter:PalMoveToLocation(${1:Location}, ${2:Speed})`).
- Hover documentation cards with parameter types and return types.
- Palworld-aware completions including C++ reflection data, Blueprint info, PalSchema DataTable rows.

### Breadcrumb navigation

Styled `📦 ModName › 📁 folder › 📄 file.ext` breadcrumb bar with distinct accent colors.

---

# 🏗️ Mod Packer & Builder

PMM includes a dedicated build workflow for mod authors.

### Projects hub

Create, rename, delete, and manage mod packaging projects visually.

### Route manifesting (`modinfo.pmm.json`)

Package mods with:

- Custom target routing per file.
- Version, author, description, and Nexus ID.
- `modinfo.pmm.json` manifest embedded inside the ZIP for instant, heuristic-free installation on client machines.

### Staging tree

Drag and drop files into the staging workspace and rearrange destination paths before packaging.

### Dual-platform output

Automatically builds both:

```
ModName_Steam_Win64.zip
ModName_Xbox_WinGDK.zip
```

with correct platform-specific path structures.

### Vortex/Manual root preset

Maps files to full game-relative paths for Vortex and manual installation compatibility.

---

# 🗄️ USMAP & Unreal Reflection Explorer

PMM includes USMAP tooling for working with cooked Palworld data.

- USMAP v4 parsing and synchronization.
- Dynamic build detection via `resources/manifest.json` — resolves game versions, UE4SS commits, and mapping paths for future Palworld updates **without recompiling the binary**.
- DataTable decoding and PalSchema schema indexing.

### Schema Explorer (DB Tab)

Live-searchable interactive explorer for all Palworld engine classes, structs, enums, properties, and FNames:

- Full inheritance hierarchy breadcrumbs (e.g. `PalPlayerCharacter → PalCharacter → Character → Pawn → Actor → Object`).
- Complete property offset tables with types and inner dimensions.
- Enum value lists and dual Schema/JSON inspection modes.

---

# 🔧 Settings & Database Inspector

- **Database Grid Inspector** — View and edit Mods, Profiles, and Settings with raw JSON validation and secure credential masking.
- **Custom Storage Redirection** — Redirect app data (profiles, library, backups) to any folder or portable directory with automated migration.
- **Toolbar Scaling** — Resize main workspace toolbars from 80% to 180%.
- **Window State Persistence** — Remembers and restores window size, position, and maximized state.
- **Network & Sources** — Configure Nexus credentials, SDK sync, and resource sources.

---

# 🌍 Localization

PMM is fully localized in 6 languages:

| | |
|---|---|
| 🇺🇸 English | 🇪🇸 Spanish |
| 🇧🇷 Portuguese | 🇨🇳 Simplified Chinese |
| 🇯🇵 Japanese | 🇰🇷 Korean |

All UI strings, dialogs, prompts, toasts, and tooltips are covered.

---

# 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│             PalModManager               │
├─────────────────────────────────────────┤
│     TypeScript / PMM-Core UI Layer      │
│     Vite + Reactive Event Mesh (bus)    │
├─────────────────────────────────────────┤
│               Tauri v2                  │
├─────────────────────────────────────────┤
│             Rust Backend                │
│                                         │
│  Installer        Nexus & NXM           │
│  Profiles         Save Scanner          │
│  Pak Inspector    USMAP / SDK           │
│  Texture Tools    Altermatic            │
│  Dependencies     Conflict Scanner      │
│  Workshop         Config Merge          │
│  Patching         File Watchers         │
└─────────────────────────────────────────┘
```

- **Frontend**: TypeScript, Vite 8 (Rolldown), Monaco Editor, Vanilla CSS, PMM-Core Framework (zero-VDOM, compile-time typed scopes, event mesh).
- **Backend**: Rust, Tauri v2 — domain-specific command modules for installer, profiles, scanner, Nexus, USMAP, save tools, Pak/UAsset inspection, texture decoding.
- **Storage**: JSON database with secure credential isolation.

---

# 🧪 Development & Testing

The codebase includes dedicated integration and module tests covering:

- Archive resilience and extraction.
- Configuration archive lifecycle and merging.
- Dependency management.
- Hybrid mod enable/disable behavior.
- Pak inspection and conflict scanning.
- USMAP parsing.
- Installation pipeline regression tests.

### Development commands

```bash
pnpm dev              # Frontend dev server
pnpm tauri dev        # Full app dev mode
pnpm build            # Frontend build
pnpm tauri build      # Production build
pnpm check            # TypeScript type checking
pnpm test             # Run Rust integration tests
pnpm test:verbose     # Verbose serialized test output
pnpm clean            # Clean build artifacts
pnpm dumps:check      # Check Palworld development dump status
pnpm dumps:sync       # Sync UE4SS dumps and resources
```

---

# 🚀 Installation

## Windows

Download the latest release from **[GitHub Releases](https://github.com/olivo28/PalModManager/releases)**.

**Option 1 — Installer (recommended)**

Run `PalModManager_1.7.1_x64-setup.exe` and follow the setup wizard.

**Option 2 — Portable**

Place `palmodmanager.exe` anywhere and launch it directly — no installation required.

### First launch

1. Open **Settings** (⚙).
2. Select your Palworld installation folder.
3. PMM detects the platform (Steam / Xbox / Workshop) automatically.
4. Configure your first profile.
5. Log in to Nexus Mods with 1-click SSO, or drag & drop a mod archive to install.

## Linux

PMM has native Linux support. If you experience crashes or a blank screen:

```bash
# Resolve Wayland/WebKitGTK crashes
GDK_BACKEND=x11 ./palmodmanager

# Resolve blank screen on Nvidia/Intel drivers
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager

# Combined
GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
```

---

# 🛠️ Building from Source

### Requirements

- Node.js (v18+)
- pnpm
- Rust & Cargo (latest stable)
- Tauri v2 system dependencies for your target platform

```bash
git clone https://github.com/olivo28/PalModManager.git
cd PalModManager

pnpm install       # Install frontend dependencies
pnpm tauri dev     # Development mode
pnpm check         # TypeScript type check
pnpm test          # Run tests
pnpm tauri build   # Production build
```

Build output:
- **Portable**: `src-tauri/target/release/palmodmanager.exe`
- **Installer**: `src-tauri/target/release/bundle/nsis/`

---

# 🔐 Security, Backups & Antivirus

## Back up your game

Before experimenting with new mods or major changes, keep independent backups of:

```
Pal/Content/Paks
Pal/Binaries/Win64/ue4ss    (Steam)
Pal/Binaries/WinGDK/ue4ss   (Xbox)
```

PMM provides automatic backups for many operations, but **these complement — not replace — your own backups**.

## Antivirus false positives

PMM is open-source and completely clean. As an independent community project without a commercial code-signing certificate, some antivirus engines may trigger heuristic detections (e.g. `Trojan:Win32/Wacatac.B!ml`).

**Why it happens:** ML heuristics flag unsigned executables that access, create, and modify files in Steam game directories.

If PMM is flagged:
1. Verify you downloaded it from the official GitHub or Nexus page.
2. Compare the release with the public repository.
3. On the Windows SmartScreen prompt, click *"More Info"* → *"Run Anyway"*.
4. Or add `palmodmanager.exe` to your antivirus exclusions.

---

# 🐛 Troubleshooting

Before reporting an issue:

1. Confirm your Palworld installation path is correct in Settings.
2. Check that required mod frameworks (UE4SS, PalSchema) are installed.
3. Review PMM's dependency and conflict information.
4. Try a clean profile to isolate the issue.
5. Back up your saves before attempting repair operations.

When reporting a bug, please include:

```
PMM version:
Palworld version / platform (Steam / Xbox):
Mod(s) involved:
Profile mode:
Steps to reproduce:
Expected behavior:
Actual behavior:
Relevant logs (found in Settings > Logs or app data folder):
```

---

# 🤝 Contributing & Feedback

Bug reports, testing feedback, and mod compatibility reports are especially valuable because PMM interacts with a rapidly changing modding ecosystem.

Before opening a pull request:

- Keep changes focused and minimal.
- Follow the existing module structure and naming conventions.
- Run `pnpm check` and `pnpm test`.
- Document user-facing features.
- Include reproduction steps for bug fixes.

---

# 💬 Community & Support

- **Discord**: [discord.gg/AHTDAUwm77](https://discord.gg/AHTDAUwm77)
- **GitHub**: [github.com/olivo28/PalModManager](https://github.com/olivo28/PalModManager)
- **Nexus Mods**: [nexusmods.com/palworld/mods/4549](https://www.nexusmods.com/palworld/mods/4549)
- **Contact**: Discord tag **olivo28** (Nexus Mods Discord, Palworld Modding Community, PalSchema)

---

# 📜 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.

---

## ❤️ Credits

Developed with ❤️ by **Olivo28**.

- **HalRiveria** — Community tester and dedicated bug reporter across multiple releases. Tracked down mod update regressions, load order edge cases, batch installer failures, and more. Thank you!
- **Valdacil** — Detailed real-world bug reports that helped identify hybrid mod update and filename tracking issues.

Special thanks to the entire Palworld modding community for testing, feedback, and compatibility reports that shape every release.

---

*PalModManager — manage your mods, inspect your game data, and build for Palworld.*
