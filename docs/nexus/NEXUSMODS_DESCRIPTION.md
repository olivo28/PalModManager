# PalModManager (PMM) — Palworld Mod Manager & Modding Toolkit

---
Latest version: **1.7.1**

Changelog: Latest notes in the pinned comments · Full history on [GitHub](https://github.com/olivo28/PalModManager)

---

## What is PalModManager?

**PalModManager (PMM)** is a native desktop application built around the Palworld modding ecosystem, covering the complete modding workflow:

- Discover and download mods from Nexus Mods without leaving the app.
- Install and update **UE4SS, PalSchema, Pak, LogicMods, Hybrid, and Altermatic** mods automatically.
- Manage UE4SS and PalSchema load order.
- Manage Steam Workshop content.
- Maintain isolated mod profiles with instant switching.
- Inspect Pak archives and Unreal Engine assets inline.
- Detect mod conflicts and game update crash risks.
- Inspect and repair Palworld save files.
- Edit configuration files in a full Monaco code editor.
- Build and package distributable mods.
- Work with Palworld development resources: SDKs, UHT data, Lua types, JMAP, USMAP mappings.

Built with **Tauri v2 + Rust** — launches in milliseconds and uses minimal RAM.

---

## Installation

1. Download **PalModManager** from the Files section (Installer or Portable EXE).
2. Run `PalModManager.exe` — or use the portable version directly, no installation needed.
3. Open **Settings** (⚙) and select your Palworld installation folder.
4. Log into Nexus Mods with 1-click SSO, browse the **Discover** tab, or drag & drop any `.zip`, `.rar`, or `.7z` mod file directly into the app to install.

---

## Features

### 🌐 Nexus Mods Integration

- **OAuth 2.0 SSO** — 1-click login with persistent session, showing your Nexus avatar, tier, endorsements, tracked mods, and published mods.
- **In-App Discovery Browser** — Search and browse the entire Palworld Nexus catalog with 13 category filters, tag multi-selectors (Includes/Excludes), Title/Description/Author search, language toggles, and adult content filters.
- **Rich Mod Details** — Formatted descriptions (BBCode/HTML), image gallery with zoom & pan lightbox, changelogs, file lists, and virus scan safety badges (Verified / Manual / Quarantine).
- **Native `nxm://` Protocol** — 1-click browser downloads piped directly into PMM's download queue with live speed (⚡ MB/s) and ETA (⏳).
- **Social Actions** — Endorse, track/untrack mods, open Community & Bug pages. `👑 Author` badge automatically detected to prevent self-endorsements.

---

### 📦 Smart Mod Installer

- **Auto-Type Detection** — Automatically identifies UE4SS, PalSchema, Pak, LogicMods, Hybrid, Altermatic, and UniPalUI mods from the archive structure.
- **Smart Heuristic Unpacker** — Handles any ZIP layout regardless of how nested or disorganized. Automatically bypasses `(STEAM)`, `(XBOX)`, `Win64`, `WinGDK`, and helper directory wrappers.
- **Interactive File Preview Tree** — Inspect ZIP contents and projected install targets before deploying, in both single and batch modes.
- **Batch Installation** — Drag and drop multiple archives at once to preview and install in bulk.
- **Installed Version Comparison** — Side-by-side view of installed vs. incoming version when updating.
- **Smart Config Merging** — Snapshots and merges `config.lua`, `settings.lua`, `config.jsonc`, and `settings.json` across updates, preserving your keybindings and settings.
- **Granular Config Review Dialog** — Interactive diff modal lets you toggle which config files or individual keys to restore before applying.
- **Pak Destination Selector** — Choose between `~mods` and `LogicMods` targets per Pak mod.

---

### 🔀 UE4SS & PalSchema Load Order

**UE4SS Load Order**
- Drag-and-drop reordering interface in the sidebar Load tab.
- Automatic state transitions between `enabled.txt` and `mods.txt`.
- Cleanly removes deleted mod entries from `mods.txt`.
- Profile-aware load order persistence.

**PalSchema Load Order** *(Windows — unique feature)*
- PalSchema has no native sorting. PMM solves this with zero-admin **NTFS Junction Points**.
- Isolates mods in `PalSchema/Storage/` and creates zero-padded junctions (`001_ModName`, `002_ModName`) in `/mods` — **no Administrator or UAC required**.
- Side-by-side dual panels manage UE4SS and PalSchema load orders simultaneously.

---

### 👤 Profile Manager

- **Isolated Profiles** — Create independent mod sets for Vanilla, Singleplayer, Multiplayer, Hardcore, Testing, or Mod Development.
- **Instant Switching** — In-place profile switching in under 50ms when dependency mode matches.
- **Virtual Mod Folders** — Explorer-style folder cards with double-click navigation, breadcrumbs, and right-click context menus.
- **Profile Packs** — Export and share a complete mod setup (mods + configuration + dependencies) as a `.pmmprofile` package for co-op play.
- **Stable Mod IDs** — Uses Nexus IDs or sanitized descriptive names, never random UUIDs.

---

### 📚 Mod Library

- Centralized archive with auto-fetched Nexus thumbnails, authors, descriptions, and update statuses.
- Retains multiple downloaded versions per mod — 1-click rollback to any previous version.
- Bulk selection and profile-aware deployment.
- **Steam Workshop** — Browse, enable, and update Steam Workshop mods natively, with full `Info.json` InstallRule routing for multi-target hybrid mods.
- Smart dependency reconciliation — Workshop mods show **✓ Managed by PMM** instead of false missing-dependency warnings.
- Preserves the **exact original archive filename** (including Nexus mod ID, file ID, and timestamp) so rollback metadata is never lost.

---

### 🔍 Conflict Scanner & Diagnostics

- **Pak Collision Detection** — Identifies overlapping asset nodes across `.pak` mods.
- **1-Click Compatibility Patch Engine** — Auto-generates merged priority patches (`zzz_PMM_Patch_*.pak`).
- **PalSchema Table Collisions** — Detects conflicting DataTable row overrides.
- **Lua Hook Collisions** — Finds mods hooking the same engine functions.
- **Crash Risk Detection** — Flags mods overwriting critical vanilla Blueprints (`BP_PalPlayerCharacter`, `WBP_TitleMenu`, etc.) with risk levels (*critical, high, moderate*) and 1-click disable.
- **Lua Hotkeys Manager** — Scans `RegisterKeyBind` calls, highlights conflicts, supports inline rebinding with recursive variable resolution.
- **Engine Schema Hook Validation** — USMAP-backed validation distinguishing native C++ classes from dynamic Blueprint hooks.
- **1-Click Mitigation** — Disable or jump to code from any conflict card.

---

### 💾 Save Doctor & World Hub

- **Auto-Discovery** — Instantly finds saves across Steam, PC Game Pass (WinGDK), and Linux Proton.
- **GVAS Integrity Validator** — Validates binary save integrity and scans for orphaned mod class references from uninstalled mods.
- **1-Click Save Rescue** — Automated backup and cleaning of crashing or orphaned saves.
- **Snapshot History & Diff Inspector** — Side-by-side backup comparison with size deltas, in-game day progression, player level differences, and 1-click restore.
- **WorldOption Inspector** — Difficulty multipliers, player character data (UID, level, Paldeck unlocks), and storage analytics.
- **Base Camp Decay Alert** — Warns when structure deterioration is enabled alongside base expansion mods.

---

### 🖼️ GPU Texture Inspector

PMM can decode and preview supported Unreal texture formats directly inside the application — no external tools needed:

- GPU-accelerated texture preview with channel filtering (RGBA).
- Dimensions, format name, and mip count display.
- PNG export.

---

### ✏️ Code Editor & IntelliSense

- **Monaco Editor** — Full syntax-highlighted editor for Lua, JSON, and JSONC configuration files.
- **Pak & UAsset Explorer** — Browse `.pak` contents with live search and category filters (Blueprints, Textures, Materials, DataTables). Inspect `.uasset`/`.uexp` inline with GPU texture preview, export/import tables, name maps, and USMAP schema properties.
- **Smart Status Bar** — Auto-switches between binary mode (asset count + size) and text mode (Ln/Col).
- **EmmyLua IntelliSense** — 1,700+ Palworld engine class definitions with tab-stop snippet parameters and hover documentation cards.
- **Breadcrumb Navigation** — Styled `📦 ModName › 📁 folder › 📄 file.ext` path display.
- **Lua Diagnostics Panel** — Real-time problem detection and highlighting.
- **JSONC Support** — Comments preserved on save, validated against stripped content.

---

### 🏗️ Mod Packer & Builder

- **Projects Hub** — Create, rename, and manage mod packaging projects visually.
- **Route Manifesting** — Packages mods with custom target routing and a `modinfo.pmm.json` manifest for instant heuristic-free installation.
- **Staging Tree** — Drag and drop files to rearrange destination paths before packaging.
- **Dual-Platform Output** — Automatically builds both `_Steam_Win64.zip` and `_Xbox_WinGDK.zip`.
- **Vortex/Manual Root Preset** — Maps files to full game-relative paths for Vortex compatibility.

---

### 🗄️ USMAP & Reflection Explorer

- Dynamic USMAP build detection — resolves game versions and mappings **without recompiling the binary**.
- **Schema Explorer (DB Tab)** — Live-searchable inspector for all Palworld engine classes, structs, enums, properties, and FNames with full inheritance hierarchy breadcrumbs.

---

### 🌍 Localization

Fully localized in **6 languages**: English · Spanish · Portuguese · Simplified Chinese · Japanese · Korean.

---

## 🐧 Linux Support & Troubleshooting

PMM has native Linux support. If you experience crashes or a blank screen:

```
# Resolve Wayland/WebKitGTK crashes
GDK_BACKEND=x11 ./palmodmanager

# Resolve blank screen on Nvidia/Intel drivers
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager

# Combined
GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./palmodmanager
```

---

## ⚠️ Backup Recommendation

> Before experimenting with new mods or major changes, keep independent backups of your game's mod directories.

PMM provides automatic backups for many operations, but these should **complement — not replace** your own backups:
- `Pal/Content/Paks`
- `Pal/Binaries/Win64/ue4ss` *(Steam)* or `Pal/Binaries/WinGDK/ue4ss` *(Xbox GDK)*

---

## 🛡️ Antivirus & False Positives

PMM is open-source and completely clean. As an independent project without a commercial code-signing certificate, some antivirus engines may trigger heuristic detections (e.g. `Trojan:Win32/Wacatac.B!ml`).

- **Why it happens:** ML heuristics flag unsigned executables that modify files in Steam directories.
- **What to do:** On the Windows SmartScreen prompt, click *"More Info"* → *"Run Anyway"*, or add `palmodmanager.exe` to your AV exclusions. Always verify you downloaded from an official source first.

---

## Requirements

- **Game:** Palworld (Steam or PC Game Pass)
- **OS:** Windows 10/11 (64-bit) or Linux (Ubuntu, Debian, SteamOS, Arch)
- **Dependencies:** UE4SS and PalSchema — PMM can install both automatically if missing

---

## 🙏 Credits

- **HalRiveria** — Community tester and dedicated bug reporter across multiple releases. Tracked down mod update regressions, load order edge cases, batch installer failures, and more. Thank you!
- **Valdacil** — Detailed real-world bug reports that helped identify hybrid mod update and filename tracking issues.

Special thanks to the entire Palworld modding community for testing, feedback, and compatibility reports.

---

## 💬 Community & Support

- **Discord:** [discord.gg/AHTDAUwm77](https://discord.gg/AHTDAUwm77)
- **GitHub:** [github.com/olivo28/PalModManager](https://github.com/olivo28/PalModManager)
- **Nexus Mods:** [nexusmods.com/palworld/mods/4549](https://www.nexusmods.com/palworld/mods/4549)
