import { disableMod, enableMod, removeMod, refreshNexusCache, setModConfig, setModConfigs, openModFolder, renameMod } from '../../api';
import { getState, updateState } from '../../state';
import { openConfigEditor } from '../editorView';
import { loadMods, renderModsView, loadProfiles, loadDependencies } from '../modsView';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t } from '../../utils/i18n';
import { detailDom, bus } from '../../framework';

export function closeDetailPanel(): void {
  detailDom.el('detail-overlay').classList.remove('visible');
  updateState({ currentDetailMod: null });
}

export async function handleRefreshDetail(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod?.nexusModId) return;
  const btn = detailDom.el('detail-refresh');
  btn.disabled = true;
  btn.textContent = t('toasts.refreshing');
  try {
    const updated = await refreshNexusCache(state.currentDetailMod.id);
    const idx = state.allMods.findIndex(m => m.id === state.currentDetailMod!.id);
    if (idx >= 0) {
      const newMods = [...state.allMods];
      newMods[idx] = updated;
      updateState({ allMods: newMods });
    }
    const { openDetailPanel } = await import('./panel');
    openDetailPanel(updated.id);
    renderModsView();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = t('toasts.refresh_info');
  }
}

export function handleDetailConfig(): void {
  const state = getState();
  if (state.currentDetailMod) {
    closeDetailPanel();
    openConfigEditor(state.currentDetailMod.id);
  }
}

export async function handleDetailToggle(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  const isWorkshop = state.currentDetailMod.nexusSummary === 'Steam Workshop Mod';
  try {
    const { suppressWatcherRefresh } = await import('../editor/watcher');
    suppressWatcherRefresh(1200);

    if (isWorkshop) {
      const { activateWorkshopMod, deactivateWorkshopMod } = await import('../../api');
      if (state.currentDetailMod.enabled) {
        await deactivateWorkshopMod(state.currentDetailMod.id);
      } else {
        await activateWorkshopMod(state.currentDetailMod.id);
      }
    } else {
      if (state.currentDetailMod.enabled) { await disableMod(state.currentDetailMod.id); }
      else { await enableMod(state.currentDetailMod.id); }
    }
    await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
    const { openDetailPanel } = await import('./panel');
    openDetailPanel(state.currentDetailMod.id);
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailRemove(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  const confirmed = await showConfirm(t('dialogs.confirm_remove_mod', { name: state.currentDetailMod.name }));
  if (confirmed) {
    try {
      await removeMod(state.currentDetailMod.id);
      closeDetailPanel();
      await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
      showToast(t('toasts.mod_removed'), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    }
  }
}

export async function handleDetailSetConfig(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const { getModComponentFolders } = await import('./helpers');
    const compFolders = getModComponentFolders(state.currentDetailMod);
    const ue4ssComp = compFolders.find(c => c.type === 'ue4ss');
    const basePath = ue4ssComp
      ? ue4ssComp.path
      : (state.currentDetailMod.enabled
        ? state.currentDetailMod.gamePath
        : state.currentDetailMod.disabledPath);
    const selected = await open({
      multiple: true,
      defaultPath: basePath,
      filters: [{ name: 'Config files', extensions: ['json', 'jsonc', 'lua', 'ini', 'cfg', 'txt'] }],
      title: t('detail.dialog_select_config_title', { name: state.currentDetailMod.name }),
    });
    if (!selected) return;
    const configPaths = Array.isArray(selected)
      ? (selected.filter(Boolean) as string[])
      : [selected as string];
    if (configPaths.length === 0) return;
    await setModConfigs(state.currentDetailMod.id, configPaths);
    await loadMods();
    const { openDetailPanel } = await import('./panel');
    openDetailPanel(state.currentDetailMod.id);
    showToast(t('toasts.settings_saved'), 'success');
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailClearConfig(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    await setModConfigs(state.currentDetailMod.id, null);
    await loadMods();
    const { openDetailPanel } = await import('./panel');
    openDetailPanel(state.currentDetailMod.id);
    showToast(t('toasts.settings_saved'), 'success');
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailOpenFolder(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    await openModFolder(state.currentDetailMod.id);
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailOpenExtraFolder(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    const { openExtraFolder } = await import('../../api');
    await openExtraFolder(state.currentDetailMod.id);
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailRename(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  const currentName = state.currentDetailMod.name;
  const header = detailDom.el('detail-name-header');
  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'rename-input';
  input.value = currentName;
  input.maxLength = 200;
  header.textContent = '';
  header.appendChild(input);
  input.focus();
  input.select();

  const done = async (save: boolean) => {
    if (save) {
      const newName = input.value.trim();
      if (newName && newName !== currentName) {
        try {
          const updated = await renameMod(state.currentDetailMod!.id, newName);
          const newAllMods = state.allMods.map((m) => (m.id === updated.id ? { ...m, name: updated.name } : m));
          updateState({ currentDetailMod: updated, allMods: newAllMods });
          header.textContent = updated.name;
          bus.emit('mod:renamed', { modId: updated.id, newName: updated.name });
          renderModsView();
          showToast(t('toasts.mod_updated', { name: newName }), 'success');
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
          header.textContent = currentName;
        }
      } else {
        header.textContent = currentName;
      }
    } else {
      header.textContent = currentName;
    }
  };

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { input.blur(); done(true); }
    if (e.key === 'Escape') { input.blur(); done(false); }
  });
  input.addEventListener('blur', () => done(true));
}
