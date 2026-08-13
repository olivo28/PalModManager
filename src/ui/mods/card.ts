import type { ModInfo } from '../../types';
import { escapeHtml } from '../../utils/helpers';
import { convertFileSrc } from '@tauri-apps/api/core';

export function isVersionNewer(local: string, remote: string): boolean {
  const localParts = local.split('.').map(p => parseInt(p.replace(/[^0-9]/g, ''), 10) || 0);
  const remoteParts = remote.split('.').map(p => parseInt(p.replace(/[^0-9]/g, ''), 10) || 0);
  const maxLen = Math.max(localParts.length, remoteParts.length);
  for (let i = 0; i < maxLen; i++) {
    const l = localParts[i] || 0;
    const r = remoteParts[i] || 0;
    if (r > l) return true;
    if (l > r) return false;
  }
  return false;
}

export function computeAvailableUpdates(mods: ModInfo[]): Map<string, string> {
  const updatesMap = new Map<string, string>();
  for (const m of mods) {
    if (m.hasPendingUpdate && m.nexusVersionCached) {
      updatesMap.set(m.id, m.nexusVersionCached);
      continue;
    }
    if (m.nexusVersionCached && m.version) {
      const normNexus = m.nexusVersionCached.replace(/^v/i, '').trim().toLowerCase();
      const normLocal = m.version.replace(/^v/i, '').trim().toLowerCase();
      const normIgnored = m.ignoredVersion ? m.ignoredVersion.replace(/^v/i, '').trim().toLowerCase() : '';
      if (normNexus !== '' && normNexus !== 'unknown' && normLocal !== 'unknown' && isVersionNewer(normLocal, normNexus) && normNexus !== normIgnored) {
        updatesMap.set(m.id, m.nexusVersionCached);
      }
    }
  }
  return updatesMap;
}

export function buildModCardHtml(mod: ModInfo, state: any): string {
  const isWorkshop = !!(mod.nexusSummary && mod.nexusSummary.startsWith('Steam Workshop Mod'));
  const updateVer = state.availableUpdates?.get(mod.id);

  if (state.viewLayout === 'list') {
    const isSelected = state.selectedModIds.has(mod.id);
    const shortPath = mod.gamePath ? mod.gamePath.replace(/\\/g, '/').split('/').slice(-3).join('/') : '';
    const extraCount = mod.extraFiles ? mod.extraFiles.length : 0;
    const extraText = extraCount > 0 ? `+${extraCount} file${extraCount === 1 ? '' : 's'}` : 'None';
    const formattedDate = mod.installDate ? mod.installDate.substring(0, 10) : 'Unknown';

    const removeBtn = isWorkshop
      ? `<span style="font-size: 10px; color: var(--text-muted); opacity: 0.6; font-weight: bold; text-transform: uppercase;">Workshop</span>`
      : `<button class="card-remove-btn" data-id="${mod.id}" title="Remove mod">✕</button>`;

    return `
    <div class="mod-card list-row-card ${mod.enabled ? '' : 'disabled'} ${isSelected ? 'selected' : ''}" data-id="${mod.id}" data-type="${mod.type}" data-is-workshop="${isWorkshop}">
      <div class="cell name-cell">
        <label class="toggle-switch">
          <input type="checkbox" class="card-toggle-input" data-id="${mod.id}" ${mod.enabled ? 'checked' : ''} />
          <span class="toggle-slider"></span>
        </label>
        <span class="mod-card-led ${mod.enabled ? 'on' : 'off'}"></span>
        <span class="mod-card-name" style="font-weight:600;">${escapeHtml(mod.name)}</span>
        ${isWorkshop ? `<span style="margin-left: 8px; font-size: 8px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 1px 4px; border-radius: 3px;">WORKSHOP</span>` : ''}
        ${updateVer ? `<span style="margin-left: 8px; font-size: 8px; font-weight: bold; background: rgba(0, 188, 255, 0.15); color: #00bcff; border: 1px solid rgba(0, 188, 255, 0.3); padding: 1px 4px; border-radius: 3px;">UPDATE AVAILABLE (v${escapeHtml(updateVer)})</span>` : ''}
      </div>
      <div class="cell status-cell">
      </div>
      <div class="cell type-cell">
        <span class="mod-card-type ${mod.type}">${mod.type}</span>
      </div>
      <div class="cell version-cell">
        <span class="mod-card-version">v${escapeHtml(mod.version)}</span>
      </div>
      <div class="cell path-cell" title="${escapeHtml(mod.gamePath)}">
        <span class="mod-card-path">${escapeHtml(shortPath || 'Not active')}</span>
      </div>
      <div class="cell extra-cell" title="${mod.extraFiles ? escapeHtml(mod.extraFiles.join('\n')) : ''}">
        <span class="mod-card-extra">${escapeHtml(extraText)}</span>
      </div>
      <div class="cell date-cell">
        <span class="mod-card-date">${escapeHtml(formattedDate)}</span>
      </div>
      <div class="cell action-cell">
        ${removeBtn}
      </div>
    </div>`;
  }

  const tags = mod.nexusTags && mod.nexusTags.length > 0
    ? `<div class="mod-card-tags">${mod.nexusTags.slice(0, 3).map(t => `<span class="mod-card-tag">${escapeHtml(t)}</span>`).join('')}</div>`
    : '';
  const catHtml = mod.nexusCategory ? `<span class="mod-card-category">${escapeHtml(mod.nexusCategory)}</span>` : '';
  const author = mod.nexusAuthor ? `<span class="mod-card-author">by ${escapeHtml(mod.nexusAuthor)}</span>` : '';
  let imageSrc = '';
  if (mod.nexusPictureUrl) {
    if (mod.nexusPictureUrl.startsWith('http://') || mod.nexusPictureUrl.startsWith('https://')) {
      imageSrc = mod.nexusPictureUrl;
    } else {
      try {
        imageSrc = convertFileSrc(mod.nexusPictureUrl);
      } catch (e) {
        console.error('Failed to convert file src:', e);
        imageSrc = '';
      }
    }
  }

  const imageHtml = imageSrc
    ? `<div class="mod-card-image-wrap"><img class="mod-card-image" src="${escapeHtml(imageSrc)}" alt="" loading="lazy" /></div>`
    : `<div class="mod-card-image-wrap"><div class="mod-card-image-placeholder ${mod.type}">${mod.type === 'ue4ss' ? 'U' : mod.type === 'palschema' ? 'PS' : mod.type === 'pak' ? 'PK' : 'LM'}</div></div>`;

  const updateBadge = updateVer
    ? `<span class="mod-card-update-badge" title="Update available to v${escapeHtml(updateVer)}">&#9650; Update (v${escapeHtml(updateVer)})</span>`
    : '';

  const removeBtn = isWorkshop
    ? `<span style="font-size: 9px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 2px 6px; border-radius: 4px; text-transform: uppercase;">Workshop</span>`
    : `<button class="card-remove-btn" data-id="${mod.id}" title="Remove mod">✕</button>`;

  const isSelected = state.selectedModIds.has(mod.id);

  return `
  <div class="mod-card ${mod.enabled ? '' : 'disabled'} ${isSelected ? 'selected' : ''}" data-id="${mod.id}" data-type="${mod.type}" data-is-workshop="${isWorkshop}">
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
        ${isWorkshop ? `<span style="margin-left: 4px; font-size: 8px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 1px 4px; border-radius: 3px;">WORKSHOP</span>` : ''}
      </div>
      ${author}
      ${tags}
    </div>
    <div class="mod-card-footer">
      <label class="toggle-switch">
        <input type="checkbox" class="card-toggle-input" data-id="${mod.id}" ${mod.enabled ? 'checked' : ''} />
        <span class="toggle-slider"></span>
      </label>
      ${removeBtn}
    </div>
  </div>`;
}

