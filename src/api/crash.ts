import { invoke } from '@tauri-apps/api/core';

export interface CrashMemoryStats {
  totalPhysicalGb?: number;
  availablePhysicalGb?: number;
  totalVirtualGb?: number;
  availableVirtualGb?: number;
  isOomRisk: boolean;
}

export interface FaultingModuleInfo {
  name: string;
  fullPath: string;
  baseAddress: number;
  size: number;
  ripOffset: number;
  category: string;
}

export interface CrashContextXml {
  errorMessage: string;
  crashType: string;
  secondsSinceStart: number;
  crashedThreadName?: string;
  crashedThreadCallstack?: string;
  callstackRaw: string;
  memoryStats: CrashMemoryStats;
  loadedModules: string[];
  gameVersion?: string;
  engineVersion?: string;
  rawXmlContent?: string;
}

export interface RawDmpModule {
  name: string;
  fullPath: string;
  baseAddressHex: string;
  sizeBytes: number;
  isFaulting: boolean;
  category: string;
}

export interface MinidumpInfo {
  exceptionCode: number;
  exceptionCodeHex: string;
  exceptionMeaning: string;
  faultingAddress: number;
  faultingAddressHex: string;
  threadId?: number;
  faultingModule?: FaultingModuleInfo;
  totalModulesCount: number;
  rawModules?: RawDmpModule[];
  rawDumpSummary?: string;
}

export interface ActiveModSnapshot {
  id: string;
  name: string;
  version: string;
  modType: string;
  author?: string;
  nexusId?: number;
  isEnabled: boolean;
}

export interface CrashActionSuggestion {
  id: string;
  labelKey: string;
  descriptionKey: string;
  actionType: string;
  targetId?: string;
  targetPath?: string;
}

export interface UnifiedCrashReport {
  id: string;
  timestamp: string;
  crashFolder: string;
  hasXml: boolean;
  hasDmp: boolean;
  hasUe4ssLog: boolean;
  severity: 'Critical' | 'Warning' | 'Fatal' | string;
  primaryCause: string;
  culpritName?: string;
  culpritType?: string;
  humanSummary: string;
  technicalDetails: string;
  originCategory: string;
  xmlContext?: CrashContextXml;
  minidump?: MinidumpInfo;
  recentLogErrors: string[];
  activeMods: ActiveModSnapshot[];
  suggestions: CrashActionSuggestion[];
}

export interface CrashHistoryItem {
  id: string;
  folderPath: string;
  timestamp: string;
  hasXml: boolean;
  hasDmp: boolean;
  folderName: string;
  origin: string;
}

export interface SupportBundleResult {
  success: boolean;
  bundlePath: string;
  fileSizeBytes: number;
  bundledFiles: string[];
  error?: string;
}

export async function listAllCrashes(gamePath: string): Promise<CrashHistoryItem[]> {
  return await invoke<CrashHistoryItem[]>('list_all_crashes', { gamePath });
}

export async function checkLatestCrash(gamePath: string): Promise<UnifiedCrashReport | null> {
  return await invoke<UnifiedCrashReport | null>('check_latest_crash', { gamePath });
}

export async function analyzeCrashFolder(folderPath: string, gamePath: string): Promise<UnifiedCrashReport> {
  return await invoke<UnifiedCrashReport>('analyze_crash_folder', { folderPath, gamePath });
}

export async function exportCrashSupportBundle(
  report: UnifiedCrashReport,
  outputDestination: string,
  gamePath: string
): Promise<SupportBundleResult> {
  return await invoke<SupportBundleResult>('export_crash_support_bundle', {
    report,
    outputDestination,
    gamePath,
  });
}

export async function toggleCrashWatcher(enabled: boolean): Promise<boolean> {
  return await invoke<boolean>('toggle_crash_watcher', { enabled });
}
