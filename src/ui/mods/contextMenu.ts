import { getState, updateState } from '../../state';
import { disableMod, enableMod, removeMod, openModFolder, openExtraFolder, openPath, openUrl, uninstallUe4ss, uninstallPalschema, openFolderByType } from '../../api';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { openDetailPanel, closeDetailPanel, getModComponentFolders } from '../detailPanel';
import { openConfigEditor } from '../editorView';
import { openWorkshopModal } from '../modal';
import { loadMods } from './loader';
import { loadProfiles, showInputModal } from './profiles';
import { loadDependencies, handleDepBadgeClick } from './dependencies';
import { handleLibraryBulkInstall, triggerInstallFromLibrary, loadLibrary, updateLibraryBulkBar } from './library';
import { renderModsView } from './renderer';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';

export function runContextAction(action: string, modId: string): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;

  switch (action) {
    case 'check-updates':
      (async () => {
        try {
          showToast(t('toasts.checking_updates', { name: mod.name }), 'info');
          const { refreshNexusCache } = await import('../../api');
          const { isVersionNewer } = await import('./card');
          const updated = await refreshNexusCache(modId);
          const idx = getState().allMods.findIndex(m => m.id === modId);
          if (idx >= 0) {
            const newMods = [...getState().allMods];
            newMods[idx] = updated;
            updateState({ allMods: newMods });
          }
          const nexusVer = updated.nexusVersionCached;
          const localVer = updated.version || '0.0';
          const normNexus = (nexusVer || '').replace(/^v/i, '').trim().toLowerCase();
          const normLocal = localVer.replace(/^v/i, '').trim().toLowerCase();
          const isNexusUpdate = nexusVer && normNexus !== normLocal && normNexus !== 'unknown' && isVersionNewer(normLocal, normNexus);

          // Also check local library
          const libEntries = getState().libraryEntries || [];
          const normModName = mod.name.toLowerCase().replace(/[^a-z0-9]/g, '');
          const normModId = mod.id.toLowerCase().replace(/[^a-z0-9]/g, '');
          const matchingLibEntries = libEntries.filter(e => {
            const normLibId = (e.modId || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            const normLibName = (e.nexusName || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            return normLibId === normModName || normLibId === normModId || (normLibName !== '' && (normLibName === normModName || normLibName === normModId)) || (e.nexusModId && mod.nexusModId && e.nexusModId === mod.nexusModId);
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

          const targetUpdateVer = highestLibVer || (isNexusUpdate ? nexusVer : null);
          const newMap = new Map(getState().availableUpdates);

          if (targetUpdateVer) {
            newMap.set(modId, targetUpdateVer);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(t('toasts.update_available', { target: targetUpdateVer, current: localVer }), 'info');
          } else {
            newMap.delete(modId);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(t('toasts.mod_up_to_date', { name: mod.name }), 'success');
          }
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      })();
      break;
    case 'update-local-mod':
      (async () => {
        try {
          const updateVer = getState().availableUpdates.get(modId);
          const libEntries = getState().libraryEntries || [];
          const normModName = mod.name.toLowerCase().replace(/[^a-z0-9]/g, '');
          const normModId = mod.id.toLowerCase().replace(/[^a-z0-9]/g, '');

          const matchingLib = libEntries.find(e => {
            const normLibId = (e.modId || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            const normLibName = (e.nexusName || '').toLowerCase().replace(/[^a-z0-9]/g, '');
            const isMatch = normLibId === normModName || normLibId === normModId || (normLibName !== '' && (normLibName === normModName || normLibName === normModId)) || (e.nexusModId && mod.nexusModId && e.nexusModId === mod.nexusModId);
            if (!isMatch) return false;
            if (updateVer) {
              const eVer = (e.version || '').replace(/^v/i, '').trim();
              return eVer === updateVer.replace(/^v/i, '').trim();
            }
            return true;
          });

          if (matchingLib) {
            const { triggerInstallFromLibrary } = await import('./library');
            await triggerInstallFromLibrary(matchingLib.modId, matchingLib.zipName);
          } else {
            const { openDetailPanel } = await import('../detailPanel');
            openDetailPanel(modId);
            showToast(t('toasts.update_available', { target: updateVer || '', current: mod.version }), 'info');
          }
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      })();
      break;
    case 'update-workshop-mod':
      (async () => {
        try {
          showToast(t('toasts.preparing_workshop_update'), 'info');
          const { activateWorkshopMod } = await import('../../api');
          await activateWorkshopMod(mod.id);
          showToast(t('toasts.mod_updated', { name: mod.name }), 'success');
          await loadMods();
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      })();
      break;
    case 'ignore-update':
      (async () => {
        try {
          const updateVer = getState().availableUpdates.get(modId);
          if (!updateVer) return;
          const { ignoreModVersion } = await import('../../api');
          await ignoreModVersion(modId, updateVer);
          showToast(t('toasts.settings_saved'), 'success');
          await loadMods();
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      })();
      break;
    case 'toggle':
      (async () => {
        try {
          const { setModProfileState } = await import('../../api');
          if (mod.enabled) { await disableMod(modId); } else { await enableMod(modId); }
          try { await setModProfileState(modId, !mod.enabled); } catch { }
          showToast(mod.enabled ? t('toasts.mod_disabled') : t('toasts.mod_enabled'), mod.enabled ? 'info' : 'success');
          await loadMods();
        } catch (e) { showToast(t('toasts.export_failed', { error: String(e) }), 'error'); }
      })();
      break;
    case 'open-folder':
      openModFolder(modId).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
      break;
    case 'open-extras':
      openExtraFolder(modId).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
      break;
    case 'edit-config':
      openConfigEditor(modId);
      break;
    case 'detail':
      openDetailPanel(modId);
      break;
    case 'visit-nexus':
      openUrl(`https://www.nexusmods.com/palworld/mods/${mod.nexusModId}`);
      break;
    case 'visit-github':
      openUrl(`https://github.com/${mod.githubRepo}`);
      break;
    case 'visit-steam-app': {
      const matchId = mod.nexusSummary?.match(/ID:\s*(\d+)/);
      const workshopId = matchId ? matchId[1] : '';
      if (workshopId) {
        openUrl(`steam://url/CommunityFilePage/${workshopId}`);
      }
      break;
    }
    case 'visit-steam-web': {
      const matchId = mod.nexusSummary?.match(/ID:\s*(\d+)/);
      const workshopId = matchId ? matchId[1] : '';
      if (workshopId) {
        openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${workshopId}`);
      }
      break;
    }
    case 'remove':
      showConfirm(t('dialogs.confirm_remove_mod', { name: mod.name })).then(confirmed => {
        if (!confirmed) return;
        removeMod(modId).then(() => {
          closeDetailPanel();
          loadMods();
          showToast(t('toasts.mod_removed'), 'success');
        }).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
      });
      break;
  }
}

export function getContextOverlay(): HTMLElement {
  return document.getElementById('context-overlay')!;
}

export function showContextMenu(modId: string, x: number, y: number): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;

  const isWorkshop = !!(mod.nexusSummary && mod.nexusSummary.startsWith('Steam Workshop Mod'));
  let html = '';
  if (!isWorkshop) {
    html += `
      <button type="button" class="context-menu-item" data-action="check-updates">
        <span class="ctx-icon">&#8634;</span>
        ${escapeHtml(t('context.check_updates'))}
      </button>
    `;
  }

  const hasUpdate = getState().availableUpdates?.has(modId);
  if (hasUpdate) {
    if (isWorkshop) {
      html += `
        <button type="button" class="context-menu-item" data-action="update-workshop-mod" style="font-weight: bold; color: #ff9d00;">
          <span class="ctx-icon">⚡</span>
          ${escapeHtml(t('context.update_mod'))}
        </button>
      `;
    } else {
      const updateVer = getState().availableUpdates.get(modId)!;
      html += `
        <button type="button" class="context-menu-item" data-action="update-local-mod" style="font-weight: bold; color: #00bcff;">
          <span class="ctx-icon">⚡</span>
          ${escapeHtml(t('context.update_mod_ver', { version: updateVer }))}
        </button>
        <button type="button" class="context-menu-item" data-action="ignore-update">
          <span class="ctx-icon">✕</span>
          ${escapeHtml(t('context.ignore_update', { version: updateVer }))}
        </button>
      `;
    }
  }
  html += `
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="toggle">
      <span class="ctx-icon">${mod.enabled ? '◌' : '●'}</span>
      ${escapeHtml(mod.enabled ? t('context.disable_mod') : t('context.enable_mod'))}
    </button>`;

  const compFolders = getModComponentFolders(mod);
  if (compFolders.length > 1) {
    for (const comp of compFolders) {
      html += `
      <button type="button" class="context-menu-item" data-action="open-comp-path" data-path="${escapeHtml(comp.path)}">
        <span class="ctx-icon">📁</span>
        ${escapeHtml(comp.buttonLabel)}
      </button>`;
    }
  } else {
    html += `
    <button type="button" class="context-menu-item" data-action="open-folder">
      <span class="ctx-icon">📁</span>
      ${escapeHtml(t('context.open_folder'))}
    </button>`;
  }

  html += `
    <button type="button" class="context-menu-item" data-action="edit-config">
      <span class="ctx-icon">⚙</span>
      ${escapeHtml(t('context.edit_config'))}
    </button>
    <button type="button" class="context-menu-item" data-action="detail">
      <span class="ctx-icon">ℹ</span>
      ${escapeHtml(t('context.view_details'))}
    </button>
  `;

  const currentProfile = getState().profiles.find(p => p.id === getState().currentProfileId);
  const folders = currentProfile?.mod_folders || [];
  const isInFolder = folders.some(f => f.mod_ids.includes(modId));
  const foldersHtml = folders.map(f => `
    <button type="button" class="context-submenu-item" data-action="move-to-folder" data-folder-id="${f.id}" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
      <span>📁</span> ${escapeHtml(f.name)}
    </button>
  `).join('');

  html += `
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      ${escapeHtml(t('context.move_to_folder'))}
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);pointer-events:none;">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && isInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${isInFolder ? `<button type="button" class="context-submenu-item" data-action="move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> ${escapeHtml(t('context.folder_none'))}
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> ${escapeHtml(t('mods.btn_new_folder'))}...
        </button>
      </div>
    </div>
  `;

  if (mod.nexusModId || mod.githubRepo) {
    html += `<div class="context-menu-sep"></div>`;
    if (mod.nexusModId) {
      html += `<button type="button" class="context-menu-item" data-action="visit-nexus">
        <span class="ctx-icon">N</span>
        ${escapeHtml(t('context.visit_nexus'))}
      </button>`;
    }
    if (mod.githubRepo) {
      html += `<button type="button" class="context-menu-item" data-action="visit-github">
        <span class="ctx-icon">G</span>
        ${escapeHtml(t('context.visit_github'))}
      </button>`;
    }
  }

  if (isWorkshop) {
    const matchId = mod.nexusSummary?.match(/ID:\s*(\d+)/);
    const workshopId = matchId ? matchId[1] : '';
    if (workshopId) {
      html += `<div class="context-menu-sep"></div>
        <button type="button" class="context-menu-item" data-action="visit-steam-app">
          <span class="ctx-icon">♨️</span>
          ${escapeHtml(t('context.visit_steam_app'))}
        </button>
        <button type="button" class="context-menu-item" data-action="visit-steam-web">
          <span class="ctx-icon">🌐</span>
          ${escapeHtml(t('context.visit_steam_web'))}
        </button>`;
    }
  }

  if (!isWorkshop) {
    html += `<div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" data-action="remove">
        <span class="ctx-icon">✕</span>
        ${escapeHtml(t('context.remove_mod'))}
      </button>`;
  }

  menu.innerHTML = html;

  menu.querySelectorAll('.has-submenu').forEach(item => {
    item.addEventListener('mouseenter', () => {
      const sub = item.querySelector('.context-submenu') as HTMLElement | null;
      if (!sub) return;
      const parentRect = item.getBoundingClientRect();
      const subWidth = 180;
      const spaceRight = window.innerWidth - parentRect.right;
      if (spaceRight < subWidth + 12) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
  });

  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
  const modCard = document.querySelector(`.mod-card[data-id="${modId}"]`);
  if (modCard) modCard.classList.add('context-active');

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if ((btn as HTMLElement).classList.contains('has-submenu')) return;
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      const path = (btn as HTMLElement).dataset.path;
      hideContextMenu();
      if (action === 'open-comp-path' && path) {
        openPath(path).catch(err => showToast(t('toasts.export_failed', { error: String(err) }), 'error'));
      } else {
        runContextAction(action, modId);
      }
    });
  });

  menu.querySelectorAll('.context-submenu-item').forEach(subBtn => {
    subBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const subAction = (subBtn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (subAction === 'move-to-folder') {
        const folderId = (subBtn as HTMLElement).dataset.folderId!;
        const targetFolder = folderId === 'none' ? null : folderId;
        const { addModToFolder } = await import('../../api');
        const state = getState();
        const updatedProfile = await addModToFolder(state.currentProfileId, targetFolder, modId);
        const profiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles });
        await loadMods();
        showToast(folderId ? 'Mod grouped into folder' : 'Mod moved to ungrouped', 'success');
      } else if (subAction === 'move-to-new-folder') {
        const newName = await showInputModal(
          t('dialogs.prompt_new_folder_name'),
          t('dialogs.prompt_new_folder_name'),
          'Skins'
        );
        if (newName === null) return;
        const trimmed = newName.trim();
        if (!trimmed) return;

        try {
          const { createModFolder, addModToFolder } = await import('../../api');
          const state = getState();
          const updatedProfile = await createModFolder(state.currentProfileId, trimmed);
          const newFolder = updatedProfile.mod_folders?.find(f => f.name.toLowerCase() === trimmed.toLowerCase());
          if (newFolder) {
            const finalProfile = await addModToFolder(state.currentProfileId, newFolder.id, modId);
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? finalProfile : p);
            updateState({ profiles });
            await loadMods();
            showToast(t('toasts.mod_grouped_success'), 'success');
          }
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

export function showFolderContextMenu(folderId: string, x: number, y: number): void {
  const state = getState();
  const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);
  const folder = currentProfile?.mod_folders?.find(f => f.id === folderId);
  if (!folder) return;

  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;

  const modsInFolder = state.allMods.filter(m => folder.mod_ids.includes(m.id));
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);
  const toggleLabel = allEnabled ? t('context.folder_disable_all') : t('context.folder_enable_all');
  const toggleIcon = allEnabled ? '⏸' : '▶';

  const folders = currentProfile?.mod_folders || [];
  const folderIndex = folders.findIndex(f => f.id === folderId);
  const canMoveUp = folderIndex > 0;
  const canMoveDown = folderIndex !== -1 && folderIndex < folders.length - 1;

  const html = `
    <button type="button" class="context-menu-item" data-action="enter-folder">
      <span class="ctx-icon">📂</span>
      ${escapeHtml(t('context.folder_enter'))}
    </button>
    ${canMoveUp ? `
    <button type="button" class="context-menu-item" data-action="move-folder-up">
      <span class="ctx-icon">▲</span>
      ${escapeHtml(t('context.folder_move_up'))}
    </button>` : ''}
    ${canMoveDown ? `
    <button type="button" class="context-menu-item" data-action="move-folder-down">
      <span class="ctx-icon">▼</span>
      ${escapeHtml(t('context.folder_move_down'))}
    </button>` : ''}
    <button type="button" class="context-menu-item" data-action="rename-folder">
      <span class="ctx-icon">✏</span>
      ${escapeHtml(t('context.folder_rename'))}
    </button>
    <button type="button" class="context-menu-item" data-action="toggle-folder-mods">
      <span class="ctx-icon">${toggleIcon}</span>
      ${escapeHtml(toggleLabel)}
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-mods">
      <span class="ctx-icon">↑</span>
      ${escapeHtml(t('context.check_updates'))}
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="delete-folder">
      <span class="ctx-icon">🗑</span>
      ${escapeHtml(t('context.folder_delete'))}
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', async (e: Event) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'enter-folder') {
        updateState({ currentFolderId: folderId });
        renderModsView();
      } else if (action === 'move-folder-up' || action === 'move-folder-down') {
        const fList = [...folders];
        const idx = fList.findIndex(f => f.id === folderId);
        if (idx === -1) return;
        const targetIdx = action === 'move-folder-up' ? idx - 1 : idx + 1;
        if (targetIdx < 0 || targetIdx >= fList.length) return;
        const [moved] = fList.splice(idx, 1);
        fList.splice(targetIdx, 0, moved);
        try {
          const { reorderModFolders } = await import('../../api');
          const updatedProfile = await reorderModFolders(state.currentProfileId, fList.map(f => f.id));
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          const { loadMods } = await import('./loader');
          await loadMods();
        } catch (err) {
          showToast(String(err), 'error');
        }
      } else if (action === 'rename-folder') {
        const renameBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .rename-btn`) as HTMLElement | null;
        renameBtn?.click();
      } else if (action === 'delete-folder') {
        const deleteBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .delete-btn`) as HTMLElement | null;
        deleteBtn?.click();
      } else if (action === 'toggle-folder-mods') {
        try {
          const { toggleFolderMods } = await import('../../api');
          const updatedProfile = await toggleFolderMods(state.currentProfileId, folderId, !allEnabled);
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          showToast(!allEnabled ? t('toasts.folder_mods_enabled') : t('toasts.folder_mods_disabled'), 'success');
          await loadMods();
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else if (action === 'check-updates-mods') {
        if (modsInFolder.length === 0) {
          showToast(t('mods.folder_empty_desc'), 'info');
          return;
        }
        showToast(t('toasts.checking_updates', { name: folder.name }), 'info');
        try {
          const { checkForUpdates } = await import('../../api');
          await checkForUpdates();
          await loadMods();
          showToast(t('toasts.all_mods_up_to_date'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

export function showGlobalContextMenu(x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  const state = getState();
  const deps = state.dependencies;
  const activeProfile = state.profiles.find(p => p.id === state.currentProfileId);

  const profileHasUe4ss = activeProfile?.ue4ss_enabled === true;
  const hasUe4ss = deps?.ue4ss_installed && profileHasUe4ss;
  const hasPalSchema = deps?.palschema_installed && profileHasUe4ss;
  const isWorkshop = deps?.ue4ss_install_mode === 'Workshop';

  const ue4ssInstalled = deps?.ue4ss_installed === true;
  const palschemaInstalled = deps?.palschema_installed === true;

  const workshopTooltip = escapeHtml(t('context.workshop_managed_hint'));

  const html = `
    <button type="button" class="context-menu-item" data-action="new-folder">
      <span class="ctx-icon">📁</span>
      ${escapeHtml(t('context.new_mod_folder'))}
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="install">
      <span class="ctx-icon">+</span>
      ${escapeHtml(t('context.install_archive'))}
    </button>
    <button type="button" class="context-menu-item" data-action="rescan">
      <span class="ctx-icon">↻</span>
      ${escapeHtml(t('context.rescan_mods'))}
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-global">
      <span class="ctx-icon">↑</span>
      ${escapeHtml(t('context.check_updates'))}
    </button>
    <button type="button" class="context-menu-item" data-action="export-json">
      <span class="ctx-icon">📤</span>
      ${escapeHtml(t('context.export_json'))}
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="check-deps">
      <span class="ctx-icon">&#8634;</span>
      ${escapeHtml(t('context.check_deps_updates'))}
    </button>
    ${ue4ssInstalled
      ? isWorkshop
        ? `<button type="button" class="context-menu-item disabled" disabled title="${workshopTooltip}">
            <span class="ctx-icon" style="opacity:0.4">✕</span>
            <span style="opacity:0.4">${escapeHtml(t('context.uninstall_ue4ss'))}</span>
            <span style="margin-left:auto;font-size:9px;opacity:0.5">Workshop</span>
           </button>`
        : `<button type="button" class="context-menu-item danger" data-action="uninstall-ue4ss">
            <span class="ctx-icon">✕</span>
            ${escapeHtml(t('context.uninstall_ue4ss'))}
           </button>`
      : `<button type="button" class="context-menu-item" data-action="update-ue4ss">
          <span class="ctx-icon">U</span>
          ${escapeHtml(t('context.install_ue4ss'))}
         </button>`
    }
    ${palschemaInstalled
      ? isWorkshop
        ? `<button type="button" class="context-menu-item disabled" disabled title="${workshopTooltip}">
            <span class="ctx-icon" style="opacity:0.4">✕</span>
            <span style="opacity:0.4">${escapeHtml(t('context.uninstall_palschema'))}</span>
            <span style="margin-left:auto;font-size:9px;opacity:0.5">Workshop</span>
           </button>`
        : `<button type="button" class="context-menu-item danger" data-action="uninstall-palschema">
            <span class="ctx-icon">✕</span>
            ${escapeHtml(t('context.uninstall_palschema'))}
           </button>`
      : `<button type="button" class="context-menu-item" data-action="update-palschema">
          <span class="ctx-icon">S</span>
          ${escapeHtml(t('context.install_palschema'))}
         </button>`
    }
    <div class="context-menu-sep"></div>
    <div style="padding: 4px 12px 2px; font-size: 9px; color: var(--text-muted); font-weight: bold; text-transform: uppercase; opacity: 0.7;">${escapeHtml(t('context.open_folder_header'))}</div>
    ${hasUe4ss ? `
      <button type="button" class="context-menu-item" data-action="open-folder-ue4ss">
        <span class="ctx-icon">📂</span>
        ${escapeHtml(t('context.folder_ue4ss'))}
      </button>
    ` : ''}
    ${hasPalSchema ? `
      <button type="button" class="context-menu-item" data-action="open-folder-palschema">
        <span class="ctx-icon">📂</span>
        ${escapeHtml(t('context.folder_palschema'))}
      </button>
    ` : ''}
    <button type="button" class="context-menu-item" data-action="open-folder-paks">
      <span class="ctx-icon">📂</span>
      ${escapeHtml(t('context.folder_paks'))}
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="settings">
      <span class="ctx-icon">⚙</span>
      ${escapeHtml(t('context.settings'))}
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.context-menu-item:not([disabled])').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      switch (action) {
        case 'manage-workshop':
          openWorkshopModal();
          break;
        case 'install':
          document.getElementById('install-btn')?.click();
          break;
        case 'rescan':
          document.getElementById('scan-btn')?.click();
          break;
        case 'check-updates-global':
          document.getElementById('check-updates-btn')?.click();
          break;
        case 'export-json':
          document.getElementById('export-json-btn')?.click();
          break;
        case 'new-folder':
          document.getElementById('new-folder-btn')?.click();
          break;
        case 'check-deps':
          showToast(t('context.checking_deps'), 'info');
          sessionStorage.removeItem('dismissed_ue4ss_update');
          sessionStorage.removeItem('dismissed_palschema_update');
          (async () => {
            try {
              const { checkDependenciesFull } = await import('../../api');
              const fullDeps = await checkDependenciesFull();
              updateState({ dependencies: fullDeps });
              const { renderDependencyBadges, handleDepBadgeClick } = await import('./dependencies');
              const { renderConflictBanner, removeConflictBanner } = await import('../conflictBanner');
              renderDependencyBadges(fullDeps);
              if (fullDeps.has_dll_conflict && fullDeps.conflicting_dlls && fullDeps.conflicting_dlls.length > 0) {
                renderConflictBanner(fullDeps.conflicting_dlls);
              } else {
                removeConflictBanner();
              }

              const ue4ssUp = fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop';
              const psUp = fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.palschema_version !== 'Workshop';

              if (ue4ssUp || psUp) {
                const updatesList: string[] = [];
                if (ue4ssUp) updatesList.push('UE4SS');
                if (psUp) updatesList.push('PalSchema');
                showToast(t('dependencies.found_dep_updates', { deps: updatesList.join(', ') }), 'info');

                if (ue4ssUp) {
                  const latestTarget = fullDeps.ue4ss_latest_date || fullDeps.ue4ss_latest_tag || 'latest';
                  const confirmed = await showConfirm(
                    t('dependencies.prompt_body_ue4ss', { installed: fullDeps.ue4ss_version || 'installed', latest: latestTarget }),
                    t('dependencies.prompt_title_ue4ss')
                  );
                  if (confirmed) {
                    const { executeInstallOrUpdate } = await import('./dependencies');
                    executeInstallOrUpdate('ue4ss', true);
                  }
                } else if (psUp) {
                  const latestVer = fullDeps.palschema_latest_version || 'latest';
                  const confirmed = await showConfirm(
                    t('dependencies.prompt_body_palschema', { installed: fullDeps.palschema_version || 'installed', latest: latestVer }),
                    t('dependencies.prompt_title_palschema')
                  );
                  if (confirmed) {
                    const { executeInstallOrUpdate } = await import('./dependencies');
                    executeInstallOrUpdate('palschema', true);
                  }
                }
              } else {
                showToast(t('dependencies.up_to_date'), 'success');
              }
            } catch (err) {
              showToast(t('toasts.export_failed', { error: String(err) }), 'error');
            }
          })();
          break;
        case 'update-ue4ss':
          handleDepBadgeClick('ue4ss');
          break;
        case 'update-palschema':
          handleDepBadgeClick('palschema');
          break;
        case 'uninstall-ue4ss':
          {
            const modState = getState();
            const dependentMods = modState.allMods.filter(m => m.enabled && (m.type === 'ue4ss' || m.type === 'hybrid'));
            const proceed = dependentMods.length > 0
              ? showConfirm(t('context.warn_uninstall_dep', { count: dependentMods.length, dep: 'UE4SS', sample: dependentMods[0].name }))
              : Promise.resolve(true);
            proceed.then(confirmed => {
              if (!confirmed) return;
              showToast(t('context.uninstalling_ue4ss'), 'info');
              uninstallUe4ss().then(msg => {
                showToast(msg, 'success');
                loadDependencies();
                loadMods();
              }).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
            });
          }
          break;
        case 'uninstall-palschema':
          {
            const modState = getState();
            const dependentMods = modState.allMods.filter(m => m.enabled && (m.type === 'palschema' || m.type === 'hybrid'));
            const proceed = dependentMods.length > 0
              ? showConfirm(t('context.warn_uninstall_dep', { count: dependentMods.length, dep: 'PalSchema', sample: dependentMods[0].name }))
              : Promise.resolve(true);
            proceed.then(confirmed => {
              if (!confirmed) return;
              showToast(t('context.uninstalling_palschema'), 'info');
              uninstallPalschema().then(msg => {
                showToast(msg, 'success');
                loadDependencies();
                loadMods();
              }).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
            });
          }
          break;
        case 'open-folder-ue4ss':
          openFolderByType('ue4ss').catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
          break;
        case 'open-folder-palschema':
          openFolderByType('palschema').catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
          break;
        case 'open-folder-paks':
          openFolderByType('paks').catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
          break;
        case 'settings':
          document.getElementById('settings-btn')?.click();
          break;
      }
    });
  });

  positionContextMenu(x, y);
}

export function hideContextMenu(): void {
  const overlay = getContextOverlay();
  overlay.classList.remove('visible');
  const menu = document.getElementById('context-menu')!;
  menu.style.display = 'none';
  menu.innerHTML = '';
  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
}

export function positionContextMenu(x: number, y: number): void {
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;
  overlay.classList.add('visible');
  menu.style.display = 'block';
  menu.style.visibility = 'hidden';

  requestAnimationFrame(() => {
    const rect = menu.getBoundingClientRect();
    const width = rect.width || 200;
    const height = rect.height || 200;

    const posX = Math.max(10, Math.min(x, window.innerWidth - width - 10));
    const posY = Math.max(10, Math.min(y, window.innerHeight - height - 10));

    menu.style.left = `${posX}px`;
    menu.style.top = `${posY}px`;
    menu.style.visibility = 'visible';
    menu.focus();
  });
}

export function showBulkContextMenu(x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  const selectedCount = getState().selectedModIds.size;

  const currentProfile = getState().profiles.find(p => p.id === getState().currentProfileId);
  const folders = currentProfile?.mod_folders || [];
  const selectedIds = Array.from(getState().selectedModIds);
  const anyInFolder = folders.some(f => selectedIds.some(id => f.mod_ids.includes(id)));
  const foldersHtml = folders.map(f => `
    <button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="${f.id}" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
      <span>📁</span> ${escapeHtml(f.name)}
    </button>
  `).join('');

  const html = `
    <div style="font-size:9px;font-weight:700;color:var(--text-muted);padding:6px 16px 2px;text-transform:uppercase">${selectedCount} ${escapeHtml(t('common.selected_count_mods', { count: selectedCount }))}</div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="bulk-enable">
      <span class="ctx-icon">●</span>
      ${escapeHtml(t('bulk.enable_selected'))}
    </button>
    <button type="button" class="context-menu-item" data-action="bulk-disable">
      <span class="ctx-icon">◌</span>
      ${escapeHtml(t('bulk.disable_selected'))}
    </button>
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      ${escapeHtml(t('context.move_to_folder'))}
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);pointer-events:none;">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && anyInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${anyInFolder ? `<button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> ${escapeHtml(t('context.remove_from_folders'))}
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="bulk-move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> ${escapeHtml(t('context.new_folder'))}
        </button>
      </div>
    </div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="bulk-remove">
      <span class="ctx-icon">✕</span>
      ${escapeHtml(t('bulk.delete_selected'))}
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.has-submenu').forEach(item => {
    item.addEventListener('mouseenter', () => {
      const sub = item.querySelector('.context-submenu') as HTMLElement | null;
      if (!sub) return;
      const parentRect = item.getBoundingClientRect();
      const subWidth = 180;
      const spaceRight = window.innerWidth - parentRect.right;
      if (spaceRight < subWidth + 12) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
  });

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if ((btn as HTMLElement).classList.contains('has-submenu')) return;
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'bulk-enable') {
        document.getElementById('bulk-enable-btn')?.click();
      } else if (action === 'bulk-disable') {
        document.getElementById('bulk-disable-btn')?.click();
      } else if (action === 'bulk-remove') {
        document.getElementById('bulk-remove-btn')?.click();
      }
    });
  });

  menu.querySelectorAll('.context-submenu-item').forEach(subBtn => {
    subBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const subAction = (subBtn as HTMLElement).dataset.action!;
      hideContextMenu();

      const modIds = Array.from(getState().selectedModIds);
      if (modIds.length === 0) return;

      if (subAction === 'bulk-move-to-folder') {
        const folderId = (subBtn as HTMLElement).dataset.folderId!;
        const targetFolder = folderId === 'none' ? null : folderId;
        showToast(targetFolder ? t('toasts.mod_grouped_success') : t('toasts.mod_ungrouped_success'), 'info');
        try {
          const { addModToFolder } = await import('../../api');
          const state = getState();
          let lastProfile = null;
          for (const modId of modIds) {
            lastProfile = await addModToFolder(state.currentProfileId, targetFolder, modId);
          }
          if (lastProfile) {
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
            updateState({ profiles });
          }
          await loadMods();
          showToast(targetFolder ? t('toasts.mod_grouped_success') : t('toasts.mod_ungrouped_success'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else if (subAction === 'bulk-move-to-new-folder') {
        const newName = await showInputModal(
          t('dialogs.prompt_new_folder_name'),
          t('dialogs.prompt_new_folder_name'),
          'Skins'
        );
        if (newName === null) return;
        const trimmed = newName.trim();
        if (!trimmed) return;

        try {
          const { createModFolder, addModToFolder } = await import('../../api');
          const state = getState();
          const updatedProfile = await createModFolder(state.currentProfileId, trimmed);
          const newFolder = updatedProfile.mod_folders?.find(f => f.name.toLowerCase() === trimmed.toLowerCase());
          if (newFolder) {
            let lastProfile = null;
            for (const modId of modIds) {
              lastProfile = await addModToFolder(state.currentProfileId, newFolder.id, modId);
            }
            if (lastProfile) {
              const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
              updateState({ profiles });
            }
            await loadMods();
            showToast(t('toasts.folder_created'), 'success');
          }
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}

export function showEditorContextMenu(x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;

  menu.innerHTML = `
    <button type="button" class="context-menu-item" id="editor-ctx-save" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">💾</span> Save <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+S</span>
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" id="editor-ctx-cut" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">✂</span> Cut <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+X</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-copy" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📋</span> Copy <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+C</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-paste" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📥</span> Paste <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+V</span>
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" id="editor-ctx-find" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">🔍</span> Find <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+F</span>
    </button>
    <button type="button" class="context-menu-item" id="editor-ctx-selectall" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">✓</span> Select All <span style="margin-left:auto;color:var(--text-muted);font-size:10px;">Ctrl+A</span>
    </button>
  `;

  positionContextMenu(x, y);

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;

  document.getElementById('editor-ctx-save')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleEditorSave } = await import('../editorView');
    await handleEditorSave();
  });

  document.getElementById('editor-ctx-cut')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    const start = editorContent.selectionStart;
    const end = editorContent.selectionEnd;
    const selectedText = editorContent.value.substring(start, end);
    if (selectedText) {
      await navigator.clipboard.writeText(selectedText);
      editorContent.value = editorContent.value.substring(0, start) + editorContent.value.substring(end);
      editorContent.selectionStart = editorContent.selectionEnd = start;
      editorContent.dispatchEvent(new Event('input'));
    }
  });

  document.getElementById('editor-ctx-copy')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    const selectedText = editorContent.value.substring(editorContent.selectionStart, editorContent.selectionEnd);
    if (selectedText) {
      await navigator.clipboard.writeText(selectedText);
    }
  });

  document.getElementById('editor-ctx-paste')!.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    try {
      const text = await navigator.clipboard.readText();
      const start = editorContent.selectionStart;
      const end = editorContent.selectionEnd;
      editorContent.value = editorContent.value.substring(0, start) + text + editorContent.value.substring(end);
      editorContent.selectionStart = editorContent.selectionEnd = start + text.length;
      editorContent.dispatchEvent(new Event('input'));
    } catch (err) {
      showToast('Clipboard access denied. Use Ctrl+V.', 'info');
    }
  });

  document.getElementById('editor-ctx-find')!.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    const ev = new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true });
    document.dispatchEvent(ev);
  });

  document.getElementById('editor-ctx-selectall')!.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (editorContent) {
      editorContent.focus();
      editorContent.select();
    }
  });
}

export function showLibraryContextMenu(modId: string | null, zipName: string | null, x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  const state = getState();
  const selectedCount = state.selectedLibraryIds.size;

  if (selectedCount > 1) {
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-install" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📥</span> ${escapeHtml(t('library.btn_install_selected', { count: selectedCount }))}
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> ${escapeHtml(t('library.btn_delete_selected_library'))}
      </button>
    `;
  } else if (modId) {
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-install" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📥</span> ${escapeHtml(t('library.btn_install_mod'))}
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> ${escapeHtml(t('library.btn_delete_from_library'))}
      </button>
    `;
  } else {
    // Empty space in Local Library
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-check-updates" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🔍</span> ${escapeHtml(t('library.btn_check_nexus_updates'))}
      </button>
      <button type="button" class="context-menu-item" id="lib-ctx-open-folder" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📁</span> ${escapeHtml(t('library.btn_open_library_folder'))}
      </button>
    `;
  }

  positionContextMenu(x, y);

  document.getElementById('lib-ctx-check-updates')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleCheckLocalLibraryOnlineUpdates } = await import('./library');
    handleCheckLocalLibraryOnlineUpdates();
  });

  document.getElementById('lib-ctx-open-folder')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { openUrl, getSettings } = await import('../../api');
    try {
      const settings = await getSettings();
      const libPath = `${settings.programPath}/mods-library`;
      await openUrl(libPath);
    } catch (err) {
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });

  document.getElementById('lib-ctx-install')?.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (selectedCount > 1) {
      handleLibraryBulkInstall();
    } else if (modId) {
      triggerInstallFromLibrary(modId);
    }
  });

  document.getElementById('lib-ctx-remove')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { removeFromLibrary } = await import('../../api');
    const { showConfirm } = await import('../confirm');
    if (selectedCount > 1) {
      const confirmed = await showConfirm(t('library.confirm_remove_bulk', { count: selectedCount }));
      if (confirmed) {
        for (const id of Array.from(state.selectedLibraryIds)) {
          await removeFromLibrary(id).catch(() => { });
        }
        showToast(t('toasts.library_mods_deleted', { count: selectedCount }), 'success');
        updateState({ selectedLibraryIds: new Set() });
        updateLibraryBulkBar();
        loadLibrary();
      }
    } else if (modId) {
      const confirmMsg = zipName
        ? t('library.confirm_remove_version', { zip: zipName })
        : t('dialogs.confirm_remove_mod', { name: modId });
      const confirmed = await showConfirm(confirmMsg);
      if (confirmed) {
        await removeFromLibrary(modId, zipName || undefined).catch(() => { });
        showToast(t('toasts.library_mod_version_removed'), 'success');
        loadLibrary();
      }
    }
  });
}

export function showWorkshopContextMenu(packageName: string | null, workshopId: string | null, x: number, y: number): void {
  const menu = document.getElementById('context-menu')!;
  
  menu.innerHTML = `
    <button type="button" class="context-menu-item" id="ws-ctx-check-updates" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">🔍</span> ${escapeHtml(t('library.btn_check_workshop_updates'))}
    </button>
    <button type="button" class="context-menu-item" id="ws-ctx-force-verify" style="display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">⚡</span> ${escapeHtml(t('library.btn_force_steam_validation'))}
    </button>
    ${workshopId ? `
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item" id="ws-ctx-open-steam" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🌐</span> ${escapeHtml(t('library.btn_open_in_steam'))}
      </button>
    ` : ''}
  `;

  positionContextMenu(x, y);

  document.getElementById('ws-ctx-check-updates')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleCheckWorkshopOnlineUpdates } = await import('./library');
    handleCheckWorkshopOnlineUpdates();
  });

  document.getElementById('ws-ctx-force-verify')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleTriggerSteamValidation } = await import('./library');
    handleTriggerSteamValidation();
  });

  if (workshopId) {
    document.getElementById('ws-ctx-open-steam')?.addEventListener('click', async (e) => {
      e.stopPropagation();
      hideContextMenu();
      const { openUrl } = await import('../../api');
      openUrl(`steam://url/CommunityFilePage/${workshopId}`);
    });
  }
}

