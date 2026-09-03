import { getImageCacheSize, getStorageUsage, getSafetyBackupInfo } from '../../../api';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { formatBytes } from './helpers';

export async function refreshImageCacheStatus(): Promise<void> {
  const badgeNet = document.getElementById('settings-image-cache-badge');
  const badgeStorage = document.getElementById('storage-images-badge');
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
  const tempBadge = document.getElementById('storage-temp-badge');
  const tempDetails = document.getElementById('storage-temp-details');
  const libBadge = document.getElementById('storage-library-badge');
  const libDetails = document.getElementById('storage-library-details');

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
  const statusElem = document.getElementById('safety-backup-status-text');
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
  const badge = document.getElementById('settings-usmap-badge');
  const gameVer = document.getElementById('settings-usmap-game-ver');
  const activeFile = document.getElementById('settings-usmap-active-file');
  const hashElem = document.getElementById('settings-usmap-hash');

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
  const badge = document.getElementById('settings-sdk-badge');
  const sourceElem = document.getElementById('settings-sdk-source');
  const classesElem = document.getElementById('settings-sdk-classes-count');
  const funcsElem = document.getElementById('settings-sdk-funcs-count');
  const pathElem = document.getElementById('settings-sdk-path');

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
  const badge = document.getElementById('settings-blueprints-badge');
  const gameVer = document.getElementById('settings-blueprints-game-ver');
  const countElem = document.getElementById('settings-blueprints-count');
  const activeFile = document.getElementById('settings-blueprints-active-file');
  const hashElem = document.getElementById('settings-blueprints-hash');

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
  const badge = document.getElementById('settings-datatables-badge');
  const countElem = document.getElementById('settings-datatables-count');
  const rowsElem = document.getElementById('settings-datatables-rows-count');
  const activeFile = document.getElementById('settings-datatables-active-file');

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
  const badge = document.getElementById('settings-schemas-badge');
  const versionElem = document.getElementById('settings-schemas-version');
  const rawCountElem = document.getElementById('settings-schemas-raw-count');
  const domainCountElem = document.getElementById('settings-schemas-domain-count');
  const enumsElem = document.getElementById('settings-schemas-enums');
  const sourceElem = document.getElementById('settings-schemas-source');

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




