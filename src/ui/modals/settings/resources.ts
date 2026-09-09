import { settingsDom } from '../../../framework';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import { formatBytes } from './helpers';
import {
  refreshUsmapStatus,
  refreshSdkStatus,
  refreshBlueprintsStatus,
  refreshDatatablesStatus,
  refreshPalSchemaSchemasStatus,
} from './status';

/**
 * Updates all live badges and metadata for development resources and master manifest
 */
export async function refreshDevResourcesStatus(): Promise<void> {
  const steamBuildElem = settingsDom.elMaybe('settings-resources-steam-build');
  const gameVerElem = settingsDom.elMaybe('settings-resources-game-ver');
  const ue4ssCommitElem = settingsDom.elMaybe('settings-resources-ue4ss-commit');

  try {
    const { getMasterResourcesStatus } = await import('../../../api');
    const master = await getMasterResourcesStatus();

    if (steamBuildElem) {
      const buildText = master.detectedSteamBuildId || master.latestSteamBuildId;
      steamBuildElem.textContent = buildText;
      steamBuildElem.style.color = master.detectedSteamBuildId ? 'var(--text-primary)' : 'var(--text-muted)';
    }

    if (gameVerElem) {
      gameVerElem.textContent = master.detectedGameVersion || master.latestGameVersion;
    }

    if (ue4ssCommitElem) {
      const commit = master.resources.find(r => r.ue4ssCommit)?.ue4ssCommit || '2281fa31';
      ue4ssCommitElem.textContent = commit;
    }

    // Update the 4 newly exposed targets: jmap, lua_types, uht, bp_sdk
    for (const res of master.resources) {
      if (res.id === 'jmap') {
        updateCardUi('jmap', res.isAvailable, res.filename, res.fileSizeBytes, res.sha256, null, res.hasLocalGameDump);
      } else if (res.id === 'lua_types') {
        updateCardUi('luatypes', res.isAvailable, res.filename, res.fileSizeBytes, res.sha256, res.totalItems, res.hasLocalGameDump);
      } else if (res.id === 'uht') {
        updateCardUi('uht', res.isAvailable, res.filename, res.fileSizeBytes, res.sha256, res.totalItems, res.hasLocalGameDump);
      } else if (res.id === 'bp_sdk') {
        updateCardUi('bpsdk', res.isAvailable, res.filename, res.fileSizeBytes, res.sha256, res.totalItems, res.hasLocalGameDump);
      }
    }
  } catch (err) {
    console.error('Failed to query master resources status:', err);
  }

  // Refresh legacy cards status
  await Promise.allSettled([
    refreshUsmapStatus(),
    refreshSdkStatus(),
    refreshBlueprintsStatus(),
    refreshDatatablesStatus(),
    refreshPalSchemaSchemasStatus(),
  ]);
}

