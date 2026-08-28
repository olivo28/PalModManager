import { refreshNexusCache, setNexusModId } from '../../api';
import { getState, updateState } from '../../state';
import { loadMods, renderModsView } from '../modsView';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import type { ModInfo } from '../../types';

export async function autoFetchNexusInfo(mod: ModInfo): Promise<void> {
  if (!mod.nexusModId) return;
  try {
    const updated = await refreshNexusCache(mod.id);
    const state = getState();
    const idx = state.allMods.findIndex(m => m.id === mod.id);
    if (idx >= 0) {
      const newMods = [...state.allMods];
      newMods[idx] = updated;
      updateState({ allMods: newMods });
    }
    const { openDetailPanel } = await import('./panel');
    openDetailPanel(updated.id);
    renderModsView();
  } catch (e) {
    console.warn('Auto-fetch nexus info failed:', e);
  }
}

export function setupNexusIdEdit(modId: string): void {
  const editBtn = document.querySelector('.nexus-id-edit-btn') as HTMLButtonElement;
  const saveBtn = document.querySelector('.nexus-id-save-btn') as HTMLButtonElement;
  const cancelBtn = document.querySelector('.nexus-id-cancel-btn') as HTMLButtonElement;
  const idRow = document.querySelector('.detail-nexus-id-row') as HTMLElement;
  const editRow = document.querySelector('.detail-nexus-edit-row') as HTMLElement;
  const input = document.querySelector('.nexus-id-input') as HTMLInputElement;
  if (!editBtn || !saveBtn || !cancelBtn || !idRow || !editRow || !input) return;

  editBtn.addEventListener('click', () => {
    idRow.style.display = 'none';
    editRow.style.display = '';
    input.focus();
  });

  cancelBtn.addEventListener('click', () => {
    editRow.style.display = 'none';
    idRow.style.display = '';
  });

  saveBtn.addEventListener('click', async () => {
    const val = input.value.trim();
    if (!val) return;
    saveBtn.disabled = true;
    try {
      await setNexusModId(modId, parseInt(val));
      await loadMods();
      const { openDetailPanel } = await import('./panel');
      openDetailPanel(modId);
      renderModsView();
      showToast(t('toasts.nexus_id_updated'), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    } finally {
      saveBtn.disabled = false;
    }
  });
}
