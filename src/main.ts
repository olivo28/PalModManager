import 'highlight.js/styles/github-dark.css';
import { getSettings, exportModsJson, setModProfileState, logFromJs } from './api';
import { getState, updateState } from './state';
import { openSettingsModal, handleInstall, handleSaveSettings, handleSettingsBrowse, handleToggleKeyVisibility, handleConfirmInstall, closeInstallModal, closeSettingsModal } from './ui/modal';
import { loadMods, handleSort, handleCheckUpdates, handleDisableAll, handleEnableAll, setupFilterListeners, renderModsView, populateAdvancedFilters, setupAdvancedFilterHandlers, setupStatusFilterHandlers, loadGameVersion, loadProfiles, loadLibrary, handleProfileChange, handleCreateProfile, setupContextMenu, loadDependencies, setupLibraryHandlers } from './ui/modsView';
import { closeDetailPanel, handleRefreshDetail, handleDetailConfig, handleDetailToggle, handleDetailRemove, handleDetailSetConfig, handleDetailClearConfig, handleDetailOpenFolder, handleDetailRename, openDetailPanel } from './ui/detailPanel';
import { switchTab, handleEditorSave, handleEditorFormat, handleEditorModChange, setupEditorKeybindings, setupEditorFindHandlers } from './ui/editorView';
import { setupDragAndDrop } from './features/dragdrop';
import { autoFetchNexusInfo } from './features/nexus';
import { showToast } from './ui/toast';
import { setupSelection } from './features/selection';

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
  const btn = document.getElementById('theme-toggle-btn') as HTMLButtonElement | null;
  if (!btn) return;
  const current = document.documentElement.dataset.theme || 'dark';
  btn.textContent = current === 'dark' ? 'Switch to Light Theme' : 'Switch to Dark Theme';
}

function safeEl(id: string): HTMLElement | null {
  return document.getElementById(id);
}

function showApp(): void {
  const loading = document.getElementById('app-loading');
  if (loading) loading.style.display = 'none';
  const app = document.getElementById('app');
  if (app) app.style.display = 'flex';
}

showApp();

async function init() {
  console.time('init');
  applyTheme(getPreferredTheme());

  try {
    await logFromJs("JS: Iniciando script de frontend (main.ts)");
    const settings = await getSettings();
    updateState({ currentSettings: settings });

    // Load additional data in parallel
    console.time('loadStartupData');
    await Promise.all([
      loadProfiles().then(() => logFromJs("JS: Perfiles cargados")),
      loadLibrary().then(() => logFromJs("JS: Librería cargada")),
    ]);
    console.timeEnd('loadStartupData');

    if (settings.gamePath) {
      console.time('loadMods');
      await loadMods();
      console.timeEnd('loadMods');
      const hasMissingMetadata = getState().allMods.some(m => !m.nexusModId && m.version === 'unknown');
      if (hasMissingMetadata) {
        console.log('Some mods missing metadata');
      }
      autoFetchNexusInfo();
    } else {
      openSettingsModal();
    }

    setupEventListeners();
  } catch (e) {
    console.error('Error initializing:', e);
  }
  console.timeEnd('init');
  // Defer slow scans so UI renders instantly
  setTimeout(() => {
    loadGameVersion();
    loadDependencies();
  }, 0);
}

function setupEventListeners() {
  safeEl('scan-btn')?.addEventListener('click', () => loadMods());
  safeEl('install-btn')?.addEventListener('click', handleInstall);
  safeEl('modal-cancel')?.addEventListener('click', closeInstallModal);
  safeEl('modal-confirm')?.addEventListener('click', handleConfirmInstall);
  safeEl('settings-btn')?.addEventListener('click', openSettingsModal);
  safeEl('settings-cancel')?.addEventListener('click', closeSettingsModal);
  safeEl('settings-save')?.addEventListener('click', handleSaveSettings);
  safeEl('settings-browse-btn')?.addEventListener('click', handleSettingsBrowse);
  safeEl('settings-toggle-key')?.addEventListener('click', handleToggleKeyVisibility);
  safeEl('nexus-api-key-link')?.addEventListener('click', async (e) => {
    e.preventDefault();
    const { openUrl } = await import('./api');
    openUrl('https://www.nexusmods.com/settings/api-keys');
  });
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
      showToast(`Exported to ${saved}`, 'success');
    } catch (e) {
      showToast('Export failed: ' + e, 'error');
    }
  });
  safeEl('detail-set-config')?.addEventListener('click', handleDetailSetConfig);
  safeEl('detail-clear-config')?.addEventListener('click', handleDetailClearConfig);
  safeEl('detail-open-folder')?.addEventListener('click', handleDetailOpenFolder);
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
      const tab = (btn as HTMLElement).dataset.tab as 'mods' | 'editor' | 'library';
      const state = getState();
      if (state.activeTab === 'editor' && tab !== 'editor') {
        const { confirmDiscardOrSave } = await import('./ui/editorView');
        const proceed = await confirmDiscardOrSave();
        if (!proceed) return;
      }
      switchTab(tab);
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
      }
    });
  });

  document.querySelectorAll('.sort-btn').forEach((btn) => {
    btn.addEventListener('click', () => handleSort(btn as HTMLButtonElement));
  });

  const searchInput = document.getElementById('search-input') as HTMLInputElement | null;
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
      const detailOverlay = document.getElementById('detail-overlay');
      if (detailOverlay?.classList.contains('visible')) closeDetailPanel();
      
      const settingsModal = document.getElementById('settings-modal');
      if (settingsModal?.classList.contains('visible')) closeSettingsModal();
      
      const profileModal = document.getElementById('profile-modal');
      if (profileModal?.classList.contains('visible')) closeProfileModal();
      
      const installModal = document.getElementById('install-modal');
      if (installModal?.classList.contains('visible')) closeInstallModal();

      // Clear mod selection on ESC if no modals are open
      const hasOpenModal = 
        (detailOverlay && detailOverlay.classList.contains('visible')) ||
        (settingsModal && settingsModal.classList.contains('visible')) ||
        (profileModal && profileModal.classList.contains('visible')) ||
        (installModal && installModal.classList.contains('visible'));
      if (!hasOpenModal) {
        import('./features/selection').then(({ clearSelection }) => clearSelection());
      }
    }
  });

  setupSelection();
  setupLibraryHandlers();
}


init();