function updateCardUi(
  prefix: 'jmap' | 'luatypes' | 'uht' | 'bpsdk',
  isAvailable: boolean,
  filename: string,
  fileSize: number,
  sha256: string | null,
  totalItems: number | null,
  hasLocalDump: boolean
): void {
  const badge = settingsDom.elMaybe(`settings-${prefix}-badge` as any);
  const fileElem = settingsDom.elMaybe(`settings-${prefix}-file` as any);
  const hashElem = settingsDom.elMaybe(`settings-${prefix}-hash` as any);
  const countElem = settingsDom.elMaybe(`settings-${prefix}-count` as any);
  const localStatusElem = settingsDom.elMaybe(`settings-${prefix}-local-status` as any);

  if (badge) {
    if (isAvailable && fileSize > 0) {
      badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
      badge.style.background = 'rgba(34,197,94,0.15)';
      badge.style.color = '#22c55e';
      badge.style.borderColor = 'rgba(34,197,94,0.3)';
    } else if (hasLocalDump) {
      badge.textContent = `📂 ${t('settings.sdk_status_local_found') || 'Local Dump Found'}`;
      badge.style.background = 'rgba(0,188,255,0.15)';
      badge.style.color = '#00bcff';
      badge.style.borderColor = 'rgba(0,188,255,0.3)';
    } else {
      badge.textContent = `⚠️ ${t('settings.status_missing') || 'Missing / Not Synced'}`;
      badge.style.background = 'rgba(239,68,68,0.15)';
      badge.style.color = '#ef4444';
      badge.style.borderColor = 'rgba(239,68,68,0.3)';
    }
  }

  if (fileElem) {
    if (isAvailable && fileSize > 0) {
      fileElem.textContent = `${filename} (${formatBytes(fileSize)})`;
    } else {
      fileElem.textContent = t('settings.resource_not_installed') || 'Package not downloaded';
    }
  }

  if (countElem && totalItems) {
    countElem.textContent = `${totalItems.toLocaleString()} files`;
  }

  if (hashElem) {
    if (sha256) {
      hashElem.textContent = sha256.substring(0, 16) + '...' + sha256.substring(sha256.length - 8);
      hashElem.title = sha256;
    } else {
      hashElem.textContent = '---';
    }
  }

  if (localStatusElem) {
    if (hasLocalDump) {
      localStatusElem.textContent = t('settings.local_game_dump_detected') || 'Detected in Palworld ue4ss directory';
      localStatusElem.style.color = '#00bcff';
    } else {
      localStatusElem.textContent = t('settings.local_game_dump_none') || 'Not found in game folder';
      localStatusElem.style.color = 'var(--text-muted)';
    }
  }
}

/**
 * Universal sync action runner with visual spinners and toasts
 */
async function runSyncAction(
  target: string,
  btn: HTMLButtonElement | null,
  icon: HTMLElement | null,
  successMsg: string
): Promise<void> {
  if (!btn) return;
  try {
    btn.disabled = true;
    if (icon) icon.classList.add('spinning');
    showToast(t('settings.resource_syncing', { target }) || `Syncing ${target} from repository...`, 'info');
    const { syncDevelopmentResource } = await import('../../../api');
    const res = await syncDevelopmentResource(target);
    if (res.success) {
      showToast(successMsg || res.message, 'success');
    } else {
      showToast(res.message, 'error');
    }
    await refreshDevResourcesStatus();
  } catch (err: any) {
    showToast(String(err), 'error');
  } finally {
    btn.disabled = false;
    if (icon) icon.classList.remove('spinning');
  }
}

/**
 * Universal export action extracting package contents into user chosen directory
 */
async function runExportAction(
  target: string,
  btn: HTMLButtonElement | null,
  dialogTitle: string
): Promise<void> {
  if (!btn) return;
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: dialogTitle || `Select Destination Folder to Export ${target}`,
    });

    if (selected && typeof selected === 'string') {
      btn.disabled = true;
      showToast(t('settings.resource_exporting', { target }) || `Extracting ${target} files into destination...`, 'info');
      const { exportDevelopmentResource } = await import('../../../api');
      const res = await exportDevelopmentResource(target, selected);
      if (res.success) {
        showToast(
          t('settings.resource_export_success', { target, count: res.totalFilesExtracted || 0 }) ||
            `Successfully exported ${res.totalFilesExtracted || 0} files from ${target}!`,
          'success'
        );
      } else {
        showToast(res.message, 'error');
      }
    }
  } catch (err: any) {
    showToast(String(err), 'error');
  } finally {
    btn.disabled = false;
  }
}

/**
 * Universal purge action runner with confirmation dialog
 */
