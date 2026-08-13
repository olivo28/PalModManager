import { getState, updateState } from '../../state';
import { disableMod, enableMod, removeMod, checkForUpdates, disableAllMods, enableAllMods } from '../../api';
import { openDetailPanel, closeDetailPanel } from '../detailPanel';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { renderModsView } from './renderer';
import { loadMods } from './loader';
import { loadProfiles, showInputModal } from './profiles';
import { escapeHtml } from '../../utils/helpers';

export function attachCardEvents(container: HTMLElement): void {
  container.querySelectorAll('.mod-card').forEach((card) => {
    card.addEventListener('dblclick', (e) => {
      if ((e.target as HTMLElement).closest('.toggle-switch')) return;
      if ((e.target as HTMLElement).closest('.mod-folder-btn')) return;
      const type = (card as HTMLElement).dataset.type;
      const id = (card as HTMLElement).dataset.id!;
      if (type === 'folder') {
        updateState({ currentFolderId: id });
        renderModsView();
      } else {
        openDetailPanel(id);
      }
    });
  });

  container.querySelectorAll('.card-toggle-input').forEach((cb) => {
    cb.addEventListener('change', async (e) => {
      e.stopPropagation();
      const target = e.currentTarget as HTMLInputElement;
      const id = target.dataset.id!;
      const isEnabled = target.checked;

      const card = target.closest('.mod-card') as HTMLElement | null;
      const isWorkshop = card ? card.dataset.isWorkshop === 'true' : false;
      if (card) {
        card.classList.toggle('disabled', !isEnabled);
        const led = card.querySelector('.mod-card-led');
        if (led) {
          led.classList.toggle('on', isEnabled);
          led.classList.toggle('off', !isEnabled);
        }
      }

      try {
        const { setModProfileState } = await import('../../api');
        if (isWorkshop) {
          const { activateWorkshopMod, deactivateWorkshopMod } = await import('../../api');
          if (isEnabled) {
            await activateWorkshopMod(id);
          } else {
            await deactivateWorkshopMod(id);
          }
          try { await setModProfileState(id, isEnabled); } catch { }
        } else {
          if (isEnabled) { await enableMod(id); } else { await disableMod(id); }
          try { await setModProfileState(id, isEnabled); } catch { }
        }
        showToast(isEnabled ? 'Mod enabled' : 'Mod disabled', isEnabled ? 'success' : 'info');
        await loadMods();
        const state = getState();
        if (state.currentDetailMod?.id === id) {
          openDetailPanel(id);
        }
      } catch (e) {
        target.checked = !isEnabled;
        if (card) {
          card.classList.toggle('disabled', isEnabled);
          const led = card.querySelector('.mod-card-led');
          if (led) {
            led.classList.toggle('on', !isEnabled);
            led.classList.toggle('off', isEnabled);
          }
        }
        console.error('Error toggling mod:', e);
        showToast('Failed to toggle mod: ' + e, 'error');
      }
    });
  });

  container.querySelectorAll('.mod-card-update-badge').forEach((badge) => {
    badge.addEventListener('click', async (e) => {
      e.stopPropagation();
      const card = (e.currentTarget as HTMLElement).closest('.mod-card') as HTMLElement | null;
      if (!card) return;
      const modId = card.dataset.id!;
      const isWorkshop = card.dataset.isWorkshop === 'true';
      const mod = getState().allMods.find(m => m.id === modId);
      if (!mod) return;

      try {
        if (isWorkshop) {
          showToast('Preparing workshop update files...', 'info');
          const { prepareWorkshopUpdateZip, analyzeZip, checkModExistsCommand } = await import('../../api');
          const { renderInstallPreview, showInstallModal } = await import('../modal');
          const zipPath = await prepareWorkshopUpdateZip(mod.id);
          const analysis = await analyzeZip(zipPath);
          const check = await checkModExistsCommand(zipPath);
          const existingMod = check.exists && check.modInfo 
            ? { id: check.modInfo.id, name: check.modInfo.name, version: check.modInfo.version } 
            : null;
          renderInstallPreview(analysis, existingMod);
          showInstallModal();
        } else {
          const updateVer = getState().availableUpdates?.get(modId);
          const libEntries = getState().libraryEntries || [];
          const normModName = mod.name.toLowerCase().replace(/[^a-z0-9]/g, '');
          const normModId = mod.id.toLowerCase().replace(/[^a-z0-9]/g, '');
          const matchingLib = libEntries.find(entry => {
            const normLibId = (entry.modId || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            const normLibName = (entry.nexusName || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            const isMatch = normLibId === normModName || normLibId === normModId || (normLibName !== '' && normLibName === normModName) || (entry.nexusModId && mod.nexusModId && entry.nexusModId === mod.nexusModId);
            if (!isMatch) return false;
            if (updateVer) {
              const eVer = (entry.version || '').replace(/^v/i, '').trim();
              return eVer === updateVer.replace(/^v/i, '').trim();
            }
            return true;
          });

          if (matchingLib) {
            const { triggerInstallFromLibrary } = await import('./library');
            await triggerInstallFromLibrary(matchingLib.modId, matchingLib.zipName);
          } else {
            const { openDetailPanel } = await import('../detailPanel');
            openDetailPanel(modId);
            showToast(`Update v${updateVer} is available`, 'info');
          }
        }
      } catch (err) {
        showToast('Failed to start update: ' + err, 'error');
      }
    });
  });

  container.querySelectorAll('.card-remove-btn').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (e.currentTarget as HTMLElement).dataset.id!;
      const mod = getState().allMods.find((m) => m.id === id);
      const name = mod ? mod.name : 'this mod';
      const confirmed = await showConfirm(`Remove "${name}" permanently?`);
      if (confirmed) {
        try {
          await removeMod(id);
          closeDetailPanel();
          await loadMods();
          showToast('Mod removed', 'success');
        } catch (e) {
          console.error('Error removing mod:', e);
          showToast('Failed to remove mod: ' + e, 'error');
        }
      }
    });
  });
}

