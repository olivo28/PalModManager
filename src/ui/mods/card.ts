import type { ModInfo } from '../../types';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { convertFileSrc } from '@tauri-apps/api/core';

export async function handleCardImageError(img: HTMLImageElement, originalUrl: string): Promise<void> {
  img.onerror = null;
  if (!originalUrl || !originalUrl.startsWith('http')) {
    img.style.display = 'none';
    if (img.nextElementSibling) (img.nextElementSibling as HTMLElement).style.display = 'flex';
    return;
  }
  try {
    const { fetchAndCacheImage } = await import('../../api');
    const cachedPath = await fetchAndCacheImage(originalUrl);
    if (cachedPath) {
      img.src = convertFileSrc(cachedPath);
      return;
    }
  } catch (e) {
    console.error('Failed to proxy fetch card image:', e);
  }
  img.style.display = 'none';
  if (img.nextElementSibling) (img.nextElementSibling as HTMLElement).style.display = 'flex';
}

if (typeof window !== 'undefined') {
  (window as any).handleCardImageError = handleCardImageError;
}

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

export function computeAvailableUpdates(mods: ModInfo[], libraryEntries?: any[]): Map<string, string> {
  const updatesMap = new Map<string, string>();
  for (const m of mods) {
    const hasUp = m.hasPendingUpdate || (m as any).has_pending_update;
    const cachedVer = m.nexusVersionCached || (m as any).nexus_version_cached;
    if (hasUp && cachedVer) {
      updatesMap.set(m.id, cachedVer);
      continue;
    }
    if (m.nexusVersionCached && m.version) {
      const normNexus = m.nexusVersionCached.replace(/^v/i, '').trim().toLowerCase();
      const normLocal = m.version.replace(/^v/i, '').trim().toLowerCase();
      const normIgnored = m.ignoredVersion ? m.ignoredVersion.replace(/^v/i, '').trim().toLowerCase() : '';
      if (normNexus !== '' && normNexus !== 'unknown' && normLocal !== 'unknown' && isVersionNewer(normLocal, normNexus) && normNexus !== normIgnored) {
        updatesMap.set(m.id, m.nexusVersionCached);
        continue;
      }
    }

    // Check if a newer version exists in local Library!
    if (libraryEntries && m.version) {
      const normLocal = m.version.replace(/^v/i, '').trim().toLowerCase();
      const normModName = m.name.toLowerCase().replace(/[^a-z0-9]/g, '');
      const normModId = m.id.toLowerCase().replace(/[^a-z0-9]/g, '');

      const matchingLibEntries = libraryEntries.filter(e => {
        const normLibId = (e.modId || '').toLowerCase().replace(/[^a-z0-9]/g, '');
        const normLibName = (e.nexusName || '').toLowerCase().replace(/[^a-z0-9]/g, '');
        return normLibId === normModName || normLibId === normModId || (normLibName !== '' && (normLibName === normModName || normLibName === normModId)) || (e.nexusModId && m.nexusModId && e.nexusModId === m.nexusModId);
      });

      let highestLibVer: string | null = null;
      for (const libEntry of matchingLibEntries) {
        const rawVer = libEntry.version || '';
        const libVer = rawVer.replace(/^v/i, '').trim().toLowerCase();
        if (libVer && libVer !== 'unknown' && normLocal !== 'unknown' && isVersionNewer(normLocal, libVer)) {
          if (!highestLibVer || isVersionNewer(highestLibVer, libVer)) {
            highestLibVer = rawVer.replace(/^v/i, '').trim();
          }
        }
      }

      if (highestLibVer) {
        const normIgnored = m.ignoredVersion ? m.ignoredVersion.replace(/^v/i, '').trim().toLowerCase() : '';
        if (highestLibVer.toLowerCase() !== normIgnored) {
          updatesMap.set(m.id, highestLibVer);
        }
      }
    }
  }
  return updatesMap;
}

export function isModMissingGamePass(mod: ModInfo, state: any): boolean {
  const platform = state.dependencies?.game_platform?.toLowerCase();
  const isXbox = platform === 'xbox' || platform === 'gamepass' || platform === 'wingdk';
  if (!isXbox) return false;

  const isPakMod = mod.type === 'pak' || mod.type === 'logicmods' || (mod.gamePath && mod.gamePath.toLowerCase().endsWith('.pak')) || (mod.extraFiles && mod.extraFiles.some(f => f.toLowerCase().endsWith('.pak')));
  if (!isPakMod) return false;

  const hasUtoc = mod.extraFiles && mod.extraFiles.some(f => f.toLowerCase().endsWith('.utoc'));
  const hasUcas = mod.extraFiles && mod.extraFiles.some(f => f.toLowerCase().endsWith('.ucas'));
  return !hasUtoc || !hasUcas;
}

