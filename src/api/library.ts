import { invoke } from '@tauri-apps/api/core';
import type { LibraryEntry, ModInfo, UpdateCheckResult, StorageUsageInfo } from './types';

export async function getLibrary(): Promise<LibraryEntry[]> {
  return invoke('get_library');
}

export async function checkLibraryUpdates(): Promise<UpdateCheckResult[]> {
  return invoke('check_library_updates');
}

export async function installModFromLibrary(modId: string): Promise<ModInfo> {
  return invoke('install_mod_from_library', { modId });
}

export async function removeFromLibrary(modId: string, zipName?: string): Promise<{ success: boolean }> {
  return invoke('remove_from_library', { modId, zipName: zipName || null });
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
