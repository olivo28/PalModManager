import { getState, updateState } from '../../state';
import { getLibrary, removeFromLibrary, getWorkshopState, setWorkshopGlobalEnabled, activateWorkshopMod, deactivateWorkshopMod, openUrl } from '../../api';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export let _librarySearchQuery = '';
export let _activeLibrarySubTab: 'local' | 'workshop' = 'local';

const WORKSHOP_TIMESTAMPS_KEY = 'pmm_workshop_mod_timestamps';
const NEW_MOD_DURATION_MS = 10 * 60 * 1000; // 10 minutes

export function syncWorkshopModTimestamps(allWorkshopMods: Array<{ packageName: string; modName: string }>): {
  newModCount: number;
  newModNames: string[];
} {
  try {
    const raw = localStorage.getItem(WORKSHOP_TIMESTAMPS_KEY);
    const now = Date.now();

    if (!raw) {
      const initMap: Record<string, number> = {};
      for (const m of allWorkshopMods) {
        initMap[m.packageName] = 0;
      }
      localStorage.setItem(WORKSHOP_TIMESTAMPS_KEY, JSON.stringify(initMap));
      return { newModCount: 0, newModNames: [] };
    }

    const map: Record<string, number> = JSON.parse(raw);
    const newModNames: string[] = [];
    let updated = false;

    for (const m of allWorkshopMods) {
      if (map[m.packageName] === undefined) {
        map[m.packageName] = now;
        newModNames.push(m.modName);
        updated = true;
      }
    }

    if (updated) {
      localStorage.setItem(WORKSHOP_TIMESTAMPS_KEY, JSON.stringify(map));
    }

    let newCount = 0;
    for (const m of allWorkshopMods) {
      const addedAt = map[m.packageName];
      if (addedAt && (now - addedAt) < NEW_MOD_DURATION_MS) {
        newCount++;
      }
    }

    return { newModCount: newCount, newModNames };
  } catch {
    return { newModCount: 0, newModNames: [] };
  }
}

export function isWorkshopModNew(packageName: string): boolean {
  try {
    const raw = localStorage.getItem(WORKSHOP_TIMESTAMPS_KEY);
    if (!raw) return false;
    const map: Record<string, number> = JSON.parse(raw);
    const addedAt = map[packageName];
    if (!addedAt) return false;
    return (Date.now() - addedAt) < NEW_MOD_DURATION_MS;
  } catch {
    return false;
  }
}

export function updateWorkshopBadges(newCount: number): void {
  const subtabBadge = document.getElementById('workshop-subtab-badge');
  const sidebarBadge = document.getElementById('sidebar-library-badge');

  if (subtabBadge) {
    if (newCount > 0) {
      subtabBadge.textContent = `+${newCount} NEW`;
      subtabBadge.style.display = 'inline-block';
    } else {
      subtabBadge.style.display = 'none';
    }
  }

  if (sidebarBadge) {
    sidebarBadge.style.display = newCount > 0 ? 'block' : 'none';
  }
}

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
    try {
      const wState = await getWorkshopState();
      const sync = syncWorkshopModTimestamps(wState.mods);
      updateWorkshopBadges(sync.newModCount);
      if (sync.newModNames.length > 0) {
        showToast(`Steam Workshop: New mod subscribed — "${sync.newModNames[0]}"`, 'success');
      }
    } catch {}
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

    // Check workshop mods timestamps for badges (10 minute window)
    try {
      const wState = await getWorkshopState();
      const sync = syncWorkshopModTimestamps(wState.mods);
      updateWorkshopBadges(sync.newModCount);
    } catch {}

    // Recompute available updates on active mods list so local library updates show on mod cards
    const currentMods = getState().allMods;
    if (currentMods && currentMods.length > 0) {
      const { computeAvailableUpdates } = await import('./card');
      const { renderModsView } = await import('./renderer');
      const updatesMap = computeAvailableUpdates(currentMods, entries);
      updateState({ availableUpdates: updatesMap });
      renderModsView();
    }
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

    // Group entries by modId
    const groupsMap = new Map<string, {
      modId: string;
      name: string;
      author: string;
      description: string;
      modType: string;
      nexusPictureUrl?: string | null;
      nexusModId?: number | null;
      isInstalled: boolean;
      installedVersion: string | null;
      versions: {
        zipName: string;
        zipSize: number;
        version: string;
        installedAt: string;
      }[];
    }>();

    for (const e of entries) {
      const parsed = parseModFilename(e.zipName);
      const cleanName = e.nexusName || parsed.name || e.modId || e.zipName;
      const ver = e.version || (parsed.version ? `${parsed.version}` : (e.nexusVersion ? `${e.nexusVersion}` : '1.0'));

      const groupKey = e.modId;
      let group = groupsMap.get(groupKey);
      if (!group) {
        group = {
          modId: e.modId,
          name: cleanName,
          author: e.author || e.nexusAuthor || '',
          description: e.description || e.nexusSummary || '',
          modType: (e.modType || '').toUpperCase(),
          nexusPictureUrl: e.nexusPictureUrl,
          nexusModId: e.nexusModId,
          isInstalled: !!e.isInstalled,
          installedVersion: e.installedVersion || null,
          versions: [],
        };
        groupsMap.set(groupKey, group);
      }

      if (!group.nexusPictureUrl && e.nexusPictureUrl) group.nexusPictureUrl = e.nexusPictureUrl;
      if (!group.author && (e.author || e.nexusAuthor)) group.author = e.author || e.nexusAuthor || '';
      if (!group.description && (e.description || e.nexusSummary)) group.description = e.description || e.nexusSummary || '';
      if (!group.modType && e.modType) group.modType = e.modType.toUpperCase();
      if (e.isInstalled) {
        group.isInstalled = true;
        if (e.installedVersion) group.installedVersion = e.installedVersion;
      }

      if (!group.versions.some(v => v.zipName === e.zipName)) {
        group.versions.push({
          zipName: e.zipName,
          zipSize: e.zipSize,
          version: ver,
          installedAt: e.installedAt,
        });
      }
    }

    const groups = Array.from(groupsMap.values());
    for (const g of groups) {
      g.versions.sort((a, b) => {
        const cleanA = a.version.replace(/^[^\d]*/, '').split('.').map(n => parseInt(n, 10) || 0);
        const cleanB = b.version.replace(/^[^\d]*/, '').split('.').map(n => parseInt(n, 10) || 0);
        for (let i = 0; i < Math.max(cleanA.length, cleanB.length); i++) {
          const numA = cleanA[i] || 0;
          const numB = cleanB[i] || 0;
          if (numA !== numB) return numB - numA;
        }
        return b.version.localeCompare(a.version);
      });
    }

