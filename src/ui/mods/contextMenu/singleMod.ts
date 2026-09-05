import { getState, updateState } from '../../../state';
import { disableMod, enableMod, removeMod, openModFolder, openExtraFolder, openPath, openUrl } from '../../../api';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { openDetailPanel, closeDetailPanel, getModComponentFolders } from '../../detailPanel';
import { openConfigEditor } from '../../editorView';
import { loadMods } from '../loader';
import { loadProfiles, showInputModal } from '../profiles';
import { loadDependencies } from '../dependencies';
import { renderModsView } from '../renderer';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { getContextOverlay, hideContextMenu, positionContextMenu } from './menuDom';
import { mainDom, detailDom } from '../../../framework';

export function runContextAction(action: string, modId: string): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;

  switch (action) {
    case 'convert-gamepass':
      (async () => {
        try {
          showToast(t('toasts.converting_gamepass', { name: mod.name }), 'info');
          const { convertModToGamepass } = await import('../../../api');
          const genFiles = await convertModToGamepass(modId);
          showToast(t('toasts.convert_gamepass_success', { name: mod.name, count: genFiles.length }), 'success');
          await loadMods();
        } catch (err: any) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      })();
      break;

    case 'check-updates':
      (async () => {
        try {
          showToast(t('toasts.checking_updates', { name: mod.name }), 'info');
          const { refreshNexusCache } = await import('../../../api');
          const { isVersionNewer } = await import('../card');
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
            const { triggerInstallFromLibrary } = await import('../library');
            showToast(t('toasts.updating_from_local_library', { name: mod.name, version: matchingLib.version || updateVer || '' }), 'info');
            await triggerInstallFromLibrary(matchingLib.modId, matchingLib.zipName);
          } else if (mod.nexusModId) {
            const hasNexusAccount = !!getState().currentSettings?.nexusAccount?.accessToken;
            if (hasNexusAccount) {
              const { openModDetails } = await import('../../discoveryView');
              await openModDetails(mod.nexusModId);
              const filesTabBtn = document.querySelector('.discovery-modal-tab[data-tab="files"]') as HTMLElement | null;
              filesTabBtn?.click();
              showToast(t('toasts.select_update_file_nexus'), 'info');
            } else {
              const { openUrl } = await import('../../../api');
              openUrl(`https://www.nexusmods.com/palworld/mods/${mod.nexusModId}?tab=files`);
              showToast(t('toasts.opening_nexus_files'), 'info');
            }
          } else {
            const { openDetailPanel } = await import('../../detailPanel');
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
          const { activateWorkshopMod } = await import('../../../api');
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
          const { ignoreModVersion } = await import('../../../api');
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
          const { setModProfileState } = await import('../../../api');
          if (mod.enabled) { await disableMod(modId); } else { await enableMod(modId); }
          try { await setModProfileState(modId, !mod.enabled); } catch { }
          showToast(mod.enabled ? t('toasts.mod_disabled') : t('toasts.mod_enabled'), mod.enabled ? 'info' : 'success');
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
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
    case 'edit-notes':
      openDetailPanel(modId);
      setTimeout(() => {
        const notesArea = detailDom.elMaybe('detail-custom-notes');
        if (notesArea) {
          notesArea.focus();
        }
      }, 100);
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
      showConfirm(t('dialogs.confirm_remove_mod', { name: mod.name })).then(async (confirmed) => {
        if (!confirmed) return;
        try {
          await removeMod(modId);
          closeDetailPanel();
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          showToast(t('toasts.mod_removed'), 'success');
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      });
      break;
  }
}

export function showContextMenu(modId: string, x: number, y: number): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;
  const overlay = getContextOverlay();
  const menu = mainDom.el('context-menu');

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
    <button type="button" class="context-menu-item" data-action="edit-notes">
      <span class="ctx-icon">📝</span>
      ${escapeHtml(t('context.edit_notes'))}
    </button>
    <button type="button" class="context-menu-item" data-action="detail">
      <span class="ctx-icon">ℹ</span>
      ${escapeHtml(t('context.view_details'))}
    </button>
  `;

  const platform = getState().dependencies?.game_platform?.toLowerCase();
  const isXbox = platform === 'xbox' || platform === 'gamepass' || platform === 'wingdk';
  const isPakMod = mod.type === 'pak' || mod.type === 'logicmods' || (mod.gamePath && mod.gamePath.toLowerCase().endsWith('.pak')) || (mod.extraFiles && mod.extraFiles.some(f => f.toLowerCase().endsWith('.pak')));
  if (isXbox && isPakMod) {
    html += `
    <button type="button" class="context-menu-item" data-action="convert-gamepass" style="color: #ffaa00;">
      <span class="ctx-icon">⚡</span>
      ${escapeHtml(t('context.convert_gamepass'))}
    </button>`;
  }

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
        const { addModToFolder } = await import('../../../api');
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
          const { createModFolder, addModToFolder } = await import('../../../api');
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
