import { invoke } from '@tauri-apps/api/core';
import type { DependencyStatus } from './types';
import type { DependencyVaultEntry } from '../types';

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

export async function getDependencyVault(depType: 'ue4ss' | 'palschema'): Promise<DependencyVaultEntry[]> {
  return invoke('get_dependency_vault', { depType });
}

export async function installDependencyFromVault(depType: 'ue4ss' | 'palschema', filename: string): Promise<string> {
  return invoke('install_dependency_from_vault', { depType, filename });
}

export async function installDependencyFromCustomZip(
  depType: 'ue4ss' | 'palschema',
  zipPath: string,
  customVersion?: string
): Promise<string> {
  return invoke('install_dependency_from_custom_zip', { depType, zipPath, customVersion: customVersion || null });
}

export async function deleteDependencyVaultEntry(depType: 'ue4ss' | 'palschema', filename: string): Promise<void> {
  return invoke('delete_dependency_vault_entry', { depType, filename });
}

export async function openDependencyVaultFolder(depType: 'ue4ss' | 'palschema'): Promise<void> {
  return invoke('open_dependency_vault_folder', { depType });
}