export function buildFolderCardHtml(folder: any, modsInFolder: ModInfo[], state: any): string {
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);
  const folderCheckbox = `<label class="toggle-switch" title="Toggle all mods in this folder" onclick="event.stopPropagation()">
    <input type="checkbox" class="folder-toggle-input" data-folder-id="${folder.id}" ${allEnabled ? 'checked' : ''} />
    <span class="toggle-slider"></span>
  </label>`;
  const isSelected = state.selectedModIds.has(folder.id);

  if (state.viewLayout === 'list') {
    return `
    <div class="mod-card folder-card list-row-card ${isSelected ? 'selected' : ''}" data-id="${folder.id}" data-type="folder">
      <div class="cell name-cell">
        <span style="margin-right: 8px;">📁</span>
        <span class="mod-card-name">${escapeHtml(folder.name)}</span>
      </div>
      <div class="cell status-cell">
        ${folderCheckbox}
      </div>
      <div class="cell type-cell">
        <span class="mod-card-type" style="color: var(--text-muted); border-color: var(--border);">Folder</span>
      </div>
      <div class="cell version-cell">
        <span>-</span>
      </div>
      <div class="cell path-cell">
        <span>-</span>
      </div>
      <div class="cell extra-cell">
        <span>${modsInFolder.length} mod${modsInFolder.length === 1 ? '' : 's'}</span>
      </div>
      <div class="cell date-cell">
        <span>-</span>
      </div>
      <div class="cell action-cell" onclick="event.stopPropagation()">
        <button class="mod-folder-btn rename-btn" style="background:none;border:none;color:var(--text-secondary);cursor:pointer;padding:4px;" data-folder-id="${folder.id}" title="Rename folder">✏</button>
        <button class="mod-folder-btn delete-btn delete" style="background:none;border:none;color:var(--text-secondary);cursor:pointer;padding:4px;" data-folder-id="${folder.id}" title="Delete folder">✕</button>
      </div>
    </div>`;
  }

  return `
  <div class="mod-card folder-card ${isSelected ? 'selected' : ''}" data-id="${folder.id}" data-type="folder" style="position:relative;">
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
  </div>`;
}
