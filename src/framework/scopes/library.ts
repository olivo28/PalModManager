import { createScope } from '../core/query';

export interface LibraryDomMap {
  'library-view': HTMLElement;
  'library-toolbar': HTMLElement;
  'workshop-subtab-badge': HTMLElement;
  'library-workshop-master-wrap': HTMLElement;
  'library-workshop-master-toggle': HTMLInputElement;
  'library-filter-wrap': HTMLElement;
  'library-filter-status': HTMLSelectElement;
  'library-sort-wrap': HTMLElement;
  'library-sort-select': HTMLSelectElement;
  'library-search-wrap': HTMLElement;
  'library-search-input': HTMLInputElement;
  'workshop-check-updates-btn': HTMLButtonElement;
  'library-refresh-btn': HTMLButtonElement;
  'library-bulk-actions-bar': HTMLElement;
  'library-bulk-selected-count': HTMLElement;
  'library-bulk-install-btn': HTMLButtonElement;
  'library-bulk-remove-btn': HTMLButtonElement;
  'library-bulk-clear-btn': HTMLButtonElement;
  'library-container': HTMLElement;
  'library-empty': HTMLElement;
}

export const libraryDom = createScope<LibraryDomMap>('library');
