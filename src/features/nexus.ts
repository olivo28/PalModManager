import { refreshNexusCache } from '../api';
import { getState, updateState } from '../state';
import { renderModsView } from '../ui/modsView';
import { showToast } from '../ui/toast';
import { t } from '../utils/i18n';

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export async function autoFetchNexusInfo(): Promise<void> {
  const state = getState();
  const needsFetch = state.allMods.filter(m =>
    m.nexusModId && (
      !m.nexusDescription ||
      !m.nexusPictureUrl ||
      !m.nexusAuthor ||
      !m.nexusVersionCached
    )
  );

  if (needsFetch.length === 0) return;

  showToast(t('toasts.fetching_nexus_count', { count: needsFetch.length }), 'info');

  let fetchedCount = 0;
  for (let i = 0; i < needsFetch.length; i++) {
    const mod = needsFetch[i];
    console.log(`[Nexus] Fetching (${i + 1}/${needsFetch.length}): "${mod.name}" (nexusId: ${mod.nexusModId})`);
    try {
      const updated = await refreshNexusCache(mod.id);
      const idx = state.allMods.findIndex(m => m.id === mod.id);
      if (idx >= 0) {
        const newMods = [...state.allMods];
        newMods[idx] = updated;
        updateState({ allMods: newMods });
        fetchedCount++;
        // Re-render immediately so the user sees each update live
        renderModsView();
      }
      await sleep(500);
    } catch (e) {
      console.warn(`[Nexus] Failed to fetch info for "${mod.name}" (${mod.nexusModId}):`, e);
    }
  }

  if (fetchedCount > 0) {
    showToast(t('toasts.updated_nexus_count', { count: fetchedCount }), 'success');
  }
}
