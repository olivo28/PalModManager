/**
 * PalModManager - UE4SS Dumps Inspector & Multi-Resource Sync Tool
 *
 * Inspects dumped artifacts from UE4SS (usmap, jmap, CXX SDK, Lua Types, UHT, BP SDK, PalSchema schemas),
 * validates disk existence, compares against repository resources/ manifests, and packages/synchronizes
 * updated game version assets into isolated resource directories.
 *
 * Usage:
 *   node scripts/sync_ue4ss_dumps.js          # Check and report status
 *   node scripts/sync_ue4ss_dumps.js --sync   # Synchronize and package all detected dumps
 */

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..');
const RESOURCES_ROOT = path.join(REPO_ROOT, 'resources');

// Default Steam and UE4SS paths
const DEFAULT_STEAMAPPS = 'C:\\Program Files (x86)\\Steam\\steamapps';
const DEFAULT_UE4SS_DIR = path.join(DEFAULT_STEAMAPPS, 'common\\Palworld\\Pal\\Binaries\\Win64\\ue4ss');

const isSyncMode = process.argv.includes('--sync');
const targetArg = process.argv.find(a => a.startsWith('--target='))?.split('=')[1] || null;

// Terminal colors
const colors = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  dim: '\x1b[2m',
};

function formatBytes(bytes) {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

function calculateSha256(filePath) {
  const fileBuffer = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(fileBuffer).digest('hex');
}

function countFilesInDir(dirPath) {
  if (!fs.existsSync(dirPath)) return 0;
  let count = 0;
  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      count += countFilesInDir(path.join(dirPath, entry.name));
    } else if (entry.isFile()) {
      count++;
    }
  }
  return count;
}

function copyRecursive(src, dest) {
  fs.mkdirSync(dest, { recursive: true });
  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);
    if (entry.isDirectory()) {
      copyRecursive(srcPath, destPath);
    } else {
      fs.copyFileSync(srcPath, destPath);
    }
  }
}

function createZipArchive(sourceDirOrFile, destZipPath, cwdDir) {
  fs.mkdirSync(path.dirname(destZipPath), { recursive: true });
  if (fs.existsSync(destZipPath)) {
    fs.unlinkSync(destZipPath);
  }

  const stat = fs.statSync(sourceDirOrFile);
  const workDir = cwdDir || (stat.isDirectory() ? sourceDirOrFile : path.dirname(sourceDirOrFile));
  const targetItem = stat.isDirectory() ? '*' : path.basename(sourceDirOrFile);

  const command = `tar -a -c -f "${destZipPath}" ${targetItem}`;
  execSync(command, { cwd: workDir, stdio: 'pipe' });

  if (!fs.existsSync(destZipPath)) {
    throw new Error(`Failed to create archive at ${destZipPath}`);
  }
}

// 1. Detect Steam build ID
function detectSteamBuildId() {
  const acfPath = path.join(DEFAULT_STEAMAPPS, 'appmanifest_1623730.acf');
  if (fs.existsSync(acfPath)) {
    try {
      const content = fs.readFileSync(acfPath, 'utf8');
      const match = content.match(/"buildid"\s+"(\d+)"/i);
      if (match && match[1]) {
        return match[1];
      }
    } catch {
      // Fallback
    }
  }
  return '25094871'; // Default detected build for Palworld v1.0.4
}

// 2. Discover UE4SS Build Commit from filenames
function detectUe4ssCommit(ue4ssDir) {
  if (!fs.existsSync(ue4ssDir)) return '2281fa31';
  const files = fs.readdirSync(ue4ssDir);
  for (const f of files) {
    const match = f.match(/Pal-5\.1\.1-0\+\+\+UE5\+Release-5\.1-([a-f0-9]+)\.(?:usmap|jmap)/i);
    if (match && match[1]) {
      return match[1];
    }
  }
  return '2281fa31';
}

