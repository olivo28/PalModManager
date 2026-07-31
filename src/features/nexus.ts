import { refreshNexusCache } from '../api';
import { getState, updateState } from '../state';
import { renderModsView } from '../ui/modsView';
import { showToast } from '../ui/toast';

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

  showToast(`Fetching Nexus info for ${needsFetch.length} mod(s)...`, 'info');

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
    showToast(`Updated Nexus info for ${fetchedCount} mod(s)`, 'success');
  }
}