async function runPurgeAction(
  target: string,
  btn: HTMLButtonElement | null,
  confirmTitle: string,
  confirmMsg: string
): Promise<void> {
  if (!btn) return;
  const confirmed = await showConfirm(
    confirmTitle || `Clear ${target}`,
    confirmMsg || `Are you sure you want to clear local cache files for ${target}?`
  );
  if (!confirmed) return;

  try {
    btn.disabled = true;
    const { purgeDevelopmentResource } = await import('../../../api');
    await purgeDevelopmentResource(target);
    showToast(t('settings.resource_purge_success', { target }) || `${target} cache cleared.`, 'success');
    await refreshDevResourcesStatus();
  } catch (err: any) {
    showToast(String(err), 'error');
  } finally {
    btn.disabled = false;
  }
}

/**
 * Initializes all event handlers for the Development Resources tab
 */
export function initDevResources(): void {
  // Global Verify All Button
  const verifyAllBtn = settingsDom.elMaybe('btn-verify-all-resources');
  const verifyAllIcon = settingsDom.elMaybe('btn-verify-all-resources-icon');
  if (verifyAllBtn) {
    verifyAllBtn.onclick = async () => {
      try {
        verifyAllBtn.disabled = true;
        if (verifyAllIcon) verifyAllIcon.classList.add('spinning');
        await refreshDevResourcesStatus();
        showToast(t('settings.resources_verified_toast') || 'All development resources verified against manifest.', 'success');
      } catch (err: any) {
        showToast(String(err), 'error');
      } finally {
        verifyAllBtn.disabled = false;
        if (verifyAllIcon) verifyAllIcon.classList.remove('spinning');
      }
    };
  }

  // 1. Unreal Mappings (.usmap)
  const syncUsmapBtn = settingsDom.elMaybe('btn-sync-usmap');
  const syncUsmapIcon = settingsDom.elMaybe('btn-sync-usmap-icon');
  if (syncUsmapBtn) {
    syncUsmapBtn.onclick = async () => {
      try {
        syncUsmapBtn.disabled = true;
        if (syncUsmapIcon) syncUsmapIcon.classList.add('spinning');
        showToast(t('settings.usmap_syncing') || 'Syncing Unreal Schema Mappings from GitHub...', 'info');
        const { syncMappingsNow } = await import('../../../api');
        const res = await syncMappingsNow();
        if (res.isSynced) {
          showToast(t('settings.usmap_sync_success') || 'USMAP mappings updated and verified successfully!', 'success');
        } else {
          showToast(t('settings.usmap_sync_complete') || 'USMAP sync finished.', 'info');
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

  // 2. C++ SDK Headers
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

  // 3. Blueprints Catalog
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

  // 4. DataTables Catalog
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

  // 5. PalSchema JSON Schemas
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

  // 6. JSON Property Mappings (.jmap)
  const syncJmapBtn = settingsDom.elMaybe('btn-sync-jmap');
  const syncJmapIcon = settingsDom.elMaybe('btn-sync-jmap-icon');
  if (syncJmapBtn) {
    syncJmapBtn.onclick = () =>
      runSyncAction('jmap', syncJmapBtn, syncJmapIcon, t('settings.jmap_sync_success') || 'JSON Property Mappings (.jmap) synced and verified!');
  }
  const exportJmapBtn = settingsDom.elMaybe('btn-export-jmap');
  if (exportJmapBtn) {
    exportJmapBtn.onclick = () => runExportAction('jmap', exportJmapBtn, t('settings.jmap_export_dialog') || 'Select Folder to Export .jmap AST');
  }
  const purgeJmapBtn = settingsDom.elMaybe('btn-purge-jmap');
  if (purgeJmapBtn) {
    purgeJmapBtn.onclick = () =>
      runPurgeAction(
        'jmap',
        purgeJmapBtn,
        t('settings.jmap_purge_title') || 'Clear .jmap Cache',
        t('settings.jmap_purge_confirm') || 'Are you sure you want to remove the local .jmap archive?'
      );
  }

  // 7. Lua EmmyLua Types (shared/types)
  const syncLuaTypesBtn = settingsDom.elMaybe('btn-sync-luatypes');
  const syncLuaTypesIcon = settingsDom.elMaybe('btn-sync-luatypes-icon');
  if (syncLuaTypesBtn) {
    syncLuaTypesBtn.onclick = () =>
      runSyncAction(
        'lua_types',
        syncLuaTypesBtn,
        syncLuaTypesIcon,
        t('settings.luatypes_sync_success') || 'EmmyLua type definitions synced & unpacked for Monaco!'
      );
  }
  const exportLuaTypesBtn = settingsDom.elMaybe('btn-export-luatypes');
  if (exportLuaTypesBtn) {
    exportLuaTypesBtn.onclick = () =>
      runExportAction('lua_types', exportLuaTypesBtn, t('settings.luatypes_export_dialog') || 'Select Folder to Export EmmyLua Definitions');
  }
  const purgeLuaTypesBtn = settingsDom.elMaybe('btn-purge-luatypes');
  if (purgeLuaTypesBtn) {
    purgeLuaTypesBtn.onclick = () =>
      runPurgeAction(
        'lua_types',
        purgeLuaTypesBtn,
        t('settings.luatypes_purge_title') || 'Clear EmmyLua Types Cache',
        t('settings.luatypes_purge_confirm') || 'Are you sure you want to remove local Lua types definitions?'
      );
  }

  // 8. Unreal Header Tool (UHT) C++ Headers
  const syncUhtBtn = settingsDom.elMaybe('btn-sync-uht');
  const syncUhtIcon = settingsDom.elMaybe('btn-sync-uht-icon');
  if (syncUhtBtn) {
    syncUhtBtn.onclick = () =>
      runSyncAction('uht', syncUhtBtn, syncUhtIcon, t('settings.uht_sync_success') || 'UHT C++ Headers synced and verified!');
  }
  const exportUhtBtn = settingsDom.elMaybe('btn-export-uht');
  if (exportUhtBtn) {
    exportUhtBtn.onclick = () =>
      runExportAction('uht', exportUhtBtn, t('settings.uht_export_dialog') || 'Select Folder to Export UHT C++ Headers (24k files)');
  }
  const purgeUhtBtn = settingsDom.elMaybe('btn-purge-uht');
  if (purgeUhtBtn) {
    purgeUhtBtn.onclick = () =>
      runPurgeAction(
        'uht',
        purgeUhtBtn,
        t('settings.uht_purge_title') || 'Clear UHT Headers Cache',
        t('settings.uht_purge_confirm') || 'Are you sure you want to remove local UHT SDK headers archive?'
      );
  }

  // 9. Blueprint SDK Dummy Assets (UE4SS_SDK)
  const syncBpSdkBtn = settingsDom.elMaybe('btn-sync-bpsdk');
  const syncBpSdkIcon = settingsDom.elMaybe('btn-sync-bpsdk-icon');
  if (syncBpSdkBtn) {
    syncBpSdkBtn.onclick = () =>
      runSyncAction('bp_sdk', syncBpSdkBtn, syncBpSdkIcon, t('settings.bpsdk_sync_success') || 'Blueprint SDK Dummy Assets synced and verified!');
  }
  const exportBpSdkBtn = settingsDom.elMaybe('btn-export-bpsdk');
  if (exportBpSdkBtn) {
    exportBpSdkBtn.onclick = () =>
      runExportAction('bp_sdk', exportBpSdkBtn, t('settings.bpsdk_export_dialog') || 'Select Project Content Folder to Export Blueprint SDK (14k assets)');
  }
  const purgeBpSdkBtn = settingsDom.elMaybe('btn-purge-bpsdk');
  if (purgeBpSdkBtn) {
    purgeBpSdkBtn.onclick = () =>
      runPurgeAction(
        'bp_sdk',
        purgeBpSdkBtn,
        t('settings.bpsdk_purge_title') || 'Clear Blueprint SDK Cache',
        t('settings.bpsdk_purge_confirm') || 'Are you sure you want to remove local Blueprint SDK dummy assets archive?'
      );
  }
}
