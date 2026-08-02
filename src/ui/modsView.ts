import { getMods, scanMods, disableMod, enableMod, removeMod, checkForUpdates, disableAllMods, enableAllMods, getGameVersion, getLibrary, installModFromLibrary, getProfiles, switchProfile, setModProfileState, checkDependencies, openModFolder, openExtraFolder, installUe4ss, installPalschema, uninstallUe4ss, uninstallPalschema, openUrl, removeFromLibrary, setHideNativeMods } from '../api';
import { getState, updateState } from '../state';
import { openDetailPanel, closeDetailPanel } from './detailPanel';
import { openConfigEditor, populateEditorModSelect } from './editorView';
import { showToast } from './toast';
import { showConfirm } from './confirm';
import { escapeHtml } from '../utils/helpers';
import type { ModInfo, Profile, LibraryEntry } from '../types';

function computeAvailableUpdates(mods: ModInfo[]): Map<string, string> {
  const updatesMap = new Map<string, string>();
  for (const m of mods) {
    if (m.nexusVersionCached && m.version) {
      const normNexus = m.nexusVersionCached.replace(/^v/i, '').trim().toLowerCase();
      const normLocal = m.version.replace(/^v/i, '').trim().toLowerCase();
      if (normNexus !== '' && normNexus !== 'unknown' && normLocal !== 'unknown' && normNexus !== normLocal && !normLocal.startsWith(normNexus)) {
        updatesMap.set(m.id, m.nexusVersionCached);
      }
    }
  }
  return updatesMap;
}

export async function loadMods(): Promise<void> {
  const container = document.getElementById('mods-container')!;

  // 1. Cargar mods desde la base de datos de manera instantánea
  try {
    const cachedMods = await getMods();
    if (cachedMods && cachedMods.length > 0) {
      const updatesMap = computeAvailableUpdates(cachedMods);
      updateState({ allMods: cachedMods, availableUpdates: updatesMap });
      renderModsView();
      populateAdvancedFilters();
      populateEditorModSelect();
      loadProfiles(); // Refresh profiles in parallel to update mod counts/badges
    } else {
      container.innerHTML = '<div id="loading-state">Scanning mods...</div>';
    }
  } catch (e) {
    console.error('Error loading cached mods:', e);
  }

  // 2. Escaneo en disco en segundo plano para sincronizar cambios
  try {
    const freshMods = await scanMods();
    const updatesMap = computeAvailableUpdates(freshMods);
    updateState({ allMods: freshMods, availableUpdates: updatesMap });
    renderModsView();
    populateAdvancedFilters();
    populateEditorModSelect();
    loadProfiles(); // Refresh profiles to get accurate installed counts
  } catch (e) {
    console.error('Error scanning mods:', e);
    const state = getState();
    if (!state.allMods || state.allMods.length === 0) {
      container.innerHTML = '<div id="empty-state">Error scanning mods.</div>';
    }
  }
}


// Persistent UI state for folded/collapsed virtual folders
const collapsedFolders = new Set<string>(JSON.parse(localStorage.getItem('pmm_collapsed_folders') || '[]'));

function toggleFolderCollapsed(folderId: string): void {
  if (collapsedFolders.has(folderId)) {
    collapsedFolders.delete(folderId);
  } else {
    collapsedFolders.add(folderId);
  }
  localStorage.setItem('pmm_collapsed_folders', JSON.stringify(Array.from(collapsedFolders)));
  renderModsView();
}

function buildModCardHtml(mod: ModInfo, state: any): string {
  const tags = mod.nexusTags && mod.nexusTags.length > 0
    ? `<div class="mod-card-tags">${mod.nexusTags.slice(0, 3).map(t => `<span class="mod-card-tag">${escapeHtml(t)}</span>`).join('')}</div>`
    : '';
  const catHtml = mod.nexusCategory ? `<span class="mod-card-category">${escapeHtml(mod.nexusCategory)}</span>` : '';
  const author = mod.nexusAuthor ? `<span class="mod-card-author">by ${escapeHtml(mod.nexusAuthor)}</span>` : '';
  const imageHtml = mod.nexusPictureUrl
    ? `<div class="mod-card-image-wrap"><img class="mod-card-image" src="${escapeHtml(mod.nexusPictureUrl)}" alt="" loading="lazy" /></div>`
    : `<div class="mod-card-image-wrap"><div class="mod-card-image-placeholder ${mod.type}">${mod.type === 'ue4ss' ? 'U' : mod.type === 'palschema' ? 'PS' : mod.type === 'pak' ? 'PK' : 'LM'}</div></div>`;

  const updateVer = state.availableUpdates?.get(mod.id);
  const updateBadge = updateVer
    ? `<span class="mod-card-update-badge" title="Update available to v${escapeHtml(updateVer)}">&#9650; Update (v${escapeHtml(updateVer)})</span>`
    : '';

  const isSelected = state.selectedModIds.has(mod.id);
  return `
  <div class="mod-card ${mod.enabled ? '' : 'disabled'} ${isSelected ? 'selected' : ''}" data-id="${mod.id}" data-type="${mod.type}" draggable="true">
    ${imageHtml}
    <div class="mod-card-body">
      <div class="mod-card-body-top">
        <span class="mod-card-name">${escapeHtml(mod.name)}</span>
        <span class="mod-card-led ${mod.enabled ? 'on' : 'off'}"></span>
      </div>
      <div class="mod-card-meta">
        <span class="mod-card-type ${mod.type}">${mod.type}</span>
        <span class="mod-card-version">v${escapeHtml(mod.version)}</span>
        ${updateBadge}
        ${catHtml}
      </div>
      ${author}
      ${tags}
    </div>
    <div class="mod-card-footer">
      <label class="toggle-switch">
        <input type="checkbox" class="card-toggle-input" data-id="${mod.id}" ${mod.enabled ? 'checked' : ''} />
        <span class="toggle-slider"></span>
      </label>
      <button class="card-remove-btn" data-id="${mod.id}" title="Remove mod">✕</button>
    </div>
  </div>`;
}

function buildFolderCardHtml(folder: any, modsInFolder: ModInfo[], state: any): string {
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);
  const folderCheckbox = `<label class="toggle-switch" title="Toggle all mods in this folder" onclick="event.stopPropagation()">
    <input type="checkbox" class="folder-toggle-input" data-folder-id="${folder.id}" ${allEnabled ? 'checked' : ''} />
    <span class="toggle-slider"></span>
  </label>`;
  const isSelected = state.selectedModIds.has(folder.id);

  return `
  <div class="mod-card folder-card ${isSelected ? 'selected' : ''}" data-id="${folder.id}" data-type="folder" draggable="true" style="position:relative;">
    <div class="folder-card-actions" style="position: absolute; top: 8px; right: 8px; display: flex; gap: 4px; opacity: 0; z-index: 10;" onclick="event.stopPropagation()">
      <button class="mod-folder-btn rename-btn" data-folder-id="${folder.id}" title="Rename folder" style="padding: 2px 6px; font-size: 11px; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); cursor: pointer; border-radius: 4px;">✏</button>
      <button class="mod-folder-btn delete-btn delete" data-folder-id="${folder.id}" title="Delete folder" style="padding: 2px 6px; font-size: 11px; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); cursor: pointer; border-radius: 4px;">✕</button>
    </div>
    <div class="mod-card-image-wrap folder-icon-wrap" style="display:flex;align-items:center;justify-content:center;height:120px;background:var(--bg-secondary);font-size:48px;">
      📁
    </div>
    <div class="mod-card-body" style="padding: 12px; display: flex; flex-direction: column; flex-grow: 1; justify-content: space-between;">
      <div class="mod-card-body-top">
        <span class="mod-card-name" style="font-weight: 600; font-size: 13px;">${escapeHtml(folder.name)}</span>
      </div>
      <div class="mod-card-meta" style="margin-top: 8px; display: flex; align-items: center; justify-content: space-between; font-size: 12px; color: var(--text-secondary);">
        <span>${modsInFolder.length} mod${modsInFolder.length === 1 ? '' : 's'}</span>
        ${folderCheckbox}
      </div>
    </div>
  </div>
  `;
}

