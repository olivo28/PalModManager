import { getState, updateState } from '../../../state';
import { getLibrary, getWorkshopState } from '../../../api';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import { listen } from '@tauri-apps/api/event';
import {
  _librarySearchQuery,
  _activeLibrarySubTab,
  _libraryFilterStatus,
  _librarySortBy,
  setLibrarySearchQuery,
  setActiveLibrarySubTab,
  setLibraryFilterStatus,
  setLibrarySortBy,
} from './state';
import {
  syncWorkshopModTimestamps,
  updateWorkshopBadges,
  updateWorkshopTabVisibility,
  handleCheckWorkshopOnlineUpdates,
} from './workshop';
import {
  handleLibraryBulkInstall,
  handleLibraryBulkRemove,
  updateLibraryBulkBar,
} from './actions';
import { renderLibraryView } from './render';

export function setupLibraryHandlers(): void {
  const searchInput = document.getElementById('library-search-input') as HTMLInputElement | null;
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      setLibrarySearchQuery(searchInput.value.trim().toLowerCase());
      renderLibraryView();
    });
  }

  const filterSelect = document.getElementById('library-filter-status') as HTMLSelectElement | null;
  if (filterSelect) {
    filterSelect.value = _libraryFilterStatus;
    filterSelect.addEventListener('change', () => {
      setLibraryFilterStatus(filterSelect.value as any);
      localStorage.setItem('pmm-library-filter', filterSelect.value);
      renderLibraryView();
    });
  }

  const sortSelect = document.getElementById('library-sort-select') as HTMLSelectElement | null;
  if (sortSelect) {
    sortSelect.value = _librarySortBy;
    sortSelect.addEventListener('change', () => {
      setLibrarySortBy(sortSelect.value as any);
      localStorage.setItem('pmm-library-sort', sortSelect.value);
      renderLibraryView();
    });
  }

  // Listen for background Steam Workshop folder changes
  listen('workshop-directory-changed', async () => {
    console.log('[INFO] Workshop directory change detected, auto-refreshing Library...');
    try {
      const wState = await getWorkshopState();
      const sync = syncWorkshopModTimestamps(wState.mods);
      updateWorkshopBadges(sync.newModCount);
      if (sync.newModNames.length > 0) {
        showToast(t('library.new_mod_subscribed', { name: sync.newModNames[0] }), 'success');
      }
    } catch {}
    if (_activeLibrarySubTab === 'workshop') {
      renderLibraryView();
    }
    try {
      const { loadMods } = await import('../../modsView');
      await loadMods();
    } catch (e) {
      console.error(e);
    }
  });

  document.getElementById('workshop-check-updates-btn')?.addEventListener('click', handleCheckWorkshopOnlineUpdates);
  document.getElementById('library-bulk-install-btn')?.addEventListener('click', handleLibraryBulkInstall);
  document.getElementById('library-bulk-remove-btn')?.addEventListener('click', handleLibraryBulkRemove);
  document.getElementById('library-bulk-clear-btn')?.addEventListener('click', () => {
    updateState({ selectedLibraryIds: new Set() });
    updateLibraryBulkBar();
    renderLibraryView();
  });

  // Tab switching handlers
  document.querySelectorAll('.library-sub-tab').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const target = e.currentTarget as HTMLButtonElement;
      const tab = target.dataset.tab as 'local' | 'workshop';

      document.querySelectorAll('.library-sub-tab').forEach(b => {
        const btnEl = b as HTMLButtonElement;
        btnEl.classList.remove('active');
        btnEl.style.background = 'transparent';
        btnEl.style.color = 'var(--text-muted)';
      });
      target.classList.add('active');
      target.style.background = 'var(--accent)';
      target.style.color = '#fff';

      setActiveLibrarySubTab(tab);

      updateState({ selectedLibraryIds: new Set() });
      updateLibraryBulkBar();

      if (searchInput) {
        searchInput.value = '';
        setLibrarySearchQuery('');
        searchInput.placeholder = tab === 'local' ? 'Search library...' : 'Search workshop...';
      }

      renderLibraryView();
    });
  });
}

export async function loadLibrary(): Promise<void> {
  try {
    const entries = await getLibrary();
    updateState({ libraryEntries: entries });
    updateWorkshopTabVisibility();
    renderLibraryView();

    // Check workshop mods timestamps for badges (10 minute window)
    try {
      const wState = await getWorkshopState();
      const sync = syncWorkshopModTimestamps(wState.mods);
      updateWorkshopBadges(sync.newModCount);
    } catch {}

    // Recompute available updates on active mods list so local library updates show on mod cards
    const currentMods = getState().allMods;
    if (currentMods && currentMods.length > 0) {
      const { computeAvailableUpdates } = await import('../card');
      const { renderModsView } = await import('../renderer');
      const updatesMap = computeAvailableUpdates(currentMods, entries);
      updateState({ availableUpdates: updatesMap });
      renderModsView();
    }
  } catch (e) {
    console.error('Failed to load library:', e);
  }
}
