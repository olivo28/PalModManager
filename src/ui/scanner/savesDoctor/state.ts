import type {
  SaveWorldSummary,
  SaveHealthReport,
} from '../../../api';

export interface SavesDoctorState {
  cachedWorlds: SaveWorldSummary[] | null;
  selectedWorldDir: string | null;
  currentHealthReport: SaveHealthReport | null;
  customSavesPath: string;
  isLoadingWorlds: boolean;
  isDeepScanning: boolean;
  isRepairing: boolean;
  isRestoringBackup: boolean;
  isCreatingBackup: boolean;
  isPruningBackups: boolean;
  isSavingMeta: boolean;
  availableProfiles: Array<{ id: string; name: string }>;
}

export const doctorState: SavesDoctorState = {
  cachedWorlds: null,
  selectedWorldDir: null,
  currentHealthReport: null,
  customSavesPath: '',
  isLoadingWorlds: false,
  isDeepScanning: false,
  isRepairing: false,
  isRestoringBackup: false,
  isCreatingBackup: false,
  isPruningBackups: false,
  isSavingMeta: false,
  availableProfiles: [],
};

// Direct export bindings for backward-compatibility
export let cachedWorlds: SaveWorldSummary[] | null = null;
export let selectedWorldDir: string | null = null;
export let currentHealthReport: SaveHealthReport | null = null;
export let customSavesPath: string = '';
export let isLoadingWorlds = false;
export let isDeepScanning = false;
export let isRepairing = false;
export let isRestoringBackup = false;
export let isCreatingBackup = false;
export let isPruningBackups = false;
export let isSavingMeta = false;
export let availableProfiles: Array<{ id: string; name: string }> = [];

export function setCachedWorlds(val: SaveWorldSummary[] | null) { cachedWorlds = val; doctorState.cachedWorlds = val; }
export function setSelectedWorldDir(val: string | null) { selectedWorldDir = val; doctorState.selectedWorldDir = val; }
export function setCurrentHealthReport(val: SaveHealthReport | null) { currentHealthReport = val; doctorState.currentHealthReport = val; }
export function setCustomSavesPath(val: string) { customSavesPath = val; doctorState.customSavesPath = val; }
export function setIsLoadingWorlds(val: boolean) { isLoadingWorlds = val; doctorState.isLoadingWorlds = val; }
export function setIsDeepScanning(val: boolean) { isDeepScanning = val; doctorState.isDeepScanning = val; }
export function setIsRepairing(val: boolean) { isRepairing = val; doctorState.isRepairing = val; }
export function setIsRestoringBackup(val: boolean) { isRestoringBackup = val; doctorState.isRestoringBackup = val; }
export function setIsCreatingBackup(val: boolean) { isCreatingBackup = val; doctorState.isCreatingBackup = val; }
export function setIsPruningBackups(val: boolean) { isPruningBackups = val; doctorState.isPruningBackups = val; }
export function setIsSavingMeta(val: boolean) { isSavingMeta = val; doctorState.isSavingMeta = val; }
export function setAvailableProfiles(val: Array<{ id: string; name: string }>) { availableProfiles = val; doctorState.availableProfiles = val; }
