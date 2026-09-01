import { getState, updateState } from '../../state';
import { disableMod, enableMod, removeMod, checkForUpdates, disableAllMods, enableAllMods } from '../../api';
import { openDetailPanel, closeDetailPanel } from '../detailPanel';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { renderModsView } from './renderer';
import { loadMods } from './loader';
import { loadProfiles, showInputModal } from './profiles';
import { loadDependencies } from './dependencies';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { bus, mainDom } from '../../framework';

export function attachCardEvents(container: HTMLElement): void {
  container.querySelectorAll('.mod-card').forEach((card) => {
    // Single click for folder accordion in list view & selection
    card.addEventListener('click', (e) => {
      if ((e.target as HTMLElement).closest('.toggle-switch, .folder-toggle-input, .card-toggle-input, .mod-folder-btn, button, input, a, select, .folder-card-actions')) return;
      const type = (card as HTMLElement).dataset.type;
      const id = (card as HTMLElement).dataset.id!;
      const state = getState();

      // Trigger selection on click
      import('../../features/selection').then(({ handleCardClick }) => {
        handleCardClick(card as HTMLElement, e as MouseEvent);
      }).catch(() => {});

      if (type === 'folder' && state.viewLayout === 'list') {
        const collapsed = new Set(state.collapsedFolderIds || []);
        const isCurrentlyCollapsed = collapsed.has(id);
        const willExpand = isCurrentlyCollapsed; // if it was collapsed, now it expands

        if (willExpand) {
          collapsed.delete(id);
        } else {
          collapsed.add(id);
        }
        updateState({ collapsedFolderIds: collapsed });

        if (state.currentSettings?.folderExpandMode === 'remember') {
          try {
            localStorage.setItem('palmodmanager_collapsed_folders', JSON.stringify(Array.from(collapsed)));
          } catch {}
        }

        card.classList.toggle('expanded', willExpand);
        card.classList.toggle('collapsed', !willExpand);
        const chevron = card.querySelector('.folder-chevron');
        if (chevron) chevron.textContent = willExpand ? '▼' : '▶';

        const childRows = container.querySelectorAll(`[data-folder-id="${id}"]`);
        childRows.forEach((r) => {
          r.classList.toggle('is-collapsed', !willExpand);
        });
      }
    });

    // Double click for opening details (or grid folder navigation)
    card.addEventListener('dblclick', (e) => {
      if ((e.target as HTMLElement).closest('.toggle-switch, .folder-toggle-input, .card-toggle-input, .mod-folder-btn, button, input, a')) return;
      const type = (card as HTMLElement).dataset.type;
      const id = (card as HTMLElement).dataset.id!;
      const state = getState();

      if (type === 'folder') {
        if (state.viewLayout === 'grid') {
          updateState({ currentFolderId: id });
          renderModsView();
        }
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
        const { suppressWatcherRefresh } = await import('../editor/watcher');
        suppressWatcherRefresh(1200);

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
        bus.emit('mod:toggled', { id, enabled: isEnabled });
        showToast(isEnabled ? t('toasts.mod_enabled') : t('toasts.mod_disabled'), isEnabled ? 'success' : 'info');
        await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
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
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
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
          showToast(t('toasts.preparing_workshop_update'), 'info');
          const { activateWorkshopMod } = await import('../../api');
          await activateWorkshopMod(mod.id);
          showToast(t('toasts.mod_updated', { name: mod.name }), 'success');
          await loadMods();
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
            showToast(t('toasts.updating_from_local_library', { name: mod.name, version: matchingLib.version || updateVer || '' }), 'info');
            await triggerInstallFromLibrary(matchingLib.modId, matchingLib.zipName);
          } else if (mod.nexusModId) {
            const hasNexusAccount = !!getState().currentSettings?.nexusAccount?.accessToken;
            if (hasNexusAccount) {
              const { openModDetails } = await import('../discoveryView');
              await openModDetails(mod.nexusModId);
              const filesTabBtn = document.querySelector('.discovery-modal-tab[data-tab="files"]') as HTMLElement | null;
              filesTabBtn?.click();
              showToast(t('toasts.select_update_file_nexus'), 'info');
            } else {
              const { openUrl } = await import('../../api');
              openUrl(`https://www.nexusmods.com/palworld/mods/${mod.nexusModId}?tab=files`);
              showToast(t('toasts.opening_nexus_files'), 'info');
            }
          } else {
            const { openDetailPanel } = await import('../detailPanel');
            openDetailPanel(modId);
            showToast(t('toasts.update_available', { target: updateVer || '', current: mod.version }), 'info');
          }
        }
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    });
  });

  container.querySelectorAll('.card-remove-btn').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (e.currentTarget as HTMLElement).dataset.id!;
      const mod = getState().allMods.find((m) => m.id === id);
      const name = mod ? mod.name : 'this mod';
      const confirmed = await showConfirm(t('dialogs.confirm_remove_mod', { name }));
      if (confirmed) {
        try {
          await removeMod(id);
          closeDetailPanel();
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          showToast(t('toasts.mod_removed'), 'success');
        } catch (e) {
          console.error('Error removing mod:', e);
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
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

  const advancedBtn = mainDom.elMaybe('advanced-filter-btn');
  const advancedDropdown = mainDom.elMaybe('advanced-filter-dropdown');
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
  const btn = mainDom.el('check-updates-btn');
  btn.disabled = true;
  btn.innerHTML = '<span class="btn-icon-text">&#8634;</span> ...';
  showToast(t('common.checking_updates'), 'info');

  try {
    const updates = await checkForUpdates();
    const updatesMap = new Map<string, string>();
    for (const u of updates) {
      updatesMap.set(u.modId, u.latestVersion);
    }
    updateState({ availableUpdates: updatesMap });
    renderModsView();

    if (updates.length === 0) {
      showToast(t('toasts.all_mods_up_to_date'), 'success');
    } else {
      showToast(t('toasts.found_updates_count', { count: updates.length }), 'success');
      for (const u of updates) {
        showToast(`${u.name}: ${u.currentVersion} → ${u.latestVersion}`, 'info');
      }
    }
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    btn.disabled = false;
    btn.innerHTML = `<span class="btn-icon-text">&#8634;</span> ${escapeHtml(t('mods.btn_updates_label'))}`;
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
    showToast(t('toasts.opening_nexus_pages', { count }), 'success');
  }
  });
}