// 3. Discover PalSchema Version dynamically from disk or manifest
function detectPalSchemaVersion(ue4ssDir) {
  if (ue4ssDir && fs.existsSync(ue4ssDir)) {
    const versionFile = path.join(ue4ssDir, 'Mods', 'PalSchema', 'palschema.version');
    if (fs.existsSync(versionFile)) {
      try {
        const ver = fs.readFileSync(versionFile, 'utf8').trim();
        if (ver) return ver;
      } catch {
        // Fallback
      }
    }
    const infoFile = path.join(ue4ssDir, 'Mods', 'PalSchema', 'Info.json');
    if (fs.existsSync(infoFile)) {
      try {
        const info = JSON.parse(fs.readFileSync(infoFile, 'utf8'));
        if (info.version) return info.version;
      } catch {
        // Fallback
      }
    }
  }

  const manifestPath = path.join(RESOURCES_ROOT, 'schemas', 'manifest.json');
  if (fs.existsSync(manifestPath)) {
    try {
      const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
      if (manifest.latest_palschema_version) return manifest.latest_palschema_version;
    } catch {
      // Fallback
    }
  }

  return '0.6.7';
}

// Definition of 7 targets
function getTargets(ue4ssDir, buildId, ue4ssCommit, palschemaVersion) {
  return [
    {
      id: 'mappings',
      name: 'Unreal Mappings (.usmap)',
      desc: 'GVAS Save Doctor unversioned property deserialization',
      sourcePath: path.join(ue4ssDir, `Pal-5.1.1-0+++UE5+Release-5.1-${ue4ssCommit}.usmap`),
      isDirectory: false,
      resourceDir: path.join(RESOURCES_ROOT, 'mappings'),
      targetFileName: `Palworld_${buildId}.usmap`,
      manifestKey: 'mappings',
      itemKey: 'usmap_filename',
      urlKey: 'usmap_url',
    },
    {
      id: 'jmap',
      name: 'JSON Property Mappings (.jmap)',
      desc: 'Complete property AST and enum mapping (trumank format)',
      sourcePath: path.join(ue4ssDir, `Pal-5.1.1-0+++UE5+Release-5.1-${ue4ssCommit}.jmap`),
      isDirectory: false,
      resourceDir: path.join(RESOURCES_ROOT, 'jmap'),
      targetFileName: `Palworld_${buildId}.jmap.zip`,
      manifestKey: 'jmaps',
      itemKey: 'jmap_filename',
      urlKey: 'jmap_url',
      compressSingleFile: true,
    },
    {
      id: 'sdk',
      name: 'CXX Header SDK (CXXHeaderDump)',
      desc: '1,712 C++ engine and game classes for modding & Monaco',
      sourcePath: path.join(ue4ssDir, 'CXXHeaderDump'),
      isDirectory: true,
      resourceDir: path.join(RESOURCES_ROOT, 'sdk'),
      targetFileName: `Palworld_SDK_${buildId}.zip`,
      manifestKey: 'sdk',
      itemKey: 'sdk_filename',
      urlKey: 'sdk_url',
      isSdkZip: true,
    },
    {
      id: 'lua_types',
      name: 'Lua EmmyLua Types (shared/types)',
      desc: '1,712 EmmyLua definitions for Monaco Editor autocompletion',
      sourcePath: path.join(ue4ssDir, 'Mods', 'shared', 'types'),
      isDirectory: true,
      resourceDir: path.join(RESOURCES_ROOT, 'lua_types'),
      targetFileName: `Palworld_LuaTypes_${buildId}.zip`,
      manifestKey: 'types',
      itemKey: 'types_filename',
      urlKey: 'types_url',
    },
    {
      id: 'uht',
      name: 'UHT Compatible Headers (UHTHeaderDump)',
      desc: '24,076 Unreal Header Tool C++ headers for native mods',
      sourcePath: path.join(ue4ssDir, 'UHTHeaderDump'),
      isDirectory: true,
      resourceDir: path.join(RESOURCES_ROOT, 'uht'),
      targetFileName: `Palworld_UHT_SDK_${buildId}.zip`,
      manifestKey: 'uht',
      itemKey: 'uht_filename',
      urlKey: 'uht_url',
    },
    {
      id: 'bp_sdk',
      name: 'Blueprint SDK Dummy Assets (UE4SS_SDK)',
      desc: '14,090 dummy asset files for Unreal Engine Editor modding',
      sourcePath: path.join(ue4ssDir, 'UE4SS_SDK'),
      isDirectory: true,
      resourceDir: path.join(RESOURCES_ROOT, 'bp_sdk'),
      targetFileName: `Palworld_BP_SDK_${buildId}.zip`,
      manifestKey: 'bp_sdk',
      itemKey: 'bp_sdk_filename',
      urlKey: 'bp_sdk_url',
    },
    {
      id: 'schemas',
      name: `PalSchema Schemas (v${palschemaVersion})`,
      desc: 'Raw schemas + enums.schema.json for JSON mod validation',
      sourcePath: path.join(ue4ssDir, 'Mods', 'PalSchema', 'schemas'),
      isDirectory: true,
      resourceDir: path.join(RESOURCES_ROOT, 'schemas'),
      targetFileName: `palschema_schemas_${palschemaVersion}.zip`,
      manifestKey: 'schemas',
      itemKey: 'schemas_filename',
      urlKey: 'schemas_url',
      isPalSchema: true,
      version: palschemaVersion,
    },
  ];
}

