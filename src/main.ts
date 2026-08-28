import 'highlight.js/styles/github-dark.css';
import './utils/imageFallback';
import { initI18n, t } from './utils/i18n';
import { getSettings, exportModsJson, setModProfileState, logFromJs, createBackup, restoreBackup, analyzeBackup, checkDependencies, installUe4ss, installPalschema, launchGame } from './api';
import { getState, updateState, subscribe } from './state';
import type { AppState } from './state';
import { openSettingsModal, handleInstall, handleSaveSettings, handleSettingsBrowse, handleConfirmInstall, closeInstallModal, closeSettingsModal, handleDataPathChange, openWorkshopModal, openAboutModal, closeAboutModal, setupAboutModal } from './ui/modal';
import { loadMods, handleSort, handleCheckUpdates, handleOpenAllUpdates, handleDisableAll, handleEnableAll, setupFilterListeners, renderModsView, populateAdvancedFilters, setupAdvancedFilterHandlers, setupStatusFilterHandlers, loadGameVersion, loadProfiles, loadLibrary, handleProfileChange, handleCreateProfile, setupContextMenu, loadDependencies, setupLibraryHandlers } from './ui/modsView';
import { closeDetailPanel, handleRefreshDetail, handleDetailConfig, handleDetailToggle, handleDetailRemove, handleDetailSetConfig, handleDetailClearConfig, handleDetailOpenFolder, handleDetailOpenExtraFolder, handleDetailRename, openDetailPanel } from './ui/detailPanel';
import { switchTab, handleEditorSave, handleEditorFormat, handleEditorModChange, setupEditorKeybindings, setupEditorFindHandlers, setupEditorFsWatcher } from './ui/editorView';
import { navigateTo } from './ui/tabManager';
import { renderDbView } from './ui/dbView';
import { setupDragAndDrop } from './features/dragdrop';
import { autoFetchNexusInfo } from './features/nexus';
import { showToast } from './ui/toast';
import { setupSelection } from './features/selection';
import { initPackerView } from './ui/packerView';
import { renderScannerView } from './ui/scannerView';
import { mainDom, bind, createBinderGroup, bus } from './framework';

const THEME_KEY = 'pmm-theme';


function getPreferredTheme(): 'dark' | 'light' {
  const stored = localStorage.getItem(THEME_KEY);
  if (stored === 'dark' || stored === 'light') return stored;
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function applyTheme(theme: 'dark' | 'light'): void {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem(THEME_KEY, theme);
  updateThemeToggleBtn();
}

function updateThemeToggleBtn(): void {
  const btn = mainDom.elMaybe('theme-toggle-btn');
  if (!btn) return;
  const current = document.documentElement.dataset.theme || 'dark';
  btn.textContent = current === 'dark' ? t('settings.btn_theme_light') : t('settings.btn_theme_dark');
}

function safeEl(id: string): HTMLElement | null {
  return document.getElementById(id);
}

function showApp(): void {
  const loading = mainDom.elMaybe('app-loading');
  if (loading) loading.style.display = 'none';
  const app = mainDom.elMaybe('app');
  if (app) app.style.display = 'flex';
}

import { loadAppTemplates } from './ui/templateLoader';

loadAppTemplates();
initI18n();
showApp();

async function init() {
  console.time('init');
  applyTheme(getPreferredTheme());

  try {
    await logFromJs("⚡ PMM-Core Reactive Engine: Frontend runtime initialized (Zero-VDOM, 11 scopes, Svelte-like)");
    console.info('%c⚡ PMM-Core Reactive Engine initialized (Zero-VDOM, 11 scopes)', 'color: #38bdf8; font-weight: bold; font-size: 13px;');
    const settings = await getSettings();
    updateState({ currentSettings: settings });
    if (settings.language) {
      initI18n(settings.language);
    }
    const { updateLoadTabVisibility } = await import('./ui/loadView');
    updateLoadTabVisibility();
    const scale = settings.toolbarScale || 1.0;
    document.documentElement.style.setProperty('--toolbar-scale', scale.toString());

    if (settings.gamePath) {
      console.time('startupSequence');
      await loadGameVersion();
      await loadProfiles();
      await loadDependencies();
      await loadLibrary();
      await loadMods();
      console.timeEnd('startupSequence');

      const hasMissingMetadata = getState().allMods.some(m => !m.nexusModId && m.version === 'unknown');
      if (hasMissingMetadata) {
        console.log('Some mods missing metadata');
      }
      autoFetchNexusInfo();
    } else {
      openSettingsModal();
    }

    setupEventListeners();
    setupEditorFsWatcher();
    initPackerView();

    // Initialize Nexus OAuth & deep link listener
    import('./features/nexus_auth').then(({ initNexusAuth }) => {
      initNexusAuth();
    }).catch(err => console.warn('Failed to init Nexus Auth:', err));

    // Reveal the window smoothly once DOM and initial UI are fully ready
    import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
      getCurrentWindow().show().catch(() => {});
    }).catch(() => {});
  } catch (e) {
    console.error('Error initializing:', e);
    import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
      getCurrentWindow().show().catch(() => {});
    }).catch(() => {});
  }
  console.timeEnd('init');
}

