import { getState, updateState } from '../../../state';
import { removeFromLibrary } from '../../../api';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import { _libraryOnlineUpdatesMap } from './state';

export async function triggerInstallFromLibrary(id: string, zipName?: string): Promise<void> {
  try {
    const { getLibraryZipPath, analyzeZip, checkModExistsCommand } = await import('../../../api');
    const { renderInstallPreview, showInstallModal } = await import('../../modal');

    const zipPath = await getLibraryZipPath(id, zipName);
    const analysis = await analyzeZip(zipPath);
    const check = await checkModExistsCommand(zipPath);

    const state = getState();
    const libEntry = state.libraryEntries?.find(e => e.modId === id && (!zipName || e.zipName === zipName)) || state.libraryEntries?.find(e => e.modId === id);
    const modInfo = check.modInfo || state.allMods.find(m => m.id === id || m.name === id);

    if (libEntry || modInfo) {
      if (!(analysis as any).nexusModId) {
        (analysis as any).nexusModId = libEntry?.nexusModId || modInfo?.nexusModId || undefined;
      }
      if (!analysis.detectedVersion || analysis.detectedVersion === '1.0.0' || analysis.detectedVersion === 'unknown') {
        if (libEntry?.version && libEntry.version !== 'unknown' && libEntry.version !== '1.0.0') {
          analysis.detectedVersion = libEntry.version;
        } else if (libEntry?.nexusVersion) {
          analysis.detectedVersion = libEntry.nexusVersion;
        }
      }
      if (!(analysis as any).nexusInfo) {
        const pic = libEntry?.nexusPictureUrl || modInfo?.nexusPictureUrl;
        const name = (libEntry as any)?.nexusName || (modInfo as any)?.nexusName || libEntry?.modId || modInfo?.name;
        const author = libEntry?.nexusAuthor || modInfo?.nexusAuthor || libEntry?.author;
        const summary = libEntry?.nexusSummary || modInfo?.nexusSummary || libEntry?.description;
        if (pic || name || author) {
          (analysis as any).nexusInfo = {
            modId: (analysis as any).nexusModId || 0,
            name: name || '',
            author: author || '',
            summary: summary || '',
            pictureUrl: pic || '',
            version: libEntry?.version || libEntry?.nexusVersion || modInfo?.version || '',
            downloads: 0,
            endorsements: 0,
          };
        }
      }
    }

    const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name, version: check.modInfo.version } : (modInfo ? { id: modInfo.id, name: modInfo.name, version: modInfo.version } : null);

    renderInstallPreview(analysis, existingMod);
    showInstallModal();
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export async function handleLibraryBulkInstall(): Promise<void> {
  const state = getState();
  const selected = Array.from(state.selectedLibraryIds);
  if (selected.length === 0) return;

  try {
    const { getLibraryZipPath } = await import('../../../api');
    const { renderBatchInstallPreview, showInstallModal } = await import('../../modal');

    const zipPaths: string[] = [];
    for (const id of selected) {
      try {
        const path = await getLibraryZipPath(id);
        zipPaths.push(path);
      } catch { }
    }

    if (zipPaths.length === 1) {
      const { analyzeZip, checkModExistsCommand } = await import('../../../api');
      const { renderInstallPreview } = await import('../../modal');
      const analysis = await analyzeZip(zipPaths[0]);
      const check = await checkModExistsCommand(zipPaths[0]);
      const existingMod = check.exists && check.modInfo ? { id: check.modInfo.id, name: check.modInfo.name, version: check.modInfo.version } : null;
      renderInstallPreview(analysis, existingMod);
      showInstallModal();
    } else if (zipPaths.length > 1) {
      await renderBatchInstallPreview(zipPaths);
    }
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export async function handleLibraryDelete(id: string, zip: string, isInstalled?: boolean): Promise<void> {
  const state = getState();
  const matchedMod = state.allMods.find(m => m.id === id || m.name === id || (m.nexusModId && state.libraryEntries.find(e => e.modId === id)?.nexusModId === m.nexusModId));
  const actuallyInstalled = isInstalled || !!matchedMod;

  if (actuallyInstalled && matchedMod) {
    const { showChoiceDialog } = await import('../../confirm');
    const choice = await showChoiceDialog(
      t('library.dialog_delete_installed_title'),
      t('library.dialog_delete_installed_desc', { name: matchedMod.name }),
      [
        { id: 'uninstall', label: t('library.btn_uninstall_and_remove'), variant: 'danger' },
        { id: 'library_only', label: t('library.btn_remove_library_only'), variant: 'primary' },
        { id: 'cancel', label: t('common.cancel'), variant: 'muted' },
      ]
    );

    if (!choice || choice === 'cancel') return;

    if (choice === 'uninstall') {
      try {
        const { removeMod } = await import('../../../api');
        await removeMod(matchedMod.id);
        const { loadMods } = await import('../loader');
        const { loadProfiles } = await import('../profiles');
        const { loadDependencies } = await import('../dependencies');
        await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
      } catch (err) {
        console.error('Failed to uninstall mod from game:', err);
      }
    }

    try {
      await removeFromLibrary(id, zip);
      showToast(choice === 'uninstall' ? t('library.toast_uninstalled_and_removed') : t('toasts.library_mod_version_removed'), 'success');
      const { loadLibrary } = await import('./listeners');
      await loadLibrary();
    } catch (err) {
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  } else {
    const { showConfirm } = await import('../../confirm');
    const confirmed = await showConfirm(t('library.confirm_remove_version', { zip }));
    if (confirmed) {
      try {
        await removeFromLibrary(id, zip);
        showToast(t('toasts.library_mod_version_removed'), 'success');
        const { loadLibrary } = await import('./listeners');
        await loadLibrary();
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    }
  }
}

export async function handleLibraryBulkRemove(): Promise<void> {
  const state = getState();
  const selected = Array.from(state.selectedLibraryIds);
  if (selected.length === 0) return;

  const { showConfirm } = await import('../../confirm');
  const confirmed = await showConfirm(t('library.confirm_remove_bulk', { count: selected.length }));
  if (confirmed) {
    let deleted = 0;
    for (const id of selected) {
      try {
        await removeFromLibrary(id);
        deleted++;
      } catch { }
    }
    showToast(t('toasts.library_mods_deleted', { count: deleted }), 'success');
    updateState({ selectedLibraryIds: new Set() });
    updateLibraryBulkBar();
    const { loadLibrary } = await import('./listeners');
    await loadLibrary();
  }
}

import { libraryDom } from '../../../framework';

export function updateLibraryBulkBar(): void {
  const state = getState();
  const bar = libraryDom.elMaybe('library-bulk-actions-bar');
  const countEl = libraryDom.elMaybe('library-bulk-selected-count');
  if (!bar || !countEl) return;

  const selectedCount = state.selectedLibraryIds.size;

  if (selectedCount > 0) {
    bar.style.display = 'flex';
    countEl.textContent = selectedCount.toString();
  } else {
    bar.style.display = 'none';
  }
}

export async function handleCheckLocalLibraryOnlineUpdates(): Promise<void> {
  showToast(t('library.checking_library_nexus_updates'), 'info');

  try {
    const { checkLibraryUpdates } = await import('../../../api');
    const { renderLibraryView } = await import('./render');
    const updates = await checkLibraryUpdates();

    _libraryOnlineUpdatesMap.clear();
    for (const u of updates) {
      _libraryOnlineUpdatesMap.set(u.modId, u.latestVersion);
    }

    if (updates.length === 0) {
      showToast(t('library.toast_local_library_up_to_date'), 'success');
    } else {
      showToast(t('toasts.found_updates_count', { count: updates.length }), 'success');
      for (const u of updates) {
        showToast(`${u.name}: ${u.currentVersion} → ${u.latestVersion}`, 'info');
      }
    }

    await renderLibraryView();
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}
