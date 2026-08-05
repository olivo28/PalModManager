import { invoke } from '@tauri-apps/api/core';
import type { ModInfo, AppSettings, Profile, LibraryEntry, DependencyStatus } from './types';

export interface ZipAnalysis {
  zipPath: string;
  detectedType: string;
  hasLua: boolean;
  hasJson: boolean;
  hasPalSchemaJson: boolean;
  hasPak: boolean;
  hasInfoJson: boolean;
  pakDestinationHint: string | null;
  rootFolder: string | null;
  fileCount: number;
  nexusModId: number | null;
  detectedVersion?: string | null;
  files?: string[];
  nexusInfo: {
    name: string;
    author: string;
    summary: string;
    version: string;
    downloads: number;
    endorsements: number;
    pictureUrl: string;
  } | null;
  modinfo?: {
    name?: string;
    version?: string;
    description?: string;
    author?: string;
    modType?: string;
  } | null;
}

export async function getSettings(): Promise<AppSettings> {
  return invoke('get_settings');
}

export async function setGamePath(path: string): Promise<AppSettings> {
  return invoke('set_game_path', { path });
}

export async function setHideNativeMods(hide: boolean): Promise<AppSettings> {
  return invoke('set_hide_native_mods', { hide });
}

export async function setDebugConsole(enabled: boolean): Promise<AppSettings> {
  return invoke('set_debug_console', { enabled });
}

export async function setCustomDataPath(path: string | null): Promise<AppSettings> {
  return invoke('set_custom_data_path', { path });
}


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

export async function fetchNexusInfo(modId: number): Promise<any> {
  return invoke('fetch_nexus_info_async', { modId });
}

export async function refreshNexusCache(modIdStr: string): Promise<ModInfo> {
  return invoke('refresh_nexus_cache', { modIdStr });
}

export interface UpdateCheckResult {
  modId: string;
  name: string;
  currentVersion: string;
  latestVersion: string;
  nexusModId: number;
}

export async function setNexusModId(modId: string, nexusId: number): Promise<ModInfo> {
  return invoke('set_nexus_mod_id', { modIdStr: modId, nexusId });
}

export async function openFolderByType(folderType: 'ue4ss' | 'palschema' | 'paks' | 'app_data' | 'profile'): Promise<void> {
  return invoke('open_folder_by_type', { folderType });
}

export async function openModFolder(modId: string): Promise<void> {
  return invoke('open_folder', { modId });
}