export function buildModCardHtml(mod: ModInfo, state: any, isChild: boolean = false, folderId: string = ''): string {
  const isWorkshop = !!(mod.nexusSummary && mod.nexusSummary.startsWith('Steam Workshop Mod'));
  const updateVer = state.availableUpdates?.get(mod.id);
  const isMissingGp = isModMissingGamePass(mod, state);

  if (state.viewLayout === 'list') {
    const isSelected = state.selectedModIds.has(mod.id);
    const shortPath = mod.gamePath ? mod.gamePath.replace(/\\/g, '/').split('/').slice(-3).join('/') : '';
    const extraCount = mod.extraFiles ? mod.extraFiles.length : 0;
    const extraText = extraCount > 0 ? `+${extraCount} ${escapeHtml(t('card.extra_files_count', { count: extraCount }))}` : escapeHtml(t('common.none'));
    const formattedDate = mod.installDate ? mod.installDate.substring(0, 10) : escapeHtml(t('common.unknown'));

    const removeBtn = isWorkshop
      ? `<span style="font-size: 10px; color: var(--text-muted); opacity: 0.6; font-weight: bold; text-transform: uppercase;">${escapeHtml(t('card.badge_workshop'))}</span>`
      : `<button class="card-remove-btn" data-id="${mod.id}" title="${escapeHtml(t('card.remove_btn_title'))}">✕</button>`;

    const childClass = isChild ? 'folder-child-row' : '';
    const childIndent = isChild ? `<span class="tree-connector">↳</span>` : '';
    const folderAttr = isChild ? `data-folder-id="${folderId}"` : '';

    return `
    <div class="mod-card list-row-card ${childClass} ${mod.enabled ? '' : 'disabled'} ${isSelected ? 'selected' : ''}" data-id="${mod.id}" data-type="${mod.type}" data-is-workshop="${isWorkshop}" ${folderAttr}>
      <div class="cell name-cell">
        ${childIndent}
        <label class="toggle-switch">
          <input type="checkbox" class="card-toggle-input" data-id="${mod.id}" ${mod.enabled ? 'checked' : ''} />
          <span class="toggle-slider"></span>
        </label>
        <span class="mod-card-led ${mod.enabled ? 'on' : 'off'}"></span>
        <span class="mod-card-name" style="font-weight:600;">${escapeHtml(mod.name)}</span>
        ${isWorkshop ? `<span style="margin-left: 8px; font-size: 8px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 1px 4px; border-radius: 3px;">${escapeHtml(t('card.badge_workshop'))}</span>` : ''}
        ${isMissingGp ? `<span style="margin-left: 8px; font-size: 8px; font-weight: bold; background: rgba(255, 170, 0, 0.15); color: #ffaa00; border: 1px solid rgba(255, 170, 0, 0.3); padding: 1px 4px; border-radius: 3px;" title="${escapeHtml(t('card.gamepass_missing_tooltip'))}">🎮 ${escapeHtml(t('card.badge_gamepass_missing'))}</span>` : ''}
        ${updateVer ? `<span style="margin-left: 8px; font-size: 8px; font-weight: bold; background: rgba(0, 188, 255, 0.15); color: #00bcff; border: 1px solid rgba(0, 188, 255, 0.3); padding: 1px 4px; border-radius: 3px;">${escapeHtml(t('card.badge_update_available', { version: updateVer }))}</span>` : ''}
        ${mod.customNotes && mod.customNotes.trim() ? `<span class="badge-mod-notes" style="margin-left: 6px; font-size: 11px; cursor: pointer;" title="${escapeHtml(mod.customNotes)}">📝</span>` : ''}
      </div>
      <div class="cell status-cell">
      </div>
      <div class="cell type-cell">
        <span class="mod-card-type ${mod.type}">${escapeHtml(mod.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : mod.type.toUpperCase())}</span>
      </div>
      <div class="cell version-cell">
        <span class="mod-card-version">v${escapeHtml(mod.version)}</span>
      </div>
      <div class="cell path-cell" title="${escapeHtml(mod.gamePath)}">
        <span class="mod-card-path">${escapeHtml(shortPath || t('common.disabled'))}</span>
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
  const author = mod.nexusAuthor ? `<span class="mod-card-author">${escapeHtml(t('common.author'))}: ${escapeHtml(mod.nexusAuthor)}</span>` : '';
  let imageSrc = '';
  if (mod.nexusPictureUrl) {
    if (mod.nexusPictureUrl.startsWith('http://') || mod.nexusPictureUrl.startsWith('https://')) {
      imageSrc = mod.nexusPictureUrl;
    } else {
      try {
        imageSrc = convertFileSrc(mod.nexusPictureUrl);
      } catch (e) {
        console.error('Failed to convert file src for nexusPictureUrl:', e);
        imageSrc = '';
      }
    }
  }

  const placeholderLetter = mod.type === 'ue4ss' ? 'U' : mod.type === 'palschema' ? 'PS' : mod.type === 'pak' ? 'PK' : 'LM';
  const imageHtml = imageSrc
    ? `<div class="mod-card-image-wrap"><img class="mod-card-image" src="${escapeHtml(imageSrc)}" alt="" loading="lazy" data-original-src="${escapeHtml(imageSrc)}" onerror="window.handleCardImageError ? window.handleCardImageError(this, this.dataset.originalSrc || '${escapeHtml(imageSrc)}') : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='flex'));" /><div class="mod-card-image-placeholder ${mod.type}" style="display:none;">${placeholderLetter}</div></div>`
    : `<div class="mod-card-image-wrap"><div class="mod-card-image-placeholder ${mod.type}">${placeholderLetter}</div></div>`;

  const updateBadge = updateVer
    ? `<span class="mod-card-update-badge" title="${escapeHtml(t('card.badge_update_available', { version: updateVer }))}">&#9650; ${escapeHtml(t('context.update_mod'))} (v${escapeHtml(updateVer)})</span>`
    : '';

  const removeBtn = isWorkshop
    ? `<span style="font-size: 9px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 2px 6px; border-radius: 4px; text-transform: uppercase;">${escapeHtml(t('card.badge_workshop'))}</span>`
    : `<button class="card-remove-btn" data-id="${mod.id}" title="${escapeHtml(t('card.remove_btn_title'))}">✕</button>`;

  const isSelected = state.selectedModIds.has(mod.id);

  return `
  <div class="mod-card ${mod.enabled ? '' : 'disabled'} ${isSelected ? 'selected' : ''}" data-id="${mod.id}" data-type="${mod.type}" data-is-workshop="${isWorkshop}">
    ${imageHtml}
    <div class="mod-card-body">
      <div class="mod-card-body-top">
        <span class="mod-card-name">${escapeHtml(mod.name)}</span>
        <div style="display: flex; align-items: center; gap: 4px;">
          ${mod.customNotes && mod.customNotes.trim() ? `<span class="badge-mod-notes" style="font-size: 11px; cursor: pointer;" title="${escapeHtml(mod.customNotes)}">📝</span>` : ''}
          <span class="mod-card-led ${mod.enabled ? 'on' : 'off'}"></span>
        </div>
      </div>
      <div class="mod-card-meta">
        <span class="mod-card-type ${mod.type}">${escapeHtml(mod.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : mod.type.toUpperCase())}</span>
        <span class="mod-card-version">v${escapeHtml(mod.version)}</span>
        ${isMissingGp ? `<span class="badge-gp-missing" style="font-size: 8px; font-weight: bold; background: rgba(255, 170, 0, 0.15); color: #ffaa00; border: 1px solid rgba(255, 170, 0, 0.3); padding: 1px 4px; border-radius: 3px; cursor: help;" title="${escapeHtml(t('card.gamepass_missing_tooltip'))}">🎮 ${escapeHtml(t('card.badge_gamepass_missing'))}</span>` : ''}
        ${mod.type === 'altermatic' && mod.nexusModId !== 1626 && !mod.name.toLowerCase().includes('altermatic - runtime') && !mod.name.toLowerCase().startsWith('altermatic') && !state.dependencies?.altermatic_installed && !state.allMods?.some((m: any) => m.enabled && (m.nexusModId === 1626 || m.name.toLowerCase().includes('altermatic - runtime') || (m.name.toLowerCase().startsWith('altermatic') && m.type === 'altermatic'))) ? `<span class="badge-dep-missing" style="font-size: 8px; font-weight: bold; background: rgba(255, 118, 117, 0.15); color: var(--type-altermatic); border: 1px solid rgba(255, 118, 117, 0.35); padding: 1px 4px; border-radius: 3px; cursor: help;" title="${escapeHtml(t('card.altermatic_missing_tooltip') || 'Requires Altermatic framework')}">⚠️ ${escapeHtml(t('card.badge_altermatic_missing') || 'ALTERMATIC MISSING')}</span>` : ''}
        ${updateBadge}
        ${catHtml}
        ${isWorkshop ? `<span style="margin-left: 4px; font-size: 8px; font-weight: bold; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.3); padding: 1px 4px; border-radius: 3px;">${escapeHtml(t('card.badge_workshop'))}</span>` : ''}
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
  const folderCheckbox = `<label class="toggle-switch" title="${escapeHtml(t('card.folder_toggle_all_title'))}" onclick="event.stopPropagation()">
    <input type="checkbox" class="folder-toggle-input" data-folder-id="${folder.id}" ${allEnabled ? 'checked' : ''} />
    <span class="toggle-slider"></span>
  </label>`;
  const isSelected = state.selectedModIds.has(folder.id);

  if (state.viewLayout === 'list') {
    const isCollapsed = state.collapsedFolderIds ? state.collapsedFolderIds.has(folder.id) : false;
    const isExpanded = !isCollapsed;
    const chevron = `<span class="folder-chevron" data-folder-id="${folder.id}">${isExpanded ? '▼' : '▶'}</span>`;

    const countText = modsInFolder.length === 1
      ? t('card.folder_mods_count_single')
      : t('card.folder_mods_count', { count: modsInFolder.length });

    let rowsHtml = `
    <div class="mod-card folder-card list-row-card ${isExpanded ? 'expanded' : 'collapsed'} ${isSelected ? 'selected' : ''}" data-id="${folder.id}" data-type="folder" data-is-expanded="${isExpanded}">
      <div class="cell name-cell">
        ${chevron}
        <span style="margin-right: 4px;">📁</span>
        <span class="mod-card-name">${escapeHtml(folder.name)}</span>
      </div>
      <div class="cell status-cell">
        ${folderCheckbox}
      </div>
      <div class="cell type-cell">
        <span class="mod-card-type" style="color: var(--text-muted); border-color: var(--border);">${escapeHtml(t('card.folder_type_label'))}</span>
      </div>
      <div class="cell version-cell">
        <span>-</span>
      </div>
      <div class="cell path-cell">
        <span>-</span>
      </div>
      <div class="cell extra-cell">
        <span>${escapeHtml(countText)}</span>
      </div>
      <div class="cell date-cell">
        <span>-</span>
      </div>
      <div class="cell action-cell" onclick="event.stopPropagation()">
        <button class="mod-folder-btn rename-btn" style="background:none;border:none;color:var(--text-secondary);cursor:pointer;padding:4px;" data-folder-id="${folder.id}" title="${escapeHtml(t('dialogs.prompt_rename_folder'))}">✏</button>
        <button class="mod-folder-btn delete-btn delete" style="background:none;border:none;color:var(--text-secondary);cursor:pointer;padding:4px;" data-folder-id="${folder.id}" title="${escapeHtml(t('dialogs.confirm_delete_folder', { name: folder.name }))}">✕</button>
      </div>
    </div>`;

    if (modsInFolder.length > 0) {
      rowsHtml += modsInFolder.map(m => {
        const rowHtml = buildModCardHtml(m, state, true, folder.id);
        if (isCollapsed) {
          return rowHtml.replace('folder-child-row', 'folder-child-row is-collapsed');
        }
        return rowHtml;
      }).join('');
    } else {
      rowsHtml += `<div class="folder-child-empty folder-child-row ${isCollapsed ? 'is-collapsed' : ''}" data-folder-id="${folder.id}" style="margin-left: 24px; padding: 8px 16px; font-size: 11px; color: var(--text-muted); font-style: italic; border-left: 2px solid var(--border);">${escapeHtml(t('mods.folder_empty_drag_hint'))}</div>`;
    }

    return rowsHtml;
  }

  const countText = modsInFolder.length === 1
    ? t('card.folder_mods_count_single')
    : t('card.folder_mods_count', { count: modsInFolder.length });

  return `
  <div class="mod-card folder-card ${isSelected ? 'selected' : ''}" data-id="${folder.id}" data-type="folder" style="position:relative;">
    <div class="folder-card-actions" style="position: absolute; top: 8px; right: 8px; display: flex; gap: 4px; opacity: 0; z-index: 10;" onclick="event.stopPropagation()">
      <button class="mod-folder-btn rename-btn" data-folder-id="${folder.id}" title="${escapeHtml(t('dialogs.prompt_rename_folder'))}" style="padding: 2px 6px; font-size: 11px; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); cursor: pointer; border-radius: 4px;">✏</button>
      <button class="mod-folder-btn delete-btn delete" data-folder-id="${folder.id}" title="${escapeHtml(t('dialogs.confirm_delete_folder', { name: folder.name }))}" style="padding: 2px 6px; font-size: 11px; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); cursor: pointer; border-radius: 4px;">✕</button>
    </div>
    <div class="mod-card-image-wrap folder-icon-wrap" style="display:flex;align-items:center;justify-content:center;height:120px;background:var(--bg-secondary);font-size:48px;">
      📁
    </div>
    <div class="mod-card-body" style="padding: 12px; display: flex; flex-direction: column; flex-grow: 1; justify-content: space-between;">
      <div class="mod-card-body-top">
        <span class="mod-card-name" style="font-weight: 600; font-size: 13px;">${escapeHtml(folder.name)}</span>
      </div>
      <div class="mod-card-meta" style="margin-top: 8px; display: flex; align-items: center; justify-content: space-between; font-size: 12px; color: var(--text-secondary);">
        <span>${escapeHtml(countText)}</span>
        ${folderCheckbox}
      </div>
    </div>
  </div>`;
}
