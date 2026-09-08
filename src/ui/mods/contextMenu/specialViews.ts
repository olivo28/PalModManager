import { getState, updateState } from '../../../state';
import { uninstallUe4ss, uninstallPalschema, openFolderByType } from '../../../api';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { openWorkshopModal } from '../../modal';
import { loadMods } from '../loader';
import { loadDependencies, handleDepBadgeClick } from '../dependencies';
import { handleLibraryBulkInstall, triggerInstallFromLibrary, loadLibrary, updateLibraryBulkBar } from '../library';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { hideContextMenu, positionContextMenu, isAnyModalActive } from './menuDom';
import { showContextMenu } from './singleMod';
import { showFolderContextMenu } from './folders';
import { showBulkContextMenu } from './batchSelection';
import { mainDom, editorDom } from '../../../framework';

export function showGlobalContextMenu(x: number, y: number): void {
  const menu = mainDom.el('context-menu');
  const state = getState();
  const deps = state.dependencies;
  const activeProfile = state.profiles.find(p => p.id === state.currentProfileId);

  const profileHasUe4ss = activeProfile?.ue4ss_enabled === true;
  const hasUe4ss = deps?.ue4ss_installed && profileHasUe4ss;
  const hasPalSchema = deps?.palschema_installed && profileHasUe4ss;
  const isUe4ssWorkshop = deps?.ue4ss_install_mode === 'Workshop';
  const isPalSchemaWorkshop = deps?.palschema_install_mode === 'Workshop' || deps?.palschema_version === 'Workshop';

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
      ? isUe4ssWorkshop
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
      ? isPalSchemaWorkshop
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
          mainDom.elMaybe('install-btn')?.click();
          break;
        case 'rescan':
          mainDom.elMaybe('scan-btn')?.click();
          break;
        case 'check-updates-global':
          mainDom.elMaybe('check-updates-btn')?.click();
          break;
        case 'export-json':
          mainDom.elMaybe('export-json-btn')?.click();
          break;
        case 'new-folder':
          mainDom.elMaybe('new-folder-btn')?.click();
          break;
        case 'check-deps':
          showToast(t('context.checking_deps'), 'info');
          sessionStorage.removeItem('dismissed_ue4ss_update');
          sessionStorage.removeItem('dismissed_palschema_update');
          (async () => {
            try {
              const { checkDependenciesFull } = await import('../../../api');
              const fullDeps = await checkDependenciesFull();
              updateState({ dependencies: fullDeps });
              const { renderDependencyBadges } = await import('../dependencies');
              const { renderConflictBanner, removeConflictBanner } = await import('../../conflictBanner');
              renderDependencyBadges(fullDeps);
              if (fullDeps.has_dll_conflict && fullDeps.conflicting_dlls && fullDeps.conflicting_dlls.length > 0) {
                renderConflictBanner(fullDeps.conflicting_dlls);
              } else {
                removeConflictBanner();
              }

              const ue4ssUp = fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop';
              const psUp = fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.palschema_install_mode !== 'Workshop' && fullDeps.palschema_version !== 'Workshop';

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
                    const { executeInstallOrUpdate } = await import('../dependencies');
                    executeInstallOrUpdate('ue4ss', true);
                  }
                } else if (psUp) {
                  const latestVer = fullDeps.palschema_latest_version || 'latest';
                  const confirmed = await showConfirm(
                    t('dependencies.prompt_body_palschema', { installed: fullDeps.palschema_version || 'installed', latest: latestVer }),
                    t('dependencies.prompt_title_palschema')
                  );
                  if (confirmed) {
                    const { executeInstallOrUpdate } = await import('../dependencies');
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
          mainDom.elMaybe('settings-btn')?.click();
          break;
      }
    });
  });

  positionContextMenu(x, y);
}

export function showEditorContextMenu(x: number, y: number): void {
  const menu = mainDom.el('context-menu');

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

  const editorContent = editorDom.elMaybe('editor-content');

  menu.querySelector('#editor-ctx-save')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleEditorSave } = await import('../../editorView');
    await handleEditorSave();
  });

  menu.querySelector('#editor-ctx-cut')?.addEventListener('click', async (e) => {
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

  menu.querySelector('#editor-ctx-copy')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (!editorContent) return;
    const selectedText = editorContent.value.substring(editorContent.selectionStart, editorContent.selectionEnd);
    if (selectedText) {
      await navigator.clipboard.writeText(selectedText);
    }
  });

  menu.querySelector('#editor-ctx-paste')?.addEventListener('click', async (e) => {
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

  menu.querySelector('#editor-ctx-find')?.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    const ev = new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true });
    document.dispatchEvent(ev);
  });

  menu.querySelector('#editor-ctx-selectall')?.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (editorContent) {
      editorContent.focus();
      editorContent.select();
    }
  });
}

export function showLibraryContextMenu(modId: string | null, zipName: string | null, x: number, y: number): void {
  const menu = mainDom.el('context-menu');
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

  menu.querySelector('#lib-ctx-check-updates')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleCheckLocalLibraryOnlineUpdates } = await import('../library');
    handleCheckLocalLibraryOnlineUpdates();
  });

  menu.querySelector('#lib-ctx-open-folder')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { openUrl, getSettings } = await import('../../../api');
    try {
      const settings = await getSettings();
      const libPath = `${settings.programPath}/mods-library`;
      await openUrl(libPath);
    } catch (err) {
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });

  menu.querySelector('#lib-ctx-install')?.addEventListener('click', (e) => {
    e.stopPropagation();
    hideContextMenu();
    if (selectedCount > 1) {
      handleLibraryBulkInstall();
    } else if (modId) {
      triggerInstallFromLibrary(modId);
    }
  });

  menu.querySelector('#lib-ctx-remove')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { removeFromLibrary } = await import('../../../api');
    const { showConfirm } = await import('../../confirm');
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
  const menu = mainDom.el('context-menu');
  
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

  menu.querySelector('#ws-ctx-check-updates')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleCheckWorkshopOnlineUpdates } = await import('../library');
    handleCheckWorkshopOnlineUpdates();
  });

  menu.querySelector('#ws-ctx-force-verify')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    hideContextMenu();
    const { handleTriggerSteamValidation } = await import('../library');
    handleTriggerSteamValidation();
  });

  if (workshopId) {
    menu.querySelector('#ws-ctx-open-steam')?.addEventListener('click', async (e) => {
      e.stopPropagation();
      hideContextMenu();
      const { openUrl } = await import('../../../api');
      openUrl(`steam://url/CommunityFilePage/${workshopId}`);
    });
  }
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
        import('../library').then(({ _activeLibrarySubTab }) => {
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
        import('../../../features/selection').then(({ updateSelection }) => {
          updateSelection(new Set([id]));
          showContextMenu(id, e.clientX, e.clientY);
        });
      }
    } else if (target.closest('#mods-container')) {
      showGlobalContextMenu(e.clientX, e.clientY);
    }
  });

  mainDom.el('context-overlay').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) {
      hideContextMenu();
    }
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hideContextMenu();
  });
}