export async function openExtraFolder(modId: string): Promise<void> {
  return invoke('open_extra_folder', { modId });
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

export async function listModFiles(modId: string): Promise<string[]> {
  return invoke('list_mod_files', { modId });
}

export async function readModFile(modId: string, filePath: string): Promise<{ content: string | null; path: string | null; configType: string | null }> {
  return invoke('read_mod_file', { modId, filePath });
}

export async function saveModFile(modId: string, filePath: string, content: string): Promise<{ success: boolean }> {
  return invoke('save_mod_file', { modId, filePath, content });
}

export async function renameMod(modId: string, newName: string): Promise<ModInfo> {
  return invoke('rename_mod', { modId, newName });
}

export async function setModVersion(modId: string, version: string): Promise<ModInfo> {
  return invoke('set_mod_version', { modId, version });
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

// Game version
export async function getGameVersion(): Promise<string | null> {
  return invoke('get_game_version');
}

// Check mod exists (for update detection)
export async function checkModExistsCommand(zipPath: string): Promise<{ exists: boolean; modInfo: ModInfo | null; modFolderName: string | null }> {
  return invoke('check_mod_exists_command', { zipPath });
}

// Update mod
export async function updateModCommand(zipPath: string, modId: string): Promise<ModInfo> {
  return invoke('update_mod_command', { zipPath, modId });
}

// Library
export async function getLibrary(): Promise<LibraryEntry[]> {
  return invoke('get_library');
}

export async function installModFromLibrary(modId: string): Promise<ModInfo> {
  return invoke('install_mod_from_library', { modId });
}

// Profiles
export async function getProfiles(): Promise<Profile[]> {
  return invoke('get_profiles');
}

export async function getCurrentProfile(): Promise<Profile> {
  return invoke('get_current_profile');
}

export async function switchProfile(profileId: string): Promise<ModInfo[]> {
  return invoke('switch_profile_command', { profileId });
}

export async function createProfile(name: string): Promise<Profile> {
  return invoke('create_profile_command', { name });
}

export async function cloneProfile(profileId: string, newName: string): Promise<Profile> {
  return invoke('clone_profile_command', { profileId, newName });
}

export async function deleteProfile(profileId: string): Promise<{ success: boolean }> {
  return invoke('delete_profile_command', { profileId });
}

export async function renameProfile(profileId: string, name: string): Promise<Profile> {
  return invoke('rename_profile_command', { profileId, name });
}

export async function clearProfile(profileId: string): Promise<Profile[]> {
  return invoke('clear_profile_command', { profileId });
}


export async function setModProfileState(modId: string, enabled: boolean): Promise<{ success: boolean }> {
  return invoke('set_mod_profile_state', { modId, enabled });
}

export async function createModFolder(profileId: string, name: string): Promise<Profile> {
  return invoke('create_mod_folder_command', { profileId, name });
}

export async function deleteModFolder(profileId: string, folderId: string): Promise<Profile> {
  return invoke('delete_mod_folder_command', { profileId, folderId });
}

export async function renameModFolder(profileId: string, folderId: string, newName: string): Promise<Profile> {
  return invoke('rename_mod_folder_command', { profileId, folderId, newName });
}

export async function addModToFolder(profileId: string, folderId: string | null, modId: string): Promise<Profile> {
  return invoke('add_mod_to_folder_command', { profileId, folderId, modId });
}

export async function toggleFolderMods(profileId: string, folderId: string, enabled: boolean): Promise<Profile> {
  return invoke('toggle_folder_mods_command', { profileId, folderId, enabled });
}

// Dependencies (UE4SS / PalSchema)
export async function checkDependencies(): Promise<DependencyStatus> {
  return invoke('check_dependencies');
}

export async function checkDependenciesFull(): Promise<DependencyStatus> {
  return invoke('check_dependencies_full');
}

export async function installUe4ss(forceDownload = true): Promise<string> {
  return invoke('install_ue4ss', { forceDownload });
}

export async function installPalschema(forceDownload = true): Promise<string> {
  return invoke('install_palschema', { forceDownload });
}

export async function uninstallUe4ss(): Promise<string> {
  return invoke('uninstall_ue4ss');
}

export async function uninstallPalschema(): Promise<string> {
  return invoke('uninstall_palschema');
}

export async function logFromJs(msg: string): Promise<void> {
  return invoke('log_from_js', { msg });
}

export async function openUrl(url: string): Promise<void> {
  return invoke('open_url', { url });
}

export async function removeFromLibrary(modId: string): Promise<{ success: boolean }> {
  return invoke('remove_from_library', { modId });
}

export async function fetchNexusInfoAsync(modId: number): Promise<any> {
  return invoke('fetch_nexus_info_async', { modId });
}

export async function getLibraryZipPath(modId: string): Promise<string> {
  return invoke('get_library_zip_path', { modId });
}

export async function copyToLibraryCommand(zipPath: string, modName?: string): Promise<LibraryEntry> {
  return invoke('copy_to_library_command', { zipPath, modName });
}

export async function createBackup(targetDir: string): Promise<string> {
  return invoke('create_backup', { targetDir });
}

export async function restoreBackup(zipPath: string): Promise<void> {
  return invoke('restore_backup', { zipPath });
}

export async function analyzeBackup(zipPath: string): Promise<{ hasUe4ss: boolean; hasPalSchema: boolean }> {
  return invoke('analyze_backup', { zipPath });
}



