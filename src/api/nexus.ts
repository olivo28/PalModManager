import { invoke } from '@tauri-apps/api/core';
import type {
  ModInfo,
  NexusAccountInfo,
  DetailedProtocolInfo,
  NexusUserEndorsement,
  NexusUserTrackedMod,
  NexusUserAuthoredMod,
  NxmModMetadata,
  DiscoveryCategory,
  DiscoveryModDetails,
  DiscoveryResponse,
} from './types';

export async function fetchNexusInfo(modId: number): Promise<any> {
  return invoke('fetch_nexus_info_async', { modId });
}

export async function fetchNexusInfoAsync(modId: number): Promise<any> {
  return invoke('fetch_nexus_info_async', { modId });
}

export async function refreshNexusCache(modIdStr: string): Promise<ModInfo> {
  return invoke('refresh_nexus_cache', { modIdStr });
}

export async function startNexusOAuth(): Promise<string> {
  return invoke('start_nexus_oauth');
}

export async function handleNexusOAuthCallback(callbackUrl: string): Promise<NexusAccountInfo> {
  return invoke('handle_nexus_oauth_callback', { callbackUrl });
}

export async function getNexusAccountStatus(): Promise<NexusAccountInfo | null> {
  return invoke('get_nexus_account_status');
}

export async function refreshNexusAccountProfile(): Promise<NexusAccountInfo | null> {
  return invoke('refresh_nexus_account_profile');
}

export async function logoutNexusAccount(): Promise<void> {
  return invoke('logout_nexus_account');
}

export async function checkNexusProtocolStatus(): Promise<DetailedProtocolInfo> {
  return invoke('check_nexus_protocol_status');
}

export async function registerNexusProtocol(scheme?: string): Promise<string> {
  return invoke('register_nexus_protocol', { scheme });
}

export async function unregisterNexusProtocol(scheme?: string): Promise<void> {
  return invoke('unregister_nexus_protocol', { scheme });
}

export async function getNexusUserEndorsements(forceRefresh?: boolean): Promise<NexusUserEndorsement[]> {
  return invoke('get_nexus_user_endorsements', { forceRefresh: !!forceRefresh });
}

export async function getNexusUserTrackedMods(forceRefresh?: boolean): Promise<NexusUserTrackedMod[]> {
  return invoke('get_nexus_user_tracked_mods', { forceRefresh: !!forceRefresh });
}

export async function getNexusUserAuthoredMods(forceRefresh?: boolean): Promise<NexusUserAuthoredMod[]> {
  return invoke('get_nexus_user_authored_mods', { forceRefresh: !!forceRefresh });
}

export async function handleNxmDownload(nxmUrl: string): Promise<any> {
  return invoke('handle_nxm_download', { nxmUrl });
}

export async function parseNxmLink(nxmUrl: string): Promise<any> {
  return invoke('parse_nxm_link', { nxmUrl });
}

export async function getNxmModMetadata(gameDomain: string, modId: number): Promise<NxmModMetadata> {
  return invoke('get_nxm_mod_metadata', { gameDomain, modId });
}

export async function downloadNxmFile(nxmUrl: string, downloadId: string): Promise<string> {
  return invoke('download_nxm_file', { nxmUrl, downloadId });
}

export async function getDiscoveryCategories(): Promise<DiscoveryCategory[]> {
  return invoke('get_discovery_categories');
}

export async function getDiscoveryMods(params?: {
  query?: string;
  descriptionQuery?: string;
  authorQuery?: string;
  uploaderQuery?: string;
  categoryId?: number;
  categoryName?: string;
  adultFilter?: string;
  supportsVortex?: boolean;
  hasUpdated?: boolean;
  languageName?: string;
  languageNames?: string[];
  includeTags?: string[];
  excludeTags?: string[];
  hideTranslations?: boolean;
  sortBy?: string;
  timeRange?: string;
  includeAdult?: boolean;
  page?: number;
  pageSize?: number;
}): Promise<DiscoveryResponse> {
  return invoke('get_discovery_mods', {
    query: params?.query || null,
    descriptionQuery: params?.descriptionQuery || null,
    authorQuery: params?.authorQuery || null,
    uploaderQuery: params?.uploaderQuery || null,
    categoryId: params?.categoryId || null,
    categoryName: params?.categoryName || null,
    adultFilter: params?.adultFilter || null,
    supportsVortex: params?.supportsVortex !== undefined ? params.supportsVortex : null,
    hasUpdated: params?.hasUpdated !== undefined ? params.hasUpdated : null,
    languageName: params?.languageName || null,
    languageNames: params?.languageNames || null,
    includeTags: params?.includeTags || null,
    excludeTags: params?.excludeTags || null,
    hideTranslations: params?.hideTranslations !== undefined ? params.hideTranslations : null,
    sortBy: params?.sortBy || null,
    timeRange: params?.timeRange || null,
    includeAdult: params?.includeAdult !== undefined ? params.includeAdult : null,
    page: params?.page || 1,
    pageSize: params?.pageSize || 36,
  });
}

export async function getDiscoveryModDetails(modId: number): Promise<DiscoveryModDetails> {
  return invoke('get_discovery_mod_details', { modId });
}

export async function endorseNexusMod(modId: number, version?: string): Promise<boolean> {
  return invoke('endorse_nexus_mod', { modId, version: version || null });
}

export async function abstainNexusMod(modId: number, version?: string): Promise<boolean> {
  return invoke('abstain_nexus_mod', { modId, version: version || null });
}

export async function trackNexusMod(modId: number): Promise<boolean> {
  return invoke('track_nexus_mod', { modId });
}

export async function untrackNexusMod(modId: number): Promise<boolean> {
  return invoke('untrack_nexus_mod', { modId });
}

export async function installDiscoveryFile(modId: number, fileId: number): Promise<string> {
  return invoke('install_discovery_file', { modId, fileId });
}

export async function fetchAndCacheImage(url: string, dnsMode?: string): Promise<string> {
  return invoke('fetch_and_cache_image', { url, dnsMode: dnsMode || null });
}

export async function getImageCacheSize(): Promise<number> {
  return invoke('get_image_cache_size');
}

export async function purgeImageCache(): Promise<void> {
  return invoke('purge_image_cache');
}
