import { getState, updateState } from '../../../state';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import { _tempCustomDataPath, setTempCustomDataPath } from './state';
import { formatBytes } from './helpers';
import { refreshSafetyBackupStatus, refreshStorageUsageStatus, refreshImageCacheStatus, refreshUsmapStatus, refreshSdkStatus, refreshBlueprintsStatus, refreshDatatablesStatus, refreshPalSchemaSchemasStatus } from './status';
import { settingsDom } from '../../../framework';

export function openSettingsModal(): void {
  const modal = settingsDom.el('settings-modal');
  const pathInput = settingsDom.el('settings-game-path');
  const hideNativeCheckbox = settingsDom.elMaybe('settings-hide-native-mods');
  const debugConsoleCheckbox = settingsDom.elMaybe('settings-debug-console');
  const forceLoadOrderUe4ssCheckbox = settingsDom.elMaybe('settings-force-load-order-ue4ss');
  const forceLoadOrderPalschemaCheckbox = settingsDom.elMaybe('settings-force-load-order-palschema');
  const pathStatus = settingsDom.el('settings-path-status');
  const state = getState();

  pathInput.value = state.currentSettings?.gamePath || '';
  if (hideNativeCheckbox) {
    hideNativeCheckbox.checked = !!state.currentSettings?.hideNativeMods;
  }
  if (debugConsoleCheckbox) {
    debugConsoleCheckbox.checked = !!state.currentSettings?.debugConsole;
  }

  const isWindows = navigator.userAgent.toLowerCase().includes('win');
  if (forceLoadOrderPalschemaCheckbox && !isWindows) {
    forceLoadOrderPalschemaCheckbox.disabled = true;
    forceLoadOrderPalschemaCheckbox.checked = false;
  }

  // Bind UE4SS Activation Mode
  const modeEnabledTxtRadio = settingsDom.elMaybe('settings-ue4ss-mode-enabled-txt');
  const modeModsTxtRadio = settingsDom.elMaybe('settings-ue4ss-mode-mods-txt');
  const activeProfile = state.currentProfile || state.profiles?.find(p => p.id === state.currentProfileId);
  const currentMode = activeProfile?.ue4ss_control_mode || activeProfile?.ue4ssControlMode || state.currentSettings?.ue4ssControlMode || 'enabled_txt';

  if (modeEnabledTxtRadio && modeModsTxtRadio) {
    if (currentMode === 'mods_txt') {
      modeModsTxtRadio.checked = true;
    } else {
      modeEnabledTxtRadio.checked = true;
    }

    modeEnabledTxtRadio.onchange = async () => {
      if (modeEnabledTxtRadio.checked) {
        try {
          const { setUe4ssControlMode } = await import('../../../api');
          const { loadMods, renderModsView } = await import('../../modsView');
          const { loadProfiles } = await import('../../mods/profiles');
          const { loadDependencies } = await import('../../mods/dependencies');
          await setUe4ssControlMode('enabled_txt');
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          renderModsView();
          showToast(t('toasts.settings_saved'), 'success');
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      }
    };

    modeModsTxtRadio.onchange = async () => {
      if (modeModsTxtRadio.checked) {
        try {
          const { setUe4ssControlMode } = await import('../../../api');
          const { loadMods, renderModsView } = await import('../../modsView');
          const { loadProfiles } = await import('../../mods/profiles');
          const { loadDependencies } = await import('../../mods/dependencies');
          await setUe4ssControlMode('mods_txt');
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          renderModsView();
          showToast(t('toasts.settings_saved'), 'success');
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      }
    };
  }

  // Bind UE4SS Build Flavor
  const flavorSelect = settingsDom.elMaybe('settings-ue4ss-flavor-select');
  if (flavorSelect) {
    flavorSelect.value = state.currentSettings?.ue4ssBuildFlavor || 'standard';
    flavorSelect.onchange = async () => {
      try {
        const { setUe4ssBuildFlavor } = await import('../../../api');
        const { loadDependencies } = await import('../../mods/dependencies');
        const settings = await setUe4ssBuildFlavor(flavorSelect.value);
        updateState({ currentSettings: settings });
        await loadDependencies(true);
        showToast(t('toasts.settings_saved'), 'success');
      } catch (e) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    };
  }

  if (forceLoadOrderUe4ssCheckbox && forceLoadOrderPalschemaCheckbox) {
    const newUe4ss = forceLoadOrderUe4ssCheckbox.cloneNode(true) as HTMLInputElement;
    forceLoadOrderUe4ssCheckbox.parentNode!.replaceChild(newUe4ss, forceLoadOrderUe4ssCheckbox);

    const newPalschema = forceLoadOrderPalschemaCheckbox.cloneNode(true) as HTMLInputElement;
    forceLoadOrderPalschemaCheckbox.parentNode!.replaceChild(newPalschema, forceLoadOrderPalschemaCheckbox);

    const activeProfile = state.currentProfile || state.profiles?.find(p => p.id === state.currentProfileId);
    newUe4ss.checked = activeProfile?.force_load_order_ue4ss !== undefined && activeProfile?.force_load_order_ue4ss !== null
      ? !!activeProfile.force_load_order_ue4ss
      : !!state.currentSettings?.forceLoadOrderUe4ss;

    newPalschema.checked = isWindows
      ? (activeProfile?.force_load_order_palschema !== undefined && activeProfile?.force_load_order_palschema !== null
          ? !!activeProfile.force_load_order_palschema
          : !!state.currentSettings?.forceLoadOrderPalschema)
      : false;

    const handleSubChange = async (elem: HTMLInputElement, systemName: string, detailMsg: string, requireConfirm: boolean) => {
      if (elem.checked && requireConfirm) {
        const confirmed = await showConfirm(
          t('settings.flo_confirm_title', { systemName }),
          t('settings.flo_confirm_body', { systemName, detailMsg }),
          t('settings.flo_confirm_btn'),
          t('common.cancel')
        );
        if (!confirmed) {
          elem.checked = false;
          return;
        }
      }
    };

    newUe4ss.addEventListener('change', () => {
      handleSubChange(newUe4ss, 'UE4SS', t('settings.flo_detail_ue4ss'), true);
    });

    newPalschema.addEventListener('change', () => {
      handleSubChange(newPalschema, 'PalSchema', t('settings.flo_detail_palschema'), true);
    });
  }

  if (state.currentSettings?.gamePath) {
    pathStatus.textContent = t('settings.path_configured');
    pathStatus.className = 'settings-path-status valid';
  } else {
    pathStatus.textContent = t('settings.path_not_configured');
    pathStatus.className = 'settings-path-status invalid';
  }

  const dataPathSelect = settingsDom.elMaybe('settings-data-path-select');
  const dataPathDisplay = settingsDom.elMaybe('settings-custom-data-path-display');
  setTempCustomDataPath(state.currentSettings?.customDataPath || null);

  if (dataPathSelect) {
    if (!_tempCustomDataPath) {
      dataPathSelect.value = 'default';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else if (_tempCustomDataPath === '__portable__') {
      dataPathSelect.value = 'portable';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else {
      dataPathSelect.value = 'custom';
      if (dataPathDisplay) {
        dataPathDisplay.style.display = 'block';
        dataPathDisplay.textContent = `Custom Folder: ${_tempCustomDataPath}`;
      }
    }
  }

  const scaleInput = settingsDom.elMaybe('settings-toolbar-scale');
  const scaleValue = settingsDom.elMaybe('settings-toolbar-scale-value');
  const initialScale = state.currentSettings?.toolbarScale || 1.0;
  if (scaleInput) {
    scaleInput.value = initialScale.toString();
    if (scaleValue) {
      scaleValue.textContent = `${Math.round(initialScale * 100)}%`;
    }

    scaleInput.addEventListener('input', () => {
      const scale = parseFloat(scaleInput.value);
      if (scaleValue) {
        scaleValue.textContent = `${Math.round(scale * 100)}%`;
      }
      document.documentElement.style.setProperty('--toolbar-scale', scale.toString());
    });
  }

  const openUe4ssBtn = settingsDom.elMaybe('open-folder-ue4ss');
  const openPalschemaBtn = settingsDom.elMaybe('open-folder-palschema');
  if (openUe4ssBtn) {
    openUe4ssBtn.style.display = !!state.dependencies?.ue4ss_installed ? '' : 'none';
  }
  if (openPalschemaBtn) {
    openPalschemaBtn.style.display = !!state.dependencies?.palschema_installed ? '' : 'none';
  }

  refreshSafetyBackupStatus();
  refreshStorageUsageStatus();
  refreshImageCacheStatus();
  refreshUsmapStatus();
  refreshSdkStatus();
  refreshBlueprintsStatus();
  refreshDatatablesStatus();
  refreshPalSchemaSchemasStatus();

  // DNS Resolver Select
  const dnsSelect = settingsDom.elMaybe('settings-dns-resolver-select');
  if (dnsSelect) {
    dnsSelect.value = state.currentSettings?.dnsResolver || 'auto';
  }

  // USMAP Sync Mappings Button
  const syncUsmapBtn = settingsDom.elMaybe('btn-sync-usmap');
  const syncUsmapIcon = settingsDom.elMaybe('btn-sync-usmap-icon');
  if (syncUsmapBtn) {
    syncUsmapBtn.onclick = async () => {
      try {
        syncUsmapBtn.disabled = true;
        if (syncUsmapIcon) syncUsmapIcon.classList.add('spinning');
        showToast(t('settings.usmap_syncing') || 'Syncing USMAP schema...', 'info');
        const { syncMappingsNow } = await import('../../../api');
        const res = await syncMappingsNow();
        if (res.isSynced) {
          showToast(t('settings.usmap_sync_success') || 'USMAP schema synced successfully!', 'success');
        } else {
          showToast(t('settings.usmap_sync_complete') || 'USMAP schema sync completed.', 'info');
        }
        await refreshUsmapStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        syncUsmapBtn.disabled = false;
        if (syncUsmapIcon) syncUsmapIcon.classList.remove('spinning');
      }
    };
  }

  // C++ SDK Sync Button
  const syncSdkBtn = settingsDom.elMaybe('btn-sync-sdk');
  const syncSdkIcon = settingsDom.elMaybe('btn-sync-sdk-icon');
  if (syncSdkBtn) {
    syncSdkBtn.onclick = async () => {
      try {
        syncSdkBtn.disabled = true;
        if (syncSdkIcon) syncSdkIcon.classList.add('spinning');
        showToast(t('settings.sdk_syncing') || 'Downloading SDK package from repository...', 'info');
        const { syncSdkFromRepo } = await import('../../../api');
        const res = await syncSdkFromRepo();
        if (res.installed) {
          showToast(t('settings.sdk_sync_success') || `SDK synced: ${res.totalClasses} classes, ${res.totalFunctions} functions!`, 'success');
        } else {
          showToast(t('settings.sdk_sync_complete') || 'SDK sync completed.', 'info');
        }
        await refreshSdkStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        syncSdkBtn.disabled = false;
        if (syncSdkIcon) syncSdkIcon.classList.remove('spinning');
      }
    };
  }

  // C++ SDK Import Local Folder Button
  const importSdkBtn = settingsDom.elMaybe('btn-import-sdk');
  if (importSdkBtn) {
    importSdkBtn.onclick = async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          directory: true,
          multiple: false,
          title: t('settings.sdk_select_folder_title') || 'Select CXXHeaderDump Folder',
        });

        if (selected && typeof selected === 'string') {
          importSdkBtn.disabled = true;
          showToast(t('settings.sdk_importing') || 'Importing and indexing SDK headers...', 'info');
          const { importLocalSdk } = await import('../../../api');
          const res = await importLocalSdk(selected);
          showToast(t('settings.sdk_import_success') || `Imported SDK: ${res.totalClasses} classes, ${res.totalFunctions} functions!`, 'success');
          await refreshSdkStatus();
        }
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        importSdkBtn.disabled = false;
      }
    };
  }

  // C++ SDK Purge Button
  const purgeSdkBtn = settingsDom.elMaybe('btn-purge-sdk');
  if (purgeSdkBtn) {
    purgeSdkBtn.onclick = async () => {
      const confirmed = await showConfirm(
        t('settings.sdk_purge_confirm_title') || 'Clear SDK Headers',
        t('settings.sdk_purge_confirm_msg') || 'Are you sure you want to clear the indexed C++ SDK headers from resources/sdk/?'
      );
      if (!confirmed) return;

      try {
        purgeSdkBtn.disabled = true;
        const { purgeSdkCache } = await import('../../../api');
        await purgeSdkCache();
        showToast(t('settings.sdk_purge_success') || 'SDK headers cleared.', 'success');
        await refreshSdkStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        purgeSdkBtn.disabled = false;
      }
    };
  }

  // Live Game Blueprints Sync Button
  const syncBlueprintsBtn = settingsDom.elMaybe('btn-sync-blueprints');
  const syncBlueprintsIcon = settingsDom.elMaybe('btn-sync-blueprints-icon');
  if (syncBlueprintsBtn) {
    syncBlueprintsBtn.onclick = async () => {
      try {
        syncBlueprintsBtn.disabled = true;
        if (syncBlueprintsIcon) syncBlueprintsIcon.classList.add('spinning');
        showToast(t('settings.blueprints_syncing') || 'Syncing Blueprints catalog from GitHub...', 'info');
        const { syncBlueprintsCatalog } = await import('../../../api');
        const res = await syncBlueprintsCatalog();
        if (res.updated) {
          showToast(t('settings.blueprints_sync_success') || `Blueprints catalog synced: ${res.totalItems.toLocaleString()} assets!`, 'success');
        } else {
          showToast(t('settings.blueprints_sync_uptodate') || 'Blueprints catalog is already up to date.', 'info');
        }
        await refreshBlueprintsStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        syncBlueprintsBtn.disabled = false;
        if (syncBlueprintsIcon) syncBlueprintsIcon.classList.remove('spinning');
      }
    };
  }

  // PalSchema DataTables Sync Button
  const syncDatatablesBtn = settingsDom.elMaybe('btn-sync-datatables');
  const syncDatatablesIcon = settingsDom.elMaybe('btn-sync-datatables-icon');
  if (syncDatatablesBtn) {
    syncDatatablesBtn.onclick = async () => {
      try {
        syncDatatablesBtn.disabled = true;
        if (syncDatatablesIcon) syncDatatablesIcon.classList.add('spinning');
        showToast(t('settings.datatables_syncing') || 'Syncing DataTables catalog from GitHub...', 'info');
        const { syncDatatablesCatalog } = await import('../../../api');
        const res = await syncDatatablesCatalog();
        if (res.updated) {
          showToast(t('settings.datatables_sync_success') || `DataTables catalog synced: ${res.totalItems} tables!`, 'success');
        } else {
          showToast(t('settings.datatables_sync_uptodate') || 'DataTables catalog is already up to date.', 'info');
        }
        await refreshDatatablesStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        syncDatatablesBtn.disabled = false;
        if (syncDatatablesIcon) syncDatatablesIcon.classList.remove('spinning');
      }
    };
  }

  // PalSchema Schemas Sync Button
  const syncSchemasBtn = settingsDom.elMaybe('btn-sync-schemas');
  const syncSchemasIcon = settingsDom.elMaybe('btn-sync-schemas-icon');
  if (syncSchemasBtn) {
    syncSchemasBtn.onclick = async () => {
      try {
        syncSchemasBtn.disabled = true;
        if (syncSchemasIcon) syncSchemasIcon.classList.add('spinning');
        showToast(t('settings.schemas_syncing') || 'Syncing PalSchema schemas specification from GitHub...', 'info');
        const { syncPalSchemaSchemas } = await import('../../../api');
        const res = await syncPalSchemaSchemas();
        if (res.updated) {
          showToast(t('settings.schemas_sync_success') || `PalSchema schemas synced: ${res.totalItems} specifications!`, 'success');
        } else {
          showToast(t('settings.schemas_sync_uptodate') || 'PalSchema schemas are already up to date.', 'info');
        }
        await refreshPalSchemaSchemasStatus();
        // Refresh Monaco in-memory schemas as well
        const { refreshMonacoPalSchemas } = await import('../../editor/monaco/schemas');
        await refreshMonacoPalSchemas();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        syncSchemasBtn.disabled = false;
        if (syncSchemasIcon) syncSchemasIcon.classList.remove('spinning');
      }
    };
  }

  // Purge Image Cache Buttons (in Network and Safety panes)
  const purgeCacheHandler = async (btn: HTMLButtonElement) => {
    try {
      btn.disabled = true;
      const { purgeImageCache } = await import('../../../api');
      await purgeImageCache();
      showToast(t('toasts.image_cache_cleared'), 'success');
      await refreshImageCacheStatus();
    } catch (err: any) {
      showToast(String(err), 'error');
    } finally {
      btn.disabled = false;
    }
  };

  const purgeCacheBtn = settingsDom.elMaybe('btn-purge-image-cache');
  if (purgeCacheBtn) {
    purgeCacheBtn.onclick = () => purgeCacheHandler(purgeCacheBtn);
  }

  const storagePurgeImagesBtn = settingsDom.elMaybe('storage-clear-images-btn');
  if (storagePurgeImagesBtn) {
    storagePurgeImagesBtn.onclick = () => purgeCacheHandler(storagePurgeImagesBtn);
  }

  // External repository link buttons
  document.querySelectorAll<HTMLButtonElement>('.open-external-link-btn').forEach(btn => {
    btn.onclick = async (e) => {
      e.preventDefault();
      e.stopPropagation();
      const url = btn.dataset.url;
      if (url) {
        try {
          const { openUrl } = await import('../../../api');
          await openUrl(url);
        } catch (err) {
          console.error('Failed to open external link:', err);
        }
      }
    };
  });

  const clearTempBtn = settingsDom.elMaybe('storage-clear-temp-btn');
  if (clearTempBtn) {
    clearTempBtn.onclick = async () => {
      try {
        clearTempBtn.disabled = true;
        const { clearTempDownloads } = await import('../../../api');
        const freed = await clearTempDownloads();
        showToast(t('toasts.temp_downloads_cleared', { size: formatBytes(freed) }), 'success');
        await refreshStorageUsageStatus();
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        clearTempBtn.disabled = false;
      }
    };
  }

  const openTempBtn = settingsDom.elMaybe('storage-open-temp-btn');
  if (openTempBtn) {
    openTempBtn.onclick = async () => {
      try {
        const { openTempFolder } = await import('../../../api');
        await openTempFolder();
      } catch (err: any) {
        showToast(String(err), 'error');
      }
    };
  }

  const openLibBtn = settingsDom.elMaybe('storage-open-library-btn');
  if (openLibBtn) {
    openLibBtn.onclick = async () => {
      try {
        const { openLibraryFolder } = await import('../../../api');
        await openLibraryFolder();
      } catch (err: any) {
        showToast(String(err), 'error');
      }
    };
  }

  // Setup Sidebar Tab navigation
  setupSettingsTabs();

  // Initialize and render Nexus account section
  import('../../../features/nexus_auth').then(({ renderNexusAccountUI, processOAuthCallback }) => {
    renderNexusAccountUI();

    const manualCallbackBtn = settingsDom.elMaybe('btn-nexus-manual-callback');
    const manualCallbackInput = settingsDom.elMaybe('nexus-manual-callback-input');
    if (manualCallbackBtn && manualCallbackInput) {
      manualCallbackBtn.onclick = async () => {
        const val = manualCallbackInput.value.trim();
        if (val) {
          await processOAuthCallback(val);
          manualCallbackInput.value = '';
        }
      };
    }
  }).catch(() => {});

  const langSelect = settingsDom.elMaybe('settings-language-select');
  if (langSelect) {
    import('../../../utils/i18n').then(({ getLocale, setLocale }) => {
      langSelect.value = getLocale();
      langSelect.onchange = () => {
        setLocale(langSelect.value as any);
        import('../../modsView').then(m => m.renderModsView()).catch(() => {});
      };
    });
  }

  const folderExpandSelect = settingsDom.elMaybe('settings-folder-expand-mode-select');
  if (folderExpandSelect) {
    folderExpandSelect.value = state.currentSettings?.folderExpandMode || 'always_expanded';
  }

  // Reset active tab to default (Game & Storage)
  const tabButtons = modal.querySelectorAll<HTMLButtonElement>('.settings-tab-button');
  const panes = modal.querySelectorAll<HTMLElement>('.settings-tab-pane');
  tabButtons.forEach(b => {
    if (b.dataset.settingsTab === 'game') {
      b.classList.add('active');
    } else {
      b.classList.remove('active');
    }
  });
  panes.forEach(p => {
    if (p.id === 'settings-pane-game') {
      p.classList.add('active');
      p.scrollTop = 0;
    } else {
      p.classList.remove('active');
    }
  });

  modal.classList.add('visible');

  requestAnimationFrame(() => {
    const activePane = modal.querySelector('.settings-tab-pane.active');
    if (activePane) {
      activePane.scrollTop = 0;
    }
  });
}

