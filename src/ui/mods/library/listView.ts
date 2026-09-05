import { getState, updateState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { convertFileSrc } from '@tauri-apps/api/core';
import { formatSize, compareVersions } from './helpers';
import { _libraryOnlineUpdatesMap, type LibraryGroup } from './state';
import { triggerInstallFromLibrary, updateLibraryBulkBar, handleLibraryDelete } from './actions';

const _expandedLibraryIds = new Set<string>();

export function renderLibraryListView(groups: LibraryGroup[], container: HTMLElement): void {
  const state = getState();

  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.style.gap = '6px';
  container.style.alignContent = 'start';
  container.style.justifyContent = 'flex-start';
  container.style.overflowY = 'auto';
  container.style.minHeight = '0';
  container.style.height = '100%';
  container.style.gridTemplateColumns = '';

  container.innerHTML = groups.map(group => {
    const isSelected = state.selectedLibraryIds.has(group.modId);
    const isExpanded = _expandedLibraryIds.has(group.modId);
    const latestVerObj = group.versions[0];
    const latestVersion = latestVerObj ? latestVerObj.version : '1.0';
    const cleanName = group.name;
    const author = group.author;
    const description = group.description;
    const modType = group.modType;

    let statusBadgeHtml = '';
    let installBtnText = t('common.install');
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

    const onlineNexusVer = _libraryOnlineUpdatesMap.get(group.modId);
    const onlineUpdateBadge = onlineNexusVer
      ? `<span style="font-size: 8px; font-weight: 700; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.4); padding: 1px 4px; border-radius: 3px; white-space: nowrap;">▲ v${escapeHtml(onlineNexusVer)}</span>`
      : '';

    // Resolve picture URL
    let resolvedSrc = group.nexusPictureUrl;
    if (!resolvedSrc) {
      const matchedMod = state.allMods.find(m => {
        if (m.name.toLowerCase() === group.modId.toLowerCase()) return true;
        if (m.nexusModId && group.nexusModId && m.nexusModId === group.nexusModId) return true;
        if (m.name.toLowerCase() === cleanName.toLowerCase()) return true;
        return false;
      });
      if (matchedMod?.nexusPictureUrl) {
        resolvedSrc = matchedMod.nexusPictureUrl;
      }
    }

    let displayImgSrc = resolvedSrc || '';
    if (displayImgSrc && !displayImgSrc.startsWith('http://') && !displayImgSrc.startsWith('https://')) {
      try { displayImgSrc = convertFileSrc(displayImgSrc); } catch (err) { console.error(err); }
    }

    const miniThumbHtml = displayImgSrc
      ? `<div style="width:32px;height:32px;border-radius:4px;border:1px solid var(--border);overflow:hidden;background:var(--bg-primary);flex-shrink:0;display:flex;align-items:center;justify-content:center;">
           <img src="${displayImgSrc}" style="width:100%;height:100%;object-fit:cover;" onerror="this.style.display='none';this.nextElementSibling&&(this.nextElementSibling.style.display='flex');" />
           <div style="display:none;font-size:14px;opacity:0.6;">📦</div>
         </div>`
      : `<div style="width:32px;height:32px;border-radius:4px;border:1px solid var(--border);background:var(--bg-primary);flex-shrink:0;display:flex;align-items:center;justify-content:center;font-size:14px;opacity:0.6;">📦</div>`;

    const versionSelectHtml = group.versions.length > 1
      ? `<select class="library-version-select form-select" data-id="${group.modId}" style="padding:3px 6px;font-size:11px;font-weight:600;background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;color:var(--text-primary);cursor:pointer;outline:none;max-width:110px;">
           ${group.versions.map((v, idx) => {
             const displayVer = v.version.startsWith('v') || v.version.startsWith('V') ? v.version : `v${v.version}`;
             return `<option value="${escapeHtml(v.zipName)}" data-version="${escapeHtml(v.version)}" data-size="${formatSize(v.zipSize)}">${escapeHtml(displayVer)} ${idx === 0 ? escapeHtml(t('library.badge_latest')) : ''}</option>`;
           }).join('')}
         </select>
         <span class="library-card-size" style="font-size:10px;color:var(--text-muted);white-space:nowrap;">${formatSize(latestVerObj.zipSize)}</span>`
      : `<span style="font-weight:600;font-size:11px;color:var(--text-primary);">${latestVersion.startsWith('v') || latestVersion.startsWith('V') ? escapeHtml(latestVersion) : 'v' + escapeHtml(latestVersion)}</span>
         <span class="library-card-size" style="font-size:10px;color:var(--text-muted);white-space:nowrap;">${formatSize(latestVerObj.zipSize)}</span>`;

    // Accordion Drawer Content
    const drawerImageHtml = displayImgSrc
      ? `<img class="library-drawer-preview" src="${displayImgSrc}" onerror="this.style.display='none';" />`
      : `<div class="library-drawer-preview" style="display:flex;align-items:center;justify-content:center;font-size:28px;opacity:0.4;">📦</div>`;

    return `
      <div class="mod-card library-card library-list-item ${isSelected ? 'selected' : ''}" data-id="${group.modId}" data-is-installed="${group.isInstalled}" data-installed-version="${escapeHtml(group.installedVersion || '')}">
        <!-- Compact Summary Row -->
        <div class="library-list-row" data-id="${group.modId}">
          <input type="checkbox" class="library-card-checkbox" data-id="${group.modId}" ${isSelected ? 'checked' : ''} />
          
          <button class="library-row-toggle-btn" data-id="${group.modId}" title="${escapeHtml(t('library.unroll_details'))}">
            ${isExpanded ? '▼' : '▶'}
          </button>

          ${miniThumbHtml}

          <div class="library-row-info">
            <span class="mod-card-name library-name-click" data-id="${group.modId}" title="${escapeHtml(cleanName)}">
              ${escapeHtml(cleanName)}
            </span>
            ${modType ? `<span class="library-type-tag ${modType.toLowerCase()}">${modType}</span>` : ''}
            ${statusBadgeHtml}
            ${onlineUpdateBadge}
          </div>

          <div class="library-row-actions">
            <div class="library-row-version-wrap">
              ${versionSelectHtml}
            </div>
            <button class="library-item-install btn-action ${isUpdateAvailable ? 'btn-action-primary' : ''}" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}">
              ${installBtnText}
            </button>
            <button class="library-item-delete btn-action btn-action-danger" data-id="${group.modId}" data-zip="${escapeHtml(latestVerObj.zipName)}" title="${escapeHtml(t('library.btn_delete_ver'))}">
              ✕
            </button>
          </div>
        </div>

        <!-- Accordion Drawer (Unrolls image & details) -->
        <div class="library-row-drawer" data-id="${group.modId}" style="display:${isExpanded ? 'flex' : 'none'};">
          ${drawerImageHtml}
          <div class="library-drawer-content">
            <div class="library-drawer-meta">
              ${author ? `<span><strong style="color:var(--text-secondary);">${escapeHtml(t('common.author'))}:</strong> ${escapeHtml(author)}</span>` : ''}
              <span><strong style="color:var(--text-secondary);">File:</strong> ${escapeHtml(latestVerObj.zipName)}</span>
              ${latestVerObj.installedAt ? `<span><strong style="color:var(--text-secondary);">Added:</strong> ${escapeHtml(latestVerObj.installedAt.substring(0, 10))}</span>` : ''}
            </div>
            ${description ? `<div class="library-drawer-desc">${escapeHtml(description)}</div>` : ''}
          </div>
        </div>
      </div>
    `;
  }).join('');

  // Attach interactive listeners for List View
  attachListViewListeners(container);
}

function attachListViewListeners(container: HTMLElement): void {
  // Toggle button click to expand/collapse drawer
  container.querySelectorAll('.library-row-toggle-btn').forEach(btn => {
    btn.addEventListener('click', (ev) => {
      ev.stopPropagation();
      const id = (btn as HTMLElement).dataset.id;
      if (!id) return;
      toggleDrawer(id, container);
    });
  });

  // Clicking row (outside controls/checkboxes) toggles drawer
  container.querySelectorAll('.library-list-row').forEach(row => {
    row.addEventListener('click', (ev) => {
      const target = ev.target as HTMLElement;
      if (
        target.closest('.library-card-checkbox') ||
        target.closest('.library-item-install') ||
        target.closest('.library-item-delete') ||
        target.closest('.library-version-select') ||
        target.closest('select') ||
        target.closest('input')
      ) {
        return;
      }
      const id = (row as HTMLElement).dataset.id;
      if (!id) return;
      toggleDrawer(id, container);
    });
  });

  // Checkbox selection: sync state on direct change
  container.querySelectorAll('.library-card-checkbox').forEach(cb => {
    cb.addEventListener('change', () => {
      const id = (cb as HTMLElement).dataset.id!;
      const state = getState();
      const newSet = new Set(state.selectedLibraryIds);
      if ((cb as HTMLInputElement).checked) {
        newSet.add(id);
      } else {
        newSet.delete(id);
      }
      updateState({ selectedLibraryIds: newSet });
      const itemEl = cb.closest('.library-card');
      if (itemEl) {
        itemEl.classList.toggle('selected', (cb as HTMLInputElement).checked);
      }
      updateLibraryBulkBar();
    });
  });

  // Version selector
  container.querySelectorAll('.library-version-select').forEach(sel => {
    sel.addEventListener('click', (ev) => ev.stopPropagation());
    sel.addEventListener('change', (ev) => {
      ev.stopPropagation();
      const select = ev.target as HTMLSelectElement;
      const item = select.closest('.library-list-item') as HTMLElement;
      if (!item) return;

      const selectedZip = select.value;
      const selectedOption = select.selectedOptions[0];
      const selectedVer = selectedOption?.dataset.version || '';
      const selectedSize = selectedOption?.dataset.size || '';

      const installBtn = item.querySelector('.library-item-install') as HTMLButtonElement | null;
      const deleteBtn = item.querySelector('.library-item-delete') as HTMLButtonElement | null;
      const statusBadge = item.querySelector('.library-status-badge') as HTMLElement | null;
      const sizeSpan = item.querySelector('.library-card-size') as HTMLElement | null;
      const installedVer = item.dataset.installedVersion || '';
      const isInstalled = item.dataset.isInstalled === 'true';

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
          statusBadge.className = 'library-status-badge badge-success';
          statusBadge.textContent = t('library.status_installed_exact', { version: installedVer });
        } else {
          statusBadge.className = 'library-status-badge badge-muted';
          statusBadge.textContent = t('library.status_not_installed');
        }
      }
    });
  });

  // Install button
  container.querySelectorAll('.library-item-install').forEach(btn => {
    btn.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const zip = (btn as HTMLElement).dataset.zip;
      await triggerInstallFromLibrary(id, zip);
    });
  });

  // Delete button
  container.querySelectorAll('.library-item-delete').forEach(btn => {
    btn.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const zip = (btn as HTMLElement).dataset.zip!;
      const item = btn.closest('.library-list-item') as HTMLElement | null;
      const isInstalled = item?.dataset.isInstalled === 'true';
      await handleLibraryDelete(id, zip, isInstalled);
    });
  });
}

function toggleDrawer(id: string, container: HTMLElement): void {
  const drawer = container.querySelector(`.library-row-drawer[data-id="${id}"]`) as HTMLElement | null;
  const toggleBtn = container.querySelector(`.library-row-toggle-btn[data-id="${id}"]`) as HTMLElement | null;
  if (!drawer) return;

  if (_expandedLibraryIds.has(id)) {
    _expandedLibraryIds.delete(id);
    drawer.style.display = 'none';
    if (toggleBtn) toggleBtn.textContent = '▶';
  } else {
    _expandedLibraryIds.add(id);
    drawer.style.display = 'flex';
    if (toggleBtn) toggleBtn.textContent = '▼';
  }
}
