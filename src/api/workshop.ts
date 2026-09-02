import { invoke } from '@tauri-apps/api/core';
import type { WorkshopOnlineCheckResult } from './types';

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

export async function prepareWorkshopUpdateZip(packageName: string): Promise<string> {
  return invoke('prepare_workshop_update_zip', { packageName });
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

export async function checkWorkshopUpdatesOnline(): Promise<WorkshopOnlineCheckResult> {
  return invoke('check_workshop_updates_online_cmd');
}

export async function triggerSteamValidation(): Promise<void> {
  return invoke('trigger_steam_validation_cmd');
}
