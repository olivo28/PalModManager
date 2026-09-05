import type {
  ModInfo,
  AppSettings,
  Profile,
  LibraryEntry,
  DependencyStatus,
  InstallManifest,
  SafetyBackupInfo,
  FileRoute,
  RouteType,
  NexusAccountInfo,
  ProtocolStatus,
  DetailedProtocolInfo,
  NexusUserEndorsement,
  NexusUserTrackedMod,
  NexusUserAuthoredMod,
  NxmModMetadata,
  NxmDownloadProgressEvent,
  DiscoveryCategory,
  DiscoveryModItem,
  DiscoveryFileItem,
  DiscoveryModDetails,
  DiscoveryResponse,
} from '../types';

export type {
  InstallManifest,
  FileRoute,
  RouteType,
  ModInfo,
  AppSettings,
  Profile,
  LibraryEntry,
  DependencyStatus,
  SafetyBackupInfo,
  NexusAccountInfo,
  ProtocolStatus,
  DetailedProtocolInfo,
  NexusUserEndorsement,
  NexusUserTrackedMod,
  NexusUserAuthoredMod,
  NxmModMetadata,
  NxmDownloadProgressEvent,
  DiscoveryCategory,
  DiscoveryModItem,
  DiscoveryFileItem,
  DiscoveryModDetails,
  DiscoveryResponse,
};

export interface ZipAnalysis {
  zipPath: string;
  detectedType: string;
  hasLua: boolean;
  hasJson: boolean;
  hasPalSchemaJson: boolean;
  hasPak: boolean;
  hasInfoJson: boolean;
  pakDestinationHint: string | null;
  rootFolder: string | null;
  fileCount: number;
  nexusModId: number | null;
  detectedVersion?: string | null;
  files?: string[];
  nexusInfo: {
    name: string;
    author: string;
    summary: string;
    version: string;
    downloads: number;
    endorsements: number;
    pictureUrl: string;
  } | null;
  modinfo?: {
    name?: string;
    version?: string;
    description?: string;
    author?: string;
    modType?: string;
  } | null;
}

export interface UpdateCheckResult {
  modId: string;
  name: string;
  currentVersion: string;
  latestVersion: string;
  nexusModId: number;
}

export interface EditorDiagnostic {
  line: number;
  column: number;
  endLine?: number;
  endColumn?: number;
  severity: 'error' | 'warning' | 'info';
  message: string;
  target: string;
  suggestion?: string;
  category: string;
}

export interface EditorCompletion {
  label: string;
  insertText: string;
  kind: 'hook' | 'class' | 'function' | 'delegate' | 'table' | 'struct' | 'api' | 'module' | 'property' | 'constant' | 'field' | 'value';
  detail?: string;
  documentation?: string;
}

export interface PakInternalItem {
  path: string;
  name: string;
  assetType: string;
}

export interface PakInspectionResult {
  pakName: string;
  totalFiles: number;
  files: PakInternalItem[];
  summaryByType: Record<string, number>;
}

export interface UAssetExportItem {
  objectName: string;
  className: string;
  outerName?: string | null;
}

export interface UAssetImportItem {
  objectName: string;
  className: string;
  classPackage: string;
}

export interface UAssetSummaryInfo {
  uassetSizeBytes: number;
  uexpSizeBytes?: number | null;
  exportCount: number;
  importCount: number;
  nameCount: number;
  packageFlags: number;
}

export interface UAssetSchemaProperty {
  name: string;
  typeName: string;
  structType?: string | null;
  enumType?: string | null;
  innerType?: string | null;
  arrayDim: number;
  index: number;
}

export interface UAssetSchemaResolvedInfo {
  matchedStructName: string;
  superType?: string | null;
  properties: UAssetSchemaProperty[];
  totalProperties: number;
  gameVersion: string;
}

export interface TexturePreviewInfo {
  dataUrl: string;
  width: number;
  height: number;
  formatName: string;
  mipCount: number;
  hasAlpha: boolean;
  sourceFile: string;
  sizeBytes: number;
}

export interface UAssetInspectionDetails {
  assetName: string;
  assetPath: string;
  assetType: string;
  engineVersion: string;
  summary: UAssetSummaryInfo;
  exports: UAssetExportItem[];
  imports: UAssetImportItem[];
  namesSample: string[];
  resolvedSchema?: UAssetSchemaResolvedInfo | null;
  texturePreview?: TexturePreviewInfo | null;
}

export interface PatchAssetSelection {
  assetPath: string;
  sourcePakPath: string;
  companionExtensions?: string[];
}