function compareVersions(a: string, b: string): number {
  const parseParts = (v: string) => v.replace(/^[^\d]*/, '').split(/[\.-]/).map(n => parseInt(n, 10) || 0);
  const partsA = parseParts(a);
  const partsB = parseParts(b);
  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const numA = partsA[i] || 0;
    const numB = partsB[i] || 0;
    if (numA !== numB) return numA - numB;
  }
  return a.localeCompare(b);
}

    container.innerHTML = groups.map(group => {
      const isSelected = state.selectedLibraryIds.has(group.modId);
      const latestVerObj = group.versions[0];
      const latestVersion = latestVerObj ? latestVerObj.version : '1.0';
      const cleanName = group.name;
      const author = group.author;
      const description = group.description;
      const modType = group.modType;

      let statusBadgeHtml = '';
      let installBtnText = 'Install';
      let isUpdateAvailable = false;

      if (group.isInstalled) {
        const cmp = group.installedVersion ? compareVersions(latestVersion, group.installedVersion) : 0;
        if (cmp > 0) {
          isUpdateAvailable = true;
          statusBadgeHtml = `<span class="library-status-badge badge-warning" title="Installed: v${escapeHtml(group.installedVersion || '')}">Installed (v${escapeHtml(group.installedVersion || '')})</span>`;
          installBtnText = `Update to v${escapeHtml(latestVersion)}`;
        } else if (cmp === 0) {
          statusBadgeHtml = `<span class="library-status-badge badge-success" title="Currently installed in game">Installed${group.installedVersion && group.installedVersion !== 'unknown' ? ' (v' + escapeHtml(group.installedVersion) + ')' : ''}</span>`;
          installBtnText = 'Reinstall';
        } else {
          statusBadgeHtml = `<span class="library-status-badge badge-success" title="Currently installed in game">Installed (v${escapeHtml(group.installedVersion || '')})</span>`;
          installBtnText = `Rollback to v${escapeHtml(latestVersion)}`;
        }
      } else {
        statusBadgeHtml = `<span class="library-status-badge badge-muted">Not Installed</span>`;
        installBtnText = group.versions.length > 1 ? `Install v${escapeHtml(latestVersion)}` : 'Install';
      }

      let imageHtml = `<div style="font-size:32px;text-align:center;color:var(--text-muted);opacity:0.8;margin:8px 0;">📦</div>`;
      let resolvedSrc = group.nexusPictureUrl;
      if (resolvedSrc) {
        if (!resolvedSrc.startsWith('http://') && !resolvedSrc.startsWith('https://')) {
          try { resolvedSrc = convertFileSrc(resolvedSrc); } catch (err) { console.error(err); }
        }
        imageHtml = `
          <div class="library-card-img-container" style="width:100%;height:85px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;position:relative;">
            <img src="${resolvedSrc}" style="width:100%;height:100%;object-fit:cover;" />
            ${modType ? `<span class="library-type-tag ${modType.toLowerCase()}">${modType}</span>` : ''}
          </div>
        `;
      } else {
        const matchedMod = state.allMods.find(m => {
          if (m.name.toLowerCase() === group.modId.toLowerCase()) return true;
          if (m.nexusModId && group.nexusModId && m.nexusModId === group.nexusModId) return true;
          if (m.name.toLowerCase() === cleanName.toLowerCase()) return true;
          return false;
        });
        if (matchedMod && matchedMod.nexusPictureUrl) {
          let src = matchedMod.nexusPictureUrl;
          if (!src.startsWith('http://') && !src.startsWith('https://')) {
            try { src = convertFileSrc(src); } catch (err) { console.error(err); }
          }
          imageHtml = `
            <div class="library-card-img-container" style="width:100%;height:85px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;position:relative;">
              <img src="${src}" style="width:100%;height:100%;object-fit:cover;" />
              ${modType ? `<span class="library-type-tag ${modType.toLowerCase()}">${modType}</span>` : ''}
            </div>
          `;
        } else {
          imageHtml = `
            <div class="library-card-img-container" style="width:100%;height:85px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;position:relative;">
              <div style="font-size:28px;opacity:0.6;">📦</div>
              ${modType ? `<span class="library-type-tag ${modType.toLowerCase()}">${modType}</span>` : ''}
            </div>
          `;
        }
      }

      const versionControlsHtml = group.versions.length > 1 ? `
        <div style="display:flex;align-items:center;gap:6px;width:100%;margin-top:auto;border-top:1px solid var(--border);padding-top:6px;">
          <select class="library-version-select form-select" data-id="${group.modId}" style="flex:1;padding:4px 6px;font-size:11px;font-weight:600;background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;color:var(--text-primary);cursor:pointer;outline:none;">
            ${group.versions.map((v, idx) => `
              <option value="${escapeHtml(v.zipName)}" data-version="${escapeHtml(v.version)}" data-size="${formatSize(v.zipSize)}">
                v${escapeHtml(v.version)} ${idx === 0 ? '(Latest)' : ''}
              </option>
            `).join('')}
          </select>
          <span class="library-card-size" style="font-size:10px;color:var(--text-muted);white-space:nowrap;">${formatSize(latestVerObj.zipSize)}</span>
        </div>
      ` : `
        <div style="display:flex;justify-content:space-between;align-items:center;font-size:10px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:6px;margin-top:auto;">
          <span style="font-weight:600;color:var(--text-primary);">v${escapeHtml(latestVersion)}</span>
          <span class="library-card-size">${formatSize(latestVerObj.zipSize)}</span>
        </div>
      `;

      return `
        <div class="mod-card library-card ${isSelected ? 'selected' : ''}" data-id="${group.modId}" data-is-installed="${group.isInstalled}" data-installed-version="${escapeHtml(group.installedVersion || '')}" style="cursor:pointer;position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
          <div class="library-card-header" style="display:flex;align-items:center;justify-content:space-between;gap:6px;width:100%;">
            <div class="card-checkbox-container" style="display:flex;align-items:center;">
              <input type="checkbox" class="library-card-checkbox" data-id="${group.modId}" ${isSelected ? 'checked' : ''} style="width:14px;height:14px;cursor:pointer;" />
            </div>
            ${statusBadgeHtml}
          </div>
          
          <div style="display:flex;flex-direction:column;gap:6px;height:100%;justify-content:space-between;">
            ${imageHtml}
            <div>
              <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:left;word-break:break-word;line-height:1.3;margin-top:2px;">
                ${escapeHtml(cleanName)}
              </div>
              ${author ? `<div style="font-size:10px;color:var(--text-muted);margin-top:1px;text-align:left;">by ${escapeHtml(author)}</div>` : ''}
              ${description ? `<div style="font-size:10px;color:var(--text-secondary);opacity:0.8;line-height:1.3;margin-top:4px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;text-overflow:ellipsis;word-break:break-word;" title="${escapeHtml(description)}">${escapeHtml(description)}</div>` : ''}
            </div>

            ${versionControlsHtml}

            <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
              <button class="library-item-install btn-action ${isUpdateAvailable ? 'btn-action-primary' : ''}" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}" style="flex:1;padding:5px 8px;font-size:11px;font-weight:600;cursor:pointer;">${installBtnText}</button>
              <button class="library-item-delete btn-action btn-action-danger" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}" title="Remove selected archive from library" style="padding:5px 8px;font-size:11px;cursor:pointer;">✕</button>
            </div>
          </div>
        </div>
      `;
    }).join('');

    container.querySelectorAll('.library-version-select').forEach(sel => {
      sel.addEventListener('click', (ev) => ev.stopPropagation());
      sel.addEventListener('change', (ev) => {
        ev.stopPropagation();
        const select = ev.target as HTMLSelectElement;
        const card = select.closest('.library-card') as HTMLElement;
        if (!card) return;
        const selectedZip = select.value;
        const selectedOption = select.selectedOptions[0];
        const selectedVer = selectedOption?.dataset.version || '';
        const selectedSize = selectedOption?.dataset.size || '';

        const installBtn = card.querySelector('.library-item-install') as HTMLButtonElement | null;
        const deleteBtn = card.querySelector('.library-item-delete') as HTMLButtonElement | null;
        const statusBadge = card.querySelector('.library-status-badge') as HTMLElement | null;
        const sizeSpan = card.querySelector('.library-card-size') as HTMLElement | null;
        const installedVer = card.dataset.installedVersion || '';
        const isInstalled = card.dataset.isInstalled === 'true';

        if (installBtn) {
          installBtn.dataset.zip = selectedZip;
          if (isInstalled) {
            const cmp = installedVer ? compareVersions(selectedVer, installedVer) : 0;
            if (cmp === 0) {
              installBtn.textContent = 'Reinstall';
              installBtn.classList.remove('btn-action-primary');
            } else if (cmp > 0) {
              installBtn.textContent = `Update to v${selectedVer}`;
              installBtn.classList.add('btn-action-primary');
            } else {
              installBtn.textContent = `Rollback to v${selectedVer}`;
              installBtn.classList.remove('btn-action-primary');
            }
          } else {
            installBtn.textContent = `Install v${selectedVer}`;
            installBtn.classList.remove('btn-action-primary');
          }
        }

        if (deleteBtn) {
          deleteBtn.dataset.zip = selectedZip;
        }

        if (sizeSpan) {
          sizeSpan.textContent = selectedSize;
        }

        if (statusBadge) {
          if (isInstalled) {
            const cmp = installedVer ? compareVersions(selectedVer, installedVer) : 0;
            if (cmp === 0) {
              statusBadge.className = 'library-status-badge badge-success';
              statusBadge.textContent = `Installed (v${installedVer})`;
            } else if (cmp > 0) {
              statusBadge.className = 'library-status-badge badge-warning';
              statusBadge.textContent = `Installed (v${installedVer})`;
            } else {
              statusBadge.className = 'library-status-badge badge-success';
              statusBadge.textContent = `Installed (v${installedVer})`;
            }
          } else {
            statusBadge.className = 'library-status-badge badge-muted';
            statusBadge.textContent = 'Not Installed';
          }
        }
      });
    });

    container.querySelectorAll('.library-item-install').forEach(btn => {
      btn.addEventListener('click', async (ev) => {
        ev.stopPropagation();
        const id = (btn as HTMLElement).dataset.id!;
        const zip = (btn as HTMLElement).dataset.zip;
        await triggerInstallFromLibrary(id, zip);
      });
    });

    container.querySelectorAll('.library-item-delete').forEach(btn => {
      btn.addEventListener('click', async (ev) => {
        ev.stopPropagation();
        const id = (btn as HTMLElement).dataset.id!;
        const zip = (btn as HTMLElement).dataset.zip!;
        if (confirm(`Are you sure you want to remove version "${zip}" from your library?\n\n(Other versions of this mod in your library will NOT be deleted)`)) {
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

        const isNew = isWorkshopModNew(m.packageName);
        const newBadge = isNew
          ? `<span style="font-size: 8px; font-weight: 700; background: linear-gradient(135deg, #00bcff, #38ef7d); color: #000; padding: 2px 6px; border-radius: 10px; box-shadow: 0 0 8px rgba(0,188,255,0.6); margin-left: 4px; letter-spacing: 0.5px;">✨ NEW</span>`
          : '';

        const badgeText = m.isFramework ? 'FRAMEWORK' : 'WORKSHOP';
        const badgeStyle = `font-size: 8px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.1)' : 'rgba(255, 157, 0, 0.1)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.2)' : 'rgba(255, 157, 0, 0.2)'}; padding: 1px 4px; border-radius: 3px;`;

        const toggleBtnText = m.isActive ? 'Deactivate' : 'Activate';
        const toggleBtnClass = m.isActive ? 'btn-action btn-action-danger' : 'btn-primary btn-sm';

        return `
          <div class="mod-card library-card workshop-card" data-package="${escapeHtml(m.packageName)}" style="position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
            <div style="position:absolute;top:10px;right:10px;z-index:5;display:flex;align-items:center;">
              <span style="${badgeStyle}">${badgeText}</span>
              ${newBadge}
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

export async function triggerInstallFromLibrary(id: string, zipName?: string): Promise<void> {
  try {
    const { getLibraryZipPath, analyzeZip, checkModExistsCommand } = await import('../../api');
    const { renderInstallPreview, showInstallModal } = await import('../modal');

    const zipPath = await getLibraryZipPath(id, zipName);
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
