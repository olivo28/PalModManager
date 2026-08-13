import { getState, updateState } from '../../state';
import { getLibrary, removeFromLibrary, getWorkshopState, setWorkshopGlobalEnabled, activateWorkshopMod, deactivateWorkshopMod, openUrl } from '../../api';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export let _librarySearchQuery = '';
export let _activeLibrarySubTab: 'local' | 'workshop' = 'local';

export function setupLibraryHandlers(): void {
  const searchInput = document.getElementById('library-search-input') as HTMLInputElement | null;
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      _librarySearchQuery = searchInput.value.trim().toLowerCase();
      renderLibraryView();
    });
  }

  // Listen for background Steam Workshop folder changes
  listen('workshop-directory-changed', async () => {
    console.log('[INFO] Workshop directory change detected, auto-refreshing Library...');
    if (_activeLibrarySubTab === 'workshop') {
      renderLibraryView();
    }
    try {
      const { loadMods } = await import('../modsView');
      await loadMods();
    } catch (e) {
      console.error(e);
    }
  });

  document.getElementById('library-bulk-install-btn')?.addEventListener('click', handleLibraryBulkInstall);
  document.getElementById('library-bulk-remove-btn')?.addEventListener('click', handleLibraryBulkRemove);
  document.getElementById('library-bulk-clear-btn')?.addEventListener('click', () => {
    updateState({ selectedLibraryIds: new Set() });
    updateLibraryBulkBar();
    renderLibraryView();
  });

  // Tab switching handlers
  document.querySelectorAll('.library-sub-tab').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const target = e.currentTarget as HTMLButtonElement;
      const tab = target.dataset.tab as 'local' | 'workshop';

      document.querySelectorAll('.library-sub-tab').forEach(b => {
        const btnEl = b as HTMLButtonElement;
        btnEl.classList.remove('active');
        btnEl.style.background = 'transparent';
        btnEl.style.color = 'var(--text-muted)';
      });
      target.classList.add('active');
      target.style.background = 'var(--accent)';
      target.style.color = '#fff';

      _activeLibrarySubTab = tab;

      if (searchInput) {
        searchInput.value = '';
        _librarySearchQuery = '';
        searchInput.placeholder = tab === 'local' ? 'Search library...' : 'Search workshop...';
      }

      renderLibraryView();
    });
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