export interface PatchBuildRequest {
  patchName: string;
  targetProfileId?: string | null;
  selections: PatchAssetSelection[];
  isGamepass: boolean;
}

export interface PatchBuildResult {
  outputPath: string;
  patchName: string;
  totalAssetsPacked: number;
  fileSizeBytes: number;
  isGamepass: boolean;
  utocPath?: string | null;
  ucasPath?: string | null;
}

export interface GeneratedPatchInfo {
  id?: string | null;
  displayName?: string | null;
  fileName: string;
  filePath: string;
  fileSizeBytes: number;
  createdAt: string;
  isGamepass: boolean;
  resolvedAssetsCount?: number;
}

export interface SaveBackupSnapshot {
  slotName: string;
  timestamp: string;
  levelSizeBytes: number;
  uncompressedSizeBytes?: number | null;
  localDataExists: boolean;
  inGameDay?: number | null;
  playerLevel?: number | null;
  hostPlayerName?: string | null;
  modRefsCount?: number | null;
  isCleanVanilla?: boolean | null;
}

export interface ExternalEditDiagnostic {
  isModified: boolean;
  toolName?: string | null;
  details: string;
  editorBackupCount: number;
  currentSizeBytes: number;
  latestBackupSizeBytes: number;
  sizeReductionPct?: number | null;
}

export interface WorldOptionSettings {
  exists: boolean;
  difficulty?: string | null;
  dayTimeSpeedRate?: number | null;
  nightTimeSpeedRate?: number | null;
  expRate?: number | null;
  palCaptureRate?: number | null;
  palSpawnNumRate?: number | null;
  palDamageRate?: number | null;
  playerDamageRate?: number | null;
  playerStomachDecreaseRate?: number | null;
  playerStaminaDecreaseRate?: number | null;
  playerAutoHpRegeneRate?: number | null;
  playerAutoHpRegeneRateInSleeping?: number | null;
  palStomachDecreaseRate?: number | null;
  palStaminaDecreaseRate?: number | null;
  palAutoHpRegeneRate?: number | null;
  palAutoHpRegeneRateInSleeping?: number | null;
  buildObjectDamageRate?: number | null;
  buildObjectDeteriorationDamageRate?: number | null;
  collectionDropRate?: number | null;
  collectionObjectHpRate?: number | null;
  collectionObjectRespawnSpeedRate?: number | null;
  enemyDropItemRate?: number | null;
  deathPenalty?: string | null;
  enablePlayerToPlayerDamage?: boolean | null;
  enableFriendlyFire?: boolean | null;
  enableInvaderEnemy?: boolean | null;
  activeUnko?: boolean | null;
  dropItemMaxNum?: number | null;
  baseCampMaxNum?: number | null;
  baseCampWorkerMaxNum?: number | null;
  dropItemAliveMaxHours?: number | null;
  guildPlayerMaxNum?: number | null;
  palEggHatchingHours?: number | null;
  workSpeedRate?: number | null;
  isMultiplay?: boolean | null;
  isPvp?: boolean | null;
  canPickupOtherGuildDeathPenaltyDrop?: boolean | null;
  enableNonLoginPenalty?: boolean | null;
  enableFastTravel?: boolean | null;
  isStartLocationSelectByMap?: boolean | null;
  existPlayerAfterLogout?: boolean | null;
  supplyDropSpan?: number | null;
}

export interface PlayerSaveInfo {
  playerUid: string;
  playerName?: string | null;
  playerLevel?: number | null;
  isHost: boolean;
  fileSizeBytes: number;
  lastPlayedDate?: string | null;
  isCorrupt: boolean;
}

export interface SaveStorageBreakdown {
  levelSavBytes: number;
  playersDirBytes: number;
  backupsDirBytes: number;
  totalWorldBytes: number;
  uncompressedLevelBytes: number;
  compressionRatioPct: number;
}

export interface WorldCustomMeta {
  nickname?: string | null;
  notes?: string | null;
  boundProfileId?: string | null;
  boundProfileName?: string | null;
  tags?: string[];
  preLaunchBackupEnabled?: boolean;
}

export interface SaveWorldSummary {
  worldId: string;
  worldName: string;
  worldDir: string;
  hostPlayerName?: string | null;
  hostPlayerUid?: string | null;
  playerLevel?: number | null;
  inGameDay?: number | null;
  saveDate?: string | null;
  levelSizeBytes: number;
  playerCount: number;
  backupCount: number;
  pmmBackupCount?: number;
  latestBackupDate?: string | null;
  hasExternalEdits: boolean;
  healthStatus: 'healthy' | 'warning' | 'corrupt' | 'external_edits';
  detectedIssuesCount: number;
  customMeta?: WorldCustomMeta | null;
  worldOptions?: WorldOptionSettings | null;
}

