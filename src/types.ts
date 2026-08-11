export type ModType = 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'hybrid';

export interface ModInfo {
  id: string;
  name: string;
  type: ModType;
  nexusModId: number | null;
  nexusUrl: string | null;
  nexusAuthor: string | null;
  nexusSummary: string | null;
  nexusPictureUrl: string | null;
  nexusEndorsements: number | null;
  nexusDownloads: number | null;
  version: string;
  installDate: string;
  sourceZip: string;
  configPath: string | null;
  configType: string | null;
  enabled: boolean;
  gamePath: string;
  disabledPath: string;
  pakDestination: string | null;
  hasEnabledTxt: boolean;
  modsTxtOrder: number | null;
  extraFiles: string[];
  nexusDescription: string | null;
  nexusVersionCached: string | null;
  nexusCachedAt: string | null;
  nexusCategory: string | null;
  nexusTags: string[];
  githubRepo: string | null;
  githubVersion: string | null;
  githubCachedAt: string | null;
  updateDate: string | null;
  libraryZip: string | null;
  ignoredVersion: string | null;
  nexusFileId: number | null;
  ignoredKeys?: string[] | null;
}

export interface AppSettings {
  gamePath: string;
  programPath: string;
  hideNativeMods?: boolean;
  debugConsole?: boolean;
  forceLoadOrder?: boolean;
  forceLoadOrderUe4ss?: boolean;
  forceLoadOrderPalschema?: boolean;
  customDataPath?: string | null;
  toolbarScale?: number;
}

export interface ModFolder {
  id: string;
  name: string;
  mod_ids: string[];
}

export interface Profile {
  id: string;
  name: string;
  created_at: string;
  installed_mod_ids: string[];
  enabled_mod_ids: string[];
  ue4ss_enabled: boolean;
  palschema_enabled: boolean;
  mod_folders?: ModFolder[];
  force_load_order_ue4ss?: boolean | null;
  force_load_order_palschema?: boolean | null;
  hide_native_mods?: boolean | null;
}

export interface LibraryEntry {
  modId: string;
  zipName: string;
  zipSize: number;
  installedAt: string;
  nexusPictureUrl?: string | null;
  nexusName?: string | null;
  nexusAuthor?: string | null;
  nexusSummary?: string | null;
  nexusModId?: number | null;
  nexusVersion?: string | null;
}


export interface DependencyStatus {
  ue4ss_installed: boolean;
  ue4ss_version: string | null;
  /** Tag name of latest UE4SS release, e.g. "experimental-palworld" */
  ue4ss_latest_tag: string | null;
  /** Date of the latest asset update in DD.MM.YYYY format */
  ue4ss_latest_date: string | null;
  ue4ss_needs_update: boolean;
  palschema_installed: boolean;
  palschema_version: string | null;
  palschema_latest_version: string | null;
  palschema_needs_update: boolean;
  game_platform: string;
}

export type RouteType = 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'companion' | 'passthrough';

export interface FileRoute {
  zipPath: string;
  destPath: string;
  routeType: RouteType;
}

export interface InstallManifest {
  folderName: string;
  displayName: string;
  modType: ModType;
  routes: FileRoute[];
  nexusModId: number | null;
  nexusFileId: number | null;
  hasPak: boolean;
  hasUe4ss: boolean;
  hasPalschema: boolean;
  version: string;
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

