import { getImageCacheSize, getStorageUsage, getSafetyBackupInfo } from '../../../api';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { settingsDom } from '../../../framework';
import { formatBytes } from './helpers';

export async function refreshImageCacheStatus(): Promise<void> {
  const badgeNet = settingsDom.elMaybe('settings-image-cache-badge');
  const badgeStorage = settingsDom.elMaybe('storage-images-badge');
  if (!badgeNet && !badgeStorage) return;

  try {
    const bytes = await getImageCacheSize();
    const formatted = formatBytes(bytes);
    if (badgeNet) badgeNet.textContent = formatted;
    if (badgeStorage) badgeStorage.textContent = formatted;
  } catch (e) {
    console.error('Failed to get image cache size:', e);
  }
}

export async function refreshStorageUsageStatus(): Promise<void> {
  const tempBadge = settingsDom.elMaybe('storage-temp-badge');
  const tempDetails = settingsDom.elMaybe('storage-temp-details');
  const libBadge = settingsDom.elMaybe('storage-library-badge');
  const libDetails = settingsDom.elMaybe('storage-library-details');

  if (!tempBadge && !libBadge) return;

  try {
    const storage = await getStorageUsage();

    if (tempBadge) {
      tempBadge.textContent = formatBytes(storage.tempDownloadsSize);
    }
    if (tempDetails) {
      tempDetails.textContent = t('settings.storage_temp_files_count', {
        count: storage.tempDownloadsCount,
        path: storage.tempDownloadsPath,
      });
      tempDetails.title = storage.tempDownloadsPath;
    }

    if (libBadge) {
      libBadge.textContent = formatBytes(storage.librarySize);
    }
    if (libDetails) {
      libDetails.textContent = t('settings.storage_library_files_count', {
        mods: storage.libraryModsCount,
        zips: storage.libraryZipsCount,
        path: storage.libraryPath,
      });
      libDetails.title = storage.libraryPath;
    }
  } catch (e) {
    console.error('Failed to load storage usage:', e);
  }
}

export async function refreshSafetyBackupStatus(): Promise<void> {
  const statusElem = settingsDom.elMaybe('safety-backup-status-text');
  if (!statusElem) return;

  try {
    const info = await getSafetyBackupInfo();
    if (info.exists && info.timestamp) {
      const dateStr = new Date(info.timestamp).toLocaleString();
      const sizeKb = info.zipSizeBytes ? Math.round(info.zipSizeBytes / 1024) : 0;
      statusElem.innerHTML = `✅ <strong>${escapeHtml(t('settings.safety_initial_snapshot'))}:</strong> ${dateStr} (${sizeKb} KB)<br>• ${escapeHtml(t('settings.safety_tracked_summary', { ue4ss: info.ue4ssModsCount, palschema: info.palschemaModsCount }))}`;
      statusElem.style.color = 'var(--text-secondary)';
    } else {
      statusElem.textContent = t('settings.safety_none');
      statusElem.style.color = 'var(--text-muted)';
    }
  } catch (e) {
    statusElem.textContent = t('settings.safety_ready');
  }
}