export function renderModsView(): void {
  const state = getState();
  const container = document.getElementById('mods-container')!;
  const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);

  let filtered = state.allMods.filter((m) => {
    if (state.statusFilter === 'enabled' && !m.enabled) return false;
    if (state.statusFilter === 'disabled' && m.enabled) return false;

    const isUe4ssMod = m.type === 'ue4ss' || m.nexusAuthor === 'UE4SS Native Mod';
    if (isUe4ssMod && currentProfile && !currentProfile.ue4ss_enabled && state.statusFilter === 'enabled') {
      return false;
    }

    if (state.currentSettings?.hideNativeMods && m.nexusAuthor === 'UE4SS Native Mod') {
      return false;
    }
    if (!state.activeFilters.has(m.type)) return false;
    if (state.tagFilters.size > 0) {
      const modTags = new Set(m.nexusTags || []);
      let hasTag = false;
      for (const tag of state.tagFilters) {
        if (modTags.has(tag)) { hasTag = true; break; }
      }
      if (!hasTag) return false;
    }
    if (state.categoryFilters.size > 0) {
      if (!m.nexusCategory || !state.categoryFilters.has(m.nexusCategory)) return false;
    }
    if (state.searchQuery) {
      const q = state.searchQuery.toLowerCase();
      const matchName = m.name.toLowerCase().includes(q);
      const matchAuthor = m.nexusAuthor && m.nexusAuthor.toLowerCase().includes(q);
      const matchType = m.type.toLowerCase().includes(q);
      const matchId = m.id.toLowerCase().includes(q);
      const matchSummary = m.nexusSummary && m.nexusSummary.toLowerCase().includes(q);
      if (!matchName && !matchAuthor && !matchType && !matchId && !matchSummary) return false;
    }
    return true;
  });

  filtered.sort((a, b) => {
    const { field, asc } = state.currentSort;
    let cmp = 0;
    switch (field) {
      case 'name':
        cmp = a.name.localeCompare(b.name);
        break;
      case 'type':
        cmp = a.type.localeCompare(b.type);
        break;
      case 'date':
        cmp = a.installDate.localeCompare(b.installDate);
        break;
      case 'status':
        cmp = (a.enabled === b.enabled) ? 0 : a.enabled ? -1 : 1;
        break;
    }
    return asc ? cmp : -cmp;
  });

  if (filtered.length === 0 && !state.currentFolderId) {
    const isProfileEmpty = state.allMods.every(m => !m.enabled);
    container.innerHTML = `<div id="empty-state">${isProfileEmpty ? 'No active mods in this profile. Switch profiles or install/enable mods from the Library.' : 'No mods match the current filters.'}</div>`;
    return;
  }

  // Virtual Folders Grouping
  const folders = currentProfile?.mod_folders || [];
  const folderModsMap = new Map<string, ModInfo[]>();
  for (const f of folders) {
    folderModsMap.set(f.id, []);
  }

  const ungroupedMods: ModInfo[] = [];

  for (const mod of filtered) {
    let placed = false;
    for (const f of folders) {
      if (f.mod_ids.includes(mod.id)) {
        folderModsMap.get(f.id)!.push(mod);
        placed = true;
        break;
      }
    }
    if (!placed) {
      ungroupedMods.push(mod);
    }
  }

  let html = '';

  if (state.currentFolderId) {
    // Render inside a specific folder
    const activeFolder = folders.find(f => f.id === state.currentFolderId);
    const modsInFolder = folderModsMap.get(state.currentFolderId) || [];
    
    html += `
    <div class="folder-breadcrumb" id="mod-root-drop-zone" style="grid-column: 1 / -1; display: flex; align-items: center; gap: 12px; margin-bottom: 16px; padding: 12px 16px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 6px;">
      <button class="btn-secondary btn-sm" id="btn-back-to-root" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">← Back to Root</button>
      <span style="font-size: 14px; font-weight: 600; color: var(--text-primary);">Root / ${escapeHtml(activeFolder ? activeFolder.name : 'Unknown Folder')}</span>
    </div>
    `;

    if (modsInFolder.length === 0) {
      html += `<div style="grid-column: 1 / -1; padding: 48px; text-align: center; color: var(--text-muted); font-size: 13px; border: 1px dashed var(--border); border-radius: 6px;">No mods in this folder. Double-click "Back to Root" or drag mods to ungroup them.</div>`;
    } else {
      html += modsInFolder.map(m => buildModCardHtml(m, state)).join('');
    }
  } else {
    // Render Root level (folders first, then ungrouped mods)
    const renderFolderCards = folders.map(f => {
      const modsInFolder = folderModsMap.get(f.id) || [];
      return buildFolderCardHtml(f, modsInFolder, state);
    }).join('');

    html += renderFolderCards;
    html += ungroupedMods.map(m => buildModCardHtml(m, state)).join('');
  }

  container.innerHTML = html;

  attachCardEvents(container);
  attachFolderEvents(container);
}

function attachCardEvents(container: HTMLElement): void {
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

      // Instant visual feedback
      const card = target.closest('.mod-card');
      if (card) {
        card.classList.toggle('disabled', !isEnabled);
        const led = card.querySelector('.mod-card-led');
        if (led) {
          led.classList.toggle('on', isEnabled);
          led.classList.toggle('off', !isEnabled);
        }
      }

      try {
        if (isEnabled) { await enableMod(id); } else { await disableMod(id); }
        // Update profile state
        try { await setModProfileState(id, isEnabled); } catch { }
        showToast(isEnabled ? 'Mod enabled' : 'Mod disabled', 'success');
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

export function populateAdvancedFilters(): void {
  const state = getState();
  const tagsSet = new Set<string>();
  const catsSet = new Set<string>();
  state.allMods.forEach(m => {
    if (m.nexusTags) m.nexusTags.forEach(t => tagsSet.add(t));
    if (m.nexusCategory) catsSet.add(m.nexusCategory);
  });

  const tagsList = document.getElementById('filter-tags-list')!;
  const tagsHtml = Array.from(tagsSet).sort().map(t => `
    <button class="filter-chip ${state.tagFilters.has(t) ? 'active' : ''}" data-type="tag" data-value="${escapeHtml(t)}">${escapeHtml(t)}</button>
  `).join('');

  const catsList = document.getElementById('filter-cats-list')!;
  const catsHtml = Array.from(catsSet).sort().map(c => `
    <button class="filter-chip ${state.categoryFilters.has(c) ? 'active' : ''}" data-type="cat" data-value="${escapeHtml(c)}">${escapeHtml(c)}</button>
  `).join('');

  tagsList.innerHTML = tagsHtml
    ? `<div class="filter-chips">${tagsHtml}</div>`
    : '<div class="filter-dropdown-empty">No tags</div>';
  catsList.innerHTML = catsHtml
    ? `<div class="filter-chips">${catsHtml}</div>`
    : '<div class="filter-dropdown-empty">No categories</div>';
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

// === GAME VERSION ===

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

// === PROFILES ===

export async function loadProfiles(): Promise<void> {
  try {
    const { getProfiles, getCurrentProfile } = await import('../api');
    const [profiles, currentProfile] = await Promise.all([
      getProfiles(),
      getCurrentProfile().catch(() => null)
    ]);

    const activeProfile = currentProfile || profiles[0] || null;
    updateState({
      profiles,
      currentProfileId: activeProfile?.id || 'default',
      currentProfile: activeProfile,
    });
    updateActiveProfileLabel();
    renderProfileList();
  } catch (e) {
    console.error('Failed to load profiles:', e);
  }
}

function updateActiveProfileLabel(): void {
  const label = document.getElementById('profile-active-label');
  if (!label) return;
  const { profiles, currentProfileId } = getState();
  const current = profiles.find(p => p.id === currentProfileId);
  label.textContent = `Profile: ${current ? current.name : 'Default'}`;
}

function renderProfileList(): void {
  const list = document.getElementById('profile-list');
  if (!list) return;
  const { profiles, currentProfileId } = getState();
  list.innerHTML = profiles.map(p => {
    const modCount = p.enabled_mod_ids ? p.enabled_mod_ids.length : 0;
    const ue4ssBadge = p.ue4ss_enabled ? `<span class="profile-badge ue4ss">UE4SS</span>` : '';
    const palschemaBadge = p.palschema_enabled ? `<span class="profile-badge palschema">PalSchema</span>` : '';
    const modCountBadge = `<span class="profile-badge count">${modCount} mod${modCount === 1 ? '' : 's'}</span>`;

    return `
    <div class="profile-item ${p.id === currentProfileId ? 'active' : ''}" data-id="${p.id}">
      <div style="display:flex;flex-direction:column;gap:4px;">
        <div style="display:flex;align-items:center;gap:8px;">
          <span class="profile-item-name">${escapeHtml(p.name)}</span>
          ${p.id === currentProfileId ? '<span class="profile-item-badge-active">ACTIVE</span>' : ''}
        </div>
        <div style="display:flex;gap:4px;align-items:center;">
          ${ue4ssBadge}
          ${palschemaBadge}
          ${modCountBadge}
        </div>
      </div>
      <div style="display:flex;align-items:center;gap:8px;">
        <button class="btn-secondary btn-sm profile-clone-btn" data-id="${p.id}" style="font-size:11px;padding:4px 8px;">Clone</button>
        ${p.id !== currentProfileId ? `<button class="btn-secondary btn-sm profile-switch-btn" data-id="${p.id}" style="font-size:11px;padding:4px 8px;">Switch</button>` : ''}
        <button class="profile-item-delete ${p.id === 'default' ? 'disabled' : ''}" data-id="${p.id}" ${p.id === 'default' ? 'disabled' : ''}>✕</button>
      </div>
    </div>`;
  }).join('');

  list.querySelectorAll('.profile-item').forEach(item => {
    item.addEventListener('click', async (e) => {
      if ((e.target as HTMLElement).closest('.profile-item-delete') || (e.target as HTMLElement).closest('.profile-clone-btn')) return;
      const id = (item as HTMLElement).dataset.id!;
      if (id === getState().currentProfileId) return;
      try {
        // 1. Confirm discard/save for editor changes first
        const { confirmDiscardOrSave, clearOriginalContent } = await import('./editorView');
        const proceed = await confirmDiscardOrSave();
        if (!proceed) return; // User cancelled, abort switch

        const mods = await switchProfile(id);
        
        // Clear editor original content cache
        clearOriginalContent();

        // Immediately update state, clear editor state, and render for instant feedback
        updateState({
          allMods: mods,
          currentProfileId: id,
          editorModId: null,
          editorFiles: [],
          editorSelectedFile: null,
          currentFolderId: null
        });
        renderModsView();

        // Reset editor UI components to prevent stale mod code from showing
        const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
        if (editorContent) {
          editorContent.value = '';
          editorContent.disabled = true;
        }
        const editorPath = document.getElementById('editor-file-path');
        if (editorPath) editorPath.textContent = '';
        const nameEl = document.getElementById('editor-current-mod-name');
        if (nameEl) nameEl.textContent = '';
        const highlightCode = document.getElementById('editor-highlight-code');
        if (highlightCode) highlightCode.innerHTML = '';
        const fileTreeEl = document.getElementById('editor-file-tree');
        if (fileTreeEl) fileTreeEl.innerHTML = '<div class="editor-file-empty">No files loaded</div>';

        const { populateEditorModSelect, renderEditorModTree } = await import('./editorView');
        populateEditorModSelect();
        renderEditorModTree();

        showToast('Profile switched', 'success');
        // Then refresh profiles + dependencies for full reactivity
        await Promise.all([loadProfiles(), loadDependencies()]);
        renderModsView(); // Re-render with fully updated profile data
      } catch (err) {
        showToast('Failed to switch profile: ' + err, 'error');
      }
    });
  });

  list.querySelectorAll('.profile-clone-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const { profiles } = getState();
      const current = profiles.find(p => p.id === id);
      const name = current ? current.name : '';

      const newName = await showInputModal(
        'Duplicate Profile',
        `Enter a name for the duplicated profile of "${name}":`,
        `${name} - Copy`
      );
      if (newName === null) return;
      const trimmed = newName.trim();
      if (!trimmed) return;

      try {
        const { cloneProfile } = await import('../api');
        await cloneProfile(id, trimmed);
        showToast('Profile duplicated', 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast('Failed to duplicate profile: ' + err, 'error');
      }
    });
  });

  list.querySelectorAll('.profile-item-delete:not(.disabled)').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      try {
        const { deleteProfile } = await import('../api');
        await deleteProfile(id);
        showToast('Profile deleted', 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast('Failed to delete profile: ' + err, 'error');
      }
    });
  });
}