function setupEventListeners() {
  safeEl('scan-btn')?.addEventListener('click', async () => {
    await loadMods();
    await loadDependencies();
  });

  safeEl('install-btn')?.addEventListener('click', handleInstall);
  safeEl('modal-close-x')?.addEventListener('click', closeInstallModal);
  safeEl('modal-cancel')?.addEventListener('click', closeInstallModal);
  safeEl('modal-confirm')?.addEventListener('click', handleConfirmInstall);
  safeEl('settings-btn')?.addEventListener('click', openSettingsModal);
  safeEl('settings-cancel')?.addEventListener('click', closeSettingsModal);
  safeEl('settings-save')?.addEventListener('click', handleSaveSettings);
  safeEl('settings-browse-btn')?.addEventListener('click', handleSettingsBrowse);
  safeEl('settings-data-path-select')?.addEventListener('change', handleDataPathChange);
  
  setupAboutModal();

  const registerOpenFolderBtn = (id: string, type: 'ue4ss' | 'palschema' | 'paks' | 'app_data' | 'profile') => {
    safeEl(id)?.addEventListener('click', async () => {
      const { openFolderByType } = await import('./api');
      try {
        await openFolderByType(type);
      } catch (err) {
        const { showToast } = await import('./ui/toast');
        showToast(String(err), 'error');
      }
    });
  };

  registerOpenFolderBtn('open-folder-paks', 'paks');
  registerOpenFolderBtn('open-folder-ue4ss', 'ue4ss');
  registerOpenFolderBtn('open-folder-palschema', 'palschema');
  registerOpenFolderBtn('open-folder-appdata', 'app_data');
  registerOpenFolderBtn('open-folder-profile', 'profile');

  safeEl('theme-toggle-btn')?.addEventListener('click', () => {
    const current = document.documentElement.dataset.theme || 'dark';
    applyTheme(current === 'dark' ? 'light' : 'dark');
  });
  safeEl('detail-close')?.addEventListener('click', closeDetailPanel);
  safeEl('detail-overlay')?.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeDetailPanel();
  });
  safeEl('detail-refresh')?.addEventListener('click', handleRefreshDetail);
  safeEl('detail-config')?.addEventListener('click', handleDetailConfig);
  safeEl('detail-toggle')?.addEventListener('click', handleDetailToggle);
  safeEl('detail-remove')?.addEventListener('click', handleDetailRemove);
  safeEl('check-updates-btn')?.addEventListener('click', handleCheckUpdates);
  safeEl('open-all-updates-btn')?.addEventListener('click', handleOpenAllUpdates);
  safeEl('disable-all-btn')?.addEventListener('click', handleDisableAll);
  safeEl('enable-all-btn')?.addEventListener('click', handleEnableAll);
  safeEl('export-json-btn')?.addEventListener('click', async () => {
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
  safeEl('backup-btn')?.addEventListener('click', async () => {
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
  safeEl('restore-backup-btn')?.addEventListener('click', async () => {
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

        const { showConfirm } = await import('./ui/confirm');
        const confirmed = await showConfirm(
          t('dialogs.confirm_backup_deps_missing', { deps: missingList.join(' & ') })
        );
        if (!confirmed) {
          showToast(t('toasts.export_failed', { error: 'Missing dependencies' }), 'error');
          return;
        }

        const { loadDependencies } = await import('./ui/modsView');

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
  safeEl('detail-set-config')?.addEventListener('click', handleDetailSetConfig);
  safeEl('detail-clear-config')?.addEventListener('click', handleDetailClearConfig);
  safeEl('detail-open-folder')?.addEventListener('click', handleDetailOpenFolder);
  safeEl('detail-open-extra-folder')?.addEventListener('click', handleDetailOpenExtraFolder);
  safeEl('detail-rename-btn')?.addEventListener('click', handleDetailRename);
  safeEl('editor-save-btn')?.addEventListener('click', handleEditorSave);
  safeEl('editor-format-btn')?.addEventListener('click', handleEditorFormat);
  safeEl('editor-mod-select')?.addEventListener('change', handleEditorModChange);

  // Profile
  safeEl('profile-select')?.addEventListener('change', (e) => {
    const select = e.currentTarget as HTMLSelectElement;
    handleProfileChange(select.value);
  });
  safeEl('profile-manager-btn')?.addEventListener('click', () => {
    const modal = document.getElementById('profile-modal');
    if (modal) {
      modal.classList.add('visible');
      modal.focus();
    }
  });
  safeEl('launch-game-btn')?.addEventListener('click', async () => {
    const { showConfirm } = await import('./ui/confirm');
    const { checkDependencies, checkDependenciesFull } = await import('./api');
    const { getState } = await import('./state');
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
        const { handleDepBadgeClick } = await import('./ui/modsView');
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
        const { handleDepBadgeClick } = await import('./ui/modsView');
        handleDepBadgeClick('palschema');
      }
      return;
    }

    // 2. Check if installed dependencies have pending updates
    let updatePromptMessage = '';
    const fullDeps = await checkDependenciesFull().catch(() => deps);
    const ue4ssNeedsUpdate = fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop';
    const palschemaNeedsUpdate = fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.palschema_version !== 'Workshop';

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
  safeEl('new-folder-btn')?.addEventListener('click', async () => {
    const { showInputModal, handleCreateFolder } = await import('./ui/modsView');
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
  safeEl('profile-create-btn')?.addEventListener('click', async () => {
    const input = document.getElementById('profile-new-name') as HTMLInputElement | null;
    if (!input || !input.value.trim()) return;
    await handleCreateProfile(input.value.trim());
    input.value = '';
  });
  safeEl('profile-new-name')?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const input = e.currentTarget as HTMLInputElement;
      if (input.value.trim()) {
        handleCreateProfile(input.value.trim());
        input.value = '';
      }
    }
  });
  function closeProfileModal(): void {
    document.getElementById('profile-modal')?.classList.remove('visible');
  }
  safeEl('profile-modal-close')?.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileModal();
  });
  safeEl('profile-modal-close-x')?.addEventListener('click', (e) => {
    e.stopPropagation();
    closeProfileModal();
  });
  safeEl('profile-modal')?.addEventListener('click', (e) => {
    if (e.target === e.currentTarget) {
      e.stopPropagation();
      closeProfileModal();
    }
  });

  // Library
  safeEl('library-refresh-btn')?.addEventListener('click', loadLibrary);

  document.querySelectorAll('.sidebar-tab').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const tab = (btn as HTMLElement).dataset.tab as any;
      const state = getState();
      if (state.activeTab === 'editor' && tab !== 'editor') {
        const { confirmDiscardOrSave } = await import('./ui/editorView');
        const proceed = await confirmDiscardOrSave();
        if (!proceed) return;
      }
      navigateTo(tab);

      if (tab === 'editor') {
        if (!state.editorModId && state.allMods.length > 0) {
          const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
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
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      // 1. Confirm / Prompt custom overlays (highest z-index, handled by their own listeners)
      const confirmOverlay = document.querySelector('.confirm-overlay');
      if (confirmOverlay) {
        return;
      }

      // 2. Unreal Engine Asset Inspector modal (z-index: 10000)
      const uassetModal = document.getElementById('uasset-inspector-modal');
      if (uassetModal) {
        uassetModal.remove();
        return;
      }

      // 3. Save Health Doctor Comparison modal (z-index: 9999)
      const saveCompareModal = document.getElementById('save-compare-modal');
      if (saveCompareModal) {
        saveCompareModal.remove();
        return;
      }

      // 4. Discovery Lightbox Image modal
      const discoveryImageModal = document.getElementById('discovery-image-modal');
      if (discoveryImageModal && (discoveryImageModal.classList.contains('visible') || (discoveryImageModal.style.display && discoveryImageModal.style.display !== 'none'))) {
        discoveryImageModal.classList.remove('visible');
        discoveryImageModal.style.display = 'none';
        return;
      }

      // 5. File Tree / Show Files Modal from installer or library (z-index: 4500)
      const fileTreeModal = document.getElementById('file-tree-modal');
      if (fileTreeModal) {
        fileTreeModal.remove();
        return;
      }

      // 6. Full files modal overlay / Archive view modals
      const fullFilesOverlay = document.getElementById('full-files-modal-overlay');
      if (fullFilesOverlay) {
        fullFilesOverlay.remove();
        return;
      }
      const archiveViewModal = document.getElementById('archive-view-modal');
      if (archiveViewModal) {
        archiveViewModal.remove();
        return;
      }
      const archiveStructureModal = document.getElementById('archive-structure-modal');
      if (archiveStructureModal) {
        archiveStructureModal.remove();
        return;
      }

      // 7. Config Diff modal
      const configDiffOverlay = document.getElementById('config-diff-modal');
      if (configDiffOverlay) {
        configDiffOverlay.remove();
        return;
      }

      // 8. Install Modal (when open above Discovery or Mods View)
      const installModal = document.getElementById('install-modal');
      if (installModal?.classList.contains('visible')) {
        closeInstallModal();
        return;
      }

      // 9. Discovery Mod Details Modal
      const discoveryModModal = document.getElementById('discovery-mod-modal');
      if (discoveryModModal && (discoveryModModal.classList.contains('visible') || (discoveryModModal.style.display && discoveryModModal.style.display !== 'none'))) {
        import('./ui/discoveryView').then(({ closeDiscoveryModal }) => closeDiscoveryModal()).catch(() => {
          discoveryModModal.classList.remove('visible');
          discoveryModModal.style.display = 'none';
        });
        return;
      }

      // 10. Nexus Profile Modal
      const nexusProfileModal = document.getElementById('nexus-profile-modal');
      if (nexusProfileModal?.classList.contains('visible')) {
        nexusProfileModal.classList.remove('visible');
        return;
      }

      // 11. Workshop Modal
      const workshopModal = document.getElementById('workshop-modal');
      if (workshopModal?.classList.contains('visible')) {
        workshopModal.classList.remove('visible');
        return;
      }

      // 12. Console Modal
      const consoleModal = document.getElementById('console-modal');
      if (consoleModal?.classList.contains('visible')) {
        consoleModal.classList.remove('visible');
        return;
      }

      // 13. Settings Modal
      const settingsModal = document.getElementById('settings-modal');
      if (settingsModal?.classList.contains('visible')) {
        closeSettingsModal();
        return;
      }

      // 14. Profile Modal
      const profileModal = document.getElementById('profile-modal');
      if (profileModal?.classList.contains('visible')) {
        closeProfileModal();
        return;
      }

      // 15. About Modal
      const aboutModal = document.getElementById('about-modal');
      if (aboutModal?.classList.contains('visible')) {
        closeAboutModal();
        return;
      }

      // 16. Detail Overlay Panel
      const detailOverlay = document.getElementById('detail-overlay');
      if (detailOverlay?.classList.contains('visible')) {
        closeDetailPanel();
        return;
      }

      // 17. If no modals are open, clear mod selection on ESC
      import('./features/selection').then(({ clearSelection }) => clearSelection());
    }
  });

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
  import('./features/nxm_queue').then(({ initNxmQueue }) => initNxmQueue()).catch(err => console.error("Failed to init NXM queue:", err));

  // Listen to system events
  import('@tauri-apps/api/event').then(({ listen }) => {
    listen<string>('dns-fallback-triggered', (event) => {
      showToast(t('toasts.dns_fallback_recovered', { provider: event.payload }), 'info');
    });
  }).catch(err => console.error("Failed to register event listeners:", err));
}


init();