export async function renderLibraryView(): Promise<void> {
  const container = document.getElementById('library-container');
  if (!container) return;

  const masterToggleWrap = document.getElementById('library-workshop-master-wrap');
  const bulkBar = document.getElementById('library-bulk-actions-bar');

  if (_activeLibrarySubTab === 'local') {
    if (masterToggleWrap) masterToggleWrap.style.display = 'none';

    let entries = getState().libraryEntries;
    const state = getState();

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
      updateLibraryBulkBar();
      return;
    }

    container.innerHTML = entries.map(e => {
      const isSelected = state.selectedLibraryIds.has(e.modId);
      const parsed = parseModFilename(e.zipName);
      const cleanName = parsed.name || e.zipName;
      const versionStr = parsed.version ? `v${parsed.version}` : '';

      let imageHtml = `<div style="font-size:32px;text-align:center;color:var(--text-muted);opacity:0.8;margin:8px 0;">📦</div>`;
      if (e.nexusPictureUrl) {
        let resolvedSrc = e.nexusPictureUrl;
        if (!resolvedSrc.startsWith('http://') && !resolvedSrc.startsWith('https://')) {
          try { resolvedSrc = convertFileSrc(resolvedSrc); } catch (err) { console.error(err); }
        }
        imageHtml = `
          <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
            <img src="${resolvedSrc}" style="width:100%;height:100%;object-fit:cover;" />
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
          let resolvedSrc = matchedMod.nexusPictureUrl;
          if (!resolvedSrc.startsWith('http://') && !resolvedSrc.startsWith('https://')) {
            try { resolvedSrc = convertFileSrc(resolvedSrc); } catch (err) { console.error(err); }
          }
          imageHtml = `
            <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
              <img src="${resolvedSrc}" style="width:100%;height:100%;object-fit:cover;" />
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
              <button class="library-item-delete btn-action btn-action-danger" data-id="${e.modId}" data-zip="${escapeHtml(e.zipName)}" style="padding:4px 8px;font-size:10px;cursor:pointer;">✕</button>
            </div>
          </div>
        </div>
      `;
    }).join('');

    container.querySelectorAll('.library-item-install').forEach(btn => {
      btn.addEventListener('click', async (ev) => {
        ev.stopPropagation();
        const id = (btn as HTMLElement).dataset.id!;
        await triggerInstallFromLibrary(id);
      });
    });

    container.querySelectorAll('.library-item-delete').forEach(btn => {
      btn.addEventListener('click', async (ev) => {
        ev.stopPropagation();
        const id = (btn as HTMLElement).dataset.id!;
        const zip = (btn as HTMLElement).dataset.zip!;
        if (confirm(`Are you sure you want to remove "${zip}" from your library?`)) {
          try {
            await removeFromLibrary(id, zip);
            showToast('Mod version removed from library', 'success');
            await loadLibrary();
          } catch (err) {
            showToast('Failed to remove: ' + err, 'error');
          }
        }
      });
    });

    updateLibraryBulkBar();
  } else if (_activeLibrarySubTab === 'workshop') {
    if (masterToggleWrap) masterToggleWrap.style.display = 'flex';
    if (bulkBar) bulkBar.style.display = 'none';

    try {
      const wState = await getWorkshopState();

      const masterToggle = document.getElementById('library-workshop-master-toggle') as HTMLInputElement | null;
      if (masterToggle) {
        masterToggle.checked = wState.globalEnabled;
        masterToggle.onchange = async () => {
          showToast(masterToggle.checked ? 'Enabling Workshop Mods...' : 'Disabling Workshop Mods...', 'info');
          await setWorkshopGlobalEnabled(masterToggle.checked);
          await renderLibraryView();
          const { loadMods } = await import('../modsView');
          await loadMods();
          showToast('Workshop state updated', 'success');
        };
      }

      let mods = wState.mods;
      if (_librarySearchQuery) {
        mods = mods.filter(m => m.modName.toLowerCase().includes(_librarySearchQuery) || m.author.toLowerCase().includes(_librarySearchQuery));
      }

      if (mods.length === 0) {
        container.innerHTML = '<div id="library-empty">No subscribed Workshop mods found. Subscribing in Steam will list them here.</div>';
        return;
      }

      container.innerHTML = mods.map(m => {
        let typeClass = 'ue4ss';
        let typeLabel = 'U';
        if (m.installType === 'palSchemaMod') {
          typeClass = 'palschema';
          typeLabel = 'PS';
        }
        const thumb = m.thumbnailPath 
          ? `<img src="${convertFileSrc(m.thumbnailPath)}" style="width:100%;height:100%;object-fit:cover;" />` 
          : `<div class="mod-card-image-placeholder ${typeClass}" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;font-weight:bold;font-size:24px;color:#fff;">${typeLabel}</div>`;
        const isDepMissing = m.dependencies.some((dep: string) => !wState.activeModList.includes(dep));
        const depWarning = isDepMissing ? `<div style="color:#ff4a4a; font-size:10px; margin-top:2px; text-align:center;">Missing dependencies: ${escapeHtml(m.dependencies.join(', '))}</div>` : '';

        const badgeText = m.isFramework ? 'FRAMEWORK' : 'WORKSHOP';
        const badgeStyle = `font-size: 8px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.1)' : 'rgba(255, 157, 0, 0.1)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.2)' : 'rgba(255, 157, 0, 0.2)'}; padding: 1px 4px; border-radius: 3px;`;

        const toggleBtnText = m.isActive ? 'Deactivate' : 'Activate';
        const toggleBtnClass = m.isActive ? 'btn-action btn-action-danger' : 'btn-primary btn-sm';

        return `
          <div class="mod-card library-card workshop-card" data-package="${escapeHtml(m.packageName)}" style="position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
            <div style="position:absolute;top:10px;right:10px;z-index:5;">
              <span style="${badgeStyle}">${badgeText}</span>
            </div>
            <div style="padding-top:14px;display:flex;flex-direction:column;gap:8px;height:100%;justify-content:space-between;min-height:160px;">
              <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
                ${thumb}
              </div>
              <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:center;word-break:break-word;line-height:1.3;flex:1;min-height:36px;display:flex;align-items:center;justify-content:center;margin-top:4px;">
                ${escapeHtml(m.modName)}
              </div>
              <div style="font-size:10px; color:var(--text-muted); text-align:center;">
                Version ${escapeHtml(m.version)} by ${escapeHtml(m.author)} (ID: ${m.workshopId})
              </div>
              ${depWarning}
              <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
                <button class="workshop-item-toggle-btn ${toggleBtnClass}" data-package="${escapeHtml(m.packageName)}" data-active="${m.isActive}" ${m.isFramework ? 'disabled style="opacity:0.5;"' : ''} style="flex:1;padding:6px;font-size:10px;cursor:pointer;">
                  ${toggleBtnText}
                </button>
                <button class="workshop-item-folder-btn btn-secondary btn-sm" data-path="${escapeHtml(wState.workshopRoot + '/' + m.workshopId)}" style="padding:6px 8px;font-size:10px;cursor:pointer;" title="Open local Workshop folder">
                  📂 Folder
                </button>
              </div>
            </div>
          </div>
        `;
      }).join('');

      container.querySelectorAll('.workshop-item-toggle-btn').forEach(btn => {
        btn.addEventListener('click', async (e) => {
          const target = e.currentTarget as HTMLButtonElement;
          const pkgName = target.dataset.package!;
          const isActive = target.dataset.active === 'true';

          target.disabled = true;
          showToast(!isActive ? 'Activating Workshop mod...' : 'Deactivating Workshop mod...', 'info');
          try {
            if (!isActive) {
              await activateWorkshopMod(pkgName);
            } else {
              await deactivateWorkshopMod(pkgName);
            }
            showToast(!isActive ? 'Activated successfully' : 'Deactivated successfully', 'success');
          } catch (err) {
            showToast('Failed to toggle mod: ' + err, 'error');
          } finally {
            target.disabled = false;
            await renderLibraryView();
            const { loadMods } = await import('../modsView');
            await loadMods();
          }
        });
      });

      container.querySelectorAll('.workshop-item-folder-btn').forEach(btn => {
        btn.addEventListener('click', async (e) => {
          e.stopPropagation();
          const target = e.currentTarget as HTMLButtonElement;
          const path = target.dataset.path!;
          try {
            await openUrl(path);
          } catch (err) {
            showToast('Failed to open folder: ' + err, 'error');
          }
        });
      });

    } catch (err) {
      container.innerHTML = `<div style="color:#ff4a4a; padding:12px; text-align:center;">Failed to load Workshop state: ${escapeHtml(String(err))}</div>`;
    }
  }
}

