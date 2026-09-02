import { invoke } from '@tauri-apps/api/core';
import type {
  EditorDiagnostic,
  EditorCompletion,
  PakInspectionResult,
  TexturePreviewInfo,
  UAssetInspectionDetails,
  PatchBuildRequest,
  PatchBuildResult,
  GeneratedPatchInfo,
  ModHotkey,
  SafetyBackupInfo,
  UsmapSearchResult,
  UsmapStructFullDetails,
  UsmapSummaryData,
  SdkStatus,
} from './types';
import type { UsmapStatus } from '../types';

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

export async function listModFiles(modId: string): Promise<string[]> {
  return invoke('list_mod_files', { modId });
}

export async function readModFile(modId: string, filePath: string): Promise<{
  content: string | null;
  path: string | null;
  configType: string | null;
  modifiedTime?: number;
  fileSize?: number;
  modVersion?: string;
}> {
  return invoke('read_mod_file', { modId, filePath });
}

export async function saveModFile(modId: string, filePath: string, content: string): Promise<{ success: boolean }> {
  return invoke('save_mod_file', { modId, filePath, content });
}

export async function deleteModFile(modId: string, filePath: string): Promise<{ success: boolean }> {
  return invoke('delete_mod_file', { modId, filePath });
}

export async function restoreModBackup(modId: string, backupFilePath: string): Promise<{ success: boolean; targetPath: string; content: string }> {
  return invoke('restore_mod_backup', { modId, backupFilePath });
}

export async function mergeModBackup(modId: string, backupFilePath: string): Promise<{ success: boolean; targetPath: string; content: string }> {
  return invoke('merge_mod_backup', { modId, backupFilePath });
}

export async function validateEditorCode(filePath: string, content: string): Promise<EditorDiagnostic[]> {
  return invoke('validate_editor_code', { filePath, content });
}

export async function scanWorkspaceProblems(modId: string): Promise<Record<string, EditorDiagnostic[]>> {
  return invoke('scan_workspace_problems', { modId });
}

export async function getEditorCompletions(filePath: string, query: string, linePrefix: string): Promise<EditorCompletion[]> {
  return invoke('get_editor_completions', { filePath, query, linePrefix });
}

export async function inspectPakAsset(modId: string, assetInternalPath: string): Promise<string[]> {
  return invoke('inspect_pak_asset', { modId, assetInternalPath });
}

export async function inspectUAssetDeep(params: {
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  zipPath?: string | null;
}): Promise<UAssetInspectionDetails> {
  return invoke('inspect_uasset_deep_cmd', {
    modId: params.modId || null,
    pakPath: params.pakPath || null,
    assetInternalPath: params.assetInternalPath,
    zipPath: params.zipPath || null,
  });
}

export async function decodeUAssetTexture(params: {
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  zipPath?: string | null;
}): Promise<TexturePreviewInfo> {
  return invoke('decode_uasset_texture_cmd', {
    modId: params.modId || null,
    pakPath: params.pakPath || null,
    assetInternalPath: params.assetInternalPath,
    zipPath: params.zipPath || null,
  });
}

export async function inspectPakFileTree(pakPath: string, zipPath?: string): Promise<PakInspectionResult> {
  return invoke('inspect_pak_file_tree', { pakPath, zipPath: zipPath || null });
}

export async function inspectModPakContents(modId: string): Promise<PakInspectionResult[]> {
  return invoke('inspect_mod_pak_contents', { modId });
}

export async function convertModToGamepass(modId: string): Promise<string[]> {
  return invoke('convert_mod_to_gamepass', { modId });
}

export async function convertAllGamepassMods(): Promise<number> {
  return invoke('convert_all_gamepass_mods');
}

export async function buildCompatibilityPak(request: PatchBuildRequest): Promise<PatchBuildResult> {
  return invoke('build_compatibility_pak_cmd', { request });
}

export async function listGeneratedPatches(): Promise<GeneratedPatchInfo[]> {
  return invoke('list_generated_patches_cmd');
}

export async function deleteGeneratedPatch(patchPath: string): Promise<void> {
  return invoke('delete_generated_patch_cmd', { patchPath });
}

export async function scanModHotkeys(): Promise<ModHotkey[]> {
  return invoke('scan_mod_hotkeys');
}

export async function updateModHotkey(
  absoluteFilePath: string,
  lineNumber: number,
  newKeys: string,
  isVariable?: boolean,
  variableName?: string | null,
): Promise<void> {
  return invoke('update_mod_hotkey', {
    absoluteFilePath,
    lineNumber,
    newKeys,
    isVariable: isVariable ?? false,
    variableName: variableName ?? null,
  });
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

export async function getMappingsStatus(): Promise<UsmapStatus> {
  return invoke('get_mappings_status');
}

export async function syncMappingsNow(): Promise<UsmapStatus> {
  return invoke('sync_mappings_now');
}

export async function getUsmapSummary(): Promise<UsmapSummaryData> {
  return invoke('get_usmap_summary');
}

export async function searchUsmapEntries(
  query: string,
  filterType: 'all' | 'structs' | 'enums' | 'names' = 'all',
  page: number = 0,
  pageSize: number = 50
): Promise<UsmapSearchResult> {
  return invoke('search_usmap_entries', { query, filterType, page, pageSize });
}

export async function getUsmapFullStructDetails(name: string): Promise<UsmapStructFullDetails | null> {
  return invoke('get_usmap_full_struct_details', { name });
}

export async function getUsmapEnumInfo(enumName: string): Promise<string[] | null> {
  return invoke('get_usmap_enum_info', { enumName });
}

export async function getSdkStatus(): Promise<SdkStatus> {
  return invoke('get_sdk_status');
}

export async function importLocalSdk(folderPath?: string): Promise<SdkStatus> {
  return invoke('import_local_sdk', { folderPath: folderPath || null });
}

export async function syncSdkFromRepo(): Promise<SdkStatus> {
  return invoke('sync_sdk_from_repo');
}

export async function purgeSdkCache(): Promise<SdkStatus> {
  return invoke('purge_sdk_cache');
}

export async function logFromJs(msg: string): Promise<void> {
  return invoke('log_from_js', { msg });
}

export async function openUrl(url: string): Promise<void> {
  return invoke('open_url', { url });
}
