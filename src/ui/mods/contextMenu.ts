import { getState, updateState } from '../../state';
import { disableMod, enableMod, removeMod, openModFolder, openExtraFolder, openUrl, uninstallUe4ss, uninstallPalschema, openFolderByType } from '../../api';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { openDetailPanel, closeDetailPanel } from '../detailPanel';
import { openConfigEditor } from '../editorView';
import { openWorkshopModal } from '../modal';
import { loadMods } from './loader';
import { loadProfiles, showInputModal } from './profiles';
import { loadDependencies, handleDepBadgeClick } from './dependencies';
import { handleLibraryBulkInstall, triggerInstallFromLibrary, loadLibrary, updateLibraryBulkBar } from './library';
import { renderModsView } from './renderer';
import { escapeHtml } from '../../utils/helpers';

export function runContextAction(action: string, modId: string): void {
  const mod = getState().allMods.find(m => m.id === modId);
  if (!mod) return;

  switch (action) {
    case 'check-updates':
      (async () => {
        try {
          showToast(`Checking updates for "${mod.name}"...`, 'info');
          const { refreshNexusCache } = await import('../../api');
          const updated = await refreshNexusCache(modId);
          const idx = getState().allMods.findIndex(m => m.id === modId);
          if (idx >= 0) {
            const newMods = [...getState().allMods];
            newMods[idx] = updated;
            updateState({ allMods: newMods });
          }
          const nexusVer = updated.nexusVersionCached;
          const localVer = updated.version;
          const normNexus = (nexusVer || '').replace(/^v/i, '').trim().toLowerCase();
          const normLocal = (localVer || '').replace(/^v/i, '').trim().toLowerCase();
          const isNewUpdate = nexusVer && normNexus !== normLocal && normNexus !== 'unknown' && !normLocal.startsWith(normNexus);

          const newMap = new Map(getState().availableUpdates);
          if (isNewUpdate) {
            newMap.set(modId, nexusVer!);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(`Update available: v${nexusVer} (current: v${localVer})`, 'info');
          } else {
            newMap.delete(modId);
            updateState({ availableUpdates: newMap });
            renderModsView();
            showToast(`"${mod.name}" is up to date`, 'success');
          }
        } catch (e) {
          showToast('Failed to check updates: ' + e, 'error');
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
          showToast('Update version ignored', 'success');
          await loadMods();
        } catch (e) {
          showToast('Failed to ignore version: ' + e, 'error');
        }
      })();
      break;
    case 'toggle':
      (async () => {
        try {
          const { setModProfileState } = await import('../../api');
          if (mod.enabled) { await disableMod(modId); } else { await enableMod(modId); }
          try { await setModProfileState(modId, !mod.enabled); } catch { }
          showToast(mod.enabled ? 'Mod disabled' : 'Mod enabled', mod.enabled ? 'info' : 'success');
          await loadMods();
        } catch (e) { showToast('Failed: ' + e, 'error'); }
      })();
      break;
    case 'open-folder':
      openModFolder(modId).catch(e => showToast('Failed: ' + e, 'error'));
      break;
    case 'open-extras':
      openExtraFolder(modId).catch(e => showToast('Failed: ' + e, 'error'));
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
    case 'remove':
      showConfirm(`Remove "${mod.name}" permanently?`).then(confirmed => {
        if (!confirmed) return;
        removeMod(modId).then(() => {
          closeDetailPanel();
          loadMods();
          showToast('Mod removed', 'success');
        }).catch(e => showToast('Failed: ' + e, 'error'));
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

  let html = `
    <button type="button" class="context-menu-item" data-action="check-updates">
      <span class="ctx-icon">&#8634;</span>
      Check for updates
    </button>
  `;
  const hasUpdate = getState().availableUpdates?.has(modId);
  if (hasUpdate) {
    const updateVer = getState().availableUpdates.get(modId)!;
    html += `
      <button type="button" class="context-menu-item" data-action="ignore-update">
        <span class="ctx-icon">✕</span>
        Ignore update (v${escapeHtml(updateVer)})
      </button>
    `;
  }
  html += `
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="toggle">
      <span class="ctx-icon">${mod.enabled ? '◌' : '●'}</span>
      ${mod.enabled ? 'Disable' : 'Enable'}
    </button>
    <button type="button" class="context-menu-item" data-action="open-folder">
      <span class="ctx-icon">📁</span>
      Open folder
    </button>
    ${mod.extraFiles && mod.extraFiles.length > 0 ? `
    <button type="button" class="context-menu-item" data-action="open-extras">
      <span class="ctx-icon">📂</span>
      Open extra folder
    </button>
    ` : ''}
    <button type="button" class="context-menu-item" data-action="edit-config">
      <span class="ctx-icon">⚙</span>
      Edit config
    </button>
    <button type="button" class="context-menu-item" data-action="detail">
      <span class="ctx-icon">ℹ</span>
      View details
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
      Move to folder...
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);pointer-events:none;">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && isInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${isInFolder ? `<button type="button" class="context-submenu-item" data-action="move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> Remove from folder
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> New folder...
        </button>
      </div>
    </div>
  `;

  const isWorkshop = mod.nexusSummary === 'Steam Workshop Mod';

  if (mod.nexusModId || mod.githubRepo) {
    html += `<div class="context-menu-sep"></div>`;
    if (mod.nexusModId) {
      html += `<button type="button" class="context-menu-item" data-action="visit-nexus">
        <span class="ctx-icon">N</span>
        Visit on NexusMods
      </button>`;
    }
    if (mod.githubRepo) {
      html += `<button type="button" class="context-menu-item" data-action="visit-github">
        <span class="ctx-icon">G</span>
        Visit on GitHub
      </button>`;
    }
  }

  if (!isWorkshop) {
    html += `<div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" data-action="remove">
        <span class="ctx-icon">✕</span>
        Remove
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
      hideContextMenu();
      runContextAction(action, modId);
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
          'New Mod Folder',
          'Enter a name for the new virtual folder:',
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
            showToast('Mod grouped into folder', 'success');
          }
        } catch (err) {
          showToast('Failed: ' + err, 'error');
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

  const modsInFolder = (state.allMods || []).filter(m => folder.mod_ids.includes(m.id));
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);

  let html = `
    <button type="button" class="context-menu-item" data-action="enter">
      <span class="ctx-icon">📂</span>
      Enter folder
    </button>
    <button type="button" class="context-menu-item" data-action="rename">
      <span class="ctx-icon">✏</span>
      Rename folder
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="toggle-mods">
      <span class="ctx-icon">${allEnabled ? '◌' : '●'}</span>
      ${allEnabled ? 'Disable all mods' : 'Enable all mods'}
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-mods">
      <span class="ctx-icon">&#8634;</span>
      Check updates for mods
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="delete">
      <span class="ctx-icon">✕</span>
      Delete folder
    </button>
  `;

  menu.innerHTML = html;

  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
  const folderCard = document.querySelector(`.mod-card.folder-card[data-id="${folderId}"]`);
  if (folderCard) folderCard.classList.add('context-active');

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'enter') {
        updateState({ currentFolderId: folderId });
        renderModsView();
      } else if (action === 'rename') {
        const renameBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .rename-btn`) as HTMLElement | null;
        renameBtn?.click();
      } else if (action === 'delete') {
        const deleteBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .delete-btn`) as HTMLElement | null;
        deleteBtn?.click();
      } else if (action === 'toggle-mods') {
        try {
          const { toggleFolderMods } = await import('../../api');
          const updatedProfile = await toggleFolderMods(state.currentProfileId, folderId, !allEnabled);
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          showToast(!allEnabled ? 'All folder mods enabled' : 'All folder mods disabled', 'success');
          await loadMods();
        } catch (err) {
          showToast('Failed to toggle: ' + err, 'error');
        }
      } else if (action === 'check-updates-mods') {
        if (modsInFolder.length === 0) {
          showToast('No mods in this folder', 'info');
          return;
        }
        showToast('Checking for updates...', 'info');
        try {
          const { checkForUpdates } = await import('../../api');
          await checkForUpdates();
          await loadMods();
          showToast('Folder mods update check completed', 'success');
        } catch (err) {
          showToast('Failed to check updates: ' + err, 'error');
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

  const workshopTooltip = 'Managed by Steam Workshop — cannot be uninstalled from PMM';

  const html = `
    <button type="button" class="context-menu-item" data-action="new-folder">
      <span class="ctx-icon">📁</span>
      New mod folder
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="install">
      <span class="ctx-icon">+</span>
      Install mod (.zip, .rar, .7z)
    </button>
    <button type="button" class="context-menu-item" data-action="rescan">
      <span class="ctx-icon">↻</span>
      Rescan mods
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-global">
      <span class="ctx-icon">↑</span>
      Check for updates
    </button>
    <button type="button" class="context-menu-item" data-action="export-json">
      <span class="ctx-icon">📤</span>
      Export mods JSON
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="check-deps">
      <span class="ctx-icon">&#8634;</span>
      Check UE4SS &amp; PalSchema updates
    </button>
    ${ue4ssInstalled
      ? isWorkshop
        ? `<button type="button" class="context-menu-item disabled" disabled title="${workshopTooltip}">
            <span class="ctx-icon" style="opacity:0.4">✕</span>
            <span style="opacity:0.4">Uninstall UE4SS</span>
            <span style="margin-left:auto;font-size:9px;opacity:0.5">Workshop</span>
           </button>`
        : `<button type="button" class="context-menu-item danger" data-action="uninstall-ue4ss">
            <span class="ctx-icon">✕</span>
            Uninstall UE4SS
           </button>`
      : `<button type="button" class="context-menu-item" data-action="update-ue4ss">
          <span class="ctx-icon">U</span>
          Install UE4SS
         </button>`
    }
    ${palschemaInstalled
      ? isWorkshop
        ? `<button type="button" class="context-menu-item disabled" disabled title="${workshopTooltip}">
            <span class="ctx-icon" style="opacity:0.4">✕</span>
            <span style="opacity:0.4">Uninstall PalSchema</span>
            <span style="margin-left:auto;font-size:9px;opacity:0.5">Workshop</span>
           </button>`
        : `<button type="button" class="context-menu-item danger" data-action="uninstall-palschema">
            <span class="ctx-icon">✕</span>
            Uninstall PalSchema
           </button>`
      : `<button type="button" class="context-menu-item" data-action="update-palschema">
          <span class="ctx-icon">S</span>
          Install PalSchema
         </button>`
    }
    <div class="context-menu-sep"></div>
    <div style="padding: 4px 12px 2px; font-size: 9px; color: var(--text-muted); font-weight: bold; text-transform: uppercase; opacity: 0.7;">Open Folder</div>
    ${hasUe4ss ? `
      <button type="button" class="context-menu-item" data-action="open-folder-ue4ss">
        <span class="ctx-icon">📂</span>
        UE4SS Mods
      </button>
    ` : ''}
    ${hasPalSchema ? `
      <button type="button" class="context-menu-item" data-action="open-folder-palschema">
        <span class="ctx-icon">📂</span>
        PalSchema Mods
      </button>
    ` : ''}
    <button type="button" class="context-menu-item" data-action="open-folder-paks">
      <span class="ctx-icon">📂</span>
      Pak Mods (Paks)
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="settings">
      <span class="ctx-icon">⚙</span>
      Settings
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
          showToast('Checking for dependency updates...', 'info');
          loadDependencies().then(() => showToast('Dependency check finished', 'success'));
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
              ? showConfirm(`Warning: You have ${dependentMods.length} enabled mod(s) that depend on UE4SS (e.g. ${dependentMods[0].name}). Uninstalling UE4SS will disable these mods. Do you want to proceed?`)
              : Promise.resolve(true);
            proceed.then(confirmed => {
              if (!confirmed) return;
              showToast('Uninstalling UE4SS...', 'info');
              uninstallUe4ss().then(msg => {
                showToast(msg, 'success');
                loadDependencies();
                loadMods();
              }).catch(e => showToast('Failed: ' + e, 'error'));
            });
          }
          break;
        case 'uninstall-palschema':
          {
            const modState = getState();
            const dependentMods = modState.allMods.filter(m => m.enabled && (m.type === 'palschema' || m.type === 'hybrid'));
            const proceed = dependentMods.length > 0
              ? showConfirm(`Warning: You have ${dependentMods.length} enabled mod(s) that depend on PalSchema (e.g. ${dependentMods[0].name}). Uninstalling PalSchema will disable these mods. Do you want to proceed?`)
              : Promise.resolve(true);
            proceed.then(confirmed => {
              if (!confirmed) return;
              showToast('Uninstalling PalSchema...', 'info');
              uninstallPalschema().then(msg => {
                showToast(msg, 'success');
                loadDependencies();
                loadMods();
              }).catch(e => showToast('Failed: ' + e, 'error'));
            });
          }
          break;
        case 'open-folder-ue4ss':
          openFolderByType('ue4ss').catch(e => showToast('Failed: ' + e, 'error'));
          break;
        case 'open-folder-palschema':
          openFolderByType('palschema').catch(e => showToast('Failed: ' + e, 'error'));
          break;
        case 'open-folder-paks':
          openFolderByType('paks').catch(e => showToast('Failed: ' + e, 'error'));
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
    <div style="font-size:9px;font-weight:700;color:var(--text-muted);padding:6px 16px 2px;text-transform:uppercase">${selectedCount} Mods Selected</div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="bulk-enable">
      <span class="ctx-icon">●</span>
      Enable Selected
    </button>
    <button type="button" class="context-menu-item" data-action="bulk-disable">
      <span class="ctx-icon">◌</span>
      Disable Selected
    </button>
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      Move selected to folder...
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);pointer-events:none;">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && anyInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${anyInFolder ? `<button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> Remove from folders
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="bulk-move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> New folder...
        </button>
      </div>
    </div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="bulk-remove">
      <span class="ctx-icon">✕</span>
      Remove Selected
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
        showToast(`Grouping ${modIds.length} mods...`, 'info');
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
          showToast(`Moved ${modIds.length} mods successfully`, 'success');
        } catch (err) {
          showToast('Failed to group mods: ' + err, 'error');
        }
      } else if (subAction === 'bulk-move-to-new-folder') {
        const newName = await showInputModal(
          'New Mod Folder',
          'Enter a name for the new virtual folder:',
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
            showToast(`Created folder and grouped ${modIds.length} mods`, 'success');
          }
        } catch (err) {
          showToast('Failed: ' + err, 'error');
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
        <span class="ctx-icon">📥</span> Install Selected (${selectedCount})
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> Remove Selected from Library
      </button>
    `;
  } else if (modId) {
    menu.innerHTML = `
      <button type="button" class="context-menu-item" id="lib-ctx-install" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">📥</span> Install Mod
      </button>
      <div class="context-menu-sep"></div>
      <button type="button" class="context-menu-item danger" id="lib-ctx-remove" style="display:flex;align-items:center;width:100%;">
        <span class="ctx-icon">🗑️</span> Remove from Library
      </button>
    `;
  } else {
    hideContextMenu();
    return;
  }

  positionContextMenu(x, y);

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
    if (selectedCount > 1) {
      if (confirm(`Remove ${selectedCount} mods from library?`)) {
        for (const id of Array.from(state.selectedLibraryIds)) {
          await removeFromLibrary(id).catch(() => { });
        }
        showToast('Mods removed from library', 'success');
        updateState({ selectedLibraryIds: new Set() });
        updateLibraryBulkBar();
        loadLibrary();
      }
    } else if (modId) {
      const confirmMsg = zipName
        ? `Remove "${zipName}" from library?`
        : 'Remove this mod from library?';
      if (confirm(confirmMsg)) {
        await removeFromLibrary(modId, zipName || undefined).catch(() => { });
        showToast(zipName ? 'Version removed from library' : 'Mod removed from library', 'success');
        loadLibrary();
      }
    }
  });
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
    const libCard = target.closest('.library-card') as HTMLElement | null;

    if (isLibraryView || libCard) {
      e.preventDefault();
      hideContextMenu();
      const id = libCard?.dataset.id || null;
      const zip = libCard?.querySelector('.library-item-delete')?.getAttribute('data-zip') || null;
      if (id) {
        showLibraryContextMenu(id, zip, e.clientX, e.clientY);
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