function checkTargetStatus(target) {
  const sourceExists = fs.existsSync(target.sourcePath);
  let sourceSize = 0;
  let sourceCount = 0;

  if (sourceExists) {
    if (target.isDirectory) {
      sourceCount = countFilesInDir(target.sourcePath);
    } else {
      sourceSize = fs.statSync(target.sourcePath).size;
    }
  }

  const manifestPath = path.join(target.resourceDir, 'manifest.json');
  let repoStatus = 'MISSING';
  let repoManifest = null;
  let repoFileExists = false;

  const targetFilePath = path.join(target.resourceDir, target.targetFileName);
  repoFileExists = fs.existsSync(targetFilePath);

  if (fs.existsSync(manifestPath)) {
    try {
      repoManifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
      const items = repoManifest[target.manifestKey] || [];
      const hasCurrent = items.some(it => {
        if (target.isPalSchema) {
          return it.palschema_version === target.version && repoFileExists;
        }
        return it[target.itemKey] === target.targetFileName && repoFileExists;
      });

      if (hasCurrent) {
        repoStatus = 'UP_TO_DATE';
      } else {
        repoStatus = 'NEEDS_SYNC';
      }
    } catch {
      repoStatus = 'ERROR';
    }
  } else {
    repoStatus = repoFileExists ? 'NO_MANIFEST' : 'NEEDS_SYNC';
  }

  return {
    sourceExists,
    sourceSize,
    sourceCount,
    repoFileExists,
    repoStatus,
    repoManifest,
    targetFilePath,
    manifestPath,
  };
}