export function showInputModal(title: string, message: string, defaultValue: string): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '3000';
    overlay.style.position = 'fixed';
    overlay.style.top = '0';
    overlay.style.left = '0';
    overlay.style.right = '0';
    overlay.style.bottom = '0';
    overlay.style.background = 'rgba(0,0,0,0.6)';
    overlay.style.backdropFilter = 'blur(4px)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';

    overlay.innerHTML = `
      <div class="modal" style="width: 400px; max-width: 90vw; border: 1px solid var(--border); background: var(--bg-primary); border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5);">
        <div class="modal-header" style="padding: 16px 20px; border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between;">
          <h3 style="margin: 0; font-size: 16px; font-weight: 600; color: var(--text-primary);">${escapeHtml(title)}</h3>
          <button class="modal-close-btn" id="input-modal-close-x" style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;">✕</button>
        </div>
        <div class="modal-body" style="padding: 20px; display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 13px; color: var(--text-secondary);">${escapeHtml(message)}</div>
          <input type="text" id="input-modal-value" value="${escapeHtml(defaultValue)}" style="width: 100%; padding: 8px 12px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); border-radius: 4px; font-size: 13px; outline: none; box-sizing: border-box;" />
        </div>
        <div class="modal-footer" style="padding: 12px 20px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end; gap: 8px;">
          <button id="input-modal-cancel" class="btn-secondary" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">Cancel</button>
          <button id="input-modal-confirm" class="btn-primary" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">Confirm</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const input = overlay.querySelector('#input-modal-value') as HTMLInputElement;
    input.focus();
    input.select();

    const cleanUp = () => {
      document.body.removeChild(overlay);
    };

    overlay.querySelector('#input-modal-close-x')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });

    overlay.querySelector('#input-modal-cancel')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });

    const handleConfirm = () => {
      const val = input.value.trim();
      cleanUp();
      resolve(val ? val : null);
    };

    overlay.querySelector('#input-modal-confirm')!.addEventListener('click', handleConfirm);

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        handleConfirm();
      }
      if (e.key === 'Escape') {
        cleanUp();
        resolve(null);
      }
    });
  });
}

export async function handleProfileChange(profileId: string): Promise<void> {
  const { currentProfileId } = getState();
  if (profileId === currentProfileId) return;
  try {
    // 1. Confirm discard/save for editor changes first
    const { confirmDiscardOrSave, clearOriginalContent } = await import('./editorView');
    const proceed = await confirmDiscardOrSave();
    if (!proceed) return; // User cancelled, abort switch

    const mods = await switchProfile(profileId);
    
    // Clear editor original content cache
    clearOriginalContent();

    // Immediately update state, clear editor, and render for instant feedback
    updateState({
      allMods: mods,
      currentProfileId: profileId,
      editorModId: null,
      editorFiles: [],
      editorSelectedFile: null,
      currentFolderId: null
    });
    renderModsView();

    // Reset editor UI components
    const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
    if (editorContent) {
      editorContent.value = '';
      editorContent.disabled = true;
    }
    const editorPath = document.getElementById('editor-file-path');
    if (editorPath) editorPath.textContent = '';
    const nameEl = document.getElementById('editor-current-mod-name');
    if (nameEl) nameEl.textContent = '';
    const highlightCode = document.getElementById('editor-highlight-code');
    if (highlightCode) highlightCode.innerHTML = '';
    const fileTreeEl = document.getElementById('editor-file-tree');
    if (fileTreeEl) fileTreeEl.innerHTML = '<div class="editor-file-empty">No files loaded</div>';

    const { populateEditorModSelect, renderEditorModTree } = await import('./editorView');
    populateEditorModSelect();
    renderEditorModTree();

    showToast('Profile switched', 'success');
    // Full refresh for complete reactivity
    await Promise.all([loadProfiles(), loadDependencies()]);

    renderModsView();

  } catch (e) {
    showToast('Failed to switch profile: ' + e, 'error');
  }
}



export async function handleCreateProfile(name: string): Promise<void> {
  try {
    const { createProfile } = await import('../api');
    const newProfile = await createProfile(name);
    showToast('Profile created', 'success');
    await loadProfiles();
    if (newProfile && newProfile.id) {
      await handleProfileChange(newProfile.id);
    }
  } catch (e) {
    showToast('Failed to create profile: ' + e, 'error');
  }
}

// === LIBRARY ===

let _librarySearchQuery = '';

export function setupLibraryHandlers(): void {
  const searchInput = document.getElementById('library-search-input') as HTMLInputElement | null;
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      _librarySearchQuery = searchInput.value.trim().toLowerCase();
      renderLibraryView();
    });
  }

  document.getElementById('library-bulk-install-btn')?.addEventListener('click', handleLibraryBulkInstall);
  document.getElementById('library-bulk-remove-btn')?.addEventListener('click', handleLibraryBulkRemove);
  document.getElementById('library-bulk-clear-btn')?.addEventListener('click', () => {
    updateState({ selectedLibraryIds: new Set() });
    updateLibraryBulkBar();
    renderLibraryView();
  });
}

export async function loadLibrary(): Promise<void> {
  try {
    const entries = await getLibrary();
    updateState({ libraryEntries: entries });
    renderLibraryView();
  } catch (e) {
    console.error('Failed to load library:', e);
  }
}

function parseModFilename(filename: string): { name: string; version: string | null; nexusId: number | null } {
  const stem = filename.replace(/\.(zip|rar)$/i, '');
  const parts = stem.split(/[ _()]/).filter(s => s);

  const idIdx = parts.findIndex(p => {
    const num = parseInt(p, 10);
    return !isNaN(num) && num >= 100 && num <= 99999 && num !== 2026 && num !== 2025 && num !== 2024;
  });

  if (idIdx >= 0) {
    const name = parts.slice(0, idIdx).join(' ');
    let version: string | null = null;
    const nexusId = parseInt(parts[idIdx], 10);
    if (idIdx + 1 < parts.length) {
      const next = parts[idIdx + 1];
      if (/^[v\d]/.test(next) && !next.includes('-')) {
        version = next;
      }
    }
    return { name: name || stem, version, nexusId: isNaN(nexusId) ? null : nexusId };
  }
  return { name: stem, version: null, nexusId: null };
}


function renderLibraryView(): void {
  const container = document.getElementById('library-container');
  if (!container) return;

  let entries = getState().libraryEntries;
  const state = getState();

  // Filter out vital dependencies (palschema, ue4ss, and version configs)
  const banned = ["palschema", "ue4ss", "palschema.version", "ue4ss.version"];
  entries = entries.filter(e => {
    const name_lower = e.zipName.toLowerCase();
    return !banned.some(b => name_lower === b || name_lower.startsWith(b + ".") || name_lower.startsWith(b + "-") || name_lower.startsWith(b + "_"));
  });

  if (_librarySearchQuery) {
    entries = entries.filter(e => e.zipName.toLowerCase().includes(_librarySearchQuery));
  }

  if (entries.length === 0) {
    container.innerHTML = '<div id="library-empty">No mods in library. Mods are automatically copied here when installed.</div>';
    return;
  }

  container.innerHTML = entries.map(e => {
    const isSelected = state.selectedLibraryIds.has(e.modId);
    const parsed = parseModFilename(e.zipName);
    const cleanName = parsed.name || e.zipName;
    const versionStr = parsed.version ? `v${parsed.version}` : '';

    let imageHtml = `<div style="font-size:32px;text-align:center;color:var(--text-muted);opacity:0.8;margin:8px 0;">📦</div>`;
    if (e.nexusPictureUrl) {
      imageHtml = `
        <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
          <img src="${e.nexusPictureUrl}" style="width:100%;height:100%;object-fit:cover;" />
        </div>
      `;
    } else {
      const matchedMod = state.allMods.find(m => {
        if (m.name.toLowerCase() === e.modId.toLowerCase()) return true;
        if (m.nexusModId && parsed.nexusId && m.nexusModId === parsed.nexusId) return true;
        if (m.name.toLowerCase() === cleanName.toLowerCase()) return true;
        return false;
      });
      if (matchedMod && matchedMod.nexusPictureUrl) {
        imageHtml = `
          <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
            <img src="${matchedMod.nexusPictureUrl}" style="width:100%;height:100%;object-fit:cover;" />
          </div>
        `;
      }
    }



    return `
      <div class="mod-card library-card ${isSelected ? 'selected' : ''}" data-id="${e.modId}" style="cursor:pointer;position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
        <div class="card-checkbox-container" style="position:absolute;top:10px;left:10px;z-index:5;">
          <input type="checkbox" class="library-card-checkbox" data-id="${e.modId}" ${isSelected ? 'checked' : ''} style="width:14px;height:14px;cursor:pointer;" />
        </div>
        <div style="padding-top:14px;display:flex;flex-direction:column;gap:8px;height:100%;justify-content:space-between;min-height:160px;">
          ${imageHtml}
          <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:center;word-break:break-word;line-height:1.3;flex:1;min-height:36px;display:flex;align-items:center;justify-content:center;margin-top:4px;">
            ${escapeHtml(cleanName)}
          </div>
          <div style="display:flex;justify-content:space-between;align-items:center;font-size:10px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:6px;margin-top:auto;">
            <span>${versionStr}</span>
            <span>${formatSize(e.zipSize)}</span>
          </div>
          <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
            <button class="library-item-install btn-action" data-id="${e.modId}" style="flex:1;padding:4px;font-size:10px;cursor:pointer;">Install</button>
            <button class="library-item-delete btn-action btn-action-danger" data-id="${e.modId}" style="padding:4px 8px;font-size:10px;cursor:pointer;">✕</button>
          </div>
        </div>
      </div>
    `;
  }).join('');

  container.querySelectorAll('.library-item-install').forEach(btn => {
    btn.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;

      // We will handle profile check before installing
      await triggerInstallFromLibrary(id);
    });
  });

  container.querySelectorAll('.library-item-delete').forEach(btn => {
    btn.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      if (confirm('Are you sure you want to remove this mod from your library?')) {
        try {
          await removeFromLibrary(id);
          showToast('Mod removed from library', 'success');
          await loadLibrary();
        } catch (err) {
          showToast('Failed to remove: ' + err, 'error');
        }
      }
    });
  });

  updateLibraryBulkBar();
}

function askTargetProfile(): Promise<string | null> {
  return new Promise((resolve) => {
    const state = getState();
    if (state.profiles.length <= 1) {
      resolve(state.currentProfileId);
      return;
    }

    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '2000';
    overlay.style.position = 'fixed';
    overlay.style.top = '0';
    overlay.style.left = '0';
    overlay.style.right = '0';
    overlay.style.bottom = '0';
    overlay.style.background = 'rgba(0,0,0,0.6)';
    overlay.style.backdropFilter = 'blur(4px)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';

    const options = state.profiles.map(p => `
      <option value="${p.id}" ${p.id === state.currentProfileId ? 'selected' : ''}>${escapeHtml(p.name)}</option>
    `).join('');

    overlay.innerHTML = `
      <div class="modal" style="width: 400px; max-width: 90vw;">
        <div class="modal-header">
          <h3>Select Target Profile</h3>
          <button class="modal-close-btn" id="profile-select-close-x">✕</button>
        </div>
        <div class="modal-body" style="gap:12px;padding:20px;">
          <div style="font-size:12px;color:var(--text-muted);">
            Multiple profiles detected. Choose where to install the mod(s):
          </div>
          <div class="settings-group" style="display:flex;flex-direction:column;gap:6px;margin-top:8px;">
            <label style="font-size:11px;font-weight:600;color:var(--text-secondary);">Target Profile</label>
            <select id="target-profile-dropdown" style="width:100%;padding:8px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:4px;color:var(--text-primary);font-size:12px;">
              ${options}
            </select>
          </div>
        </div>
        <div class="modal-footer" style="padding:16px 20px;">
          <button id="profile-select-cancel" class="btn-secondary" style="cursor:pointer;">Cancel</button>
          <button id="profile-select-confirm" class="btn-primary" style="cursor:pointer;">Install</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const cleanUp = () => {
      document.body.removeChild(overlay);
    };

    document.getElementById('profile-select-close-x')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });
    document.getElementById('profile-select-cancel')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });
    document.getElementById('profile-select-confirm')!.addEventListener('click', () => {
      const val = (document.getElementById('target-profile-dropdown') as HTMLSelectElement).value;
      cleanUp();
      resolve(val);
    });
  });
}