export function handleSort(btn: HTMLButtonElement): void {
  const state = getState();
  const sortField = btn.dataset.sort!;
  let newSort = { field: sortField, asc: true };
  if (state.currentSort.field === sortField) {
    newSort.asc = !state.currentSort.asc;
  }
  updateState({ currentSort: newSort });

  document.querySelectorAll('.sort-btn').forEach((b) => {
    b.classList.remove('active');
    b.querySelector('.arrow')!.textContent = '\u25B2';
  });
  btn.classList.add('active');
  btn.querySelector('.arrow')!.textContent = newSort.asc ? '\u25B2' : '\u25BC';
  renderModsView();
}

function syncFilterUI(filters: Set<string>): void {
  document.querySelectorAll('.quick-filter-btn[data-filter]').forEach(b => {
    b.classList.toggle('active', filters.has((b as HTMLElement).dataset.filter!));
  });
}

export function setupFilterListeners(): void {
  document.querySelectorAll('.quick-filter-btn[data-filter]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const filter = (btn as HTMLElement).dataset.filter!;
      const state = getState();
      const newFilters = new Set(state.activeFilters);
      if (newFilters.has(filter)) {
        newFilters.delete(filter);
      } else {
        newFilters.add(filter);
      }
      if (newFilters.size === 0) {
        newFilters.add('ue4ss');
        newFilters.add('palschema');
        newFilters.add('pak');
        newFilters.add('logicmods');
        newFilters.add('hybrid');
      }
      updateState({ activeFilters: newFilters });
      syncFilterUI(newFilters);
      renderModsView();
    });
  });

  const advancedBtn = document.getElementById('advanced-filter-btn');
  const advancedDropdown = document.getElementById('advanced-filter-dropdown');
  if (advancedBtn && advancedDropdown) {
    advancedBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      advancedDropdown.style.display = advancedDropdown.style.display === 'none' ? 'block' : 'none';
    });
    document.addEventListener('click', () => {
      advancedDropdown.style.display = 'none';
    });
    advancedDropdown.addEventListener('click', (e) => e.stopPropagation());
  }
}

