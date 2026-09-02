import type {
  DbSnapshot,
  DbTable,
} from './types';
import type {
  UsmapSummaryData,
  UsmapSearchItem,
  UsmapSearchResult,
  UsmapStructFullDetails,
} from '../../api';

export interface DbViewState {
  snapshot: DbSnapshot | null;
  activeTable: DbTable;
  selectedRecordType: 'mod' | 'profile' | 'settings' | 'usmap_struct' | 'usmap_enum' | 'usmap_name' | null;
  selectedRecordId: string;

  // USMAP State
  usmapSummary: UsmapSummaryData | null;
  usmapQuery: string;
  usmapCategory: 'all' | 'structs' | 'enums' | 'names';
  usmapPage: number;
  usmapSearchResult: UsmapSearchResult | null;
  selectedUsmapItem: UsmapSearchItem | null;
  selectedUsmapStructDetails: UsmapStructFullDetails | null;
  selectedUsmapEnumValues: string[] | null;
  usmapInspectorMode: 'visual' | 'json';
}

export const USMAP_PAGE_SIZE = 40;

export const dbState: DbViewState = {
  snapshot: null,
  activeTable: 'mods',
  selectedRecordType: null,
  selectedRecordId: '',

  usmapSummary: null,
  usmapQuery: '',
  usmapCategory: 'all',
  usmapPage: 0,
  usmapSearchResult: null,
  selectedUsmapItem: null,
  selectedUsmapStructDetails: null,
  selectedUsmapEnumValues: null,
  usmapInspectorMode: 'visual',
};