export async function refreshUsmapStatus(): Promise<void> {
  const badge = settingsDom.elMaybe('settings-usmap-badge');
  const gameVer = settingsDom.elMaybe('settings-usmap-game-ver');
  const activeFile = settingsDom.elMaybe('settings-usmap-active-file');
  const hashElem = settingsDom.elMaybe('settings-usmap-hash');

  if (!badge && !gameVer) return;

  try {
    const { getMappingsStatus } = await import('../../../api');
    const status = await getMappingsStatus();

    if (badge) {
      if (status.isSynced && status.localUsmapExists) {
        badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
        badge.style.background = 'rgba(34,197,94,0.15)';
        badge.style.color = '#22c55e';
        badge.style.borderColor = 'rgba(34,197,94,0.3)';
      } else if (status.localUsmapExists) {
        badge.textContent = `📦 ${t('settings.usmap_status_available') || 'Available (Bundled)'}`;
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

    if (gameVer) {
      const verText = status.installedBuild.gameVersion || 'Unknown';
      const buildText = status.installedBuild.buildId ? ` (Steam Build: ${status.installedBuild.buildId})` : '';
      gameVer.textContent = `${verText}${buildText}`;
    }

    if (activeFile) {
      if (status.localUsmapExists) {
        const sizeFormatted = formatBytes(status.localFileSize);
        const fileName = status.activeMapping?.usmapFilename || 'Palworld.usmap';
        activeFile.textContent = `${fileName} (${sizeFormatted})`;
      } else {
        activeFile.textContent = t('settings.usmap_not_installed') || 'No mapping file installed';
      }
    }

    if (hashElem) {
      if (status.localSha256) {
        hashElem.textContent = status.localSha256.substring(0, 16) + '...' + status.localSha256.substring(status.localSha256.length - 8);
        hashElem.title = status.localSha256;
      } else {
        hashElem.textContent = '---';
      }
    }
  } catch (e) {
    console.error('Failed to get USMAP mappings status:', e);
    if (badge) {
      badge.textContent = `⚠️ ${t('settings.status_error') || 'Error'}`;
    }
  }
}

export async function refreshSdkStatus(): Promise<void> {
  const badge = settingsDom.elMaybe('settings-sdk-badge');
  const sourceElem = settingsDom.elMaybe('settings-sdk-source');
  const classesElem = settingsDom.elMaybe('settings-sdk-classes-count');
  const funcsElem = settingsDom.elMaybe('settings-sdk-funcs-count');
  const pathElem = settingsDom.elMaybe('settings-sdk-path');

  if (!badge && !sourceElem) return;

  try {
    const { getSdkStatus } = await import('../../../api');
    const status = await getSdkStatus();

    if (badge) {
      if (status.installed && status.totalClasses > 0) {
        badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
        badge.style.background = 'rgba(34,197,94,0.15)';
        badge.style.color = '#22c55e';
        badge.style.borderColor = 'rgba(34,197,94,0.3)';
      } else if (status.localGameCxxFound) {
        badge.textContent = `📂 ${t('settings.sdk_status_local_found') || 'Local Folder Detected'}`;
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

    if (sourceElem) {
      sourceElem.textContent = status.source || 'Not Installed';
    }

    if (classesElem) {
      classesElem.textContent = `${status.totalClasses.toLocaleString()} classes`;
    }

    if (funcsElem) {
      funcsElem.textContent = `${status.totalFunctions.toLocaleString()} functions & delegates`;
    }

    if (pathElem) {
      pathElem.textContent = status.path || '---';
      pathElem.title = status.path;
    }
  } catch (e) {
    console.error('Failed to get SDK status:', e);
    if (badge) {
      badge.textContent = `⚠️ ${t('settings.status_error') || 'Error'}`;
    }
  }
}

export async function refreshBlueprintsStatus(): Promise<void> {
  const badge = settingsDom.elMaybe('settings-blueprints-badge');
  const gameVer = settingsDom.elMaybe('settings-blueprints-game-ver');
  const countElem = settingsDom.elMaybe('settings-blueprints-count');
  const activeFile = settingsDom.elMaybe('settings-blueprints-active-file');
  const hashElem = settingsDom.elMaybe('settings-blueprints-hash');

  if (!badge && !gameVer) return;

  try {
    const { getReflectionCatalogsStatus } = await import('../../../api');
    const status = await getReflectionCatalogsStatus();

    if (badge) {
      if (status.totalBlueprints > 0) {
        badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
        badge.style.background = 'rgba(34,197,94,0.15)';
        badge.style.color = '#22c55e';
        badge.style.borderColor = 'rgba(34,197,94,0.3)';
      } else {
        badge.textContent = `⚠️ ${t('settings.status_missing') || 'Missing / Not Synced'}`;
        badge.style.background = 'rgba(239,68,68,0.15)';
        badge.style.color = '#ef4444';
        badge.style.borderColor = 'rgba(239,68,68,0.3)';
      }
    }

    if (gameVer) {
      const verText = status.blueprintsGameVer || 'Unknown';
      const buildText = status.blueprintsBuildId ? ` (Steam Build: ${status.blueprintsBuildId})` : '';
      gameVer.textContent = `${verText}${buildText}`;
    }

    if (countElem) {
      countElem.textContent = `${status.totalBlueprints.toLocaleString()} classes & cooked assets`;
    }

    if (activeFile) {
      if (status.totalBlueprints > 0) {
        const sizeFormatted = formatBytes(status.blueprintsSize);
        activeFile.textContent = `${status.blueprintsFilename} (${sizeFormatted})`;
      } else {
        activeFile.textContent = t('settings.blueprints_not_installed') || 'Not Installed';
      }
    }

    if (hashElem) {
      if (status.blueprintsSha256) {
        hashElem.textContent = status.blueprintsSha256.substring(0, 16) + '...' + status.blueprintsSha256.substring(status.blueprintsSha256.length - 8);
        hashElem.title = status.blueprintsSha256;
      } else {
        hashElem.textContent = '---';
      }
    }
  } catch (e) {
    console.error('Failed to get Blueprints catalog status:', e);
    if (badge) {
      badge.textContent = `⚠️ ${t('settings.status_error') || 'Error'}`;
    }
  }
}

export async function refreshDatatablesStatus(): Promise<void> {
  const badge = settingsDom.elMaybe('settings-datatables-badge');
  const countElem = settingsDom.elMaybe('settings-datatables-count');
  const rowsElem = settingsDom.elMaybe('settings-datatables-rows-count');
  const activeFile = settingsDom.elMaybe('settings-datatables-active-file');

  if (!badge && !countElem) return;

  try {
    const { getReflectionCatalogsStatus } = await import('../../../api');
    const status = await getReflectionCatalogsStatus();

    if (badge) {
      if (status.totalDatatables > 0) {
        badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
        badge.style.background = 'rgba(34,197,94,0.15)';
        badge.style.color = '#22c55e';
        badge.style.borderColor = 'rgba(34,197,94,0.3)';
      } else {
        badge.textContent = `⚠️ ${t('settings.status_missing') || 'Missing / Not Synced'}`;
        badge.style.background = 'rgba(239,68,68,0.15)';
        badge.style.color = '#ef4444';
        badge.style.borderColor = 'rgba(239,68,68,0.3)';
      }
    }

    if (countElem) {
      countElem.textContent = `${status.totalDatatables.toLocaleString()} DataTables`;
    }

    if (rowsElem) {
      rowsElem.textContent = `${status.totalDatatableRows.toLocaleString()} rows decoded`;
    }

    if (activeFile) {
      activeFile.textContent = status.datatablesActiveFile || 'dt_index.json';
    }
  } catch (e) {
    console.error('Failed to get DataTables catalog status:', e);
    if (badge) {
      badge.textContent = `⚠️ ${t('settings.status_error') || 'Error'}`;
    }
  }
}

export async function refreshPalSchemaSchemasStatus(): Promise<void> {
  const badge = settingsDom.elMaybe('settings-schemas-badge');
  const versionElem = settingsDom.elMaybe('settings-schemas-version');
  const rawCountElem = settingsDom.elMaybe('settings-schemas-raw-count');
  const domainCountElem = settingsDom.elMaybe('settings-schemas-domain-count');
  const enumsElem = settingsDom.elMaybe('settings-schemas-enums');
  const sourceElem = settingsDom.elMaybe('settings-schemas-source');

  if (!badge && !rawCountElem) return;

  try {
    const { getPalSchemaSchemasCatalog } = await import('../../../api');
    const catalog = await getPalSchemaSchemasCatalog();

    if (badge) {
      if (catalog.is_available && catalog.total_raw_schemas > 0) {
        badge.textContent = `✅ ${t('settings.status_synced_ready') || 'Synced & Ready'}`;
        badge.style.background = 'rgba(34,197,94,0.15)';
        badge.style.color = '#22c55e';
        badge.style.borderColor = 'rgba(34,197,94,0.3)';
      } else {
        badge.textContent = `⚠️ ${t('settings.status_missing') || 'Missing / Not Synced'}`;
        badge.style.background = 'rgba(239,68,68,0.15)';
        badge.style.color = '#ef4444';
        badge.style.borderColor = 'rgba(239,68,68,0.3)';
      }
    }

    if (versionElem) {
      const ver = catalog.version ? (catalog.version.startsWith('v') ? catalog.version : `v${catalog.version}`) : 'v0.6.6';
      versionElem.textContent = ver;
    }

    if (rawCountElem) {
      rawCountElem.textContent = `${catalog.total_raw_schemas.toLocaleString()} schemas (raw/DT_*.schema.json)`;
    }

    if (domainCountElem) {
      domainCountElem.textContent = `${catalog.total_domain_schemas} models (Items, Pals, Buildings, Skins, Utility)`;
    }

    if (enumsElem) {
      enumsElem.textContent = catalog.has_enums ? `Included (${t('settings.schemas_enums_included') || 'enums.schema.json'})` : 'Missing';
    }

    if (sourceElem) {
      sourceElem.textContent = catalog.source_location || 'None';
    }
  } catch (e) {
    console.error('Failed to get PalSchema schemas catalog status:', e);
    if (badge) {
      badge.textContent = `⚠️ ${t('settings.status_error') || 'Error'}`;
    }
  }
}