export async function handleCheckUpdates(): Promise<void> {
  const btn = document.getElementById('check-updates-btn')! as HTMLButtonElement;
  btn.disabled = true;
  btn.innerHTML = '<span class="btn-icon-text">&#8634;</span> Checking...';
  showToast('Checking for updates...', 'info');

  try {
    const updates = await checkForUpdates();
    const updatesMap = new Map<string, string>();
    for (const u of updates) {
      updatesMap.set(u.modId, u.latestVersion);
    }
    updateState({ availableUpdates: updatesMap });
    renderModsView();

    if (updates.length === 0) {
      showToast('All mods are up to date', 'success');
    } else {
      showToast(`Found ${updates.length} mod(s) with updates available`, 'success');
      for (const u of updates) {
        showToast(`${u.name}: ${u.currentVersion} → ${u.latestVersion}`, 'info');
      }
    }
  } catch (e) {
    showToast('Failed to check updates: ' + e, 'error');
  } finally {
    btn.disabled = false;
    btn.innerHTML = '<span class="btn-icon-text">&#8634;</span> Updates';
  }
}

export function handleOpenAllUpdates(): void {
  const state = getState();
  const updates = state.availableUpdates;
  let count = 0;
  import('../../api').then(({ openUrl }) => {
  for (const [modId, _] of updates) {
    const mod = state.allMods.find(m => m.id === modId);
    if (mod && mod.nexusModId) {
      const url = `https://www.nexusmods.com/palworld/mods/${mod.nexusModId}`;
      openUrl(url).catch((e: any) => console.error(e));
      count++;
    }
  }
  if (count > 0) {
    showToast(`Opening ${count} NexusMods update page(s) in browser`, 'success');
  }
  });
}

export async function handleDisableAll(): Promise<void> {
  showToast('Disabling all mods...', 'info');
  try {
    const result = await disableAllMods();
    showToast(`Disabled ${result.disabled} mod(s)`, 'success');
    await loadMods();
  } catch (e) {
    showToast('Failed to disable all: ' + e, 'error');
  }
}

export async function handleEnableAll(): Promise<void> {
  showToast('Enabling all mods...', 'info');
  try {
    const result = await enableAllMods();
    showToast(`Enabled ${result.enabled} mod(s)`, 'success');
    await loadMods();
  } catch (e) {
    showToast('Failed to enable all: ' + e, 'error');
  }
}

export function setupAdvancedFilterHandlers(): void {
  document.getElementById('filter-tags-list')!.addEventListener('click', (e) => {
    const chip = (e.target as HTMLElement).closest('.filter-chip') as HTMLElement | null;
    if (!chip || chip.dataset.type !== 'tag') return;
    const value = chip.dataset.value!;
    const state = getState();
    const newFilters = new Set(state.tagFilters);
    if (newFilters.has(value)) newFilters.delete(value);
    else newFilters.add(value);
    chip.classList.toggle('active');
    updateState({ tagFilters: newFilters });
    renderModsView();
  });

  document.getElementById('filter-cats-list')!.addEventListener('click', (e) => {
    const chip = (e.target as HTMLElement).closest('.filter-chip') as HTMLElement | null;
    if (!chip || chip.dataset.type !== 'cat') return;
    const value = chip.dataset.value!;
    const state = getState();
    const newFilters = new Set(state.categoryFilters);
    if (newFilters.has(value)) newFilters.delete(value);
    else newFilters.add(value);
    chip.classList.toggle('active');
    updateState({ categoryFilters: newFilters });
    renderModsView();
  });
}

export function setupStatusFilterHandlers(): void {
  document.querySelectorAll('.status-filter-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const status = (btn as HTMLElement).dataset.status as 'all' | 'enabled' | 'disabled';
      document.querySelectorAll('.status-filter-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      updateState({ statusFilter: status });
      renderModsView();
    });
  });
}

