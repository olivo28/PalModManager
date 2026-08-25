import { invoke } from '@tauri-apps/api/core';
import type {
  ModInfo,
  AppSettings,
  Profile,
  LibraryEntry,
  DependencyStatus,
  InstallManifest,
  SafetyBackupInfo,
  FileRoute,
  RouteType,
  NexusAccountInfo,
  ProtocolStatus,
  DetailedProtocolInfo,
  NexusUserEndorsement,
  NexusUserTrackedMod,
  NexusUserAuthoredMod,
  NxmModMetadata,
  NxmDownloadProgressEvent,
  DiscoveryCategory,
  DiscoveryModItem,
  DiscoveryFileItem,
  DiscoveryModDetails,
  DiscoveryResponse,
} from './types';

export type {
  InstallManifest,
  FileRoute,
  RouteType,
  ModInfo,
  AppSettings,
  Profile,
  LibraryEntry,
  DependencyStatus,
  SafetyBackupInfo,
  NexusAccountInfo,
  ProtocolStatus,
  DetailedProtocolInfo,
  NexusUserEndorsement,
  NexusUserTrackedMod,
  NexusUserAuthoredMod,
  NxmModMetadata,
  NxmDownloadProgressEvent,
  DiscoveryCategory,
  DiscoveryModItem,
  DiscoveryFileItem,
  DiscoveryModDetails,
  DiscoveryResponse,
};

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

export async function setForceLoadOrder(enabled: boolean): Promise<AppSettings> {
  return invoke('set_force_load_order', { enabled });
}

export async function setForceLoadOrderUe4ss(enabled: boolean): Promise<AppSettings> {
  return invoke('set_force_load_order_ue4ss', { enabled });
}

export async function setForceLoadOrderPalschema(enabled: boolean): Promise<AppSettings> {
  return invoke('set_force_load_order_palschema', { enabled });
}

export async function setCustomDataPath(path: string | null): Promise<AppSettings> {
  return invoke('set_custom_data_path', { path });
}

