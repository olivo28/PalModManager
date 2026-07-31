import { disableMod, enableMod, removeMod, refreshNexusCache, setModConfig, setNexusModId, openModFolder, readConfig, renameMod, setModVersion, checkGitHubVersion, setGithubVersion, openUrl } from '../api';
import { getState, updateState } from '../state';
import { openConfigEditor } from './editorView';
import { loadMods, renderModsView } from './modsView';
import { showToast } from './toast';
import { showConfirm } from './confirm';
import { escapeHtml } from '../utils/helpers';
import { descriptionToHtml } from '../utils/bbcode';
import type { ModInfo } from '../types';

export function openDetailPanel(modId: string): void {
  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);
  if (!mod) return;
  updateState({ currentDetailMod: mod });

  const panel = document.getElementById('detail-panel')!;
  panel.dataset.id = modId;

  document.getElementById('detail-name-header')!.textContent = mod.name;
  document.getElementById('detail-type')!.textContent = mod.type;
  document.getElementById('detail-type')!.className = `mod-type-badge ${mod.type}`;
  renderVersion(mod);
  document.getElementById('detail-status')!.textContent = mod.enabled ? 'Enabled' : 'Disabled';
  document.getElementById('detail-status')!.className = `detail-status ${mod.enabled ? 'enabled' : 'disabled'}`;

  const toggleBtn = document.getElementById('detail-toggle')! as HTMLButtonElement;
  toggleBtn.textContent = mod.enabled ? 'Disable' : 'Enable';
  toggleBtn.dataset.enabled = String(mod.enabled);

  const nexusSection = document.getElementById('detail-nexus')!;
  const descSection = document.getElementById('detail-description')!;
  const imgEl = document.getElementById('detail-image')! as HTMLImageElement;
  const imgContainer = document.getElementById('detail-image-container')!;

  if (mod.nexusModId) {
    const hasNexusInfo = mod.nexusAuthor || mod.nexusDescription || mod.nexusEndorsements !== null;
    if (hasNexusInfo) {
      const tagsHtml = mod.nexusTags && mod.nexusTags.length > 0
        ? `<div class="detail-row"><span class="detail-label">Tags:</span> <span class="detail-tags-list">${mod.nexusTags.map(t => `<span class="detail-tag-chip">${escapeHtml(t)}</span>`).join('')}</span></div>`
        : '';
      const catHtml = mod.nexusCategory
        ? `<div class="detail-row"><span class="detail-label">Category:</span> ${escapeHtml(mod.nexusCategory)}</div>`
        : '';
      nexusSection.innerHTML = `
        <div class="detail-row"><span class="detail-label">NexusMods:</span> <span class="detail-nexus-id-row"><a class="nexus-link" href="https://www.nexusmods.com/palworld/mods/${mod.nexusModId}" target="_blank">#${mod.nexusModId}</a> <button class="btn-tiny nexus-id-edit-btn">Edit</button></span></div>
        <div class="detail-row detail-nexus-edit-row" style="display:none"><span class="detail-label"></span> <span><input type="text" class="nexus-id-input" value="${mod.nexusModId}" /><button class="btn-tiny nexus-id-save-btn" style="margin-left:4px">Save</button><button class="btn-tiny nexus-id-cancel-btn">Cancel</button></span></div>
        ${mod.nexusAuthor ? `<div class="detail-row"><span class="detail-label">Author:</span> ${escapeHtml(mod.nexusAuthor)}</div>` : ''}
        ${catHtml}
        ${mod.nexusEndorsements !== null ? `<div class="detail-row"><span class="detail-label">Endorsements:</span> ${mod.nexusEndorsements.toLocaleString()}</div>` : ''}
        ${mod.nexusCachedAt ? `<div class="detail-row"><span class="detail-label">Last updated:</span> ${new Date(mod.nexusCachedAt).toLocaleDateString()}</div>` : ''}
        ${tagsHtml}
      `;
      nexusSection.style.display = 'block';
      setupNexusIdEdit(mod.id);
    } else {
      nexusSection.innerHTML = `
        <div class="detail-row"><span class="detail-label">NexusMods:</span> <span class="detail-nexus-id-row"><a class="nexus-link" href="https://www.nexusmods.com/palworld/mods/${mod.nexusModId}" target="_blank">#${mod.nexusModId}</a> <button class="btn-tiny nexus-id-edit-btn">Edit</button></span></div>
        <div class="detail-row detail-nexus-edit-row" style="display:none"><span class="detail-label"></span> <span><input type="text" class="nexus-id-input" value="${mod.nexusModId}" /><button class="btn-tiny nexus-id-save-btn" style="margin-left:4px">Save</button><button class="btn-tiny nexus-id-cancel-btn">Cancel</button></span></div>
        <div class="detail-row"><span class="detail-label"></span> <span class="nexus-fetching">(fetching info...)</span></div>`;
      nexusSection.style.display = 'block';
      setupNexusIdEdit(mod.id);
      autoFetchNexusInfo(mod);
    }
  } else {
    nexusSection.innerHTML = `
      <div class="detail-row"><span class="detail-label">NexusMods:</span>
        <span><input type="text" class="nexus-id-input" placeholder="Enter NexusMods ID..." /><button class="btn-tiny nexus-id-add-btn" style="margin-left:4px">Save</button></span>
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
        showToast('NexusMods ID updated', 'success');
      } catch (e) {
        showToast('Failed to update NexusMods ID: ' + e, 'error');
      } finally {
        addBtn.disabled = false;
      }
    };
    addBtn.addEventListener('click', doSave);
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') doSave(); });
  }

  if (mod.nexusPictureUrl) {
    imgEl.src = mod.nexusPictureUrl;
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

  document.getElementById('detail-install-date')!.textContent = mod.installDate !== 'unknown' ? new Date(mod.installDate).toLocaleString() : 'Unknown';
  document.getElementById('detail-source-zip')!.textContent = mod.sourceZip || 'N/A';

  // Root folder
  const gamePath = mod.enabled ? mod.gamePath : mod.disabledPath;
  document.getElementById('detail-root-folder')!.textContent = gamePath;

  // Duplicate detection
  const duplicateRow = document.getElementById('detail-duplicate-row')!;
  const duplicateWarning = document.getElementById('detail-duplicate-warning')!;
  const similar = state.allMods.filter(m =>
    m.id !== mod.id &&
    (m.name.toLowerCase().includes(mod.name.toLowerCase().split(/[^a-z0-9]/i).slice(0, 3).join(' ')) ||
     mod.name.toLowerCase().includes(m.name.toLowerCase().split(/[^a-z0-9]/i).slice(0, 3).join(' ')))
  );
  if (similar.length > 0) {
    duplicateWarning.textContent = `Possible duplicate: ${similar.map(m => m.name).join(', ')}`;
    duplicateRow.style.display = '';
  } else {
    duplicateRow.style.display = 'none';
  }

  const configPathEl = document.getElementById('detail-config-path')!;
  const configRow = configPathEl.closest('.detail-row') as HTMLElement;
  const isPakType = mod.type === 'pak' || mod.type === 'logicmods';

  if (isPakType) {
    configPathEl.textContent = 'N/A';
    configRow.style.display = 'none';
  } else {
    configPathEl.textContent = mod.configPath || 'Not detected';
    configRow.style.display = '';
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
      showToast('NexusMods ID updated', 'success');
    } catch (e) {
      showToast('Failed to update NexusMods ID: ' + e, 'error');
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
  btn.textContent = 'Refreshing...';
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
    showToast('Failed to refresh: ' + e, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Refresh info';
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
  try {
    if (state.currentDetailMod.enabled) { await disableMod(state.currentDetailMod.id); }
    else { await enableMod(state.currentDetailMod.id); }
    await loadMods();
    openDetailPanel(state.currentDetailMod.id);
  } catch (e) {
    showToast('Failed to toggle: ' + e, 'error');
  }
}

export async function handleDetailRemove(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  const confirmed = await showConfirm(`Remove "${state.currentDetailMod.name}" permanently?`);
  if (confirmed) {
    try {
      await removeMod(state.currentDetailMod.id);
      closeDetailPanel();
      await loadMods();
      showToast('Mod removed', 'success');
    } catch (e) {
      showToast('Failed to remove: ' + e, 'error');
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
      title: 'Select config file for ' + state.currentDetailMod.name,
    });
    if (!selected) return;
    const configPath = typeof selected === 'string' ? selected : selected as string;
    await setModConfig(state.currentDetailMod.id, configPath);
    await loadMods();
    openDetailPanel(state.currentDetailMod.id);
    showToast('Config file set', 'success');
  } catch (e) {
    showToast('Failed to set config: ' + e, 'error');
  }
}

export async function handleDetailClearConfig(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    await setModConfig(state.currentDetailMod.id, null);
    await loadMods();
    openDetailPanel(state.currentDetailMod.id);
    showToast('Config file cleared', 'success');
  } catch (e) {
    showToast('Failed to clear config: ' + e, 'error');
  }
}

export async function handleDetailOpenFolder(): Promise<void> {
  const state = getState();
  if (!state.currentDetailMod) return;
  try {
    await openModFolder(state.currentDetailMod.id);
  } catch (e) {
    showToast('Failed to open folder: ' + e, 'error');
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
          showToast('Mod renamed successfully', 'success');
        } catch (e) {
          showToast('Rename failed: ' + e, 'error');
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
  el.innerHTML = `<span class="version-value">v${escapeHtml(mod.version)}</span> <button class="btn-tiny version-edit-btn">Edit</button>`;
  const editBtn = el.querySelector('.version-edit-btn') as HTMLButtonElement;
  const valSpan = el.querySelector('.version-value') as HTMLSpanElement;
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
            showToast('Version updated', 'success');
          } catch (e) {
            showToast('Version update failed: ' + e, 'error');
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
      <div class="detail-row"><span class="detail-label">GitHub:</span>
        <span><input type="text" class="github-repo-input" placeholder="owner/repo..." /><button class="btn-tiny github-add-btn" style="margin-left:4px">Add</button></span>
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
        showToast('GitHub repo added', 'success');
      } catch (e) {
        showToast('GitHub check failed: ' + e, 'error');
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
    : '<span class="github-version-value" style="color:var(--text-muted)">Unknown</span>';
  const cachedInfo = mod.githubCachedAt
    ? `<div class="detail-row"><span class="detail-label">Checked:</span> ${new Date(mod.githubCachedAt).toLocaleDateString()}</div>`
    : '';

  container.innerHTML = `
    <div class="detail-row"><span class="detail-label">GitHub:</span> <a class="nexus-link" href="${repoLink}" target="_blank">${escapeHtml(mod.githubRepo)}</a></div>
    <div class="detail-row"><span class="detail-label">Version:</span> ${versionDisplay} <button class="btn-tiny github-refresh-btn">Refresh</button></div>
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
      showToast('GitHub version updated', 'success');
    } catch (e) {
      showToast('GitHub check failed: ' + e, 'error');
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