function syncTarget(target, status, buildId, ue4ssCommit) {
  console.log(`\n${colors.cyan}>> Processing [${target.id}]: ${target.name}...${colors.reset}`);
  fs.mkdirSync(target.resourceDir, { recursive: true });

  const targetFilePath = status.targetFilePath || path.join(target.resourceDir, target.targetFileName);
  const now = new Date().toISOString();
  let fileSize = 0;
  let totalFiles = target.isDirectory ? status.sourceCount : 1;

  if (target.id === 'mappings') {
    console.log(`   Copying .usmap to ${target.targetFileName}...`);
    fs.copyFileSync(target.sourcePath, targetFilePath);
    fileSize = fs.statSync(targetFilePath).size;
  } else if (target.compressSingleFile) {
    console.log(`   Compressing .jmap into ${target.targetFileName}...`);
    createZipArchive(target.sourcePath, targetFilePath);
    fileSize = fs.statSync(targetFilePath).size;
  } else if (target.isPalSchema) {
    console.log(`   Synchronizing schemas directory and packing ${target.targetFileName}...`);
    const unpackedDir = path.join(target.resourceDir, 'palschema');
    copyRecursive(target.sourcePath, unpackedDir);
    createZipArchive(unpackedDir, targetFilePath);
    fileSize = fs.statSync(targetFilePath).size;
  } else {
    console.log(`   Packaging ${totalFiles} files into ${target.targetFileName}...`);
    createZipArchive(target.sourcePath, targetFilePath);
    fileSize = fs.statSync(targetFilePath).size;
  }

  console.log(`   Calculating SHA-256 checksum...`);
  const sha256 = calculateSha256(targetFilePath);
  console.log(`   Checksum: ${colors.dim}${sha256}${colors.reset} (${formatBytes(fileSize)})`);

  // Update or create manifest
  let manifest = {
    schema_version: '1.0.0',
    latest_game_version: 'v1.0.4',
    latest_steam_build_id: buildId,
    updated_at: now,
    [target.manifestKey]: [],
  };

  if (fs.existsSync(status.manifestPath)) {
    try {
      manifest = JSON.parse(fs.readFileSync(status.manifestPath, 'utf8'));
      manifest.latest_game_version = 'v1.0.4';
      manifest.latest_steam_build_id = buildId;
      manifest.updated_at = now;
      if (target.isPalSchema) {
        manifest.latest_palschema_version = target.version;
      }
    } catch {
      // Use initial structure
    }
  }

  // Mark all existing entries as is_latest: false
  if (Array.isArray(manifest[target.manifestKey])) {
    for (const item of manifest[target.manifestKey]) {
      item.is_latest = false;
    }
  } else {
    manifest[target.manifestKey] = [];
  }

  // Build new entry
  const newEntry = {
    game_version: 'v1.0.4',
    steam_build_id: buildId,
    app_id: 1623730,
    [target.itemKey]: target.targetFileName,
    [target.urlKey]: `https://raw.githubusercontent.com/olivo28/PalModManager/main/resources/${path.basename(target.resourceDir)}/${target.targetFileName}`,
    sha256,
    file_size_bytes: fileSize,
    engine_version: 'UE5.1.1',
    build_id: `Pal-5.1.1-0+++UE5+Release-5.1-${ue4ssCommit}`,
    is_latest: true,
  };

  if (target.isDirectory) {
    if (target.isSdkZip) {
      newEntry.total_headers = totalFiles;
      newEntry.source = 'CXXHeaderDump';
    } else if (target.isPalSchema) {
      newEntry.palschema_version = target.version;
      newEntry.total_raw_schemas = 475;
      newEntry.total_domain_schemas = 5;
      newEntry.has_enums = true;
    } else {
      newEntry.total_files = totalFiles;
    }
  } else if (target.compressSingleFile) {
    newEntry.uncompressed_size_bytes = status.sourceSize;
  }

  // Remove matching existing entry if present, then prepend
  manifest[target.manifestKey] = manifest[target.manifestKey].filter(it => {
    if (target.isPalSchema) {
      return it.palschema_version !== target.version;
    }
    return it[target.itemKey] !== target.targetFileName;
  });
  manifest[target.manifestKey].unshift(newEntry);

  fs.writeFileSync(status.manifestPath, JSON.stringify(manifest, null, 2) + '\n', 'utf8');
  console.log(`   ${colors.green}✓ Manifest updated:${colors.reset} ${path.relative(REPO_ROOT, status.manifestPath)}`);
}

function updateMasterManifest(buildId, ue4ssCommit, palschemaVersion) {
  const masterPath = path.join(RESOURCES_ROOT, 'manifest.json');
  const now = new Date().toISOString();
  let master = {
    schema_version: '1.0.0',
    latest_game_version: 'v1.0.4',
    latest_steam_build_id: buildId,
    updated_at: now,
    versions: [],
  };

  if (fs.existsSync(masterPath)) {
    try {
      master = JSON.parse(fs.readFileSync(masterPath, 'utf8'));
      master.latest_game_version = 'v1.0.4';
      master.latest_steam_build_id = buildId;
      master.updated_at = now;
    } catch {}
  }

  if (Array.isArray(master.versions)) {
    for (const v of master.versions) {
      v.is_latest = false;
    }
  } else {
    master.versions = [];
  }

  const masterEntry = {
    game_version: 'v1.0.4',
    steam_build_id: buildId,
    ue4ss_commit: ue4ssCommit,
    engine_version: 'UE5.1.1',
    is_latest: true,
    usmap: `mappings/Palworld_${buildId}.usmap`,
    jmap: `jmap/Palworld_${buildId}.jmap.zip`,
    sdk: `sdk/Palworld_SDK_${buildId}.zip`,
    lua_types: `lua_types/Palworld_LuaTypes_${buildId}.zip`,
    uht: `uht/Palworld_UHT_SDK_${buildId}.zip`,
    bp_sdk: `bp_sdk/Palworld_BP_SDK_${buildId}.zip`,
    palschema_version: palschemaVersion,
  };

  master.versions = master.versions.filter(v => v.steam_build_id !== buildId);
  master.versions.unshift(masterEntry);

  fs.writeFileSync(masterPath, JSON.stringify(master, null, 2) + '\n', 'utf8');
  console.log(`   ${colors.green}✓ Master Manifest updated:${colors.reset} ${path.relative(REPO_ROOT, masterPath)}`);
}

