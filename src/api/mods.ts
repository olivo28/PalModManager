import { invoke } from '@tauri-apps/api/core';
import type { ModInfo, InstallManifest, ZipAnalysis, UpdateCheckResult, ConfigDiff, ArchivedConfigInfo } from './types';

export async function getMods(): Promise<ModInfo[]> {
  return invoke('get_mods');
}

export async function scanMods(): Promise<ModInfo[]> {
  return invoke('scan_mods');
}

export async function analyzeZip(zipPath: string): Promise<ZipAnalysis> {
  return invoke('analyze_zip', { zipPath });
}

export async function installMod(
  zipPath: string,
  customType: string | null,
  pakDestination: string | null,
  customName: string | null = null,
): Promise<ModInfo> {
  return invoke('install_mod_command', {
    zipPath,
    customType,
    pakDestination,
    customName,
  });
}

export async function buildInstallManifest(
  zipPath: string,
  gamePath: string,
  pakDestination: string | null,
  customName: string | null
): Promise<InstallManifest> {
  return invoke('build_install_manifest', { zipPath, gamePath, pakDestination, customName });
}

export async function installModWithManifest(
  manifest: InstallManifest,
  zipPath: string
): Promise<ModInfo> {
  return invoke('install_mod_with_manifest', { manifest, zipPath });
}

export async function removeMod(modId: string): Promise<{ success: boolean }> {
  return invoke('remove_mod', { modId });
}

export async function disableMod(modId: string): Promise<{ success: boolean }> {
  return invoke('disable_mod', { modId });
}

export async function enableMod(modId: string): Promise<{ success: boolean }> {
  return invoke('enable_mod', { modId });
}

export async function readConfig(modId: string): Promise<{ content: string | null; path: string | null; configType: string | null }> {
  return invoke('read_config', { modId });
}

export async function saveConfig(modId: string, content: string): Promise<{ success: boolean }> {
  return invoke('save_config', { modId, content });
}

export async function setNexusModId(modId: string, nexusId: number): Promise<ModInfo> {
  return invoke('set_nexus_mod_id', { modIdStr: modId, nexusId });
}

export async function checkForUpdates(): Promise<UpdateCheckResult[]> {
  return invoke('check_for_updates');
}

export async function disableAllMods(): Promise<{ success: boolean; disabled: number }> {
  return invoke('disable_all_mods');
}

export async function enableAllMods(): Promise<{ success: boolean; enabled: number }> {
  return invoke('enable_all_mods');
}

export async function setModConfig(modId: string, configPath: string | null): Promise<ModInfo> {
  return invoke('set_mod_config', { modId, configPath });
}

export async function setModConfigs(modId: string, configPaths: string[] | null): Promise<ModInfo> {
  return invoke('set_mod_configs', { modId, configPaths });
}

export async function saveModNotes(modId: string, notes: string): Promise<void> {
  return invoke('save_mod_notes', { modId, notes });
}

export async function renameMod(modId: string, newName: string): Promise<ModInfo> {
  return invoke('rename_mod', { modId, newName });
}

export async function setModVersion(modId: string, version: string): Promise<ModInfo> {
  return invoke('set_mod_version', { modId, version });
}

export async function setModIgnoredKeys(modId: string, ignoredKeys: string[]): Promise<ModInfo> {
  return invoke('set_mod_ignored_keys', { modId, ignoredKeys });
}

export async function checkGitHubVersion(repo: string): Promise<string> {
  return invoke('check_github_version', { repo });
}

export async function setGithubVersion(modId: string, repo: string, version: string): Promise<ModInfo> {
  return invoke('set_github_version', { modId, repo, version });
}

export async function exportModsJson(path: string): Promise<string> {
  return invoke('export_mods_json', { path });
}

export async function getGameVersion(): Promise<string | null> {
  return invoke('get_game_version');
}

export async function checkModExistsCommand(zipPath: string): Promise<{ exists: boolean; modInfo: ModInfo | null; modFolderName: string | null }> {
  return invoke('check_mod_exists_command', { zipPath });
}

export async function updateModCommand(zipPath: string, modId: string): Promise<ModInfo> {
  return invoke('update_mod_command', { zipPath, modId });
}

export async function changePakDestination(modId: string, destination: string): Promise<ModInfo> {
  return invoke('change_pak_destination', { modId, destination });
}

export async function ignoreModVersion(modId: string, version: string | null): Promise<void> {
  return invoke('ignore_mod_version', { modId, version });
}

export async function previewConfigDiff(zipPath: string, modId: string): Promise<ConfigDiff[]> {
  return invoke('preview_config_diff', { zipPath, modId });
}

export async function scanConflicts(): Promise<any> {
  return invoke('scan_conflicts');
}

export async function mergeModsAsHybrid(primaryModId: string, secondaryModId: string): Promise<ModInfo> {
  return invoke('merge_mods_as_hybrid', { primaryModId, secondaryModId });
}

export async function checkArchivedConfig(nexusModId: number | null, modName: string): Promise<ArchivedConfigInfo | null> {
  return invoke('check_archived_config', { nexusModId, modName });
}

export async function previewArchivedConfigDiff(zipPath: string, archiveId: string): Promise<ConfigDiff[]> {
  return invoke('preview_archived_config_diff', { zipPath, archiveId });
}

export async function applyArchivedConfig(
  modId: string,
  archiveId: string,
  ignoredFiles?: string[],
  ignoredKeys?: string[],
): Promise<boolean> {
  return invoke('apply_archived_config', {
    modId,
    archiveId,
    ignoredFiles: ignoredFiles || null,
    ignoredKeys: ignoredKeys || null,
  });
}