async function triggerInstallFromLibrary(id: string): Promise<void> {
  try {
    const { getLibraryZipPath, analyzeZip, checkModExistsCommand } = await import('../api');
    const { renderInstallPreview, showInstallModal } = await import('./modal');

    const zipPath = await getLibraryZipPath(id);
    const analysis = await analyzeZip(zipPath);
    const check = await checkModExistsCommand(zipPath);

    const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name } : null;

    renderInstallPreview(analysis, existingMod);
    showInstallModal();
  } catch (err) {
    showToast('Failed to open install preview: ' + err, 'error');
  }
}

async function handleLibraryBulkInstall(): Promise<void> {
  const state = getState();
  const selected = Array.from(state.selectedLibraryIds);
  if (selected.length === 0) return;

  try {
    const { getLibraryZipPath } = await import('../api');
    const { renderBatchInstallPreview, showInstallModal } = await import('./modal');

    const zipPaths: string[] = [];
    for (const id of selected) {
      try {
        const path = await getLibraryZipPath(id);
        zipPaths.push(path);
      } catch { }
    }

    if (zipPaths.length === 1) {
      const { analyzeZip, checkModExistsCommand } = await import('../api');
      const { renderInstallPreview } = await import('./modal');
      const analysis = await analyzeZip(zipPaths[0]);
      const check = await checkModExistsCommand(zipPaths[0]);
      const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name } : null;
      renderInstallPreview(analysis, existingMod);
      showInstallModal();
    } else if (zipPaths.length > 1) {
      await renderBatchInstallPreview(zipPaths);
    }
  } catch (err) {
    showToast('Failed to prepare batch install: ' + err, 'error');
  }
}


async function handleLibraryBulkRemove(): Promise<void> {
  const state = getState();
  const selected = Array.from(state.selectedLibraryIds);
  if (selected.length === 0) return;

  if (confirm(`Are you sure you want to delete ${selected.length} mod(s) from your library?`)) {
    let deleted = 0;
    for (const id of selected) {
      try {
        await removeFromLibrary(id);
        deleted++;
      } catch { }
    }
    showToast(`Deleted ${deleted} mods from library`, 'success');
    updateState({ selectedLibraryIds: new Set() });
    updateLibraryBulkBar();
    await loadLibrary();
  }
}

