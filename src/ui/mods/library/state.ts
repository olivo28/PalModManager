export interface LibraryState {
  searchQuery: string;
  activeSubTab: 'local' | 'workshop';
  filterStatus: 'all' | 'installed' | 'not_installed' | 'updates';
  sortBy: 'name:asc' | 'name:desc' | 'installed:first' | 'not_installed:first' | 'date:desc';
  onlineUpdatesMap: Map<string, string>;
}

export type LibraryLayout = 'grid' | 'list';

export interface LibraryGroup {
  modId: string;
  name: string;
  author: string;
  description: string;
  modType: string;
  nexusPictureUrl?: string | null;
  nexusModId?: number | null;
  isInstalled: boolean;
  installedVersion: string | null;
  versions: {
    zipName: string;
    zipSize: number;
    version: string;
    installedAt: string;
  }[];
}

export let _librarySearchQuery = '';
export let _activeLibrarySubTab: 'local' | 'workshop' = 'local';
export let _libraryFilterStatus: 'all' | 'installed' | 'not_installed' | 'updates' = (localStorage.getItem('pmm-library-filter') as any) || 'all';
export let _librarySortBy: 'name:asc' | 'name:desc' | 'installed:first' | 'not_installed:first' | 'date:desc' = (localStorage.getItem('pmm-library-sort') as any) || 'installed:first';
export let _libraryLayout: LibraryLayout = (localStorage.getItem('pmm-library-layout') as LibraryLayout) || 'grid';
export const _libraryOnlineUpdatesMap: Map<string, string> = new Map();

export function setLibrarySearchQuery(val: string) { _librarySearchQuery = val; }
export function setActiveLibrarySubTab(val: 'local' | 'workshop') { _activeLibrarySubTab = val; }
export function setLibraryFilterStatus(val: 'all' | 'installed' | 'not_installed' | 'updates') { _libraryFilterStatus = val; }
export function setLibrarySortBy(val: 'name:asc' | 'name:desc' | 'installed:first' | 'not_installed:first' | 'date:desc') { _librarySortBy = val; }
export function setLibraryLayout(val: LibraryLayout) {
  _libraryLayout = val;
  localStorage.setItem('pmm-library-layout', val);
}