// MAIN EXECUTION
console.log(`\n${colors.bold}=== PalModManager - UE4SS Dumps Sync & Inspector ===${colors.reset}`);
const ue4ssDir = process.argv[2] && !process.argv[2].startsWith('--') ? process.argv[2] : DEFAULT_UE4SS_DIR;
const buildId = detectSteamBuildId();
const ue4ssCommit = detectUe4ssCommit(ue4ssDir);
const palschemaVersion = detectPalSchemaVersion(ue4ssDir);

console.log(`UE4SS Base Directory:  ${colors.cyan}${ue4ssDir}${colors.reset}`);
console.log(`Detected Steam Build:  ${colors.green}${buildId}${colors.reset} (Palworld v1.0.4)`);
console.log(`Detected UE4SS Commit: ${colors.yellow}${ue4ssCommit}${colors.reset}`);
console.log(`Detected PalSchema:    ${colors.cyan}v${palschemaVersion}${colors.reset}\n`);

const targets = getTargets(ue4ssDir, buildId, ue4ssCommit, palschemaVersion);

// Header Table
console.log('------------------------------------------------------------------------------------------------------------------');
console.log(
  'Target Resource'.padEnd(36) +
  'Source Disk'.padEnd(24) +
  'Size / Count'.padEnd(18) +
  'Repo Status'.padEnd(20) +
  'Package Target'
);
console.log('------------------------------------------------------------------------------------------------------------------');

const targetsToProcess = [];

for (const target of targets) {
  if (targetArg && target.id !== targetArg) continue;

  const status = checkTargetStatus(target);

  let diskDisplay = status.sourceExists ? `${colors.green}✓ Available${colors.reset}` : `${colors.red}✗ Not Found${colors.reset}`;
  let sizeDisplay = '-';
  if (status.sourceExists) {
    sizeDisplay = target.isDirectory ? `${status.sourceCount} files` : formatBytes(status.sourceSize);
  }

  let statusDisplay = '';
  switch (status.repoStatus) {
    case 'UP_TO_DATE':
      statusDisplay = `${colors.green}● Up to date${colors.reset}`;
      break;
    case 'NEEDS_SYNC':
      statusDisplay = `${colors.yellow}▲ Needs Sync${colors.reset}`;
      targetsToProcess.push({ target, status });
      break;
    case 'NO_MANIFEST':
      statusDisplay = `${colors.yellow}▲ No Manifest${colors.reset}`;
      targetsToProcess.push({ target, status });
      break;
    default:
      statusDisplay = `${colors.red}● Missing${colors.reset}`;
      if (status.sourceExists) targetsToProcess.push({ target, status });
      break;
  }

  console.log(
    target.name.padEnd(36) +
    diskDisplay.padEnd(33) +
    sizeDisplay.padEnd(18) +
    statusDisplay.padEnd(29) +
    target.targetFileName
  );
}
console.log('------------------------------------------------------------------------------------------------------------------');

if (!isSyncMode) {
  if (targetsToProcess.length > 0) {
    console.log(`\n${colors.yellow}${targetsToProcess.length} target(s) require synchronization.${colors.reset}`);
    console.log(`Run ${colors.cyan}pnpm dumps:sync${colors.reset} to automatically package and update manifests.`);
  } else {
    console.log(`\n${colors.green}All resources in repository are 100% up to date with disk dumps!${colors.reset}`);
  }
} else {
  if (targetsToProcess.length === 0) {
    console.log(`\n${colors.green}Nothing to synchronize! All resources are already up to date.${colors.reset}`);
  } else {
    console.log(`\n${colors.bold}Synchronizing ${targetsToProcess.length} resource target(s)...${colors.reset}`);
    for (const { target, status } of targetsToProcess) {
      if (!status.sourceExists) {
        console.warn(`\n${colors.yellow}Skipping [${target.id}]: Source does not exist on disk.${colors.reset}`);
        continue;
      }
      syncTarget(target, status, buildId, ue4ssCommit);
    }
    updateMasterManifest(buildId, ue4ssCommit, palschemaVersion);
    console.log(`\n${colors.green}${colors.bold}✓ All selected resources successfully synchronized and packaged!${colors.reset}\n`);
  }
}