export function updateLibraryBulkBar(): void {
  const state = getState();
  const bar = document.getElementById('library-bulk-actions-bar');
  const countEl = document.getElementById('library-bulk-selected-count');
  if (!bar || !countEl) return;

  const selectedCount = state.selectedLibraryIds.size;

  if (selectedCount > 0) {
    bar.style.display = 'flex';
    countEl.textContent = selectedCount.toString();
  } else {
    bar.style.display = 'none';
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}


// === DEPENDENCIES (UE4SS / PalSchema) ===

export async function loadDependencies(): Promise<void> {
  try {
    const deps = await checkDependencies();
    // Also fetch remote versions in background
    import('../api').then(({ checkDependenciesFull }) => {
      checkDependenciesFull().then(fullDeps => {
        updateState({ dependencies: fullDeps });
        renderDependencyBadges(fullDeps);
      }).catch(() => { });
    });
    updateState({ dependencies: deps });
    renderDependencyBadges(deps);
  } catch (e) {
    console.error('Failed to check dependencies:', e);
  }
}

function handleDepBadgeClick(type: 'ue4ss' | 'palschema'): void {
  const deps = getState().dependencies;
  if (!deps) return;
  const isInstalled = type === 'ue4ss' ? deps.ue4ss_installed : deps.palschema_installed;
  const needsUpdate = type === 'ue4ss' ? deps.ue4ss_needs_update : deps.palschema_needs_update;

  if (type === 'palschema' && !deps.ue4ss_installed) {
    showConfirm('UE4SS is not detected. PalSchema requires UE4SS to operate. Would you like to install UE4SS first?')
      .then(async (confirmed) => {
        if (!confirmed) {
          showToast('PalSchema installation cancelled: UE4SS dependency missing.', 'info');
          return;
        }
        try {
          showToast('Installing UE4SS from GitHub (Okaetsu/UE4SS-Palworld)...', 'info');
          const ue4ssMsg = await installUe4ss();
          showToast(ue4ssMsg, 'success');
          await loadDependencies();

          const action = isInstalled ? 'Updating' : 'Installing';
          showToast(`${action} PalSchema from GitHub (Okaetsu/PalSchema)...`, 'info');
          const psMsg = await installPalschema();
          showToast(psMsg, 'success');
          await loadDependencies();
          await loadMods();
        } catch (e) {
          showToast('Failed: ' + e, 'error');
        }
      });
    return;
  }

  if (!isInstalled || needsUpdate) {
    const action = isInstalled ? 'Updating' : 'Installing';
    const sourceInfo = type === 'ue4ss' ? 'UE4SS from GitHub (Okaetsu/UE4SS-Palworld)' : 'PalSchema from GitHub (Okaetsu/PalSchema)';
    showToast(`${action} ${sourceInfo}...`, 'info');
    const promise = type === 'ue4ss' ? installUe4ss() : installPalschema();
    promise.then(async (msg) => {
      showToast(msg, 'success');
      await loadProfiles();
      await loadDependencies();
      await loadMods();
    }).catch(e => showToast('Failed: ' + e, 'error'));
  }
}

function renderDependencyBadges(deps: import('../types').DependencyStatus): void {
  const platformEl = document.getElementById('game-platform-badge');
  if (platformEl) {
    if (deps.game_platform && deps.game_platform !== 'Unknown') {
      platformEl.textContent = deps.game_platform;
      platformEl.style.display = '';
      platformEl.className = 'game-platform-badge ' + deps.game_platform.toLowerCase();
    } else {
      platformEl.style.display = 'none';
    }
  }

  const ue4ssEl = document.getElementById('ue4ss-badge');
  const psEl = document.getElementById('palschema-badge');
  if (!ue4ssEl || !psEl) return;

  // UE4SS
  if (deps.ue4ss_installed) {
    // Format version date from DD.MM.YYYY to more readable format
    const formatDMY = (dmy: string): string => {
      const parts = dmy.split('.');
      if (parts.length === 3) {
        const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
        const m = parseInt(parts[1], 10) - 1;
        return `${parseInt(parts[0], 10)} ${months[m] ?? parts[1]} ${parts[2]}`;
      }
      return dmy;
    };
    const verDisplay = deps.ue4ss_version ? ` · ${formatDMY(deps.ue4ss_version)}` : '';
    ue4ssEl.textContent = `UE4SS${verDisplay}`;
    ue4ssEl.className = `dep-badge ${deps.ue4ss_needs_update ? 'warn' : 'ok'}`;
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = deps.ue4ss_needs_update ? 'pointer' : 'default';
    if (deps.ue4ss_needs_update) {
      const latestDisplay = deps.ue4ss_latest_date ? formatDMY(deps.ue4ss_latest_date) : '?';
      ue4ssEl.title = `Update available (${latestDisplay}) — click to update`;
    } else {
      ue4ssEl.title = `UE4SS (experimental-palworld) — Up to date`;
    }
  } else {
    ue4ssEl.textContent = 'UE4SS ✕';
    ue4ssEl.className = 'dep-badge missing';
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = 'pointer';
    ue4ssEl.title = 'Not installed — click to install';
  }

  // PalSchema
  if (deps.palschema_installed) {
    const ver = deps.palschema_version ? ` v${deps.palschema_version}` : deps.palschema_latest_version ? ` v${deps.palschema_latest_version}` : '';
    psEl.textContent = `PalSchema${ver}`;
    psEl.className = `dep-badge ${deps.palschema_needs_update ? 'warn' : 'ok'}`;
    psEl.style.display = '';
    psEl.style.cursor = deps.palschema_needs_update ? 'pointer' : 'default';
    psEl.title = deps.palschema_needs_update ? 'Click to update' : 'Up to date';
  } else {
    psEl.textContent = 'PalSchema ✕';
    psEl.className = 'dep-badge missing';
    psEl.style.display = '';
    psEl.style.cursor = 'pointer';
    psEl.title = 'Not installed — click to install';
  }

  // Attach click handlers
  ue4ssEl.onclick = () => handleDepBadgeClick('ue4ss');
  psEl.onclick = () => handleDepBadgeClick('palschema');
}

// === CONTEXT MENU ===

function runContextAction(action: string, modId: string): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;

  switch (action) {
    case 'check-updates':
      (async () => {
        try {
          showToast(`Checking updates for "${mod.name}"...`, 'info');
          const { refreshNexusCache } = await import('../api');
          const updated = await refreshNexusCache(modId);
          // Update state
          const idx = getState().allMods.findIndex(m => m.id === modId);
          if (idx >= 0) {
            const newMods = [...getState().allMods];
            newMods[idx] = updated;
            updateState({ allMods: newMods });
          }
          const nexusVer = updated.nexusVersionCached;
          const localVer = updated.version;
          const normNexus = (nexusVer || '').replace(/^v/i, '').trim().toLowerCase();
          const normLocal = (localVer || '').replace(/^v/i, '').trim().toLowerCase();
          const isNewUpdate = nexusVer && normNexus !== normLocal && normNexus !== 'unknown' && !normLocal.startsWith(normNexus);
          
          const newMap = new Map(getState().availableUpdates);
          if (isNewUpdate) {
            newMap.set(modId, nexusVer!);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(`Update available: v${nexusVer} (current: v${localVer})`, 'info');
          } else {
            newMap.delete(modId);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(`"${mod.name}" is up to date`, 'success');
          }
        } catch (e) {
          showToast('Failed to check updates: ' + e, 'error');
        }
      })();
      break;
    case 'toggle':
      (async () => {
        try {
          if (mod.enabled) { await disableMod(modId); } else { await enableMod(modId); }
          try { await setModProfileState(modId, !mod.enabled); } catch { }
          showToast(mod.enabled ? 'Mod disabled' : 'Mod enabled', 'success');
          await loadMods();
        } catch (e) { showToast('Failed: ' + e, 'error'); }
      })();
      break;
    case 'open-folder':
      openModFolder(modId).catch(e => showToast('Failed: ' + e, 'error'));
      break;
    case 'open-extras':
      openExtraFolder(modId).catch(e => showToast('Failed: ' + e, 'error'));
      break;
    case 'edit-config':
      openConfigEditor(modId);
      break;
    case 'detail':
      openDetailPanel(modId);
      break;
    case 'visit-nexus':
      openUrl(`https://www.nexusmods.com/palworld/mods/${mod.nexusModId}`);
      break;
    case 'visit-github':
      openUrl(`https://github.com/${mod.githubRepo}`);
      break;
    case 'remove':
      showConfirm(`Remove "${mod.name}" permanently?`).then(confirmed => {
        if (!confirmed) return;
        removeMod(modId).then(() => {
          closeDetailPanel();
          loadMods();
          showToast('Mod removed', 'success');
        }).catch(e => showToast('Failed: ' + e, 'error'));
      });
      break;
  }
}