export function setupSettingsTabs(): void {
  const tabButtons = document.querySelectorAll<HTMLButtonElement>('.settings-tab-button');
  const panes = document.querySelectorAll<HTMLElement>('.settings-tab-pane');

  tabButtons.forEach(btn => {
    btn.onclick = () => {
      const targetTab = btn.dataset.settingsTab;
      if (!targetTab) return;

      tabButtons.forEach(b => b.classList.remove('active'));
      panes.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const activePane = document.getElementById(`settings-pane-${targetTab}`);
      if (activePane) {
        activePane.classList.add('active');
        activePane.scrollTop = 0;
      }
      if (targetTab === 'safety') {
        refreshSafetyBackupStatus();
        refreshStorageUsageStatus();
        refreshImageCacheStatus();
      } else if (targetTab === 'network') {
        refreshImageCacheStatus();
      }
    };
  });
}

export function closeSettingsModal(): void {
  const modal = settingsDom.elMaybe('settings-modal');
  if (modal) {
    modal.classList.remove('visible');
    const tabButtons = modal.querySelectorAll<HTMLButtonElement>('.settings-tab-button');
    const panes = modal.querySelectorAll<HTMLElement>('.settings-tab-pane');
    tabButtons.forEach(b => {
      if (b.dataset.settingsTab === 'game') {
        b.classList.add('active');
      } else {
        b.classList.remove('active');
      }
    });
    panes.forEach(p => {
      if (p.id === 'settings-pane-game') {
        p.classList.add('active');
        p.scrollTop = 0;
      } else {
        p.classList.remove('active');
      }
    });
  }
  const savedScale = getState().currentSettings?.toolbarScale || 1.0;
  document.documentElement.style.setProperty('--toolbar-scale', savedScale.toString());
}

export function openSettingsToTab(tabName: string): void {
  openSettingsModal();
  const tabButtons = document.querySelectorAll<HTMLButtonElement>('.settings-tab-button');
  const panes = document.querySelectorAll<HTMLElement>('.settings-tab-pane');

  tabButtons.forEach(b => {
    if (b.dataset.settingsTab === tabName) {
      b.classList.add('active');
    } else {
      b.classList.remove('active');
    }
  });

  panes.forEach(p => {
    if (p.id === `settings-pane-${tabName}`) {
      p.classList.add('active');
      p.scrollTop = 0;
    } else {
      p.classList.remove('active');
    }
  });
}
