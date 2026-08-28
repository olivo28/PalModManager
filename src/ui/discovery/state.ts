import { type DiscoveryCategory, type DiscoveryModDetails } from '../../api';
import { getState } from '../../state';

export function isUserPremium(): boolean {
  const account: any = getState().currentSettings?.nexusAccount;
  return !!(account?.isPremium || account?.is_premium);
}

export interface DiscoveryState {
  categories: DiscoveryCategory[];
  currentQuery: string;
  currentCategoryId: number | null;
  currentSort: string;
  currentTimeRange: string;
  currentNsfwFilter: 'hide' | 'blur' | 'show';
  currentPage: number;
  PAGE_SIZE: number;
  totalCount: number;
  isLoading: boolean;
  isInitialized: boolean;
  currentModalMod: DiscoveryModDetails | null;
  lightboxZoom: number;
  lightboxPanX: number;
  lightboxPanY: number;
  isPanning: boolean;
  panStartX: number;
  panStartY: number;
  selectedIncludeTags: Set<string>;
  selectedExcludeTags: Set<string>;
}

export const NEXUS_PALWORLD_TAGS = [
  "2 Players", "3-4 Players", "5-6 Players", "7-8 Players", "9+ Players",
  "AI Assisted", "AI Media", "AI-Generated Content",
  "Animation - Modified", "Animation - New", "Bug Fixes", "Camera",
  "Character Preset", "Cheating", "Chinese", "Collection Asset",
  "Compatibility Patch", "Compilation", "Dutch", "English",
  "Extreme violence", "Face", "Fair and balanced", "Foliage (Plants)",
  "French", "Gameplay", "German", "Hair", "Horror",
  "Humour, Joke or Just for Fun", "ini tweak", "Italian", "Japanese",
  "Korean", "Mandarin", "Polish", "Portuguese", "Russian", "Spanish", "Ukrainian"
];

export const discState: DiscoveryState = {
  categories: [],
  currentQuery: '',
  currentCategoryId: null,
  currentSort: localStorage.getItem('pmm-discovery-sort') || 'date_published',
  currentTimeRange: localStorage.getItem('pmm-discovery-timerange') || 'all',
  currentNsfwFilter: (localStorage.getItem('pmm-discovery-nsfw') as any) || 'hide',
  currentPage: 1,
  PAGE_SIZE: 36,
  totalCount: 0,
  isLoading: false,
  isInitialized: false,
  currentModalMod: null,
  lightboxZoom: 1,
  lightboxPanX: 0,
  lightboxPanY: 0,
  isPanning: false,
  panStartX: 0,
  panStartY: 0,
  selectedIncludeTags: new Set<string>(),
  selectedExcludeTags: new Set<string>(),
};