export interface PmmWorldBackup {
  fileName: string;
  filePath: string;
  fileSizeBytes: number;
  createdAt: string;
  worldNameHint?: string | null;
}

export interface OrphanedModRef {
  modHintName: string;
  assetPath: string;
  occurrences: number;
}

export interface SaveHealthReport {
  worldId: string;
  worldName: string;
  levelSavPath: string;
  hostPlayerName?: string | null;
  hostPlayerUid?: string | null;
  playerLevel?: number | null;
  inGameDay?: number | null;
  isValidGvas: boolean;
  compressionType: string;
  uncompressedSize: number;
  healthStatus: 'healthy' | 'warning' | 'corrupt' | 'external_edits';
  summaryMessage: string;
  orphanedModRefs: OrphanedModRef[];
  rawModPathsFound: string[];
  totalModReferences: number;
  backupCount: number;
  latestBackupDate?: string | null;
  availableBackups: SaveBackupSnapshot[];
  pmmBackups?: PmmWorldBackup[];
  hasExternalEdits: boolean;
  externalEditDetails?: ExternalEditDiagnostic | null;
  canRepair: boolean;
  canRestoreBackup: boolean;
  worldOptions?: WorldOptionSettings | null;
  playerRoster?: PlayerSaveInfo[];
  storageBreakdown?: SaveStorageBreakdown | null;
  customMeta?: WorldCustomMeta | null;
}

export interface SaveRepairResult {
  success: boolean;
  backupZipPath: string;
  sanitizedRefsCount: number;
  message: string;
}

export interface ImportProfileResult {
  success: boolean;
  profileId: string;
  profileName: string;
  modCount: number;
  dependenciesInstalled: number;
}

export interface ModHotkey {
  modId: string;
  modName: string;
  filePath: string;
  absoluteFilePath: string;
  lineNumber: number;
  keys: string;
  rawLine: string;
  variableName?: string | null;
  definitionFilePath?: string | null;
  definitionAbsolutePath?: string | null;
  definitionLineNumber?: number | null;
  isVariable?: boolean;
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

export interface StorageUsageInfo {
  tempDownloadsSize: number;
  tempDownloadsCount: number;
  tempDownloadsPath: string;
  librarySize: number;
  libraryModsCount: number;
  libraryZipsCount: number;
  libraryPath: string;
}

export interface WorkshopOnlineModItem {
  workshopId: number;
  modName: string;
  packageName: string;
  localTimeUpdated: number;
  remoteTimeUpdated: number;
  hasRemoteUpdate: boolean;
  isDownloadedToDisk: boolean;
}

export interface WorkshopOnlineCheckResult {
  totalChecked: number;
  pendingSteamDownloads: WorkshopOnlineModItem[];
  readyToInstallUpdates: WorkshopOnlineModItem[];
}

export interface UsmapProperty {
  index: number;
  name: string;
  typeName: string;
  structType?: string | null;
  enumType?: string | null;
  innerType?: string | null;
  arrayDim: number;
}

export interface UsmapSearchItem {
  name: string;
  category: 'struct' | 'enum' | 'name';
  superType?: string | null;
  propertyCount: number;
  enumValuesCount: number;
  preview: string;
}

export interface UsmapSearchResult {
  totalItems: number;
  page: number;
  pageSize: number;
  items: UsmapSearchItem[];
}

export interface UsmapStructFullDetails {
  name: string;
  superType?: string | null;
  inheritanceChain: string[];
  properties: UsmapProperty[];
  totalPropertiesWithAncestors: number;
}

export interface UsmapSummaryData {
  loaded: boolean;
  totalStructs: number;
  totalEnums: number;
  totalNames: number;
  gameVersion?: string;
  path?: string;
  sha256?: string;
  gameBuild?: string;
}

export interface SdkStatus {
  installed: boolean;
  totalClasses: number;
  totalFunctions: number;
  source: string;
  gameVersion: string;
  path: string;
  localGameCxxFound: boolean;
  localGameCxxPath?: string | null;
}

export interface ArchivedConfigInfo {
  archiveId: string;
  modName: string;
  modId: string;
  nexusModId: number | null;
  archivedAt: string;
  files: string[];
}
