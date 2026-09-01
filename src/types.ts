export type ModType = 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'hybrid' | 'altermatic';

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
  hasPendingUpdate?: boolean | null;
  originLoadMethod?: string | null;
  customNotes?: string | null;
}

export interface NexusAccountInfo {
  userId?: number | null;
  username?: string | null;
  avatarUrl?: string | null;
  isPremium: boolean;
  isSupporter: boolean;
  roles: string[];
  accessToken?: string | null;
  refreshToken?: string | null;
  tokenExpiresAt?: number | null;
  kudos?: number | null;
  profileViews?: number | null;
  endorsementsGiven?: number | null;
  joinedDate?: string | null;
  lastActiveDate?: string | null;
  aboutMe?: string | null;
  modCount?: number | null;
}

export interface NexusUserEndorsement {
  modId: number;
  domainName: string;
  date?: string | null;
  version?: string | null;
  status?: string | null;
  modTitle?: string | null;
  pictureUrl?: string | null;
  summary?: string | null;
}

export interface NexusUserTrackedMod {
  modId: number;
  domainName: string;
  modTitle?: string | null;
  pictureUrl?: string | null;
  summary?: string | null;
}

export interface NexusUserAuthoredMod {
  modId: number;
  name: string;
  summary?: string | null;
  version?: string | null;
  downloads?: number | null;
  endorsements?: number | null;
  pictureUrl?: string | null;
  gameName?: string | null;
  domainName?: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
}

export type ProtocolStatus = 
  | { status: 'Registered'; data: { path: string } }
  | { status: 'OutdatedPath'; data: { current_exe: string; registered_path: string } }
  | { status: 'NotRegistered' };

export interface DetailedProtocolInfo {
  palmodmanager: ProtocolStatus;
  nxm: ProtocolStatus;
  nxmHandlerName?: string | null;
  nxmHandlerPath?: string | null;
}

export interface NxmModMetadata {
  modId: number;
  name: string;
  summary?: string | null;
  pictureUrl?: string | null;
  version?: string | null;
  author?: string | null;
}

export interface NxmDownloadProgressEvent {
  downloadId: string;
  bytesDownloaded: number;
  totalBytes?: number | null;
  percentage: number;
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
  language?: string;
  nexusAccount?: NexusAccountInfo | null;
  dnsResolver?: string;
  cacheRemoteImages?: boolean;
  folderExpandMode?: 'always_expanded' | 'always_collapsed' | 'remember';
  ue4ssControlMode?: 'enabled_txt' | 'mods_txt' | string | null;
}

export interface ModFolder {
  id: string;
  name: string;
  mod_ids: string[];
}

export type DependencyMode = 'standard' | 'workshop' | 'none';

export interface Profile {
  id: string;
  name: string;
  created_at: string;
  installed_mod_ids: string[];
  enabled_mod_ids: string[];
  ue4ss_enabled: boolean;
  palschema_enabled: boolean;
  dependency_mode?: DependencyMode;
  mod_folders?: ModFolder[];
  force_load_order_ue4ss?: boolean | null;
  force_load_order_palschema?: boolean | null;
  hide_native_mods?: boolean | null;
  ue4ss_version?: string | null;
  palschema_version?: string | null;
  altermatic_version?: string | null;
  unipalui_version?: string | null;
  compatibility_patches?: string[] | null;
  ue4ssControlMode?: 'enabled_txt' | 'mods_txt' | string | null;
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
  author?: string | null;
  description?: string | null;
  version?: string | null;
  modType?: string | null;
  isInstalled?: boolean;
  installedVersion?: string | null;
}


export interface DependencyStatus {
  ue4ss_installed: boolean;
  ue4ss_version: string | null;
  /** Tag name of latest UE4SS release, e.g. "experimental-palworld" */
  ue4ss_latest_tag: string | null;
  /** Date of the latest asset update in DD.MM.YYYY format */
  ue4ss_latest_date: string | null;
  ue4ss_needs_update: boolean;
  /** How UE4SS was installed: "Standard", "Workshop", or "NotFound" */
  ue4ss_install_mode: string;
  palschema_installed: boolean;
  palschema_version: string | null;
  palschema_latest_version: string | null;
  palschema_needs_update: boolean;
  game_platform: string;
  has_dll_conflict?: boolean;
  conflicting_dlls?: string[];
  ue4ss_updated_from?: string | null;
  palschema_updated_from?: string | null;
  altermatic_installed?: boolean;
  unipalui_installed?: boolean;
}

export interface SafetyBackupInfo {
  exists: boolean;
  timestamp: string | null;
  pmmVersion: string | null;
  zipSizeBytes: number | null;
  totalEntries: number;
  ue4ssModsCount: number;
  palschemaModsCount: number;
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

export interface DiscoveryCategory {
  categoryId: number;
  name: string;
}

export interface DiscoveryModItem {
  modId: number;
  name: string;
  summary: string;
  author: string;
  version: string;
  downloads: number;
  endorsements: number;
  pictureUrl: string;
  categoryId?: number | null;
  categoryName?: string | null;
  createdAt: string;
  updatedAt: string;
  containsAdultContent?: boolean;
  isEndorsed?: boolean;
  isTracked?: boolean;
}

export interface DiscoveryFileItem {
  fileId: number;
  name: string;
  version: string;
  categoryId: number;
  categoryName: string;
  isPrimary: boolean;
  sizeInBytes: number;
  sizeFormatted: string;
  uploadedAt: string;
  uploadedTimestamp?: number | null;
  description: string;
  uniqueDownloads?: number | null;
  totalDownloads?: number | null;
  scanStatus?: string | null;
  changelogEntries?: string[] | null;
}

export interface DiscoveryModDetails {
  modId: number;
  name: string;
  summary: string;
  description: string;
  author: string;
  version: string;
  downloads: number;
  endorsements: number;
  pictureUrl: string;
  createdAt: string;
  updatedAt: string;
  categoryId?: number | null;
  categoryName?: string | null;
  containsAdultContent?: boolean;
  files: DiscoveryFileItem[];
  images: string[];
  isEndorsed?: boolean;
  isTracked?: boolean;
}

export interface DiscoveryResponse {
  mods: DiscoveryModItem[];
  totalCount: number;
  page: number;
  pageSize: number;
}

export interface DependencyVaultEntry {
  depType: 'ue4ss' | 'palschema';
  version: string;
  filename: string;
  filePath: string;
  fileSize: number;
  modifiedTime: string;
  isInstalled: boolean;
  isCustom: boolean;
}

export interface InstalledBuildInfo {
  gameVersion: string | null;
  buildId: string | null;
  lastUpdated: string | null;
  appId: number | null;
  detectionSource: string;
}

export interface MappingEntry {
  gameVersion: string;
  steamBuildId?: string | null;
  appId?: number | null;
  usmapFilename: string;
  usmapUrl: string;
  sha256: string;
  fileSizeBytes: number;
  uploadedAt: string;
  isLatest: boolean;
  notes?: string | null;
}

export interface UsmapStatus {
  installedBuild: InstalledBuildInfo;
  activeMapping?: MappingEntry | null;
  isSynced: boolean;
  localUsmapExists: boolean;
  localFileSize: number;
  localSha256?: string | null;
  latestRemoteVersion?: string | null;
  errorMessage?: string | null;
  mappingsPath: string;
}