function getContextOverlay(): HTMLElement {
  return document.getElementById('context-overlay')!;
}

function showContextMenu(modId: string, x: number, y: number): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;

  let html = `
    <button type="button" class="context-menu-item" data-action="check-updates">
      <span class="ctx-icon">&#8634;</span>
      Check for updates
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="toggle">
      <span class="ctx-icon">${mod.enabled ? '◌' : '●'}</span>
      ${mod.enabled ? 'Disable' : 'Enable'}
    </button>
    <button type="button" class="context-menu-item" data-action="open-folder">
      <span class="ctx-icon">📁</span>
      Open folder
    </button>
    ${mod.extraFiles && mod.extraFiles.length > 0 ? `
    <button type="button" class="context-menu-item" data-action="open-extras">
      <span class="ctx-icon">📂</span>
      Open extra folder
    </button>
    ` : ''}
    <button type="button" class="context-menu-item" data-action="edit-config">
      <span class="ctx-icon">⚙</span>
      Edit config
    </button>
    <button type="button" class="context-menu-item" data-action="detail">
      <span class="ctx-icon">ℹ</span>
      View details
    </button>
  `;

  // Folders Submenu
  const currentProfile = getState().profiles.find(p => p.id === getState().currentProfileId);
  const folders = currentProfile?.mod_folders || [];
  const isInFolder = folders.some(f => f.mod_ids.includes(modId));
  const foldersHtml = folders.map(f => `
    <button type="button" class="context-submenu-item" data-action="move-to-folder" data-folder-id="${f.id}" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
      <span>📁</span> ${escapeHtml(f.name)}
    </button>
  `).join('');

  html += `
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      Move to folder...
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && isInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${isInFolder ? `<button type="button" class="context-submenu-item" data-action="move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> Remove from folder
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> New folder...
        </button>
      </div>
    </div>
  `;

  if (mod.nexusModId || mod.githubRepo) {
    html += `<div class="context-menu-sep"></div>`;
    if (mod.nexusModId) {
      html += `<button type="button" class="context-menu-item" data-action="visit-nexus">
        <span class="ctx-icon">N</span>
        Visit on NexusMods
      </button>`;
    }
    if (mod.githubRepo) {
      html += `<button type="button" class="context-menu-item" data-action="visit-github">
        <span class="ctx-icon">G</span>
        Visit on GitHub
      </button>`;
    }
  }

  html += `<div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="remove">
      <span class="ctx-icon">✕</span>
      Remove
    </button>`;

  menu.innerHTML = html;

  // Smart submenu positioning: flip to left when near right edge
  menu.querySelectorAll('.has-submenu').forEach(item => {
    item.addEventListener('mouseenter', () => {
      const sub = item.querySelector('.context-submenu') as HTMLElement | null;
      if (!sub) return;
      const parentRect = item.getBoundingClientRect();
      const subWidth = 180;
      const spaceRight = window.innerWidth - parentRect.right;
      if (spaceRight < subWidth + 12) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
  });

  // Mark this mod card as context-active (cleared when menu closes)
  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
  const modCard = document.querySelector(`.mod-card[data-id="${modId}"]`);
  if (modCard) modCard.classList.add('context-active');

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if ((btn as HTMLElement).classList.contains('has-submenu')) return;
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();
      runContextAction(action, modId);
    });
  });

  menu.querySelectorAll('.context-submenu-item').forEach(subBtn => {
    subBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const subAction = (subBtn as HTMLElement).dataset.action!;
      hideContextMenu();
      
      if (subAction === 'move-to-folder') {
        const folderId = (subBtn as HTMLElement).dataset.folderId!;
        const targetFolder = folderId === 'none' ? null : folderId;
        await handleAddModToFolder(targetFolder, modId);
      } else if (subAction === 'move-to-new-folder') {
        const newName = await showInputModal(
          'New Mod Folder',
          'Enter a name for the new virtual folder:',
          'Skins'
        );
        if (newName === null) return;
        const trimmed = newName.trim();
        if (!trimmed) return;
        
        try {
          const { createModFolder, addModToFolder } = await import('../api');
          const state = getState();
          const updatedProfile = await createModFolder(state.currentProfileId, trimmed);
          const newFolder = updatedProfile.mod_folders?.find(f => f.name.toLowerCase() === trimmed.toLowerCase());
          if (newFolder) {
            await handleAddModToFolder(newFolder.id, modId);
          }
        } catch (err) {
          showToast('Failed: ' + err, 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

function showFolderContextMenu(folderId: string, x: number, y: number): void {
  const state = getState();
  const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);
  const folder = currentProfile?.mod_folders?.find(f => f.id === folderId);
  if (!folder) return;

  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;

  const modsInFolder = (state.allMods || []).filter(m => folder.mod_ids.includes(m.id));
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);

  let html = `
    <button type="button" class="context-menu-item" data-action="enter">
      <span class="ctx-icon">📂</span>
      Enter folder
    </button>
    <button type="button" class="context-menu-item" data-action="rename">
      <span class="ctx-icon">✏</span>
      Rename folder
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="toggle-mods">
      <span class="ctx-icon">${allEnabled ? '◌' : '●'}</span>
      ${allEnabled ? 'Disable all mods' : 'Enable all mods'}
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-mods">
      <span class="ctx-icon">&#8634;</span>
      Check updates for mods
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="delete">
      <span class="ctx-icon">✕</span>
      Delete folder
    </button>
  `;

  menu.innerHTML = html;

  // Mark this folder card as context-active
  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
  const folderCard = document.querySelector(`.mod-card.folder-card[data-id="${folderId}"]`);
  if (folderCard) folderCard.classList.add('context-active');

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'enter') {
        updateState({ currentFolderId: folderId });
        renderModsView();
      } else if (action === 'rename') {
        const renameBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .rename-btn`) as HTMLElement | null;
        renameBtn?.click();
      } else if (action === 'delete') {
        const deleteBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .delete-btn`) as HTMLElement | null;
        deleteBtn?.click();
      } else if (action === 'toggle-mods') {
        try {
          const { toggleFolderMods } = await import('../api');
          const updatedProfile = await toggleFolderMods(state.currentProfileId, folderId, !allEnabled);
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          showToast(!allEnabled ? 'All folder mods enabled' : 'All folder mods disabled', 'success');
          await loadMods();
        } catch (err) {
          showToast('Failed to toggle: ' + err, 'error');
        }
      } else if (action === 'check-updates-mods') {
        if (modsInFolder.length === 0) {
          showToast('No mods in this folder', 'info');
          return;
        }
        showToast('Checking for updates...', 'info');
        try {
          const { checkForUpdates } = await import('../api');
          await checkForUpdates();
          await loadMods();
          showToast('Folder mods update check completed', 'success');
        } catch (err) {
          showToast('Failed to check updates: ' + err, 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

function showGlobalContextMenu(x: number, y: number): void {
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;
  const deps = getState().dependencies;

  const hasUe4ss = deps?.ue4ss_installed;
  const hasPalSchema = deps?.palschema_installed;

  const html = `
    <button type="button" class="context-menu-item" data-action="new-folder">
      <span class="ctx-icon">📁</span>
      New mod folder
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="install">
      <span class="ctx-icon">+</span>
      Install mod (.zip, .rar, .7z)
    </button>
    <button type="button" class="context-menu-item" data-action="rescan">
      <span class="ctx-icon">↻</span>
      Rescan mods
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-global">
      <span class="ctx-icon">↑</span>
      Check for updates
    </button>
    <button type="button" class="context-menu-item" data-action="export-json">
      <span class="ctx-icon">📤</span>
      Export mods JSON
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="check-deps">
      <span class="ctx-icon">&#8634;</span>
      Check UE4SS & PalSchema updates
    </button>
    ${hasUe4ss ? `
      <button type="button" class="context-menu-item danger" data-action="uninstall-ue4ss">
        <span class="ctx-icon">✕</span>
        Uninstall UE4SS
      </button>
    ` : `
      <button type="button" class="context-menu-item" data-action="update-ue4ss">
        <span class="ctx-icon">U</span>
        Install UE4SS
      </button>
    `}
    ${hasPalSchema ? `
      <button type="button" class="context-menu-item danger" data-action="uninstall-palschema">
        <span class="ctx-icon">✕</span>
        Uninstall PalSchema
      </button>
    ` : `
      <button type="button" class="context-menu-item" data-action="update-palschema">
        <span class="ctx-icon">S</span>
        Install PalSchema
      </button>
    `}
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="settings">
      <span class="ctx-icon">⚙</span>
      Settings
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      switch (action) {
        case 'install':
          document.getElementById('install-btn')?.click();
          break;
        case 'rescan':
          document.getElementById('scan-btn')?.click();
          break;
        case 'check-updates-global':
          document.getElementById('check-updates-btn')?.click();
          break;
        case 'export-json':
          document.getElementById('export-json-btn')?.click();
          break;
        case 'new-folder':
          document.getElementById('new-folder-btn')?.click();
          break;
        case 'check-deps':
          showToast('Checking for dependency updates...', 'info');
          loadDependencies().then(() => showToast('Dependency check finished', 'success'));
          break;
        case 'update-ue4ss':
          handleDepBadgeClick('ue4ss');
          break;
        case 'update-palschema':
          handleDepBadgeClick('palschema');
          break;
        case 'uninstall-ue4ss':
          showToast('Uninstalling UE4SS...', 'info');
          uninstallUe4ss().then(msg => {
            showToast(msg, 'success');
            loadDependencies();
            loadMods();
          }).catch(e => showToast('Failed: ' + e, 'error'));
          break;
        case 'uninstall-palschema':
          showToast('Uninstalling PalSchema...', 'info');
          uninstallPalschema().then(msg => {
            showToast(msg, 'success');
            loadDependencies();
            loadMods();
          }).catch(e => showToast('Failed: ' + e, 'error'));
          break;

        case 'settings':
          document.getElementById('settings-btn')?.click();
          break;
      }
    });
  });

  positionContextMenu(x, y);
}

