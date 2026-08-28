import { getState } from '../../../state';
import { getWorkshopState, setWorkshopGlobalEnabled, activateWorkshopMod, deactivateWorkshopMod, openUrl, removeFromLibrary } from '../../../api';
import { showToast } from '../../toast';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  _librarySearchQuery,
  _activeLibrarySubTab,
  _libraryFilterStatus,
  _librarySortBy,
  _libraryOnlineUpdatesMap,
} from './state';
import { formatSize, parseModFilename } from './helpers';
import { updateWorkshopTabVisibility, isWorkshopModNew } from './workshop';
import { updateLibraryBulkBar, triggerInstallFromLibrary } from './actions';

export async function renderLibraryView(): Promise<void> {
  const container = document.getElementById('library-container');
  if (!container) return;

  updateWorkshopTabVisibility();

  const masterToggleWrap = document.getElementById('library-workshop-master-wrap');
  const bulkBar = document.getElementById('library-bulk-actions-bar');
  const wsCheckUpdatesBtn = document.getElementById('workshop-check-updates-btn');

  if (_activeLibrarySubTab === 'local') {
    if (masterToggleWrap) masterToggleWrap.style.display = 'none';
    if (wsCheckUpdatesBtn) wsCheckUpdatesBtn.style.display = 'none';

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
      container.innerHTML = `<div id="library-empty">${escapeHtml(t('library.empty_local'))}</div>`;
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
      // Ignore temporary nexus_*.zip if a properly named zip exists for this mod
      const hasProperZip = entries.some(o => o.modId === e.modId && !o.zipName.toLowerCase().startsWith('nexus_'));
      if (hasProperZip && e.zipName.toLowerCase().startsWith('nexus_')) {
        continue;
      }

      const parsed = parseModFilename(e.zipName);
      const cleanName = e.nexusName || parsed.name || e.modId || e.zipName;
      let ver = e.version || (parsed.version ? `${parsed.version}` : (e.nexusVersion ? `${e.nexusVersion}` : '1.0'));
      if (ver.toLowerCase().startsWith('v')) {
        ver = ver.substring(1);
      }

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

      const existingVerIdx = group.versions.findIndex(v => v.zipName === e.zipName || v.version === ver);
      if (existingVerIdx === -1) {
        group.versions.push({
          zipName: e.zipName,
          zipSize: e.zipSize,
          version: ver,
          installedAt: e.installedAt,
        });
      } else if (!e.zipName.toLowerCase().startsWith('nexus_') && group.versions[existingVerIdx].zipName.toLowerCase().startsWith('nexus_')) {
        group.versions[existingVerIdx] = {
          zipName: e.zipName,
          zipSize: e.zipSize,
          version: ver,
          installedAt: e.installedAt,
        };
      }
    }

    let groups = Array.from(groupsMap.values());
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

    // Filter by installation status
    if (_libraryFilterStatus === 'installed') {
      groups = groups.filter(g => g.isInstalled);
    } else if (_libraryFilterStatus === 'not_installed') {
      groups = groups.filter(g => !g.isInstalled);
    } else if (_libraryFilterStatus === 'updates') {
      groups = groups.filter(g => {
        const onlineNexusVer = g.nexusModId ? _libraryOnlineUpdatesMap.get(g.nexusModId.toString()) : null;
        const hasOnlineUpdate = !!(onlineNexusVer && g.installedVersion && compareVersions(onlineNexusVer, g.installedVersion) > 0);
        const hasLocalUpdate = !!(g.isInstalled && g.installedVersion && g.versions.length > 0 && compareVersions(g.versions[0].version, g.installedVersion) > 0);
        return hasOnlineUpdate || hasLocalUpdate;
      });
    }

    // Sort groups
    groups.sort((a, b) => {
      switch (_librarySortBy) {
        case 'name:asc':
          return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
        case 'name:desc':
          return b.name.localeCompare(a.name, undefined, { sensitivity: 'base', numeric: true });
        case 'installed:first':
          if (a.isInstalled !== b.isInstalled) return b.isInstalled ? 1 : -1;
          return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
        case 'not_installed:first':
          if (a.isInstalled !== b.isInstalled) return a.isInstalled ? 1 : -1;
          return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
        case 'date:desc': {
          const dateA = a.versions[0]?.installedAt || '';
          const dateB = b.versions[0]?.installedAt || '';
          return dateB.localeCompare(dateA);
        }
        default:
          return a.name.localeCompare(b.name);
      }
    });

    if (groups.length === 0) {
      container.innerHTML = `<div id="library-empty">${escapeHtml(t('library.empty_local'))}</div>`;
      updateLibraryBulkBar();
      return;
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
          statusBadgeHtml = `<span class="library-status-badge badge-warning" title="${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}">${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}</span>`;
          installBtnText = t('library.btn_update_to', { version: latestVersion });
        } else if (cmp === 0) {
          statusBadgeHtml = `<span class="library-status-badge badge-success" title="${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}">${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}</span>`;
          installBtnText = t('library.btn_reinstall');
        } else {
          statusBadgeHtml = `<span class="library-status-badge badge-success" title="${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}">${escapeHtml(t('library.status_installed_exact', { version: group.installedVersion || '' }))}</span>`;
          installBtnText = t('library.btn_rollback_to', { version: latestVersion });
        }
      } else {
        statusBadgeHtml = `<span class="library-status-badge badge-muted">${escapeHtml(t('library.status_not_installed'))}</span>`;
        installBtnText = group.versions.length > 1 ? t('library.btn_install_ver', { version: latestVersion }) : t('common.install');
      }

      // Check if there is an online Nexus update for this library card
      const onlineNexusVer = _libraryOnlineUpdatesMap.get(group.modId);
      const onlineUpdateBadge = onlineNexusVer
        ? `<span style="font-size: 7.5px; font-weight: 700; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.4); padding: 2px 5px; border-radius: 3px; letter-spacing: 0.2px; white-space: nowrap;">▲ ${escapeHtml(t('card.badge_update_available', { version: onlineNexusVer }))}</span>`
        : '';

      let imageHtml = `<div style="font-size:32px;text-align:center;color:var(--text-muted);opacity:0.8;margin:8px 0;">📦</div>`;
      let resolvedSrc = group.nexusPictureUrl;
      if (resolvedSrc) {
        let displaySrc = resolvedSrc;
        if (!resolvedSrc.startsWith('http://') && !resolvedSrc.startsWith('https://')) {
          try { displaySrc = convertFileSrc(resolvedSrc); } catch (err) { console.error(err); }
        }
        imageHtml = `
          <div class="library-card-img-container" style="width:100%;height:85px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;position:relative;">
            <img src="${displaySrc}" data-original-src="${resolvedSrc}" style="width:100%;height:100%;object-fit:cover;" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='block'));" />
            <div style="display:none;font-size:28px;opacity:0.6;">📦</div>
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
          let displaySrc = src;
          if (!src.startsWith('http://') && !src.startsWith('https://')) {
            try { displaySrc = convertFileSrc(src); } catch (err) { console.error(err); }
          }
          imageHtml = `
            <div class="library-card-img-container" style="width:100%;height:85px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;position:relative;">
              <img src="${displaySrc}" data-original-src="${src}" style="width:100%;height:100%;object-fit:cover;" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='block'));" />
              <div style="display:none;font-size:28px;opacity:0.6;">📦</div>
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
            ${group.versions.map((v, idx) => {
              const displayVer = v.version.startsWith('v') || v.version.startsWith('V') ? v.version : `v${v.version}`;
              return `
                <option value="${escapeHtml(v.zipName)}" data-version="${escapeHtml(v.version)}" data-size="${formatSize(v.zipSize)}">
                  ${escapeHtml(displayVer)} ${idx === 0 ? escapeHtml(t('library.badge_latest')) : ''}
                </option>
              `;
            }).join('')}
          </select>
          <span class="library-card-size" style="font-size:10px;color:var(--text-muted);white-space:nowrap;">${formatSize(latestVerObj.zipSize)}</span>
        </div>
      ` : `
        <div style="display:flex;justify-content:space-between;align-items:center;font-size:10px;color:var(--text-muted);border-top:1px solid var(--border);padding-top:6px;margin-top:auto;">
          <span style="font-weight:600;color:var(--text-primary);">${latestVersion.startsWith('v') || latestVersion.startsWith('V') ? escapeHtml(latestVersion) : 'v' + escapeHtml(latestVersion)}</span>
          <span class="library-card-size">${formatSize(latestVerObj.zipSize)}</span>
        </div>
      `;

      return `
        <div class="mod-card library-card ${isSelected ? 'selected' : ''}" data-id="${group.modId}" data-is-installed="${group.isInstalled}" data-installed-version="${escapeHtml(group.installedVersion || '')}" style="cursor:pointer;position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
          <div class="library-card-header" style="display:flex;align-items:center;justify-content:space-between;gap:6px;width:100%;">
            <div class="card-checkbox-container" style="display:flex;align-items:center;">
              <input type="checkbox" class="library-card-checkbox" data-id="${group.modId}" ${isSelected ? 'checked' : ''} style="width:14px;height:14px;cursor:pointer;" />
            </div>
            <div style="display:flex;align-items:center;gap:4px;flex-wrap:wrap;justify-content:flex-end;">
              ${statusBadgeHtml}
              ${onlineUpdateBadge}
            </div>
          </div>
          
          <div style="display:flex;flex-direction:column;gap:6px;height:100%;justify-content:space-between;">
            ${imageHtml}
            <div>
              <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:left;word-break:break-word;line-height:1.3;margin-top:2px;">
                ${escapeHtml(cleanName)}
              </div>
              ${author ? `<div style="font-size:10px;color:var(--text-muted);margin-top:1px;text-align:left;">${escapeHtml(t('common.author'))}: ${escapeHtml(author)}</div>` : ''}
              ${description ? `<div style="font-size:10px;color:var(--text-secondary);opacity:0.8;line-height:1.3;margin-top:4px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;text-overflow:ellipsis;word-break:break-word;" title="${escapeHtml(description)}">${escapeHtml(description)}</div>` : ''}
            </div>

            ${versionControlsHtml}

            <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
              <button class="library-item-install btn-action ${isUpdateAvailable ? 'btn-action-primary' : ''}" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}" style="flex:1;padding:5px 8px;font-size:11px;font-weight:600;cursor:pointer;">${installBtnText}</button>
              <button class="library-item-delete btn-action btn-action-danger" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}" title="${escapeHtml(t('library.btn_delete_ver'))}" style="padding:5px 8px;font-size:11px;cursor:pointer;">✕</button>
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
              installBtn.textContent = t('library.btn_reinstall');
              installBtn.classList.remove('btn-action-primary');
            } else if (cmp > 0) {
              installBtn.textContent = t('library.btn_update_to', { version: selectedVer });
              installBtn.classList.add('btn-action-primary');
            } else {
              installBtn.textContent = t('library.btn_rollback_to', { version: selectedVer });
              installBtn.classList.remove('btn-action-primary');
            }
          } else {
            installBtn.textContent = t('library.btn_install_ver', { version: selectedVer });
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
              statusBadge.textContent = t('library.status_installed_exact', { version: installedVer });
            } else if (cmp > 0) {
              statusBadge.className = 'library-status-badge badge-warning';
              statusBadge.textContent = t('library.status_installed_exact', { version: installedVer });
            } else {
              statusBadge.className = 'library-status-badge badge-success';
              statusBadge.textContent = t('library.status_installed_exact', { version: installedVer });
            }
          } else {
            statusBadge.className = 'library-status-badge badge-muted';
            statusBadge.textContent = t('library.status_not_installed');
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
        const { showConfirm } = await import('../../confirm');
        const confirmed = await showConfirm(t('library.confirm_remove_version', { zip }));
        if (confirmed) {
          try {
            await removeFromLibrary(id, zip);
            showToast(t('toasts.library_mod_version_removed'), 'success');
            const { loadLibrary } = await import('./listeners');
            await loadLibrary();
          } catch (err) {
            showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          }
        }
      });
    });

    updateLibraryBulkBar();
  } else if (_activeLibrarySubTab === 'workshop') {
    if (masterToggleWrap) masterToggleWrap.style.display = 'flex';
    if (wsCheckUpdatesBtn) wsCheckUpdatesBtn.style.display = 'inline-flex';
    if (bulkBar) bulkBar.style.display = 'none';

    try {
      const wState = await getWorkshopState();

      const masterToggle = document.getElementById('library-workshop-master-toggle') as HTMLInputElement | null;
      if (masterToggle) {
        masterToggle.checked = wState.globalEnabled;
        masterToggle.onchange = async () => {
          showToast(masterToggle.checked ? t('toasts.workshop_enabling') : t('toasts.workshop_disabling'), 'info');
          await setWorkshopGlobalEnabled(masterToggle.checked);
          await renderLibraryView();
          const { loadMods } = await import('../../modsView');
          await loadMods();
          showToast(t('toasts.workshop_state_updated'), 'success');
        };
      }

      let mods = wState.mods;
      if (_librarySearchQuery) {
        mods = mods.filter((m: any) => m.modName.toLowerCase().includes(_librarySearchQuery) || m.author.toLowerCase().includes(_librarySearchQuery));
      }

      if (_libraryFilterStatus === 'installed') {
        mods = mods.filter((m: any) => m.isInstalled || wState.activeModList.includes(m.packageName));
      } else if (_libraryFilterStatus === 'not_installed') {
        mods = mods.filter((m: any) => !m.isInstalled && !wState.activeModList.includes(m.packageName));
      } else if (_libraryFilterStatus === 'updates') {
        mods = mods.filter((m: any) => m.hasPendingUpdate || (m.isInstalled && m.installedVersion && m.installedVersion !== m.version));
      }

      mods.sort((a: any, b: any) => {
        const isInstalledA = a.isInstalled || wState.activeModList.includes(a.packageName);
        const isInstalledB = b.isInstalled || wState.activeModList.includes(b.packageName);

        switch (_librarySortBy) {
          case 'name:asc':
            return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
          case 'name:desc':
            return (b.modName || '').localeCompare(a.modName || '', undefined, { sensitivity: 'base', numeric: true });
          case 'installed:first':
            if (isInstalledA !== isInstalledB) return isInstalledB ? 1 : -1;
            return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
          case 'not_installed:first':
            if (isInstalledA !== isInstalledB) return isInstalledA ? 1 : -1;
            return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
          case 'date:desc':
            return (b.workshopId || 0) - (a.workshopId || 0);
          default:
            return (a.modName || '').localeCompare(b.modName || '');
        }
      });

      if (mods.length === 0) {
        container.innerHTML = `<div id="library-empty">${escapeHtml(t('library.empty_workshop'))}</div>`;
        return;
      }

      container.innerHTML = mods.map((m: any) => {
        let typeClass = 'ue4ss';
        let typeLabel = 'U';
        if (m.installType === 'palSchemaMod') {
          typeClass = 'palschema';
          typeLabel = 'PS';
        }
        const thumb = m.thumbnailPath 
          ? `<div style="width:100%;height:100%;position:relative;"><img src="${convertFileSrc(m.thumbnailPath)}" style="width:100%;height:100%;object-fit:cover;" onerror="this.onerror=null; this.style.display='none'; if (this.nextElementSibling) this.nextElementSibling.style.display='flex';" /><div class="mod-card-image-placeholder ${typeClass}" style="display:none;width:100%;height:100%;align-items:center;justify-content:center;font-weight:bold;font-size:24px;color:#fff;">${typeLabel}</div></div>` 
          : `<div class="mod-card-image-placeholder ${typeClass}" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;font-weight:bold;font-size:24px;color:#fff;">${typeLabel}</div>`;
        const isDepMissing = m.dependencies.some((dep: string) => !wState.activeModList.includes(dep));
        const depWarning = isDepMissing ? `<div style="color:#ff4a4a; font-size:10px; margin-top:2px; text-align:center;">${escapeHtml(t('library.missing_deps_warning', { deps: m.dependencies.join(', ') }))}</div>` : '';

        const isNew = isWorkshopModNew(m.packageName);
        const newBadge = isNew
          ? `<span style="font-size: 7.5px; font-weight: 700; background: linear-gradient(135deg, #00bcff, #38ef7d); color: #000; padding: 2px 5px; border-radius: 8px; box-shadow: 0 0 6px rgba(0,188,255,0.5); letter-spacing: 0.3px; white-space: nowrap;">✨ ${escapeHtml(t('card.badge_new'))}</span>`
          : '';

        const badgeText = m.isFramework ? 'FRAMEWORK' : t('card.badge_workshop');
        const badgeStyle = `font-size: 7.5px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.1)' : 'rgba(255, 157, 0, 0.1)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.2)' : 'rgba(255, 157, 0, 0.2)'}; padding: 2px 4px; border-radius: 3px; white-space: nowrap;`;

        const hasUpdate = m.hasPendingUpdate || (m.isInstalled && m.installedVersion && m.installedVersion !== m.version);
        const updateBadge = hasUpdate
          ? `<span style="font-size: 7.5px; font-weight: 700; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.4); padding: 2px 5px; border-radius: 3px; letter-spacing: 0.2px; white-space: nowrap;">▲ ${escapeHtml(t('card.badge_update_available', { version: m.version }))}</span>`
          : '';

        let versionTextHtml = '';
        if (m.isInstalled) {
          if (hasUpdate) {
            versionTextHtml = `
              <div style="font-size:10px; color:var(--text-muted); text-align:center; display:flex; flex-direction:column; gap:3px;">
                <div>${escapeHtml(t('detail.installed_label'))}: <b style="color:var(--text-primary);">v${escapeHtml(m.installedVersion || '1.0.0')}</b> &bull; Workshop: <b style="color:#00bcff;">v${escapeHtml(m.version)}</b></div>
                <div style="font-size:9px; color:var(--text-muted);">${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})</div>
              </div>`;
          } else {
            versionTextHtml = `
              <div style="font-size:10px; color:var(--text-muted); text-align:center;">
                ${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} <span style="color:#38ef7d; font-weight:600; margin-left:2px;">(${escapeHtml(t('common.installed'))} ✓)</span>
              </div>`;
          }
        } else {
          versionTextHtml = `
            <div style="font-size:10px; color:var(--text-muted); text-align:center;">
              ${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})
            </div>`;
        }

        const toggleBtnText = m.isActive ? t('common.disable') : t('common.enable');
        const toggleBtnClass = m.isActive ? 'btn-action btn-action-danger' : 'btn-primary btn-sm';

        const updateBtn = (hasUpdate && m.isActive)
          ? `<button class="workshop-item-update-btn btn-primary btn-sm" data-package="${escapeHtml(m.packageName)}" style="padding:6px;font-size:10px;cursor:pointer;background:rgba(255, 157, 0, 0.2);color:#ff9d00;border:1px solid rgba(255, 157, 0, 0.4);" title="${escapeHtml(t('library.btn_update_to', { version: m.version }))}">▲ ${escapeHtml(t('library.btn_update_to', { version: m.version }))}</button>`
          : '';

        return `
          <div class="mod-card library-card workshop-card" data-package="${escapeHtml(m.packageName)}" style="position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
            <div style="position:absolute;top:8px;right:8px;z-index:5;display:flex;align-items:center;gap:3px;max-width:calc(100% - 16px);flex-wrap:wrap;justify-content:flex-end;">
              <span style="${badgeStyle}">${badgeText}</span>
              ${updateBadge}
              ${newBadge}
            </div>
            <div style="padding-top:16px;display:flex;flex-direction:column;gap:8px;height:100%;justify-content:space-between;min-height:160px;">
              <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
                ${thumb}
              </div>
              <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:center;word-break:break-word;line-height:1.3;flex:1;min-height:36px;display:flex;align-items:center;justify-content:center;margin-top:4px;">
                ${escapeHtml(m.modName)}
              </div>
              ${versionTextHtml}
              ${depWarning}
              ${updateBtn ? `<div style="display:flex;flex-direction:column;margin-top:2px;">${updateBtn}</div>` : ''}
              <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
                <button class="workshop-item-toggle-btn ${toggleBtnClass}" data-package="${escapeHtml(m.packageName)}" data-active="${m.isActive}" ${m.isFramework ? 'disabled style="opacity:0.5;"' : ''} style="flex:1;padding:6px;font-size:10px;cursor:pointer;">
                  ${toggleBtnText}
                </button>
                <button class="workshop-item-folder-btn btn-secondary btn-sm" data-path="${escapeHtml(wState.workshopRoot + '/' + m.workshopId)}" style="padding:6px 8px;font-size:10px;cursor:pointer;" title="${escapeHtml(t('library.workshop_open_folder_title'))}">
                  📁 ${escapeHtml(t('common.folder'))}
                </button>
              </div>
            </div>
          </div>
        `;
      }).join('');

      container.querySelectorAll('.workshop-item-update-btn').forEach(btn => {
        btn.addEventListener('click', async (e) => {
          const target = e.currentTarget as HTMLButtonElement;
          const pkgName = target.dataset.package!;
          target.disabled = true;
          showToast(t('toasts.preparing_workshop_update'), 'info');
          try {
            await activateWorkshopMod(pkgName);
            showToast(t('toasts.mod_updated', { name: pkgName }), 'success');
          } catch (err) {
            showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          } finally {
            target.disabled = false;
            await renderLibraryView();
            const { loadMods } = await import('../../modsView');
            await loadMods();
          }
        });
      });

      container.querySelectorAll('.workshop-item-toggle-btn').forEach(btn => {
        btn.addEventListener('click', async (e) => {
          const target = e.currentTarget as HTMLButtonElement;
          const pkgName = target.dataset.package!;
          const isActive = target.dataset.active === 'true';

          target.disabled = true;
          showToast(!isActive ? t('toasts.workshop_activating') : t('toasts.workshop_deactivating'), 'info');
          try {
            if (!isActive) {
              await activateWorkshopMod(pkgName);
            } else {
              await deactivateWorkshopMod(pkgName);
            }
            showToast(!isActive ? t('toasts.workshop_activated') : t('toasts.workshop_deactivated'), 'success');
          } catch (err) {
            showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          } finally {
            target.disabled = false;
            await renderLibraryView();
            const { loadMods } = await import('../../modsView');
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
            showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          }
        });
      });

    } catch (err) {
      container.innerHTML = `<div style="color:#ff4a4a; padding:12px; text-align:center;">Failed to load Workshop state: ${escapeHtml(String(err))}</div>`;
    }
  }
}
