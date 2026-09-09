import { mainDom, settingsDom, detailDom, editorDom, libraryDom, bind, createBinderGroup, bus } from '../framework';
import { t } from '../utils/i18n';
import { getState, updateState, subscribe } from '../state';
import type { AppState } from '../state';
import { exportModsJson, createBackup, restoreBackup, analyzeBackup, checkDependencies, installUe4ss, installPalschema, launchGame } from '../api';
import { setupModalListeners } from '../ui/modal';
import {
  loadMods,
  handleCheckUpdates,
  handleOpenAllUpdates,
  handleDisableAll,
  handleEnableAll,
  setupFilterListeners,
  renderModsView,
  setupAdvancedFilterHandlers,
  setupStatusFilterHandlers,
  loadGameVersion,
  loadProfiles,
  loadLibrary,
  handleProfileChange,
  handleCreateProfile,
  setupContextMenu,
  loadDependencies,
  setupLibraryHandlers,
} from '../ui/modsView';
import {
  closeDetailPanel,
  handleRefreshDetail,
  handleDetailConfig,
  handleDetailToggle,
  handleDetailRemove,
  handleDetailSetConfig,
  handleDetailClearConfig,
  handleDetailOpenFolder,
  handleDetailOpenExtraFolder,
  handleDetailRename,
} from '../ui/detailPanel';
import {
  handleEditorSave,
  handleEditorRevert,
  handleEditorFormat,
  handleEditorModChange,
  setupEditorKeybindings,
  setupEditorFindHandlers,
} from '../ui/editorView';
import { navigateTo } from '../ui/tabManager';
import { renderDbView } from '../ui/dbView';
import { setupDragAndDrop } from '../features/dragdrop';
import { showToast } from '../ui/toast';
import { setupSelection } from '../features/selection';
import { renderScannerView } from '../ui/scannerView';
import { applyTheme } from './theme';
import { setupGlobalShortcuts } from './shortcuts';

function closeProfileModal(): void {
  mainDom.elMaybe('profile-modal')?.classList.remove('visible');
}

