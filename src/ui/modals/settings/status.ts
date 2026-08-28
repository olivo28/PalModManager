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