export function attachFolderEvents(container: HTMLElement): void {
  const backBtn = container.querySelector('#btn-back-to-root');
  if (backBtn) {
    backBtn.addEventListener('click', () => {
      updateState({ currentFolderId: null });
      renderModsView();
    });
  }

  container.querySelectorAll('.folder-toggle-input').forEach(input => {
    input.addEventListener('change', async (e) => {
      e.stopPropagation();
      const target = e.currentTarget as HTMLInputElement;
      const folderId = target.dataset.folderId!;
      const enabled = target.checked;

      try {
        const { toggleFolderMods } = await import('../../api');
        const state = getState();
        const updatedProfile = await toggleFolderMods(state.currentProfileId, folderId, enabled);

        const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles: updatedProfiles });

        showToast(enabled ? 'All folder mods enabled' : 'All folder mods disabled', 'success');
        await loadMods();
      } catch (err) {
        target.checked = !enabled;
        showToast('Failed to toggle folder mods: ' + err, 'error');
      }
    });
  });

  container.querySelectorAll('.mod-folder-btn.rename-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const folderId = (btn as HTMLElement).dataset.folderId!;
      const state = getState();
      const folder = state.profiles.find(p => p.id === state.currentProfileId)?.mod_folders?.find(f => f.id === folderId);
      if (!folder) return;

      const newName = await showInputModal('Rename Folder', 'Enter new folder name:', folder.name);
      if (newName === null) return;
      const trimmed = newName.trim();
      if (!trimmed) return;

      try {
        const { renameModFolder } = await import('../../api');
        const updatedProfile = await renameModFolder(state.currentProfileId, folderId, trimmed);

        const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles: updatedProfiles });

        showToast('Folder renamed', 'success');
        await loadMods();
      } catch (err) {
        showToast('Failed to rename folder: ' + err, 'error');
      }
    });
  });

  container.querySelectorAll('.mod-folder-btn.delete-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const folderId = (btn as HTMLElement).dataset.folderId!;
      const state = getState();
      const folder = state.profiles.find(p => p.id === state.currentProfileId)?.mod_folders?.find(f => f.id === folderId);
      const name = folder ? folder.name : 'this folder';

      const confirmed = await showConfirm(`Delete folder "${name}"? Mods inside will not be deleted (they will just return to Ungrouped).`);
      if (confirmed) {
        try {
          const { deleteModFolder } = await import('../../api');
          const updatedProfile = await deleteModFolder(state.currentProfileId, folderId);

          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });

          showToast('Folder deleted', 'success');
          if (state.currentFolderId === folderId) {
            updateState({ currentFolderId: null });
          }
          await loadMods();
        } catch (err) {
          showToast('Failed to delete folder: ' + err, 'error');
        }
      }
    });
  });
}

export async function handleAddModToFolder(folderId: string | null, modId: string): Promise<void> {
  const { currentProfileId } = getState();
  try {
    const { addModToFolder } = await import('../../api');
    const updatedProfile = await addModToFolder(currentProfileId, folderId, modId);

    const state = getState();
    const profiles = state.profiles.map(p => p.id === currentProfileId ? updatedProfile : p);
    updateState({ profiles });

    await loadMods();
    showToast(folderId ? 'Mod grouped into folder' : 'Mod moved to ungrouped', 'success');
  } catch (err) {
    showToast('Failed to move mod: ' + err, 'error');
  }
}

export async function handleCreateFolder(name: string): Promise<void> {
  const { currentProfileId } = getState();
  try {
    const { createModFolder } = await import('../../api');
    const updatedProfile = await createModFolder(currentProfileId, name);
    showToast('Folder created', 'success');

    const state = getState();
    const profiles = state.profiles.map(p => p.id === currentProfileId ? updatedProfile : p);
    updateState({ profiles });

    await loadMods();
  } catch (err) {
    showToast('Failed to create folder: ' + err, 'error');
  }
}
export { setupCardDragToFolder } from './dragDrop';
