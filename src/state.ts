import type { ModInfo, AppSettings, Profile, LibraryEntry, DependencyStatus } from './types';
import type { ZipAnalysis } from './api';

export interface AppState {
  allMods: ModInfo[];
  activeFilters: Set<string>;
  tagFilters: Set<string>;
  categoryFilters: Set<string>;
  statusFilter: 'all' | 'enabled' | 'disabled';
  currentAnalysis: ZipAnalysis | null;
  currentConfigModId: string | null;
  currentSettings: AppSettings | null;
  currentDetailMod: ModInfo | null;
  searchQuery: string;
  currentSort: { field: string; asc: boolean };
  activeTab: 'mods' | 'editor' | 'library';
  editorModId: string | null;
  editorFiles: string[];
  editorSelectedFile: string | null;
  editorPreviewMode: boolean;
  profiles: Profile[];
  currentProfileId: string;
  currentProfile: Profile | null;
  libraryEntries: LibraryEntry[];
  gameVersion: string | null;
  dependencies: DependencyStatus | null;
  selectedModIds: Set<string>;
  selectedLibraryIds: Set<string>;
}

let state: AppState = {
  allMods: [],
  activeFilters: new Set(['ue4ss', 'palschema', 'pak', 'logicmods']),
  tagFilters: new Set(),
  categoryFilters: new Set(),
  statusFilter: 'all',
  currentAnalysis: null,
  currentConfigModId: null,
  currentSettings: null,
  currentDetailMod: null,
  searchQuery: '',
  currentSort: { field: 'name', asc: true },
  activeTab: 'mods',
  editorModId: null,
  editorFiles: [],
  editorSelectedFile: null,
  editorPreviewMode: false,
  profiles: [],
  currentProfileId: 'default',
  currentProfile: null,
  libraryEntries: [],
  gameVersion: null,
  dependencies: null,
  selectedModIds: new Set(),
  selectedLibraryIds: new Set(),
};


export function getState(): AppState {
  return state;
}

export function updateState(partial: Partial<AppState>): void {
  state = { ...state, ...partial };
}
