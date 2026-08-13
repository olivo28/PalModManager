import { getState, updateState } from '../../state';
import { buildModCardHtml, buildFolderCardHtml } from './card';
import { escapeHtml } from '../../utils/helpers';
import type { ModInfo } from '../../types';

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

export function renderModsView(): void {
  const state = getState();
  const container = document.getElementById('mods-container');
  if (!container) return;

  const openAllBtn = document.getElementById('open-all-updates-btn');
  if (openAllBtn) {
    const hasNexusUpdates = Array.from(state.availableUpdates.keys()).some(id => {
      const m = state.allMods.find(mod => mod.id === id);
      return m && m.nexusModId;
    });
    openAllBtn.style.display = hasNexusUpdates ? 'inline-block' : 'none';
  }

  const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);

  if (state.viewLayout === 'list') {
    container.classList.add('list-view');
  } else {
    container.classList.remove('list-view');
  }

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

  if (state.viewLayout === 'list') {
    const sortField = state.currentSort.field;
    const isAsc = state.currentSort.asc;
    const arrow = (field: string) => {
      if (sortField === field) {
        return isAsc ? ' ▲' : ' ▼';
      }
      return '';
    };

    const headerHtml = `
    <div class="list-header-row" style="grid-column: 1 / -1;">
      <div class="list-header-col sortable name-col" data-sort="name">Name${arrow('name')}</div>
      <div class="list-header-col sortable status-col" data-sort="status">Status${arrow('status')}</div>
      <div class="list-header-col sortable type-col" data-sort="type">Type${arrow('type')}</div>
      <div class="list-header-col version-col">Version</div>
      <div class="list-header-col path-col">Installed Path</div>
      <div class="list-header-col extra-col">Extra Files</div>
      <div class="list-header-col sortable date-col" data-sort="date">Date Installed${arrow('date')}</div>
      <div class="list-header-col action-col">Action</div>
    </div>
    `;

    if (state.searchQuery) {
      html += headerHtml;
      html += filtered.map(m => buildModCardHtml(m, state, false)).join('');
    } else {
      const renderFolderCards = folders.map(f => {
        const modsInFolder = folderModsMap.get(f.id) || [];
        return buildFolderCardHtml(f, modsInFolder, state);
      }).join('');

      if (renderFolderCards || ungroupedMods.length > 0) {
        html += headerHtml;
      }
      html += renderFolderCards;
      html += ungroupedMods.map(m => buildModCardHtml(m, state, false)).join('');
    }
  } else {
    if (state.currentFolderId) {
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
      if (state.searchQuery) {
        html += filtered.map(m => buildModCardHtml(m, state)).join('');
      } else {
        const renderFolderCards = folders.map(f => {
          const modsInFolder = folderModsMap.get(f.id) || [];
          return buildFolderCardHtml(f, modsInFolder, state);
        }).join('');

        html += renderFolderCards;
        html += ungroupedMods.map(m => buildModCardHtml(m, state)).join('');
      }
    }
  }

  container.innerHTML = html;

  if (state.viewLayout === 'list') {
    container.querySelectorAll('.list-header-col.sortable').forEach(col => {
      col.addEventListener('click', () => {
        const sortField = (col as HTMLElement).dataset.sort!;
        const currentSort = getState().currentSort;
        let asc = true;
        if (currentSort.field === sortField) {
          asc = !currentSort.asc;
        }
        updateState({ currentSort: { field: sortField, asc } });
        renderModsView();
      });
    });
  }

  // Import event binders dynamically or call them from events.ts later
  import('./events').then(({ attachCardEvents, attachFolderEvents, setupCardDragToFolder }) => {
    attachCardEvents(container);
    attachFolderEvents(container);
    setupCardDragToFolder(container);
  });
}
