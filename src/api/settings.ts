import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from './types';

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

export async function setFolderExpandMode(mode: 'always_expanded' | 'always_collapsed' | 'remember'): Promise<AppSettings> {
  return invoke('set_folder_expand_mode', { mode });
}

export async function setToolbarScale(scale: number): Promise<AppSettings> {
  return invoke('set_toolbar_scale', { scale });
}

export async function setUe4ssControlMode(mode: string): Promise<AppSettings> {
  return invoke('set_ue4ss_control_mode', { mode });
}

export async function setUe4ssBuildFlavor(flavor: string): Promise<AppSettings> {
  return invoke('set_ue4ss_build_flavor', { flavor });
}

export async function setDnsResolver(dnsMode: string): Promise<AppSettings> {
  return invoke('set_dns_resolver', { dnsMode });
}

export async function setCacheRemoteImages(enabled: boolean): Promise<AppSettings> {
  return invoke('set_cache_remote_images', { enabled });
}