function isAnyModalActive(): boolean {
  const visibleOverlay = document.querySelector('.modal-overlay.visible, .detail-overlay.visible, .confirm-overlay, .detail-overlay[style*="display: flex"]');
  return !!visibleOverlay;
}

export function setupContextMenu(): void {
  document.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement;

    if (isAnyModalActive() || target.closest('.modal-overlay, .detail-overlay, .confirm-overlay, .modal, #sidebar, #toolbar, #quick-filter-bar, #search-wrap, #sort-bar, #editor-toolbar, #library-toolbar')) {
      e.preventDefault();
      hideContextMenu();
      return;
    }

    if (target.closest('#editor-view')) {
      e.preventDefault();
      hideContextMenu();
      if (target.closest('#editor-content-area')) {
        showEditorContextMenu(e.clientX, e.clientY);
      }
      return;
    }

    const isLibraryView = target.closest('#library-view');
    const workshopCard = target.closest('.workshop-card') as HTMLElement | null;
    const libCard = target.closest('.library-card:not(.workshop-card)') as HTMLElement | null;

    if (workshopCard) {
      e.preventDefault();
      hideContextMenu();
      const pkg = workshopCard.dataset.package || null;
      const wid = workshopCard.querySelector('.workshop-item-folder-btn')?.getAttribute('data-path')?.split('/').pop() || null;
      showWorkshopContextMenu(pkg, wid, e.clientX, e.clientY);
      return;
    }

    if (isLibraryView || libCard) {
      e.preventDefault();
      hideContextMenu();
      const id = libCard?.dataset.id || null;
      const zip = libCard?.querySelector('.library-item-delete')?.getAttribute('data-zip') || null;
      if (id) {
        showLibraryContextMenu(id, zip, e.clientX, e.clientY);
      } else if (isLibraryView) {
        import('./library').then(({ _activeLibrarySubTab }) => {
          if (_activeLibrarySubTab === 'workshop') {
            showWorkshopContextMenu(null, null, e.clientX, e.clientY);
          } else {
            showLibraryContextMenu(null, null, e.clientX, e.clientY);
          }
        });
      }
      return;
    }

    e.preventDefault();
    hideContextMenu();

    const state = getState();
    const card = target.closest('.mod-card') as HTMLElement | null;

    if (state.selectedModIds.size > 1) {
      showBulkContextMenu(e.clientX, e.clientY);
    } else if (card && card.dataset.id) {
      const id = card.dataset.id;
      const type = card.dataset.type;
      if (type === 'folder') {
        showFolderContextMenu(id, e.clientX, e.clientY);
      } else {
        import('../../features/selection').then(({ updateSelection }) => {
          updateSelection(new Set([id]));
          showContextMenu(id, e.clientX, e.clientY);
        });
      }
    } else if (target.closest('#mods-container')) {
      showGlobalContextMenu(e.clientX, e.clientY);
    }
  });

  document.getElementById('context-overlay')!.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) {
      hideContextMenu();
    }
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hideContextMenu();
  });
}
