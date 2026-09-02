import { invoke } from '@tauri-apps/api/core';
import type {
  SaveWorldSummary,
  SaveHealthReport,
  SaveRepairResult,
  PmmWorldBackup,
  WorldCustomMeta,
  SaveBackupSnapshot,
} from './types';

export async function listSaveWorlds(customDir?: string): Promise<SaveWorldSummary[]> {
  return invoke('list_save_worlds_cmd', { customDir: customDir || null });
}

export async function deepScanSaveHealth(worldDir: string): Promise<SaveHealthReport> {
  return invoke('deep_scan_save_cmd', { worldDir });
}

export async function repairSaveHealth(worldDir: string): Promise<SaveRepairResult> {
  return invoke('repair_save_cmd', { worldDir });
}

export async function restoreSaveBackup(worldDir: string, backupSlot: string): Promise<SaveRepairResult> {
  return invoke('restore_save_backup_cmd', { worldDir, backupSlot });
}

export async function listPmmWorldBackups(worldNameFilter?: string): Promise<PmmWorldBackup[]> {
  return invoke('list_pmm_world_backups_cmd', { worldNameFilter: worldNameFilter || null });
}

export async function restorePmmWorldBackup(worldDir: string, backupFilePath: string): Promise<SaveRepairResult> {
  return invoke('restore_pmm_world_backup_cmd', { worldDir, backupFilePath });
}

export async function deletePmmWorldBackup(backupFilePath: string): Promise<void> {
  return invoke('delete_pmm_world_backup_cmd', { backupFilePath });
}

export async function openPmmWorldBackupsFolder(): Promise<void> {
  return invoke('open_pmm_world_backups_folder_cmd');
}

export async function createWorldBackup(worldDir: string, customDest?: string): Promise<string> {
  return invoke('create_world_backup_cmd', { worldDir, customDest: customDest || null });
}

export async function openWorldFolder(worldDir: string): Promise<void> {
  return invoke('open_world_folder_cmd', { worldDir });
}

export async function exportWorldZip(worldDir: string, targetPath: string): Promise<string> {
  return invoke('export_world_zip_cmd', { worldDir, targetPath });
}

export async function pruneWorldBackups(worldDir: string, keepCount: number = 5): Promise<number> {
  return invoke('prune_world_backups_cmd', { worldDir, keepCount });
}

export async function saveWorldCustomMeta(worldDir: string, meta: WorldCustomMeta): Promise<void> {
  return invoke('save_world_custom_meta_cmd', { worldDir, meta });
}

export async function getWorldCustomMeta(worldDir: string): Promise<WorldCustomMeta> {
  return invoke('get_world_custom_meta_cmd', { worldDir });
}

export async function inspectSnapshotDetails(worldDir: string, slotName: string): Promise<SaveBackupSnapshot> {
  return invoke('inspect_snapshot_details_cmd', { worldDir, slotName });
}
