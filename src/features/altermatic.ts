import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../ui/toast';
import { t } from '../utils/i18n';

export interface AltermaticDepStatus {
  altermaticPresent: boolean;
  unipaluiPresent: boolean;
}

/**
 * Synchronizes the Altermatic _LoadList.json with the currently enabled mods
 */
export async function syncAltermaticLoadList(showSuccessToast: boolean = false): Promise<string[]> {
  try {
    const list = await invoke<string[]>('sync_altermatic_load_list');
    if (showSuccessToast && list.length > 0) {
      showToast(t('altermatic.load_list_synced', { count: list.length }), 'success');
    }
    return list;
  } catch (err) {
    console.error('Failed to sync Altermatic load list:', err);
    return [];
  }
}

/**
 * Checks if Altermatic.pak and UniPalUI.pak are installed in LogicMods
 */
export async function getAltermaticDepStatus(): Promise<AltermaticDepStatus> {
  try {
    return await invoke<AltermaticDepStatus>('get_altermatic_dep_status');
  } catch (err) {
    console.error('Failed to check Altermatic dependencies:', err);
    return { altermaticPresent: false, unipaluiPresent: false };
  }
}

/**
 * Checks dependencies and displays warnings/toasts if required components are missing
 */
export async function checkAndWarnAltermaticDeps(): Promise<boolean> {
  const status = await getAltermaticDepStatus();
  
  if (!status.altermaticPresent) {
    showToast(t('altermatic.dep_missing_altermatic'), 'error');
    return false;
  }

  if (!status.unipaluiPresent) {
    showToast(t('altermatic.dep_missing_unipalui'), 'warning');
  }

  return true;
}