export async function triggerInstallFromLibrary(id: string): Promise<void> {
  try {
    const { getLibraryZipPath, analyzeZip, checkModExistsCommand } = await import('../../api');
    const { renderInstallPreview, showInstallModal } = await import('../modal');

    const zipPath = await getLibraryZipPath(id);
    const analysis = await analyzeZip(zipPath);
    const check = await checkModExistsCommand(zipPath);

    const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name, version: check.modInfo.version } : null;

    renderInstallPreview(analysis, existingMod);
    showInstallModal();
  } catch (err) {
    showToast('Failed to open install preview: ' + err, 'error');
  }
}

export async function handleLibraryBulkInstall(): Promise<void> {
  const state = getState();
  const selected = Array.from(state.selectedLibraryIds);
  if (selected.length === 0) return;

  try {
    const { getLibraryZipPath } = await import('../../api');
    const { renderBatchInstallPreview, showInstallModal } = await import('../modal');

    const zipPaths: string[] = [];
    for (const id of selected) {
      try {
        const path = await getLibraryZipPath(id);
        zipPaths.push(path);
      } catch { }
    }

    if (zipPaths.length === 1) {
      const { analyzeZip, checkModExistsCommand } = await import('../../api');
      const { renderInstallPreview } = await import('../modal');
      const analysis = await analyzeZip(zipPaths[0]);
      const check = await checkModExistsCommand(zipPaths[0]);
      const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name, version: check.modInfo.version } : null;
      renderInstallPreview(analysis, existingMod);
      showInstallModal();
    } else if (zipPaths.length > 1) {
      await renderBatchInstallPreview(zipPaths);
    }
  } catch (err) {
    showToast('Failed to prepare batch install: ' + err, 'error');
  }
}

export async function handleLibraryBulkRemove(): Promise<void> {
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
