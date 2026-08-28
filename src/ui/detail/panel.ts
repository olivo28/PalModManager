import { setNexusModId, openUrl, changePakDestination } from '../../api';
import { getState, updateState } from '../../state';
import { convertFileSrc } from '@tauri-apps/api/core';
import { loadMods, renderModsView } from '../modsView';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { descriptionToHtml } from '../../utils/bbcode';
import { isModMissingGamePass } from '../mods/card';
import { formatDisplayPath, getModComponentFolders } from './helpers';
import { autoFetchNexusInfo, setupNexusIdEdit } from './nexus';
import { renderVersion, renderGithubSection, setupDetailTabs } from './sections';
import { detailDom } from '../../framework';

export function openDetailPanel(modId: string): void {
  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);
  if (!mod) return;
  updateState({ currentDetailMod: mod });

  const panel = detailDom.el('detail-panel');
  panel.dataset.id = modId;

  detailDom.el('detail-name-header').textContent = mod.name;
  const typeLabel = mod.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : mod.type.toUpperCase();
  detailDom.el('detail-type').textContent = typeLabel;
  detailDom.el('detail-type').className = `mod-type-badge ${mod.type}`;
  renderVersion(mod);
  detailDom.el('detail-status').textContent = mod.enabled ? t('common.enabled') : t('common.disabled');
  detailDom.el('detail-status').className = `detail-status ${mod.enabled ? 'enabled' : 'disabled'}`;

  const toggleBtn = detailDom.el('detail-toggle');
  toggleBtn.textContent = mod.enabled ? t('common.disabled') : t('common.enabled');
  toggleBtn.dataset.enabled = String(mod.enabled);

  const nexusSection = detailDom.el('detail-nexus');
  const descSection = detailDom.el('detail-description');
  const imgEl = detailDom.el('detail-image');
  const imgContainer = detailDom.el('detail-image-container');

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
    imgEl.setAttribute('data-original-src', mod.nexusPictureUrl);
    imgEl.onerror = () => {
      if ((window as any).handleUniversalImageFallback) {
        (window as any).handleUniversalImageFallback(imgEl);
      } else {
        imgContainer.style.display = 'none';
      }
    };
    imgContainer.style.display = 'block';
  } else {
    imgContainer.style.display = 'none';
  }

  if (mod.nexusDescription) {
    descSection.innerHTML = descriptionToHtml(mod.nexusDescription);
    descSection.querySelectorAll('img').forEach((descImg) => {
      descImg.setAttribute('data-original-src', descImg.src);
      descImg.onerror = () => {
        if ((window as any).handleUniversalImageFallback) {
          (window as any).handleUniversalImageFallback(descImg);
        }
      };
    });
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
  detailDom.el('detail-install-date').textContent = formattedInstallDate;
  detailDom.el('detail-source-zip').textContent = mod.sourceZip || t('common.none');

  // Technical Component rows & Dynamic Action Buttons
  const componentsContainer = detailDom.elMaybe('detail-components-container');
  const folderButtonsContainer = detailDom.elMaybe('detail-folder-buttons');
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
            const { openPath } = await import('../../api');
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
  const duplicateRow = detailDom.el('detail-duplicate-row');
  const duplicateWarning = detailDom.el('detail-duplicate-warning');
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

  // Game Pass IoStore Compatibility Check & Conversion
  const gpWarningRow = detailDom.elMaybe('detail-gamepass-warning-row');
  const gpConvertBtn = detailDom.elMaybe('detail-convert-gamepass-btn');
  const isMissingGp = isModMissingGamePass(mod, state);

  if (gpWarningRow && gpConvertBtn) {
    if (isMissingGp) {
      gpWarningRow.style.display = '';
      gpConvertBtn.disabled = false;
      gpConvertBtn.onclick = async () => {
        try {
          gpConvertBtn.disabled = true;
          gpConvertBtn.innerHTML = `<span>⏳</span> <span>${escapeHtml(t('scanner.converting_gamepass'))}</span>`;
          const { convertModToGamepass } = await import('../../api');
          const genFiles = await convertModToGamepass(mod.id);
          showToast(t('toasts.convert_gamepass_success', { name: mod.name, count: genFiles.length }), 'success');
          await loadMods();
          openDetailPanel(mod.id);
        } catch (err: any) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          gpConvertBtn.disabled = false;
          gpConvertBtn.innerHTML = `<span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_gamepass'))}</span>`;
        }
      };
    } else {
      gpWarningRow.style.display = 'none';
    }
  }

  const configPathEl = detailDom.el('detail-config-path');
  const configRow = configPathEl.closest('.detail-row') as HTMLElement;
  const isPakType = mod.type === 'pak' || mod.type === 'logicmods';

  const pakDestRow = detailDom.el('detail-pak-destination-row');
  const pakDestSelect = detailDom.el('detail-pak-destination-select');

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

  // Render Pak Contents Inspection for .pak / logicmods / hybrid mods with pak
  const pakContentsContainer = detailDom.elMaybe('detail-pak-contents-container');
  const hasPakFiles = mod.gamePath.toLowerCase().endsWith('.pak') || mod.extraFiles.some(f => f.toLowerCase().endsWith('.pak'));

  if (pakContentsContainer) {
    if (hasPakFiles) {
      pakContentsContainer.style.display = 'flex';
      pakContentsContainer.innerHTML = `
        <div style="background:var(--bg-secondary); border:1px solid var(--border); border-radius:6px; padding:10px 12px; display:flex; flex-direction:column; gap:8px;">
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <div style="display:flex; align-items:center; gap:6px;">
              <span style="font-size:14px;">📦</span>
              <span style="font-size:12px; font-weight:700; color:var(--text-primary);">${escapeHtml(t('detail.pak_contents_title') || 'Internal .pak Contents')}</span>
            </div>
            <button type="button" id="detail-load-pak-contents-btn" class="btn-tiny" style="font-size:10.5px; padding:3px 8px; border-color:var(--accent); color:var(--accent);">
              🔍 ${escapeHtml(t('detail.btn_load_pak_contents') || 'Inspect .pak Assets')}
            </button>
          </div>
          <div id="detail-pak-contents-body" style="display:none; flex-direction:column; gap:8px; margin-top:4px;"></div>
        </div>
      `;

      const loadPakBtn = document.getElementById('detail-load-pak-contents-btn') as HTMLButtonElement | null;
      const pakBodyEl = document.getElementById('detail-pak-contents-body') as HTMLElement | null;

      if (loadPakBtn && pakBodyEl) {
        loadPakBtn.addEventListener('click', async () => {
          if (pakBodyEl.style.display === 'flex') {
            pakBodyEl.style.display = 'none';
            loadPakBtn.textContent = `🔍 ${t('detail.btn_load_pak_contents') || 'Inspect .pak Assets'}`;
            return;
          }

          loadPakBtn.disabled = true;
          loadPakBtn.innerHTML = `<span>⏳</span> <span>${escapeHtml(t('scanner.loading') || 'Loading...')}</span>`;

          try {
            const { inspectModPakContents } = await import('../../api');
            const pakResults = await inspectModPakContents(mod.id);

            loadPakBtn.disabled = false;
            loadPakBtn.textContent = `🔼 ${t('common.hide') || 'Hide'}`;
            pakBodyEl.style.display = 'flex';

            pakBodyEl.innerHTML = `
              <input type="text" id="detail-pak-search-input" placeholder="${escapeHtml(t('scanner.search_placeholder') || 'Search internal assets...')}" style="width:100%; background:var(--bg-primary); border:1px solid var(--border); color:var(--text-primary); border-radius:4px; padding:4px 8px; font-size:11px; outline:none;" />
              <div id="detail-pak-results-list" style="display:flex; flex-direction:column; gap:8px; max-height:260px; overflow-y:auto; padding-right:2px;">
                ${pakResults.map(p => `
                  <div style="background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:6px 8px; display:flex; flex-direction:column; gap:4px;">
                    <div style="display:flex; justify-content:space-between; align-items:center; flex-wrap:wrap; gap:4px;">
                      <span style="font-family:monospace; font-size:11px; font-weight:700; color:var(--accent);">📦 ${escapeHtml(p.pakName)}</span>
                      <span class="badge" style="font-size:9px; padding:1px 5px; background:rgba(0,188,255,0.15); color:#00bcff; border:1px solid rgba(0,188,255,0.3);">${p.totalFiles} ${escapeHtml(t('scanner.assets_count_label') || 'assets')}</span>
                    </div>
                    <div class="detail-pak-file-list" style="display:flex; flex-direction:column; gap:2px; margin-top:2px;">
                      ${p.files.map(f => `
                        <div class="detail-pak-item-row" data-search="${escapeHtml((f.name + ' ' + f.path + ' ' + f.assetType).toLowerCase())}" style="display:flex; justify-content:space-between; align-items:center; padding:3px 6px; border-radius:3px; font-family:monospace; font-size:10px; background:rgba(255,255,255,0.02);">
                          <span style="color:var(--text-primary); text-overflow:ellipsis; overflow:hidden; white-space:nowrap; max-width:68%;" title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span>
                          <div style="display:flex; align-items:center; gap:5px;">
                            <span style="font-size:8.5px; padding:1px 4px; border-radius:2px; background:rgba(255,255,255,0.06); color:var(--text-secondary);">${escapeHtml(f.assetType)}</span>
                            ${f.path.toLowerCase().endsWith('.uasset') || f.path.toLowerCase().endsWith('.uexp') ? `
                              <button class="btn-inspect-uasset-action" data-mod="${escapeHtml(mod.id)}" data-path="${escapeHtml(f.path)}" title="${escapeHtml(t('scanner.uasset_btn_inspect') || 'Deep Inspect Asset')}" style="background:rgba(0,188,255,0.15); border:1px solid rgba(0,188,255,0.3); border-radius:3px; color:var(--accent); cursor:pointer; font-size:9.5px; padding:1px 5px; display:flex; align-items:center; gap:2px;">
                                <span>🔍</span>
                              </button>
                            ` : ''}
                          </div>
                        </div>
                      `).join('')}
                    </div>
                  </div>
                `).join('')}
              </div>
            `;

            const searchInput = document.getElementById('detail-pak-search-input') as HTMLInputElement | null;
            if (searchInput) {
              searchInput.addEventListener('input', () => {
                const q = searchInput.value.trim().toLowerCase();
                const itemRows = pakBodyEl.querySelectorAll('.detail-pak-item-row');
                itemRows.forEach(row => {
                  const txt = (row as HTMLElement).dataset.search || '';
                  (row as HTMLElement).style.display = (!q || txt.includes(q)) ? 'flex' : 'none';
                });
              });
            }

            pakBodyEl.querySelectorAll('.btn-inspect-uasset-action').forEach(btn => {
              btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const modId = (btn as HTMLElement).dataset.mod;
                const assetPath = (btn as HTMLElement).dataset.path;
                if (!assetPath) return;
                import('../modals/uassetInspector').then(({ openUAssetInspectorModal }) => {
                  openUAssetInspectorModal({
                    modId: modId || null,
                    assetInternalPath: assetPath,
                  });
                });
              });
            });
          } catch (err: any) {
            loadPakBtn.disabled = false;
            loadPakBtn.textContent = `🔍 ${t('detail.btn_load_pak_contents') || 'Inspect .pak Assets'}`;
            pakBodyEl.style.display = 'flex';
            pakBodyEl.innerHTML = `<span style="font-size:11px; color:var(--danger);">⚠️ ${escapeHtml(String(err))}</span>`;
          }
        });
      }
    } else {
      pakContentsContainer.style.display = 'none';
      pakContentsContainer.innerHTML = '';
    }
  }

  // Populate Folder Dropdown
  const folderSelect = detailDom.elMaybe('detail-folder-select');
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
        const { addModToFolder } = await import('../../api');
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
  detailDom.el('detail-body').scrollTop = 0;
  detailDom.queryAll('.detail-tab').forEach(t => t.classList.remove('active'));
  detailDom.query('.detail-tab[data-tab="info"]')?.classList.add('active');
  detailDom.el('detail-info-tab').style.display = '';
  detailDom.el('detail-tech-tab').style.display = 'none';

  detailDom.el('detail-overlay').classList.add('visible');
  setupDetailTabs();
}

// Intercept NexusMods/GitHub link clicks inside the detail panel to open in default browser
document.addEventListener('click', (e) => {
  const link = (e.target as HTMLElement).closest('.nexus-link') as HTMLAnchorElement | null;
  if (link && link.href && detailDom.elMaybe('detail-panel')?.contains(link)) {
    e.preventDefault();
    openUrl(link.href).catch(err => console.error('Failed to open link:', err));
  }
});

