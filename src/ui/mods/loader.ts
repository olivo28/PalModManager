import { getMods, scanMods, getGameVersion } from '../../api';
import { getState, updateState } from '../../state';
import { renderModsView } from './renderer';
import { computeAvailableUpdates } from './card';
import { populateAdvancedFilters } from './renderer';
import { populateEditorModSelect } from '../editorView';
import { loadProfiles } from './profiles';
import { showToast } from '../toast';

export async function loadMods(): Promise<void> {
  const container = document.getElementById('mods-container');
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
      loadProfiles();
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
      showToast(`Workshop mod "${mod.name}" was updated by Steam! Right-click to Update Mod.`, 'info');
    }

    updateState({ allMods: freshMods, availableUpdates: updatesMap });
    renderModsView();
    populateAdvancedFilters();
    populateEditorModSelect();
    loadProfiles();
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
    const el = document.getElementById('game-version-badge');
    if (el) {
      el.textContent = version ? (version.toLowerCase() === 'palworld' ? 'PalWorld' : `PalWorld ${version}`) : '';
      el.style.display = version ? '' : 'none';
    }
  } catch (e) {
    console.error('Failed to get game version:', e);
  }
}
