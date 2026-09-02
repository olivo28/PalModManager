import { invoke } from '@tauri-apps/api/core';
import type { Profile, ModInfo, ImportProfileResult } from './types';

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

export async function exportProfilePack(profileId: string, targetPath: string): Promise<string> {
  return invoke('export_profile_pack_cmd', { profileId, targetPath });
}

export async function importProfilePack(sourcePath: string, customName?: string): Promise<ImportProfileResult> {
  return invoke('import_profile_pack_cmd', { sourcePath, customName });
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
