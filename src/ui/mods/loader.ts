import { getMods, scanMods, getGameVersion } from '../../api';
import { getState, updateState } from '../../state';
import { renderModsView } from './renderer';
import { computeAvailableUpdates } from './card';
import { populateAdvancedFilters } from './renderer';
import { populateEditorModSelect } from '../editorView';
import { loadProfiles } from './profiles';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { mainDom } from '../../framework';

export async function loadMods(): Promise<void> {
  const container = mainDom.elMaybe('mods-container');
  if (!container) return;

  // 1. Instant load cached database mods
  try {
    const cachedMods = await getMods();
    if (cachedMods && cachedMods.length > 0) {
      const updatesMap = computeAvailableUpdates(cachedMods, getState().libraryEntries);
      updateState({ allMods: cachedMods, availableUpdates: updatesMap });
      renderModsView();
      populateAdvancedFilters();
      populateEditorModSelect();
    } else {
      container.innerHTML = '<div id="loading-state">Scanning mods...</div>';
    }
  } catch (e) {
    console.error('Error loading cached mods:', e);
  }

  // 2. Background sync scan disk mods
  try {
    const oldMods = getState().allMods || [];
    const freshMods = await scanMods();
    const updatesMap = computeAvailableUpdates(freshMods, getState().libraryEntries);

    // Alert user if Steam Workshop updates were found
    const newlyUpdated = freshMods.filter(m => {
      const isWorkshop = !!(m.nexusSummary && m.nexusSummary.startsWith('Steam Workshop Mod'));
      if (!isWorkshop) return false;
      const old = oldMods.find(o => o.id === m.id);
      return m.hasPendingUpdate && (!old || !old.hasPendingUpdate || old.version !== m.version);
    });
    for (const mod of newlyUpdated) {
      showToast(t('toasts.workshop_mod_updated', { name: mod.name }), 'info');
    }

    const modsChanged = oldMods.length !== freshMods.length || JSON.stringify(oldMods) !== JSON.stringify(freshMods);
    updateState({ allMods: freshMods, availableUpdates: updatesMap });

    if (modsChanged || oldMods.length === 0) {
      renderModsView();
      populateAdvancedFilters();
      populateEditorModSelect();
    }
  } catch (e) {
    console.error('Error scanning mods:', e);
    const state = getState();
    if (!state.allMods || state.allMods.length === 0) {
      container.innerHTML = '<div id="empty-state">Error scanning mods.</div>';
    }
  }
}

export async function loadGameVersion(): Promise<void> {
  try {
    const version = await getGameVersion();
    updateState({ gameVersion: version });
    const el = mainDom.elMaybe('game-version-badge');
    if (el) {
      el.textContent = version ? (version.toLowerCase() === 'palworld' ? 'PalWorld' : `PalWorld ${version}`) : '';
      el.style.display = version ? '' : 'none';
    }
  } catch (e) {
    console.error('Failed to get game version:', e);
  }
}