export async function setLanguage(language: string): Promise<AppSettings> {
  return invoke('set_language', { language });
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

export async function openPath(path: string): Promise<void> {
  return invoke('open_path', { path });
}

export async function launchGame(): Promise<void> {
  return invoke('launch_game');
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

// Change pak destination
export async function changePakDestination(modId: string, destination: string): Promise<ModInfo> {
  return invoke('change_pak_destination', { modId, destination });
}

// Library
export async function getLibrary(): Promise<LibraryEntry[]> {
  return invoke('get_library');
}

export async function checkLibraryUpdates(): Promise<UpdateCheckResult[]> {
  return invoke('check_library_updates');
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

export async function reorderModFolders(profileId: string, folderIds: string[]): Promise<Profile> {
  return invoke('reorder_mod_folders_command', { profileId, folderIds });
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

export async function removeFromLibrary(modId: string, zipName?: string): Promise<{ success: boolean }> {
  return invoke('remove_from_library', { modId, zipName: zipName || null });
}

export async function fetchNexusInfoAsync(modId: number): Promise<any> {
  return invoke('fetch_nexus_info_async', { modId });
}

export async function getLibraryZipPath(modId: string, zipName?: string): Promise<string> {
  return invoke('get_library_zip_path', { modId, zipName: zipName || null });
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

export interface ModHotkey {
  modId: string;
  modName: string;
  filePath: string;
  absoluteFilePath: string;
  lineNumber: number;
  keys: string;
  rawLine: string;
}

export async function scanModHotkeys(): Promise<ModHotkey[]> {
  return invoke('scan_mod_hotkeys');
}

export async function updateModHotkey(absoluteFilePath: string, lineNumber: number, newKeys: string): Promise<void> {
  return invoke('update_mod_hotkey', { absoluteFilePath, lineNumber, newKeys });
}

export async function ignoreModVersion(modId: string, version: string | null): Promise<void> {
  return invoke('ignore_mod_version', { modId, version });
}

export async function setToolbarScale(scale: number): Promise<AppSettings> {
  return invoke('set_toolbar_scale', { scale });
}

export interface ChangedKeyDetail {
  key: string;
  old_value: string;
  new_value: string;
}

export interface ConfigDiff {
  file_name: string;
  keys_user_changed: ChangedKeyDetail[];
  keys_added_by_author: string[];
  keys_removed_by_author: string[];
}

export async function previewConfigDiff(zipPath: string, modId: string): Promise<ConfigDiff[]> {
  return invoke('preview_config_diff', { zipPath, modId });
}

// Workshop APIs
export async function getWorkshopMods(): Promise<any[]> {
  return invoke('get_workshop_mods');
}

export async function getWorkshopState(): Promise<any> {
  return invoke('get_workshop_state');
}

export async function activateWorkshopMod(packageName: string): Promise<void> {
  return invoke('activate_workshop_mod_cmd', { packageName });
}

export async function deactivateWorkshopMod(packageName: string): Promise<void> {
  return invoke('deactivate_workshop_mod_cmd', { packageName });
}

export async function setWorkshopGlobalEnabled(enabled: boolean): Promise<void> {
  return invoke('set_workshop_global_enabled', { enabled });
}

export async function cleanConflictDlls(): Promise<string[]> {
  return invoke('clean_conflict_dlls');
}

export async function resetWorkshopCache(): Promise<void> {
  return invoke('reset_workshop_cache');
}

export async function getSafetyBackupInfo(): Promise<SafetyBackupInfo> {
  return invoke('get_safety_backup_info_command');
}

export async function triggerSafetyBackup(): Promise<boolean> {
  return invoke('trigger_safety_backup_command');
}

export async function restoreSafetyBackup(): Promise<void> {
  return invoke('restore_safety_backup_command');
}

export interface StorageUsageInfo {
  tempDownloadsSize: number;
  tempDownloadsCount: number;
  tempDownloadsPath: string;
  librarySize: number;
  libraryModsCount: number;
  libraryZipsCount: number;
  libraryPath: string;
}

export async function getStorageUsage(): Promise<StorageUsageInfo> {
  return invoke('get_storage_usage_command');
}

export async function clearTempDownloads(): Promise<number> {
  return invoke('clear_temp_downloads_command');
}

export async function openTempFolder(): Promise<void> {
  return invoke('open_temp_folder_command');
}

export async function openLibraryFolder(): Promise<void> {
  return invoke('open_library_folder_command');
}

export async function prepareWorkshopUpdateZip(packageName: string): Promise<string> {
  return invoke('prepare_workshop_update_zip', { packageName });
}

export async function scanConflicts(): Promise<any> {
  return invoke('scan_conflicts');
}

export async function getUe4ssLoadOrder(): Promise<any[]> {
  return invoke('get_ue4ss_load_order');
}

export async function saveUe4ssLoadOrder(order: any[]): Promise<void> {
  return invoke('save_ue4ss_load_order', { order });
}

export async function getPalschemaLoadOrder(): Promise<any[]> {
  return invoke('get_palschema_load_order');
}

export async function savePalschemaLoadOrder(order: any[]): Promise<void> {
  return invoke('save_palschema_load_order', { order });
}

export interface WorkshopOnlineModItem {
  workshopId: number;
  modName: string;
  packageName: string;
  localTimeUpdated: number;
  remoteTimeUpdated: number;
  hasRemoteUpdate: boolean;
  isDownloadedToDisk: boolean;
}

export interface WorkshopOnlineCheckResult {
  totalChecked: number;
  pendingSteamDownloads: WorkshopOnlineModItem[];
  readyToInstallUpdates: WorkshopOnlineModItem[];
}

export async function checkWorkshopUpdatesOnline(): Promise<WorkshopOnlineCheckResult> {
  return invoke('check_workshop_updates_online_cmd');
}

export async function triggerSteamValidation(): Promise<void> {
  return invoke('trigger_steam_validation_cmd');
}

export async function startNexusOAuth(): Promise<string> {
  return invoke('start_nexus_oauth');
}

export async function handleNexusOAuthCallback(callbackUrl: string): Promise<NexusAccountInfo> {
  return invoke('handle_nexus_oauth_callback', { callbackUrl });
}

export async function getNexusAccountStatus(): Promise<NexusAccountInfo | null> {
  return invoke('get_nexus_account_status');
}

export async function refreshNexusAccountProfile(): Promise<NexusAccountInfo | null> {
  return invoke('refresh_nexus_account_profile');
}

export async function logoutNexusAccount(): Promise<void> {
  return invoke('logout_nexus_account');
}

export async function checkNexusProtocolStatus(): Promise<DetailedProtocolInfo> {
  return invoke('check_nexus_protocol_status');
}

export async function registerNexusProtocol(scheme?: string): Promise<string> {
  return invoke('register_nexus_protocol', { scheme });
}

export async function unregisterNexusProtocol(scheme?: string): Promise<void> {
  return invoke('unregister_nexus_protocol', { scheme });
}

export async function getNexusUserEndorsements(forceRefresh?: boolean): Promise<NexusUserEndorsement[]> {
  return invoke('get_nexus_user_endorsements', { forceRefresh: !!forceRefresh });
}

export async function getNexusUserTrackedMods(forceRefresh?: boolean): Promise<NexusUserTrackedMod[]> {
  return invoke('get_nexus_user_tracked_mods', { forceRefresh: !!forceRefresh });
}

export async function getNexusUserAuthoredMods(forceRefresh?: boolean): Promise<NexusUserAuthoredMod[]> {
  return invoke('get_nexus_user_authored_mods', { forceRefresh: !!forceRefresh });
}

export async function handleNxmDownload(nxmUrl: string): Promise<any> {
  return invoke('handle_nxm_download', { nxmUrl });
}

export async function parseNxmLink(nxmUrl: string): Promise<any> {
  return invoke('parse_nxm_link', { nxmUrl });
}

export async function getNxmModMetadata(gameDomain: string, modId: number): Promise<NxmModMetadata> {
  return invoke('get_nxm_mod_metadata', { gameDomain, modId });
}

export async function downloadNxmFile(nxmUrl: string, downloadId: string): Promise<string> {
  return invoke('download_nxm_file', { nxmUrl, downloadId });
}

export async function getDiscoveryCategories(): Promise<DiscoveryCategory[]> {
  return invoke('get_discovery_categories');
}

export async function getDiscoveryMods(params?: {
  query?: string;
  descriptionQuery?: string;
  authorQuery?: string;
  uploaderQuery?: string;
  categoryId?: number;
  categoryName?: string;
  adultFilter?: string;
  supportsVortex?: boolean;
  hasUpdated?: boolean;
  languageName?: string;
  languageNames?: string[];
  includeTags?: string[];
  excludeTags?: string[];
  hideTranslations?: boolean;
  sortBy?: string;
  timeRange?: string;
  includeAdult?: boolean;
  page?: number;
  pageSize?: number;
}): Promise<DiscoveryResponse> {
  return invoke('get_discovery_mods', {
    query: params?.query || null,
    descriptionQuery: params?.descriptionQuery || null,
    authorQuery: params?.authorQuery || null,
    uploaderQuery: params?.uploaderQuery || null,
    categoryId: params?.categoryId || null,
    categoryName: params?.categoryName || null,
    adultFilter: params?.adultFilter || null,
    supportsVortex: params?.supportsVortex !== undefined ? params.supportsVortex : null,
    hasUpdated: params?.hasUpdated !== undefined ? params.hasUpdated : null,
    languageName: params?.languageName || null,
    languageNames: params?.languageNames || null,
    includeTags: params?.includeTags || null,
    excludeTags: params?.excludeTags || null,
    hideTranslations: params?.hideTranslations !== undefined ? params.hideTranslations : null,
    sortBy: params?.sortBy || null,
    timeRange: params?.timeRange || null,
    includeAdult: params?.includeAdult !== undefined ? params.includeAdult : null,
    page: params?.page || 1,
    pageSize: params?.pageSize || 36,
  });
}

export async function getDiscoveryModDetails(modId: number): Promise<DiscoveryModDetails> {
  return invoke('get_discovery_mod_details', { modId });
}

export async function endorseNexusMod(modId: number, version?: string): Promise<boolean> {
  return invoke('endorse_nexus_mod', { modId, version: version || null });
}

export async function abstainNexusMod(modId: number, version?: string): Promise<boolean> {
  return invoke('abstain_nexus_mod', { modId, version: version || null });
}

export async function trackNexusMod(modId: number): Promise<boolean> {
  return invoke('track_nexus_mod', { modId });
}

export async function untrackNexusMod(modId: number): Promise<boolean> {
  return invoke('untrack_nexus_mod', { modId });
}

export async function installDiscoveryFile(modId: number, fileId: number): Promise<string> {
  return invoke('install_discovery_file', { modId, fileId });
}









