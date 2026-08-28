import {
  deepScanSaveHealth,
  repairSaveHealth,
  restoreSaveBackup,
  createWorldBackup,
  openWorldFolder,
  exportWorldZip,
  pruneWorldBackups,
  saveWorldCustomMeta,
  type WorldCustomMeta,
} from '../../../api';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import {
  doctorState,
  cachedWorlds,
  selectedWorldDir,
  currentHealthReport,
  customSavesPath,
  availableProfiles,
  isCreatingBackup,
  isPruningBackups,
  isSavingMeta,
  isDeepScanning,
  isRepairing,
  isRestoringBackup,
  setCachedWorlds,
  setSelectedWorldDir,
  setCurrentHealthReport,
  setCustomSavesPath,
  setIsCreatingBackup,
  setIsPruningBackups,
  setIsSavingMeta,
  setIsDeepScanning,
  setIsRepairing,
  setIsRestoringBackup,
} from './state';
import { showSnapshotComparisonModal } from './snapshotModal';

export function attachSavesDoctorListeners(
  container: HTMLElement,
  rerenderCallback: (container: HTMLElement) => Promise<void>
): void {
  // World selection
  container.querySelectorAll('.world-list-card').forEach(card => {
    card.addEventListener('click', async () => {
      const worldDir = (card as HTMLElement).dataset.worldDir;
      const curSelected = selectedWorldDir || doctorState.selectedWorldDir;
      if (!worldDir || worldDir === curSelected) return;
      setSelectedWorldDir(worldDir);
      setCurrentHealthReport(null);
      await rerenderCallback(container);
    });
  });

  // Refresh button
  const refreshBtn = container.querySelector('#doctor-refresh-btn');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', async () => {
      setCachedWorlds(null);
      setCurrentHealthReport(null);
      await rerenderCallback(container);
      showToast(t('common.refreshed') || 'Saves list refreshed', 'info');
    });
  }

  // Choose Custom Folder button
  const customFolderBtn = container.querySelector('#doctor-custom-folder-btn');
  if (customFolderBtn) {
    customFolderBtn.addEventListener('click', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          directory: true,
          multiple: false,
          title: t('scanner.dialog_select_saves_folder') || 'Select Palworld SaveGames Folder',
        });
        if (selected && typeof selected === 'string') {
          setCustomSavesPath(selected);
          setCachedWorlds(null);
          setSelectedWorldDir(null);
          setCurrentHealthReport(null);
          await rerenderCallback(container);
          showToast('Loaded custom saves directory', 'success');
        }
      } catch (err) {
        showToast(String(err), 'error');
      }
    });
  }

  // Backup World Now Button
  const backupWorldBtn = container.querySelector('#btn-backup-world');
  if (backupWorldBtn) {
    backupWorldBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curCreating = isCreatingBackup || doctorState.isCreatingBackup;
      if (!curWorldDir || curCreating) return;
      setIsCreatingBackup(true);
      await rerenderCallback(container);

      try {
        const backupZip = await createWorldBackup(curWorldDir);
        const parts = backupZip.split(/[/\\]/);
        const fileName = parts[parts.length - 1] || backupZip;
        showToast((t('scanner.toast_backup_success') || 'Backup created successfully: {name}').replace('{name}', fileName), 'success');
        setCachedWorlds(null);
      } catch (err: any) {
        showToast(`Backup failed: ${String(err)}`, 'error');
      } finally {
        setIsCreatingBackup(false);
        await rerenderCallback(container);
      }
    });
  }

  // Open Save Folder in Explorer Button
  const openFolderBtn = container.querySelector('#btn-open-world-folder');
  if (openFolderBtn) {
    openFolderBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      if (!curWorldDir) return;
      try {
        await openWorldFolder(curWorldDir);
      } catch (err: any) {
        showToast(`Failed to open directory: ${String(err)}`, 'error');
      }
    });
  }

  // Export Save to ZIP Button
  const exportZipBtn = container.querySelector('#btn-export-world-zip');
  if (exportZipBtn) {
    exportZipBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      if (!curWorldDir) return;
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const defaultName = `PalworldSave_${new Date().toISOString().slice(0, 10)}.zip`;
        const targetPath = await save({
          defaultPath: defaultName,
          filters: [{ name: 'Zip Archive', extensions: ['zip'] }],
          title: t('scanner.export_dialog_title') || 'Export World Save (.zip)',
        });

        if (targetPath && typeof targetPath === 'string') {
          await exportWorldZip(curWorldDir, targetPath);
          showToast(`Exported save world to ${targetPath}`, 'success');
        }
      } catch (err: any) {
        showToast(`Export failed: ${String(err)}`, 'error');
      }
    });
  }

  // Prune Backups Button
  const pruneBackupsBtn = container.querySelector('#btn-prune-backups');
  if (pruneBackupsBtn) {
    pruneBackupsBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curPruning = isPruningBackups || doctorState.isPruningBackups;
      if (!curWorldDir || curPruning) return;
      const confirmed = await showConfirm(
        t('scanner.confirm_prune_title') || 'Clean Old Auto-Backups',
        t('scanner.confirm_prune_msg') || 'Palworld stores backup snapshots every 10 minutes without deleting them. This will delete old snapshots, keeping only the 5 most recent. Proceed?'
      );
      if (!confirmed) return;

      setIsPruningBackups(true);
      await rerenderCallback(container);

      try {
        const deleted = await pruneWorldBackups(curWorldDir, 5);
        showToast((t('scanner.toast_prune_success') || 'Cleaned {count} old snapshot backups.').replace('{count}', String(deleted)), 'success');
        setCachedWorlds(null);
        if (currentHealthReport || doctorState.currentHealthReport) {
          const rep = await deepScanSaveHealth(curWorldDir);
          setCurrentHealthReport(rep);
        }
      } catch (err: any) {
        showToast(`Prune failed: ${String(err)}`, 'error');
      } finally {
        setIsPruningBackups(false);
        await rerenderCallback(container);
      }
    });
  }

  // Save Custom World Meta (Nickname, Profile, Notes)
  const saveMetaBtn = container.querySelector('#btn-save-meta');
  if (saveMetaBtn) {
    saveMetaBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curSaving = isSavingMeta || doctorState.isSavingMeta;
      if (!curWorldDir || curSaving) return;
      const nicknameInput = container.querySelector('#meta-nickname-input') as HTMLInputElement;
      const profileSelect = container.querySelector('#meta-profile-select') as HTMLSelectElement;
      const notesInput = container.querySelector('#meta-notes-input') as HTMLTextAreaElement;

      const curProfiles = availableProfiles.length > 0 ? availableProfiles : doctorState.availableProfiles;
      const profileId = profileSelect?.value || null;
      const profileName = profileId ? curProfiles.find(p => p.id === profileId)?.name || null : null;

      const meta: WorldCustomMeta = {
        nickname: nicknameInput?.value?.trim() || null,
        notes: notesInput?.value?.trim() || null,
        boundProfileId: profileId,
        boundProfileName: profileName,
        tags: [],
        preLaunchBackupEnabled: false,
      };

      setIsSavingMeta(true);
      try {
        await saveWorldCustomMeta(curWorldDir, meta);
        showToast('World metadata saved successfully!', 'success');
        setCachedWorlds(null);
        await rerenderCallback(container);
      } catch (err: any) {
        showToast(`Failed to save world metadata: ${String(err)}`, 'error');
      } finally {
        setIsSavingMeta(false);
      }
    });
  }

  // Deep Scan Button
  const deepScanBtn = container.querySelector('#btn-run-deep-scan');
  if (deepScanBtn) {
    deepScanBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curDeep = isDeepScanning || doctorState.isDeepScanning;
      if (!curWorldDir || curDeep) return;
      setIsDeepScanning(true);
      await rerenderCallback(container);

      try {
        const rep = await deepScanSaveHealth(curWorldDir);
        setCurrentHealthReport(rep);
      } catch (err: any) {
        showToast(`Deep scan failed: ${String(err)}`, 'error');
      } finally {
        setIsDeepScanning(false);
        await rerenderCallback(container);
      }
    });
  }

  // Repair Button
  const repairBtn = container.querySelector('#btn-repair-save');
  if (repairBtn) {
    repairBtn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curRepairing = isRepairing || doctorState.isRepairing;
      if (!curWorldDir || curRepairing) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_repair_title') || 'Rescue & Clean Savegame',
        t('scanner.confirm_repair_msg') || 'This will create an automatic ZIP backup of your world and sanitize orphaned mod references. Proceed?'
      );
      if (!confirmed) return;

      setIsRepairing(true);
      await rerenderCallback(container);

      try {
        const result = await repairSaveHealth(curWorldDir);
        showToast(result.message, 'success');
        const rep = await deepScanSaveHealth(curWorldDir);
        setCurrentHealthReport(rep);
        setCachedWorlds(null);
      } catch (err: any) {
        showToast(`Repair failed: ${String(err)}`, 'error');
      } finally {
        setIsRepairing(false);
        await rerenderCallback(container);
      }
    });
  }

  // Compare Snapshot Buttons
  container.querySelectorAll('.btn-compare-snap').forEach(btn => {
    btn.addEventListener('click', () => {
      const slot = (btn as HTMLElement).dataset.slot;
      const time = (btn as HTMLElement).dataset.time || slot;
      const sizeStr = (btn as HTMLElement).dataset.size;
      const hasLocal = (btn as HTMLElement).dataset.local === 'true';
      const dayStr = (btn as HTMLElement).dataset.day;
      const levelStr = (btn as HTMLElement).dataset.level;
      const hostStr = (btn as HTMLElement).dataset.host;
      const curWorlds = cachedWorlds || doctorState.cachedWorlds;
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const world = curWorlds?.find(w => w.worldDir === curWorldDir) || curWorlds?.[0] || null;
      if (!slot || !world) return;

      const snapSize = parseInt(sizeStr || '0', 10);
      const inGameDay = dayStr ? parseInt(dayStr, 10) : undefined;
      const playerLevel = levelStr ? parseInt(levelStr, 10) : undefined;
      
      showSnapshotComparisonModal(world, {
        slotName: slot,
        timestamp: time || slot,
        levelSizeBytes: snapSize,
        localDataExists: hasLocal,
        inGameDay,
        playerLevel,
        hostPlayerName: hostStr || undefined,
      }, container, rerenderCallback);
    });
  });

  // Restore Snapshot Buttons
  container.querySelectorAll('.btn-restore-snap').forEach(btn => {
    btn.addEventListener('click', async () => {
      const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
      const curRestoring = isRestoringBackup || doctorState.isRestoringBackup;
      if (!curWorldDir || curRestoring) return;
      const slot = (btn as HTMLElement).dataset.slot;
      const time = (btn as HTMLElement).dataset.time || slot;
      if (!slot) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_restore_backup_title') || 'Restore World Snapshot',
        (t('scanner.confirm_restore_backup_msg') || 'This will create a safety ZIP of your current world and restore snapshot \'{timestamp}\'. Proceed?').replace('{timestamp}', time || slot)
      );
      if (!confirmed) return;

      setIsRestoringBackup(true);
      await rerenderCallback(container);

      try {
        const result = await restoreSaveBackup(curWorldDir, slot);
        showToast((t('scanner.toast_restore_success') || 'World successfully restored from snapshot {timestamp}.').replace('{timestamp}', time || slot), 'success');
        const rep = await deepScanSaveHealth(curWorldDir);
        setCurrentHealthReport(rep);
        setCachedWorlds(null);
      } catch (err: any) {
        showToast(`Restore failed: ${String(err)}`, 'error');
      } finally {
        setIsRestoringBackup(false);
        await rerenderCallback(container);
      }
    });
  });
}