function hideContextMenu(): void {
  const overlay = getContextOverlay();
  overlay.classList.remove('visible');
  const menu = document.getElementById('context-menu')!;
  menu.style.display = 'none';
  menu.innerHTML = '';
  // Clear any mod card that was highlighted by right-click
  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
}

function positionContextMenu(x: number, y: number): void {
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;
  overlay.classList.add('visible');
  menu.style.display = 'block';
  menu.style.visibility = 'hidden';

  requestAnimationFrame(() => {
    const rect = menu.getBoundingClientRect();
    const width = rect.width || 200;
    const height = rect.height || 200;

    const posX = Math.max(10, Math.min(x, window.innerWidth - width - 10));
    const posY = Math.max(10, Math.min(y, window.innerHeight - height - 10));

    menu.style.left = `${posX}px`;
    menu.style.top = `${posY}px`;
    menu.style.visibility = 'visible';
    menu.focus();
  });
}

function showBulkContextMenu(x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  const selectedCount = getState().selectedModIds.size;

  const currentProfile = getState().profiles.find(p => p.id === getState().currentProfileId);
  const folders = currentProfile?.mod_folders || [];
  const selectedIds = Array.from(getState().selectedModIds);
  const anyInFolder = folders.some(f => selectedIds.some(id => f.mod_ids.includes(id)));
  const foldersHtml = folders.map(f => `
    <button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="${f.id}" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
      <span>📁</span> ${escapeHtml(f.name)}
    </button>
  `).join('');

  const html = `
    <div style="font-size:9px;font-weight:700;color:var(--text-muted);padding:6px 16px 2px;text-transform:uppercase">${selectedCount} Mods Selected</div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="bulk-enable">
      <span class="ctx-icon">●</span>
      Enable Selected
    </button>
    <button type="button" class="context-menu-item" data-action="bulk-disable">
      <span class="ctx-icon">◌</span>
      Disable Selected
    </button>
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      Move selected to folder...
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && anyInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${anyInFolder ? `<button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> Remove from folders
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="bulk-move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> New folder...
        </button>
      </div>
    </div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="bulk-remove">
      <span class="ctx-icon">✕</span>
      Remove Selected
    </button>
  `;

  menu.innerHTML = html;

  // Smart submenu positioning: flip to left when near right edge
  menu.querySelectorAll('.has-submenu').forEach(item => {
    item.addEventListener('mouseenter', () => {
      const sub = item.querySelector('.context-submenu') as HTMLElement | null;
      if (!sub) return;
      const parentRect = item.getBoundingClientRect();
      const subWidth = 180;
      const spaceRight = window.innerWidth - parentRect.right;
      if (spaceRight < subWidth + 12) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
  });

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if ((btn as HTMLElement).classList.contains('has-submenu')) return;
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'bulk-enable') {
        document.getElementById('bulk-enable-btn')?.click();
      } else if (action === 'bulk-disable') {
        document.getElementById('bulk-disable-btn')?.click();
      } else if (action === 'bulk-remove') {
        document.getElementById('bulk-remove-btn')?.click();
      }
    });
  });

  menu.querySelectorAll('.context-submenu-item').forEach(subBtn => {
    subBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const subAction = (subBtn as HTMLElement).dataset.action!;
      hideContextMenu();

      const modIds = Array.from(getState().selectedModIds);
      if (modIds.length === 0) return;

      if (subAction === 'bulk-move-to-folder') {
        const folderId = (subBtn as HTMLElement).dataset.folderId!;
        const targetFolder = folderId === 'none' ? null : folderId;
        showToast(`Grouping ${modIds.length} mods...`, 'info');
        try {
          const { addModToFolder } = await import('../api');
          const state = getState();
          let lastProfile = null;
          for (const modId of modIds) {
            lastProfile = await addModToFolder(state.currentProfileId, targetFolder, modId);
          }
          if (lastProfile) {
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
            updateState({ profiles });
          }
          await loadMods();
          showToast(`Moved ${modIds.length} mods successfully`, 'success');
        } catch (err) {
          showToast('Failed to group mods: ' + err, 'error');
        }
      } else if (subAction === 'bulk-move-to-new-folder') {
        const newName = await showInputModal(
          'New Mod Folder',
          'Enter a name for the new virtual folder:',
          'Skins'
        );
        if (newName === null) return;
        const trimmed = newName.trim();
        if (!trimmed) return;
        
        try {
          const { createModFolder, addModToFolder } = await import('../api');
          const state = getState();
          const updatedProfile = await createModFolder(state.currentProfileId, trimmed);
          const newFolder = updatedProfile.mod_folders?.find(f => f.name.toLowerCase() === trimmed.toLowerCase());
          if (newFolder) {
            let lastProfile = null;
            for (const modId of modIds) {
              lastProfile = await addModToFolder(state.currentProfileId, newFolder.id, modId);
            }
            if (lastProfile) {
              const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
              updateState({ profiles });
            }
            await loadMods();
            showToast(`Created folder and grouped ${modIds.length} mods`, 'success');
          }
        } catch (err) {
          showToast('Failed: ' + err, 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

function showEditorContextMenu(x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;

  menu.innerHTML = `
    <button type="button" class="context-menu-item" id="editor-ctx-save" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">💾</span> Save <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+S</span>
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" id="editor-ctx-cut" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">✂</span> Cut <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+X</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-copy" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📋</span> Copy <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+C</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-paste" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📥</span> Paste <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+V</span>
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" id="editor-ctx-find" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">🔍</span> Find <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+F</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-selectall" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">✓</span> Select All <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+A</span>
    </button>
  `;

  positionContextMenu(x, y);

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;

  document.getElementById('editor-ctx-save')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleEditorSave } = await import('./editorView');
    await handleEditorSave();
  });

  document.getElementById('editor-ctx-cut')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    const start = editorContent.selectionStart;
    const end = editorContent.selectionEnd;
    const selectedText = editorContent.value.substring(start, end);
    if (selectedText) {
      await navigator.clipboard.writeText(selectedText);
      editorContent.value = editorContent.value.substring(0, start) + editorContent.value.substring(end);
      editorContent.selectionStart = editorContent.selectionEnd = start;
      editorContent.dispatchEvent(new Event('input'));
    }
  });

  document.getElementById('editor-ctx-copy')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    const selectedText = editorContent.value.substring(editorContent.selectionStart, editorContent.selectionEnd);
    if (selectedText) {
      await navigator.clipboard.writeText(selectedText);
    }
  });

  document.getElementById('editor-ctx-paste')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    try {
      const text = await navigator.clipboard.readText();
      const start = editorContent.selectionStart;
      const end = editorContent.selectionEnd;
      editorContent.value = editorContent.value.substring(0, start) + text + editorContent.value.substring(end);
      editorContent.selectionStart = editorContent.selectionEnd = start + text.length;
      editorContent.dispatchEvent(new Event('input'));
    } catch (err) {
      showToast('Clipboard access denied. Use Ctrl+V.', 'info');
    }
  });

  document.getElementById('editor-ctx-find')!.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    const ev = new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true });
    document.dispatchEvent(ev);
  });

  document.getElementById('editor-ctx-selectall')!.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (editorContent) {
      editorContent.focus();
      editorContent.select();
    }
  });
}

function showLibraryContextMenu(modId: string | null, x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  const state = getState();
  const selectedCount = state.selectedLibraryIds.size;

  if (selectedCount > 1) {
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-install" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📥</span> Install Selected (${selectedCount})
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> Remove Selected from Library
      </button>
    `;
  } else if (modId) {
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-install" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📥</span> Install Mod
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> Remove from Library
      </button>
    `;
  } else {
    hideContextMenu();
    return;
  }

  positionContextMenu(x, y);

  document.getElementById('lib-ctx-install')?.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (selectedCount > 1) {
      handleLibraryBulkInstall();
    } else if (modId) {
      triggerInstallFromLibrary(modId);
    }
  });

  document.getElementById('lib-ctx-remove')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (selectedCount > 1) {
      if (confirm(`Remove ${selectedCount} mods from library?`)) {
        for (const id of Array.from(state.selectedLibraryIds)) {
          await removeFromLibrary(id).catch(() => { });
        }
        showToast('Mods removed from library', 'success');
        updateState({ selectedLibraryIds: new Set() });
        updateLibraryBulkBar();
        loadLibrary();
      }
    } else if (modId) {
      if (confirm('Remove this mod from library?')) {
        await removeFromLibrary(modId).catch(() => { });
        showToast('Mod removed from library', 'success');
        loadLibrary();
      }
    }
  });
}

function isAnyModalActive(): boolean {
  const visibleOverlay = document.querySelector('.modal-overlay.visible, .detail-overlay.visible, .confirm-overlay, .detail-overlay[style*="display: flex"]');
  return !!visibleOverlay;
}

export function setupContextMenu(): void {
  document.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement;

    // Suppress context menu if any modal is active or if clicked on non-interactive bars/sidebar
    if (isAnyModalActive() || target.closest('.modal-overlay, .detail-overlay, .confirm-overlay, .modal, #sidebar, #toolbar, #quick-filter-bar, #search-wrap, #sort-bar, #editor-toolbar, #library-toolbar')) {
      e.preventDefault();
      hideContextMenu();
      return;
    }

    // Editor View: ONLY allow context menu if right-click is inside the editor content area (#editor-content-area)
    if (target.closest('#editor-view')) {
      e.preventDefault();
      hideContextMenu();
      if (target.closest('#editor-content-area')) {
        showEditorContextMenu(e.clientX, e.clientY);
      }
      return;
    }

    // Library View
    const isLibraryView = target.closest('#library-view');
    const libCard = target.closest('.library-card') as HTMLElement | null;

    if (isLibraryView || libCard) {
      e.preventDefault();
      hideContextMenu();
      const id = libCard?.dataset.id || null;
      if (id) {
        showLibraryContextMenu(id, e.clientX, e.clientY);
      }
      return;
    }

    // Mods View
    e.preventDefault();
    hideContextMenu();

    const state = getState();
    const card = target.closest('.mod-card') as HTMLElement | null;

    if (state.selectedModIds.size > 1) {
      showBulkContextMenu(e.clientX, e.clientY);
    } else if (card && card.dataset.id) {
      const id = card.dataset.id;
      const type = card.dataset.type;
      if (type === 'folder') {
        showFolderContextMenu(id, e.clientX, e.clientY);
      } else {
        import('../features/selection').then(({ updateSelection }) => {
          updateSelection(new Set([id]));
          showContextMenu(id, e.clientX, e.clientY);
        });
      }
    } else if (target.closest('#mods-container')) {
      showGlobalContextMenu(e.clientX, e.clientY);
    }
  });


  // Click on overlay backdrop closes the menu
  document.getElementById('context-overlay')!.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) {
      hideContextMenu();
    }
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hideContextMenu();
  });
}

function attachFolderEvents(container: HTMLElement): void {
  // Back to Root Button Click
  const backBtn = container.querySelector('#btn-back-to-root');
  if (backBtn) {
    backBtn.addEventListener('click', () => {
      updateState({ currentFolderId: null });
      renderModsView();
    });
  }

  // Folder Toggle Input (enable/disable all mods in folder)
  container.querySelectorAll('.folder-toggle-input').forEach(input => {
    input.addEventListener('change', async (e) => {
      e.stopPropagation();
      const target = e.currentTarget as HTMLInputElement;
      const folderId = target.dataset.folderId!;
      const enabled = target.checked;
      
      try {
        const { toggleFolderMods } = await import('../api');
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

  // Rename Folder
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
        const { renameModFolder } = await import('../api');
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

  // Delete Folder
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
          const { deleteModFolder } = await import('../api');
          const updatedProfile = await deleteModFolder(state.currentProfileId, folderId);
          
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          
          showToast('Folder deleted', 'success');
          // If deleted folder was active, clear navigation
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

  // HTML5 Drag and Drop Event Listeners
  container.querySelectorAll('.mod-card').forEach(card => {
    card.addEventListener('dragstart', (e: Event) => {
      e.stopPropagation();
      const de = e as DragEvent;
      const modId = (card as HTMLElement).dataset.id!;
      const state = getState();
      if (de.dataTransfer) {
        de.dataTransfer.effectAllowed = 'move';
      }
      if (state.selectedModIds.has(modId)) {
        const ids = Array.from(state.selectedModIds).join(',');
        de.dataTransfer?.setData('text', ids);
      } else {
        de.dataTransfer?.setData('text', modId);
      }
      card.classList.add('dragging');
    });

    card.addEventListener('dragend', (e: Event) => {
      e.stopPropagation();
      card.classList.remove('dragging');
    });
  });

  // Drop on Folder Cards
  container.querySelectorAll('.mod-card.folder-card').forEach(folderCard => {
    folderCard.addEventListener('dragenter', (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      folderCard.classList.add('drag-over');
    });

    folderCard.addEventListener('dragover', (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      const de = e as DragEvent;
      if (de.dataTransfer) {
        de.dataTransfer.dropEffect = 'move';
      }
      folderCard.classList.add('drag-over');
    });

    folderCard.addEventListener('dragleave', (e: Event) => {
      e.stopPropagation();
      folderCard.classList.remove('drag-over');
    });

    folderCard.addEventListener('drop', async (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      folderCard.classList.remove('drag-over');
      const de = e as DragEvent;
      const data = de.dataTransfer?.getData('text');
      const folderId = (folderCard as HTMLElement).dataset.id!;
      if (data) {
        const modIds = data.split(',');
        try {
          const { addModToFolder } = await import('../api');
          const state = getState();
          let lastProfile = null;
          for (const id of modIds) {
            // Avoid adding folders to other folders
            const isFolder = container.querySelector(`.mod-card[data-id="${id}"]`)?.getAttribute('data-type') === 'folder';
            if (isFolder || id === folderId) continue;
            lastProfile = await addModToFolder(state.currentProfileId, folderId, id);
          }
          if (lastProfile) {
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
            updateState({ profiles });
          }
          await loadMods();
          showToast(`Moved ${modIds.length} mod(s) to folder`, 'success');
        } catch (err) {
          showToast('Failed to move: ' + err, 'error');
        }
      }
    });
  });

  const rootZone = container.querySelector('#mod-root-drop-zone');
  if (rootZone) {
    rootZone.addEventListener('dragenter', (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      rootZone.classList.add('drag-over');
    });

    rootZone.addEventListener('dragover', (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      const de = e as DragEvent;
      if (de.dataTransfer) {
        de.dataTransfer.dropEffect = 'move';
      }
      rootZone.classList.add('drag-over');
    });

    rootZone.addEventListener('dragleave', (e: Event) => {
      e.stopPropagation();
      rootZone.classList.remove('drag-over');
    });

    rootZone.addEventListener('drop', async (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      rootZone.classList.remove('drag-over');
      const de = e as DragEvent;
      const data = de.dataTransfer?.getData('text');
      if (data) {
        const modIds = data.split(',');
        try {
          const { addModToFolder } = await import('../api');
          const state = getState();
          let lastProfile = null;
          for (const id of modIds) {
            // Avoid adding folders to other folders
            const isFolder = container.querySelector(`.mod-card[data-id="${id}"]`)?.getAttribute('data-type') === 'folder';
            if (isFolder) continue;
            lastProfile = await addModToFolder(state.currentProfileId, null, id);
          }
          if (lastProfile) {
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
            updateState({ profiles });
          }
          await loadMods();
          showToast(`Ungrouped ${modIds.length} mod(s)`, 'success');
        } catch (err) {
          showToast('Failed to ungroup: ' + err, 'error');
        }
      }
    });
  }

  // Allow dragenter and dragover on the entire container to prevent the blocked cursor
  container.addEventListener('dragenter', (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
  });

  container.addEventListener('dragover', (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
    const de = e as DragEvent;
    if (de.dataTransfer) {
      de.dataTransfer.dropEffect = 'move';
    }
  });
}

async function handleAddModToFolder(folderId: string | null, modId: string): Promise<void> {
  const { currentProfileId } = getState();
  try {
    const { addModToFolder } = await import('../api');
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
    const { createModFolder } = await import('../api');
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