export async function handleDisableAll(): Promise<void> {
  showToast(t('toasts.disabled_all_success', { count: '' }), 'info');
  try {
    const result = await disableAllMods();
    showToast(t('toasts.disabled_all_success', { count: result.disabled }), 'success');
    bus.emit('mods:refresh', undefined);
    await loadMods();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleEnableAll(): Promise<void> {
  showToast(t('toasts.enabled_all_success', { count: '' }), 'info');
  try {
    const result = await enableAllMods();
    showToast(t('toasts.enabled_all_success', { count: result.enabled }), 'success');
    bus.emit('mods:refresh', undefined);
    await loadMods();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export function setupAdvancedFilterHandlers(): void {
  mainDom.elMaybe('filter-tags-list')?.addEventListener('click', (e) => {
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

  mainDom.elMaybe('filter-cats-list')?.addEventListener('click', (e) => {
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

        showToast(enabled ? t('toasts.folder_mods_enabled') : t('toasts.folder_mods_disabled'), 'success');
        await loadMods();
      } catch (err) {
        target.checked = !enabled;
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
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

      const newName = await showInputModal(t('dialogs.prompt_rename_folder'), t('dialogs.prompt_rename_folder'), folder.name);
      if (newName === null) return;
      const trimmed = newName.trim();
      if (!trimmed) return;

      try {
        const { renameModFolder } = await import('../../api');
        const updatedProfile = await renameModFolder(state.currentProfileId, folderId, trimmed);

        const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles: updatedProfiles });

        showToast(t('toasts.folder_renamed'), 'success');
        await loadMods();
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
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

      const confirmed = await showConfirm(t('dialogs.confirm_delete_folder', { name }));
      if (confirmed) {
        try {
          const { deleteModFolder } = await import('../../api');
          const updatedProfile = await deleteModFolder(state.currentProfileId, folderId);

          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });

          showToast(t('toasts.folder_deleted'), 'success');
          if (state.currentFolderId === folderId) {
            updateState({ currentFolderId: null });
          }
          await loadMods();
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });
}

export async function handleAddModToFolder(folderId: string | null, modId: string): Promise<void> {
  await handleAddMultipleModsToFolder(folderId, [modId]);
}

export async function handleAddMultipleModsToFolder(folderId: string | null, modIds: string[]): Promise<void> {
  if (!modIds || modIds.length === 0) return;
  const { currentProfileId } = getState();
  try {
    const { addModToFolder } = await import('../../api');
    let updatedProfile;
    for (const id of modIds) {
      updatedProfile = await addModToFolder(currentProfileId, folderId, id);
    }

    if (updatedProfile) {
      const state = getState();
      const profiles = state.profiles.map(p => p.id === currentProfileId ? updatedProfile : p);
      updateState({ profiles });
    }

    await loadMods();
    showToast(folderId ? t('toasts.mod_grouped_success') : t('toasts.mod_ungrouped_success'), 'success');
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export async function handleCreateFolder(name: string): Promise<void> {
  const { currentProfileId } = getState();
  try {
    const { createModFolder } = await import('../../api');
    const updatedProfile = await createModFolder(currentProfileId, name);
    showToast(t('toasts.folder_created'), 'success');

    const state = getState();
    const profiles = state.profiles.map(p => p.id === currentProfileId ? updatedProfile : p);
    updateState({ profiles });

    await loadMods();
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}
export { setupCardDragToFolder } from './dragDrop';