export function setupEventListeners(): void {
  setupModalListeners();

  mainDom.elMaybe('scan-btn')?.addEventListener('click', async () => {
    await loadMods();
    await loadDependencies();
  });

  const registerOpenFolderBtn = (id: 'open-folder-paks' | 'open-folder-ue4ss' | 'open-folder-palschema' | 'open-folder-appdata' | 'open-folder-profile', type: 'ue4ss' | 'palschema' | 'paks' | 'app_data' | 'profile') => {
    settingsDom.elMaybe(id)?.addEventListener('click', async () => {
      const { openFolderByType } = await import('../api');
      try {
        await openFolderByType(type);
      } catch (err) {
        const { showToast } = await import('../ui/toast');
        showToast(String(err), 'error');
      }
    });
  };

  registerOpenFolderBtn('open-folder-paks', 'paks');
  registerOpenFolderBtn('open-folder-ue4ss', 'ue4ss');
  registerOpenFolderBtn('open-folder-palschema', 'palschema');
  registerOpenFolderBtn('open-folder-appdata', 'app_data');
  registerOpenFolderBtn('open-folder-profile', 'profile');

  mainDom.elMaybe('theme-toggle-btn')?.addEventListener('click', () => {
    const current = document.documentElement.dataset.theme || 'dark';
    applyTheme(current === 'dark' ? 'light' : 'dark');
  });
  detailDom.elMaybe('detail-close')?.addEventListener('click', closeDetailPanel);
  detailDom.elMaybe('detail-overlay')?.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeDetailPanel();
  });
  detailDom.elMaybe('detail-refresh')?.addEventListener('click', handleRefreshDetail);
  detailDom.elMaybe('detail-config')?.addEventListener('click', handleDetailConfig);
  detailDom.elMaybe('detail-toggle')?.addEventListener('click', handleDetailToggle);
  detailDom.elMaybe('detail-remove')?.addEventListener('click', handleDetailRemove);
  mainDom.elMaybe('check-updates-btn')?.addEventListener('click', handleCheckUpdates);
  mainDom.elMaybe('open-all-updates-btn')?.addEventListener('click', handleOpenAllUpdates);
  mainDom.elMaybe('disable-all-btn')?.addEventListener('click', handleDisableAll);
  mainDom.elMaybe('enable-all-btn')?.addEventListener('click', handleEnableAll);
  mainDom.elMaybe('export-json-btn')?.addEventListener('click', async () => {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        defaultPath: 'palmodmanager_mods.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });
      if (!path) return;
      const saved = await exportModsJson(typeof path === 'string' ? path : path as string);
      showToast(t('toasts.export_success', { path: saved }), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    }
  });
  mainDom.elMaybe('backup-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('mods.dialog_backup_folder_title'),
      });
      if (!selected) return;
      const targetDir = typeof selected === 'string' ? selected : selected as string;
      showToast(t('toasts.backup_creating'), 'info');
      const savedPath = await createBackup(targetDir);
      showToast(t('toasts.backup_created', { path: savedPath }), 'success');
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    }
  });
  mainDom.elMaybe('restore-backup-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{ name: 'PMM Backup Zip', extensions: ['zip'] }],
        title: t('mods.dialog_restore_zip_title'),
      });
      if (!selected) return;
      const zipPath = typeof selected === 'string' ? selected : selected as string;

      // 1. Analyze ZIP contents
      const analysis = await analyzeBackup(zipPath);

      // 2. Check current dependencies status
      const depStatus = await checkDependencies();

      let missingUe4ss = analysis.hasUe4ss && !depStatus.ue4ss_installed;
      let missingPalSchema = analysis.hasPalSchema && !depStatus.palschema_installed;

      if (missingUe4ss || missingPalSchema) {
        let missingList: string[] = [];
        if (missingUe4ss) missingList.push('UE4SS');
        if (missingPalSchema) missingList.push('PalSchema');

        const { showConfirm } = await import('../ui/confirm');
        const confirmed = await showConfirm(
          t('dialogs.confirm_backup_deps_missing', { deps: missingList.join(' & ') })
        );
        if (!confirmed) {
          showToast(t('toasts.export_failed', { error: 'Missing dependencies' }), 'error');
          return;
        }

        const { loadDependencies } = await import('../ui/modsView');

        // Install missing dependencies
        if (missingUe4ss) {
          showToast(t('toasts.installing_dep', { dep: 'UE4SS' }), 'info');
          await installUe4ss();
          await loadDependencies();
        }
        if (missingPalSchema) {
          showToast(t('toasts.installing_dep', { dep: 'PalSchema' }), 'info');
          await installPalschema();
          await loadDependencies();
        }
        showToast(t('dependencies.up_to_date'), 'success');
      }

      showToast(t('toasts.backup_restoring'), 'info');
      await restoreBackup(zipPath);
      showToast(t('toasts.backup_restored'), 'success');
      loadMods();
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    }
  });
  detailDom.elMaybe('detail-set-config')?.addEventListener('click', handleDetailSetConfig);
  detailDom.elMaybe('detail-clear-config')?.addEventListener('click', handleDetailClearConfig);
  detailDom.elMaybe('detail-open-folder')?.addEventListener('click', handleDetailOpenFolder);
  detailDom.elMaybe('detail-open-extra-folder')?.addEventListener('click', handleDetailOpenExtraFolder);
  detailDom.elMaybe('detail-rename-btn')?.addEventListener('click', handleDetailRename);
  editorDom.elMaybe('editor-save-btn')?.addEventListener('click', handleEditorSave);
  editorDom.elMaybe('editor-revert-btn')?.addEventListener('click', handleEditorRevert);
  editorDom.elMaybe('editor-format-btn')?.addEventListener('click', handleEditorFormat);
  editorDom.elMaybe('editor-mod-select')?.addEventListener('change', handleEditorModChange);

  // Profile
  mainDom.elMaybe('profile-select')?.addEventListener('change', (e) => {
    const select = e.currentTarget as HTMLSelectElement;
    handleProfileChange(select.value);
  });
  mainDom.elMaybe('profile-manager-btn')?.addEventListener('click', () => {
    const modal = mainDom.elMaybe('profile-modal');
    if (modal) {
      modal.classList.add('visible');
      modal.focus();
    }
  });
  mainDom.elMaybe('launch-game-btn')?.addEventListener('click', async () => {
    const { showConfirm } = await import('../ui/confirm');
    const { checkDependencies, checkDependenciesFull } = await import('../api');
    const { getState } = await import('../state');
    const state = getState();

    // 1. Check if active mods need UE4SS or PalSchema that aren't installed
    const activeMods = state.allMods.filter(m => m.enabled);
    const hasUe4ssMods = activeMods.some(m => m.type === 'ue4ss' || m.type === 'hybrid');
    const hasPalSchemaMods = activeMods.some(m => m.type === 'palschema' || m.type === 'hybrid');

    const deps = await checkDependencies();

    if (hasUe4ssMods && !deps.ue4ss_installed) {
      const confirmed = await showConfirm(
        t('sidebar.launch_game'),
        t('launch.warn_missing_ue4ss'),
        t('common.install'),
        t('common.cancel')
      );
      if (confirmed) {
        const { handleDepBadgeClick } = await import('../ui/modsView');
        handleDepBadgeClick('ue4ss');
      }
      return;
    }

    if (hasPalSchemaMods && !deps.palschema_installed) {
      const confirmed = await showConfirm(
        t('sidebar.launch_game'),
        t('launch.warn_missing_palschema'),
        t('common.install'),
        t('common.cancel')
      );
      if (confirmed) {
        const { handleDepBadgeClick } = await import('../ui/modsView');
        handleDepBadgeClick('palschema');
      }
      return;
    }

    // 2. Check if installed dependencies have pending updates
    let updatePromptMessage = '';
    const fullDeps = await checkDependenciesFull().catch(() => deps);
    const ue4ssNeedsUpdate = fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop';
    const palschemaNeedsUpdate = fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.palschema_install_mode !== 'Workshop' && fullDeps.palschema_version !== 'Workshop';

    if (ue4ssNeedsUpdate && palschemaNeedsUpdate) {
      updatePromptMessage = t('launch.warn_update_both', {
        ue4ssCurrent: fullDeps.ue4ss_version || 'old',
        ue4ssTarget: fullDeps.ue4ss_latest_date || fullDeps.ue4ss_latest_tag || 'latest',
        palschemaCurrent: fullDeps.palschema_version || 'old',
        palschemaTarget: fullDeps.palschema_latest_version || 'latest'
      });
    } else if (ue4ssNeedsUpdate) {
      updatePromptMessage = t('launch.warn_update_ue4ss', {
        current: fullDeps.ue4ss_version || 'old',
        target: fullDeps.ue4ss_latest_date || fullDeps.ue4ss_latest_tag || 'latest'
      });
    } else if (palschemaNeedsUpdate) {
      updatePromptMessage = t('launch.warn_update_palschema', {
        current: fullDeps.palschema_version || 'old',
        target: fullDeps.palschema_latest_version || 'latest'
      });
    }

    const confirmMsg = updatePromptMessage 
      ? `${updatePromptMessage}\n\n${t('launch.confirm')}`
      : t('launch.confirm');

    const confirmed = await showConfirm(
      t('launch.title'),
      confirmMsg,
      t('launch.btn_confirm'),
      t('common.cancel')
    );
    if (!confirmed) return;
    try {
      await launchGame();
    } catch (e) {
      showToast(t('toasts.game_launch_failed', { error: String(e) }), 'error');
    }
  });
  mainDom.elMaybe('new-folder-btn')?.addEventListener('click', async () => {
    const { showInputModal, handleCreateFolder } = await import('../ui/modsView');
    const newName = await showInputModal(
      t('dialogs.prompt_new_folder_name'),
      t('dialogs.prompt_new_folder_name'),
      'Skins'
    );
    if (newName === null) return;
    const trimmed = newName.trim();
    if (!trimmed) return;
    await handleCreateFolder(trimmed);
  });
  mainDom.elMaybe('profile-create-btn')?.addEventListener('click', async () => {
    const input = mainDom.elMaybe('profile-new-name');
    if (!input || !input.value.trim()) return;
    await handleCreateProfile(input.value.trim());
    input.value = '';
  });
  mainDom.elMaybe('profile-new-name')?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const input = e.currentTarget as HTMLInputElement;
      if (input.value.trim()) {
        handleCreateProfile(input.value.trim());
        input.value = '';
      }
    }
  });
  mainDom.elMaybe('profile-import-btn')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    const { handleImportProfilePack } = await import('../ui/mods/profiles');
    await handleImportProfilePack();
  });
  mainDom.elMaybe('profile-modal-close')?.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileModal();
  });
  mainDom.elMaybe('profile-modal-close-x')?.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileModal();
  });
  mainDom.elMaybe('profile-modal')?.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) {
      e.stopPropagation();
      closeProfileModal();
    }
  });

  // Library
  libraryDom.elMaybe('library-refresh-btn')?.addEventListener('click', loadLibrary);

  document.querySelectorAll('.sidebar-tab').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const tab = (btn as HTMLElement).dataset.tab as any;
      const state = getState();
      if (state.activeTab === 'editor' && tab !== 'editor') {
        const { updateUnsavedIndicator } = await import('../ui/editorView');
        updateUnsavedIndicator();
      }
      navigateTo(tab);

      if (tab === 'editor') {
        if (!state.editorModId && state.allMods.length > 0) {
          const select = editorDom.elMaybe('editor-mod-select');
          if (select) {
            select.value = state.allMods[0].id;
            handleEditorModChange();
          }
        }
      } else if (tab === 'library') {
        loadLibrary();
      } else if (tab === 'scanner') {
        renderScannerView();
      } else if (tab === 'db') {
        renderDbView();
      }
    });
  });

  const sortSelect = mainDom.elMaybe('sort-select');
  if (sortSelect) {
    const current = getState().currentSort;
    sortSelect.value = `${current.field}:${current.asc ? 'asc' : 'desc'}`;
    sortSelect.addEventListener('change', () => {
      const val = sortSelect.value;
      const [field, dir] = val.split(':');
      updateState({ currentSort: { field, asc: dir === 'asc' } });
      renderModsView();
    });
  }

  // Layout Toggle
  const gridBtn = mainDom.elMaybe('layout-grid-btn');
  const listBtn = mainDom.elMaybe('layout-list-btn');

  function updateLayoutUI(layout: 'grid' | 'list') {
    if (layout === 'grid') {
      gridBtn?.classList.add('active');
      listBtn?.classList.remove('active');
    } else {
      listBtn?.classList.add('active');
      gridBtn?.classList.remove('active');
    }
  }

  updateLayoutUI(getState().viewLayout);

  gridBtn?.addEventListener('click', () => {
    updateState({ viewLayout: 'grid' });
    localStorage.setItem('pmm-layout', 'grid');
    updateLayoutUI('grid');
    renderModsView();
  });

  listBtn?.addEventListener('click', () => {
    updateState({ viewLayout: 'list' });
    localStorage.setItem('pmm-layout', 'list');
    updateLayoutUI('list');
    renderModsView();
  });

  const searchInput = mainDom.elMaybe('search-input');
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      updateState({ searchQuery: searchInput.value.toLowerCase() });
      renderModsView();
    });
  }

  setupFilterListeners();
  setupAdvancedFilterHandlers();
  setupStatusFilterHandlers();
  setupDragAndDrop();
  setupEditorKeybindings();
  setupEditorFindHandlers();
  setupContextMenu();
  setupGlobalShortcuts();
  setupSelection();
  setupLibraryHandlers();

  // PMM-Core Declarative State-to-DOM Bindings
  const appBinder = createBinderGroup<AppState>(
    bind(mainDom, 'profile-active-label', {
      text: (s) => s.currentProfile?.name || s.currentProfileId || 'Default'
    }),
    bind(mainDom, 'layout-grid-btn', {
      className: (s) => `layout-btn ${s.viewLayout === 'grid' ? 'active' : ''}`
    }),
    bind(mainDom, 'layout-list-btn', {
      className: (s) => `layout-btn ${s.viewLayout === 'list' ? 'active' : ''}`
    })
  );
  subscribe(appBinder);
  appBinder(getState());

  // PMM-Core Event Mesh Listeners
  bus.on('mods:refresh', () => {
    loadMods().catch(err => console.error("EventBus loadMods error:", err));
  });

  bus.on('backup:restored', () => {
    Promise.all([loadMods(), loadProfiles(), loadDependencies()])
      .catch(err => console.error("EventBus backup:restored reload error:", err));
  });

  bus.on('project:packed', ({ modName }) => {
    loadLibrary().catch(err => console.error("EventBus project:packed reload library error:", err));
  });

  bus.on('workshop:updated', () => {
    Promise.all([loadLibrary(), loadMods()])
      .catch(err => console.error("EventBus workshop:updated reload error:", err));
  });

  bus.on('profile:switched', () => {
    loadGameVersion().catch(err => console.error("EventBus profile:switched version check error:", err));
  });

  // Initialize NXM Download Queue
  import('../features/nxm_queue').then(({ initNxmQueue }) => initNxmQueue()).catch(err => console.error("Failed to init NXM queue:", err));

  // Listen to system events
  import('@tauri-apps/api/event').then(({ listen }) => {
    listen<string>('dns-fallback-triggered', (event) => {
      showToast(t('toasts.dns_fallback_recovered', { provider: event.payload }), 'info');
    });
  }).catch(err => console.error("Failed to register event listeners:", err));
}
