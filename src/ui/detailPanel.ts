import { disableMod, enableMod, removeMod, refreshNexusCache, setModConfig, setNexusModId, openModFolder, readConfig, renameMod, setModVersion, checkGitHubVersion, setGithubVersion, openUrl, changePakDestination, ignoreModVersion } from '../api';
import { getState, updateState } from '../state';
import { convertFileSrc } from '@tauri-apps/api/core';
import { openConfigEditor } from './editorView';
import { loadMods, renderModsView } from './modsView';
import { showToast } from './toast';
import { showConfirm } from './confirm';
import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';
import { descriptionToHtml } from '../utils/bbcode';
import type { ModInfo } from '../types';

export interface ModComponentFolder {
  type: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other';
  label: string;
  buttonLabel: string;
  path: string;
}

export function formatDisplayPath(fullPath: string): string {
  if (!fullPath) return '';
  const normalized = fullPath.replace(/\//g, '\\');
  const lower = normalized.toLowerCase();
  const palIdx = lower.lastIndexOf('\\pal\\');
  if (palIdx !== -1) {
    return normalized.substring(palIdx + 1);
  }
  if (lower.startsWith('pal\\')) {
    return normalized;
  }
  return normalized;
}

export function getModComponentFolders(mod: ModInfo): ModComponentFolder[] {
  const allPaths: string[] = [];
  const primaryPath = mod.enabled ? mod.gamePath : mod.disabledPath;
  if (primaryPath) allPaths.push(primaryPath);
  if (mod.extraFiles && Array.isArray(mod.extraFiles)) {
    for (const f of mod.extraFiles) {
      if (f && !allPaths.includes(f)) {
        allPaths.push(f);
      }
    }
  }

  const components: ModComponentFolder[] = [];

  for (const p of allPaths) {
    const lower = p.toLowerCase().replace(/\\/g, '/');
    let compType: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other' = 'other';
    let label = t('detail.comp_other_folder');
    let buttonLabel = t('detail.btn_open_folder');

    if (lower.includes('palschema/mods') || (lower.includes('palschema') && !lower.endsWith('.pak'))) {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.comp_palschema_folder');
    } else if (lower.includes('ue4ss/mods') || (lower.includes('ue4ss') && !lower.endsWith('.pak'))) {
      compType = 'ue4ss';
      label = t('detail.comp_ue4ss_folder');
      buttonLabel = t('detail.comp_ue4ss_folder');
    } else if (lower.includes('logicmods') || (lower.endsWith('.pak') && lower.includes('logicmods'))) {
      compType = 'logicmods';
      label = t('detail.comp_logicmods_folder');
      buttonLabel = t('detail.comp_logicmods_folder');
    } else if (lower.endsWith('.pak') || lower.includes('content/paks') || lower.includes('~mods')) {
      compType = 'pak';
      label = t('detail.comp_pak_folder');
      buttonLabel = t('detail.comp_pak_folder');
    } else if (mod.type === 'ue4ss') {
      compType = 'ue4ss';
      label = t('detail.comp_ue4ss_folder');
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'palschema') {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'pak') {
      compType = 'pak';
      label = t('detail.comp_pak_folder');
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'logicmods') {
      compType = 'logicmods';
      label = t('detail.comp_logicmods_folder');
      buttonLabel = t('detail.btn_open_folder');
    }

    if (!components.some(c => c.path === p)) {
      components.push({ type: compType, label, buttonLabel, path: p });
    }
  }

  const typeOrder: Record<string, number> = {
    ue4ss: 1,
    palschema: 2,
    pak: 3,
    logicmods: 4,
    other: 5,
  };

  components.sort((a, b) => (typeOrder[a.type] ?? 99) - (typeOrder[b.type] ?? 99));

  if (components.length === 1) {
    components[0].buttonLabel = t('detail.btn_open_folder');
  }

  return components;
}

export function openDetailPanel(modId: string): void {
  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);
  if (!mod) return;
  updateState({ currentDetailMod: mod });

  const panel = document.getElementById('detail-panel')!;
  panel.dataset.id = modId;

  document.getElementById('detail-name-header')!.textContent = mod.name;
  const typeLabel = mod.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : mod.type.toUpperCase();
  document.getElementById('detail-type')!.textContent = typeLabel;
  document.getElementById('detail-type')!.className = `mod-type-badge ${mod.type}`;
  renderVersion(mod);
  document.getElementById('detail-status')!.textContent = mod.enabled ? t('common.enabled') : t('common.disabled');
  document.getElementById('detail-status')!.className = `detail-status ${mod.enabled ? 'enabled' : 'disabled'}`;

  const toggleBtn = document.getElementById('detail-toggle')! as HTMLButtonElement;
  toggleBtn.textContent = mod.enabled ? t('common.disabled') : t('common.enabled');
  toggleBtn.dataset.enabled = String(mod.enabled);

  const nexusSection = document.getElementById('detail-nexus')!;
  const descSection = document.getElementById('detail-description')!;
  const imgEl = document.getElementById('detail-image')! as HTMLImageElement;
  const imgContainer = document.getElementById('detail-image-container')!;

  if (mod.nexusModId) {
    const hasNexusInfo = mod.nexusAuthor || mod.nexusDescription || mod.nexusEndorsements !== null;
    if (hasNexusInfo) {
      const tagsHtml = mod.nexusTags && mod.nexusTags.length > 0
        ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.tags_label'))}</span> <span class="detail-tags-list">${mod.nexusTags.map(t => `<span class="detail-tag-chip">${escapeHtml(t)}</span>`).join('')}</span></div>`
        : '';
      const catHtml = mod.nexusCategory
        ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.category_label'))}</span> ${escapeHtml(mod.nexusCategory)}</div>`
        : '';
      nexusSection.innerHTML = `
        <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.nexus_label'))}</span> <span class="detail-nexus-id-row"><a class="nexus-link" href="https://www.nexusmods.com/palworld/mods/${mod.nexusModId}" target="_blank">#${mod.nexusModId}</a> <button class="btn-tiny nexus-id-edit-btn">${escapeHtml(t('detail.nexus_edit'))}</button></span></div>
        <div class="detail-row detail-nexus-edit-row" style="display:none"><span class="detail-label"></span> <span><input type="text" class="nexus-id-input" value="${mod.nexusModId}" /><button class="btn-tiny nexus-id-save-btn" style="margin-left:4px">${escapeHtml(t('detail.nexus_save'))}</button><button class="btn-tiny nexus-id-cancel-btn">${escapeHtml(t('detail.nexus_cancel'))}</button></span></div>
        ${mod.nexusAuthor ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.author_label'))}</span> ${escapeHtml(mod.nexusAuthor)}</div>` : ''}
        ${catHtml}
        ${mod.nexusEndorsements !== null ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.endorsements_label'))}</span> ${mod.nexusEndorsements.toLocaleString()}</div>` : ''}
        ${mod.nexusCachedAt ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.last_updated_label'))}</span> ${new Date(mod.nexusCachedAt).toLocaleDateString()}</div>` : ''}
        ${tagsHtml}
      `;
      nexusSection.style.display = 'block';
      setupNexusIdEdit(mod.id);
      if (!mod.nexusDescription) {
        autoFetchNexusInfo(mod);
      }
    } else {
      nexusSection.innerHTML = `
        <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.nexus_label'))}</span> <span class="detail-nexus-id-row"><a class="nexus-link" href="https://www.nexusmods.com/palworld/mods/${mod.nexusModId}" target="_blank">#${mod.nexusModId}</a> <button class="btn-tiny nexus-id-edit-btn">${escapeHtml(t('detail.nexus_edit'))}</button></span></div>
        <div class="detail-row detail-nexus-edit-row" style="display:none"><span class="detail-label"></span> <span><input type="text" class="nexus-id-input" value="${mod.nexusModId}" /><button class="btn-tiny nexus-id-save-btn" style="margin-left:4px">${escapeHtml(t('detail.nexus_save'))}</button><button class="btn-tiny nexus-id-cancel-btn">${escapeHtml(t('detail.nexus_cancel'))}</button></span></div>
        <div class="detail-row"><span class="detail-label"></span> <span class="nexus-fetching">${escapeHtml(t('detail.nexus_fetching'))}</span></div>`;
      nexusSection.style.display = 'block';
      setupNexusIdEdit(mod.id);
      autoFetchNexusInfo(mod);
    }
  } else {
    nexusSection.innerHTML = `
      <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.nexus_label'))}</span>
        <span><input type="text" class="nexus-id-input" placeholder="${escapeHtml(t('detail.nexus_placeholder'))}" /><button class="btn-tiny nexus-id-add-btn" style="margin-left:4px">${escapeHtml(t('detail.nexus_save'))}</button></span>
      </div>`;
    nexusSection.style.display = 'block';
    const input = nexusSection.querySelector('.nexus-id-input') as HTMLInputElement;
    const addBtn = nexusSection.querySelector('.nexus-id-add-btn') as HTMLButtonElement;
    const doSave = async () => {
      const val = input.value.trim();
      if (!val) return;
      addBtn.disabled = true;
      try {
        await setNexusModId(mod.id, parseInt(val));
        await loadMods();
        openDetailPanel(mod.id);
        renderModsView();
        showToast(t('toasts.nexus_id_updated'), 'success');
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      } finally {
        addBtn.disabled = false;
      }
    };
    addBtn.addEventListener('click', doSave);
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') doSave(); });
  }

  if (mod.nexusPictureUrl) {
    let resolvedSrc = mod.nexusPictureUrl;
    if (!resolvedSrc.startsWith('http://') && !resolvedSrc.startsWith('https://')) {
      try {
        resolvedSrc = convertFileSrc(resolvedSrc);
      } catch (e) {
        console.error('Failed to convert file src:', e);
      }
    }
    imgEl.src = resolvedSrc;
    imgContainer.style.display = 'block';
  } else {
    imgContainer.style.display = 'none';
  }

  if (mod.nexusDescription) {
    descSection.innerHTML = descriptionToHtml(mod.nexusDescription);
    descSection.style.display = 'block';
  } else if (mod.nexusSummary) {
    descSection.textContent = mod.nexusSummary;
    descSection.style.display = 'block';
  } else {
    descSection.style.display = 'none';
  }

  renderGithubSection(mod);

  let formattedInstallDate = t('common.none');
  if (mod.installDate && mod.installDate !== 'unknown' && mod.installDate.trim() !== '') {
    const d = new Date(mod.installDate);
    formattedInstallDate = isNaN(d.getTime()) ? t('common.none') : d.toLocaleString();
  }
  document.getElementById('detail-install-date')!.textContent = formattedInstallDate;
  document.getElementById('detail-source-zip')!.textContent = mod.sourceZip || t('common.none');

  // Technical Component rows & Dynamic Action Buttons
  const componentsContainer = document.getElementById('detail-components-container');
  const folderButtonsContainer = document.getElementById('detail-folder-buttons');
  const compFolders = getModComponentFolders(mod);

  if (componentsContainer) {
    if (compFolders.length > 0) {
      componentsContainer.innerHTML = compFolders.map(c => `
        <div class="detail-row" style="display: flex; align-items: flex-start; gap: 8px;">
          <span class="detail-label" style="min-width: 115px; font-weight: 600;">${escapeHtml(c.label)}</span>
          <span style="word-break: break-all; font-size: 11px; flex: 1; color: var(--text-primary); font-family: monospace;" title="${escapeHtml(c.path)}">${escapeHtml(formatDisplayPath(c.path))}</span>
        </div>
      `).join('');
    } else {
      componentsContainer.innerHTML = `
        <div class="detail-row">
          <span class="detail-label">${escapeHtml(t('detail.root_label'))}</span>
          <span style="color: var(--text-muted);">${escapeHtml(t('common.none'))}</span>
        </div>
      `;
    }
  }

  if (folderButtonsContainer) {
    if (compFolders.length > 0) {
      folderButtonsContainer.style.display = 'flex';
      folderButtonsContainer.innerHTML = compFolders.map(c => `
        <button class="btn-action detail-open-comp-folder" data-path="${escapeHtml(c.path)}" style="flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 7px 10px; font-size: 11px; font-weight: 600; white-space: nowrap;" title="${escapeHtml(c.path)}">
          <span style="font-size: 13px;">📁</span>
          <span>${escapeHtml(c.buttonLabel)}</span>
        </button>
      `).join('');

      folderButtonsContainer.querySelectorAll<HTMLButtonElement>('.detail-open-comp-folder').forEach(btn => {
        btn.addEventListener('click', async () => {
          const path = btn.dataset.path;
          if (!path) return;
          try {
            const { openPath } = await import('../api');
            await openPath(path);
          } catch (err) {
            showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          }
        });
      });
    } else {
      folderButtonsContainer.style.display = 'none';
      folderButtonsContainer.innerHTML = '';
    }
  }

  // Duplicate detection
  const duplicateRow = document.getElementById('detail-duplicate-row')!;
  const duplicateWarning = document.getElementById('detail-duplicate-warning')!;
  const similar = state.allMods.filter(m =>
    m.id !== mod.id &&
    (m.name.toLowerCase().includes(mod.name.toLowerCase().split(/[^a-z0-9]/i).slice(0, 3).join(' ')) ||
     mod.name.toLowerCase().includes(m.name.toLowerCase().split(/[^a-z0-9]/i).slice(0, 3).join(' ')))
  );
  if (similar.length > 0) {
    duplicateWarning.textContent = t('detail.duplicate_warning', { names: similar.map(m => m.name).join(', ') });
    duplicateRow.style.display = '';
  } else {
    duplicateRow.style.display = 'none';
  }

  const configPathEl = document.getElementById('detail-config-path')!;
  const configRow = configPathEl.closest('.detail-row') as HTMLElement;
  const isPakType = mod.type === 'pak' || mod.type === 'logicmods';

  const pakDestRow = document.getElementById('detail-pak-destination-row')!;
  const pakDestSelect = document.getElementById('detail-pak-destination-select') as HTMLSelectElement;

  if (isPakType) {
    configPathEl.textContent = 'N/A';
    configPathEl.title = '';
    configRow.style.display = 'none';
    
    pakDestRow.style.display = '';
    const currentDest = mod.pakDestination || (mod.type === 'logicmods' ? 'LogicMods' : '~mods');
    pakDestSelect.value = currentDest;

    const newSelect = pakDestSelect.cloneNode(true) as HTMLSelectElement;
    pakDestSelect.parentNode!.replaceChild(newSelect, pakDestSelect);

    newSelect.addEventListener('change', async () => {
      const selectedDest = newSelect.value;
      try {
        const updated = await changePakDestination(mod.id, selectedDest);
        const idx = state.allMods.findIndex(m => m.id === mod.id);
        if (idx >= 0) {
          const newMods = [...state.allMods];
          newMods[idx] = updated;
          updateState({ allMods: newMods });
        }
        await loadMods();
        openDetailPanel(updated.id);
        renderModsView();
        showToast(t('toasts.pak_dest_changed', { dest: selectedDest }), 'success');
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        newSelect.value = currentDest;
      }
    });
  } else {
    configPathEl.textContent = mod.configPath ? formatDisplayPath(mod.configPath) : t('common.none');
    configPathEl.title = mod.configPath || '';
    configRow.style.display = '';
    pakDestRow.style.display = 'none';
  }

  // Populate Folder Dropdown
  const folderSelect = document.getElementById('detail-folder-select') as HTMLSelectElement | null;
  if (folderSelect) {
    const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);
    const folders = currentProfile?.mod_folders || [];
    const currentFolder = folders.find(f => f.mod_ids.includes(mod.id));
    
    folderSelect.innerHTML = `<option value="">${escapeHtml(t('detail.folder_none'))}</option>` + 
      folders.map(f => `<option value="${escapeHtml(f.id)}" ${currentFolder?.id === f.id ? 'selected' : ''}>${escapeHtml(f.name)}</option>`).join('');
      
    const newSelect = folderSelect.cloneNode(true) as HTMLSelectElement;
    folderSelect.parentNode!.replaceChild(newSelect, folderSelect);
    
    newSelect.addEventListener('change', async () => {
      const selectedFolderId = newSelect.value || null;
      try {
        const { addModToFolder } = await import('../api');
        const updatedProfile = await addModToFolder(state.currentProfileId, selectedFolderId, mod.id);
        
        const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles: updatedProfiles });
        
        await loadMods();
        showToast(selectedFolderId ? t('toasts.folder_assigned') : t('toasts.folder_unassigned'), 'success');
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    });
  }

  // Reset scroll position and tabs — fixes state persistence across mods
  document.getElementById('detail-body')!.scrollTop = 0;
  document.querySelectorAll('.detail-tab').forEach(t => t.classList.remove('active'));
  (document.querySelector('.detail-tab[data-tab="info"]') as HTMLElement)?.classList.add('active');
  document.getElementById('detail-info-tab')!.style.display = '';
  document.getElementById('detail-tech-tab')!.style.display = 'none';

  document.getElementById('detail-overlay')!.classList.add('visible');
  setupDetailTabs();
}

let detailTabsSetup = false;
function setupDetailTabs(): void {
  if (detailTabsSetup) return;
  detailTabsSetup = true;
  document.querySelectorAll('.detail-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      const tabName = (tab as HTMLElement).dataset.tab!;
      document.querySelectorAll('.detail-tab').forEach((t) => t.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById('detail-info-tab')!.style.display = tabName === 'info' ? '' : 'none';
      document.getElementById('detail-tech-tab')!.style.display = tabName === 'tech' ? '' : 'none';
    });
  });
}


async function autoFetchNexusInfo(mod: ModInfo): Promise<void> {
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
    openDetailPanel(updated.id);
    renderModsView();
  } catch (e) {
    console.warn('Auto-fetch nexus info failed:', e);
  }
}

function setupNexusIdEdit(modId: string): void {
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

export function closeDetailPanel(): void {
  document.getElementById('detail-overlay')!.classList.remove('visible');
  updateState({ currentDetailMod: null });
}

export async function handleRefreshDetail(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod?.nexusModId) return;
  const btn = document.getElementById('detail-refresh')! as HTMLButtonElement;
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
    const { suppressWatcherRefresh } = await import('./editor/watcher');
    suppressWatcherRefresh(1200);

    if (isWorkshop) {
      const { activateWorkshopMod, deactivateWorkshopMod } = await import('../api');
      if (state.currentDetailMod.enabled) {
        await deactivateWorkshopMod(state.currentDetailMod.id);
      } else {
        await activateWorkshopMod(state.currentDetailMod.id);
      }
    } else {
      if (state.currentDetailMod.enabled) { await disableMod(state.currentDetailMod.id); }
      else { await enableMod(state.currentDetailMod.id); }
    }
    await loadMods();
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
      await loadMods();
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
    const basePath = state.currentDetailMod.enabled
      ? state.currentDetailMod.gamePath
      : state.currentDetailMod.disabledPath;
    const selected = await open({
      multiple: false,
      defaultPath: basePath,
      filters: [{ name: 'Config files', extensions: ['json', 'lua'] }],
      title: t('detail.dialog_select_config_title', { name: state.currentDetailMod.name }),
    });
    if (!selected) return;
    const configPath = typeof selected === 'string' ? selected : selected as string;
    await setModConfig(state.currentDetailMod.id, configPath);
    await loadMods();
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
    await setModConfig(state.currentDetailMod.id, null);
    await loadMods();
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
    const { openExtraFolder } = await import('../api');
    await openExtraFolder(state.currentDetailMod.id);
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleDetailRename(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  const currentName = state.currentDetailMod.name;
  const header = document.getElementById('detail-name-header')!;
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
          updateState({ currentDetailMod: updated });
          header.textContent = updated.name;
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

function renderVersion(mod: ModInfo): void {
  const el = document.getElementById('detail-version')!;
  const state = getState();
  const updateVer = state.availableUpdates?.get(mod.id);
  
  let updateBadge = '';
  if (updateVer) {
    updateBadge = `
      <span class="mod-card-update-badge" title="${escapeHtml(t('card.badge_update_available', { version: updateVer }))}" style="margin-left: 6px; vertical-align: middle;">&#9650; ${escapeHtml(t('context.update_mod'))} (v${escapeHtml(updateVer)})</span>
      <button class="btn-tiny ignore-update-btn" data-latest="${escapeHtml(updateVer)}" style="margin-left: 6px; vertical-align: middle; background: rgba(255,255,255,0.05); color: var(--text-muted); border: 1px solid var(--border);">${escapeHtml(t('common.cancel'))}</button>
    `;
  }

  const ignoredLabel = mod.ignoredVersion
    ? `<span style="font-size: 10px; color: var(--text-muted); margin-left: 6px; vertical-align: middle;">(${escapeHtml(t('context.ignore_update', { version: mod.ignoredVersion }))}) <button class="btn-tiny unignore-update-btn" style="margin-left: 4px; vertical-align: middle; background: transparent; border: none; color: var(--accent); cursor: pointer; text-decoration: underline; padding: 0;">${escapeHtml(t('common.retry'))}</button></span>`
    : '';

  el.innerHTML = `<span class="version-value">v${escapeHtml(mod.version)}</span> ${updateBadge} ${ignoredLabel} <button class="btn-tiny version-edit-btn" style="margin-left: 6px;">${escapeHtml(t('common.edit'))}</button>`;
  
  const editBtn = el.querySelector('.version-edit-btn') as HTMLButtonElement;
  const valSpan = el.querySelector('.version-value') as HTMLSpanElement;
  
  // Event listeners for ignore/unignore
  const ignoreBtn = el.querySelector('.ignore-update-btn') as HTMLButtonElement | null;
  if (ignoreBtn) {
    ignoreBtn.addEventListener('click', async () => {
      const latest = ignoreBtn.dataset.latest || '';
      try {
        await ignoreModVersion(mod.id, latest);
        showToast(t('toasts.settings_saved'), 'success');
        await loadMods();
        openDetailPanel(mod.id);
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    });
  }

  const unignoreBtn = el.querySelector('.unignore-update-btn') as HTMLButtonElement | null;
  if (unignoreBtn) {
    unignoreBtn.addEventListener('click', async () => {
      try {
        await ignoreModVersion(mod.id, null);
        showToast(t('toasts.settings_saved'), 'success');
        await loadMods();
        openDetailPanel(mod.id);
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    });
  }

  editBtn.addEventListener('click', () => {
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'version-input';
    input.value = mod.version;
    input.maxLength = 50;
    valSpan.textContent = '';
    valSpan.appendChild(input);
    input.focus();
    input.select();
    editBtn.style.display = 'none';
    const done = async (save: boolean) => {
      if (save) {
        const newVer = input.value.trim();
        if (newVer && newVer !== mod.version) {
          try {
            const updated = await setModVersion(mod.id, newVer);
            const state = getState();
            const idx = state.allMods.findIndex(m => m.id === mod.id);
            if (idx >= 0) {
              const newMods = [...state.allMods];
              newMods[idx] = updated;
              updateState({ allMods: newMods });
            }
            renderVersion(updated);
            renderModsView();
            showToast(t('toasts.mod_updated', { name: mod.name }), 'success');
          } catch (e) {
            showToast(t('toasts.export_failed', { error: String(e) }), 'error');
            renderVersion(mod);
          }
        } else {
          renderVersion(mod);
        }
      } else {
        renderVersion(mod);
      }
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') { input.blur(); done(true); }
      if (e.key === 'Escape') { input.blur(); done(false); }
    });
    input.addEventListener('blur', () => done(true));
  });
}

function renderGithubSection(mod: ModInfo): void {
  const container = document.getElementById('detail-github') as HTMLElement | null;
  if (!container) return;

  if (!mod.githubRepo) {
    container.innerHTML = `
      <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.github_label'))}</span>
        <span><input type="text" class="github-repo-input" placeholder="owner/repo..." /><button class="btn-tiny github-add-btn" style="margin-left:4px">${escapeHtml(t('common.save'))}</button></span>
      </div>`;
    container.style.display = 'block';
    const input = container.querySelector('.github-repo-input') as HTMLInputElement;
    const addBtn = container.querySelector('.github-add-btn') as HTMLButtonElement;
    const doAdd = async () => {
      const repo = input.value.trim();
      if (!repo) return;
      addBtn.disabled = true;
      try {
        const latest = await checkGitHubVersion(repo);
        const updated = await setGithubVersion(mod.id, repo, latest);
        const state = getState();
        const idx = state.allMods.findIndex(m => m.id === mod.id);
        if (idx >= 0) {
          const newMods = [...state.allMods];
          newMods[idx] = updated;
          updateState({ allMods: newMods });
        }
        renderGithubSection(updated);
        renderModsView();
        showToast(t('toasts.settings_saved'), 'success');
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        renderGithubSection(mod);
      }
    };
    addBtn.addEventListener('click', doAdd);
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') doAdd(); });
    return;
  }

  const repoLink = `https://github.com/${mod.githubRepo}`;
  const versionDisplay = mod.githubVersion
    ? `<span class="github-version-value">${escapeHtml(mod.githubVersion)}</span>`
    : `<span class="github-version-value" style="color:var(--text-muted)">${escapeHtml(t('common.unknown'))}</span>`;
  const cachedInfo = mod.githubCachedAt
    ? `<div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.last_updated_label'))}</span> ${new Date(mod.githubCachedAt).toLocaleDateString()}</div>`
    : '';

  container.innerHTML = `
    <div class="detail-row"><span class="detail-label">${escapeHtml(t('detail.github_label'))}</span> <a class="nexus-link" href="${repoLink}" target="_blank">${escapeHtml(mod.githubRepo)}</a></div>
    <div class="detail-row"><span class="detail-label">${escapeHtml(t('common.version'))}:</span> ${versionDisplay} <button class="btn-tiny github-refresh-btn">${escapeHtml(t('common.refresh'))}</button></div>
    ${cachedInfo}
  `;

  const refreshBtn = container.querySelector('.github-refresh-btn') as HTMLButtonElement;
  refreshBtn.addEventListener('click', async () => {
    refreshBtn.disabled = true;
    refreshBtn.textContent = '...';
    try {
      const latest = await checkGitHubVersion(mod.githubRepo!);
      const updated = await setGithubVersion(mod.id, mod.githubRepo!, latest);
      const state = getState();
      const idx = state.allMods.findIndex(m => m.id === mod.id);
      if (idx >= 0) {
        const newMods = [...state.allMods];
        newMods[idx] = updated;
        updateState({ allMods: newMods });
      }
      renderGithubSection(updated);
      renderModsView();
      showToast(t('toasts.settings_saved'), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      renderGithubSection(mod);
    }
  });
}

// Interceptar clics en enlaces de NexusMods/GitHub dentro del panel de detalles y abrirlos en el navegador por defecto
document.addEventListener('click', (e) => {
  const link = (e.target as HTMLElement).closest('.nexus-link') as HTMLAnchorElement | null;
  if (link && link.href && document.getElementById('detail-panel')?.contains(link)) {
    e.preventDefault();
    openUrl(link.href).catch(err => console.error('Failed to open link:', err));
  }
});
