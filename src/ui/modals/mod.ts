import { openSettingsModal, closeSettingsModal, handleDataPathChange, handleSettingsBrowse, handleSaveSettings } from './settings';
import { handleInstall, closeInstallModal, handleInstallConfirm } from './installer';
import { openConsoleModal } from './console';

export { openSettingsModal, closeSettingsModal, handleDataPathChange, handleSettingsBrowse, handleSaveSettings, _tempCustomDataPath } from './settings';
export { showInstallModal, closeInstallModal, setModalStatus, getCleanNameFromFilename, showFileTreeModal, renderInstallPreview, renderBatchInstallPreview, handleInstallConfirm as handleConfirmInstall, handleInstall, _pendingUpdateModId, _pendingBatchPaths } from './installer';
export { openWorkshopModal, refreshWorkshopUI } from './workshop';
export { openConsoleModal, pushToLogBuffer, _logBuffer } from './console';

export function setupModalListeners(): void {
  // Global buttons & click handlers
  const settingsBtn = document.getElementById('settings-btn');
  if (settingsBtn) {
    settingsBtn.addEventListener('click', () => {
      openSettingsModal();
    });
  }

  const settingsCloseX = document.getElementById('settings-modal-close-x');
  const settingsCloseBtn = document.getElementById('settings-modal-close');
  if (settingsCloseX && settingsCloseBtn) {
    settingsCloseX.onclick = closeSettingsModal;
    settingsCloseBtn.onclick = closeSettingsModal;
  }

  const browseBtn = document.getElementById('settings-browse');
  if (browseBtn) {
    browseBtn.addEventListener('click', () => {
      handleSettingsBrowse();
    });
  }

  const selectDataPath = document.getElementById('settings-data-path-select');
  if (selectDataPath) {
    selectDataPath.addEventListener('change', () => {
      handleDataPathChange();
    });
  }

  const saveSettingsBtn = document.getElementById('settings-save');
  if (saveSettingsBtn) {
    saveSettingsBtn.addEventListener('click', () => {
      handleSaveSettings();
    });
  }

  // Safety Backup & Troubleshooting buttons
  const restoreBtn = document.getElementById('safety-restore-btn');
  if (restoreBtn) {
    restoreBtn.addEventListener('click', async () => {
      const { showConfirm } = await import('../confirm');
      const { showToast } = await import('../toast');
      const { restoreSafetyBackup } = await import('../../api');
      const confirmed = await showConfirm(
        'Restore Pre-PMM State',
        'This will restore your original <code>mods.txt</code>, <code>enabled.txt</code> states, configs, and original mod folders from the initial snapshot taken before PMM modified anything.<br><br><strong>Note:</strong> .pak files are not modified/restored. Do you want to proceed?',
        'Yes, Restore',
        'Cancel'
      );
      if (confirmed) {
        try {
          showToast('Restoring initial backup...', 'info');
          await restoreSafetyBackup();
          showToast('Original mod setup restored successfully!', 'success');
          const { loadDependencies, loadMods } = await import('../modsView');
          await loadDependencies();
          await loadMods();
          const { refreshSafetyBackupStatus } = await import('./settings');
          refreshSafetyBackupStatus();
        } catch (e: any) {
          showToast('Failed to restore backup: ' + e, 'error');
        }
      }
    });
  }

  const backupNowBtn = document.getElementById('safety-backup-now-btn');
  if (backupNowBtn) {
    backupNowBtn.addEventListener('click', async () => {
      const { showToast } = await import('../toast');
      const { triggerSafetyBackup } = await import('../../api');
      try {
        showToast('Creating new safety snapshot...', 'info');
        await triggerSafetyBackup();
        showToast('Safety snapshot created successfully!', 'success');
        const { refreshSafetyBackupStatus } = await import('./settings');
        refreshSafetyBackupStatus();
      } catch (e: any) {
        showToast('Failed to create snapshot: ' + e, 'error');
      }
    });
  }

  const resetWsCacheBtn = document.getElementById('reset-workshop-cache-btn');
  if (resetWsCacheBtn) {
    resetWsCacheBtn.addEventListener('click', async () => {
      const { showConfirm } = await import('../confirm');
      const { showToast } = await import('../toast');
      const { resetWorkshopCache } = await import('../../api');
      const confirmed = await showConfirm(
        'Reset Workshop UE4SS Cache',
        'This will delete <code>Mods\\ManagedMods\\UE4SSExperimentalPW</code>. Next time you launch Palworld, Steam Workshop will automatically extract a fresh copy of UE4SS.<br><br>Do you want to proceed?',
        'Reset Cache',
        'Cancel'
      );
      if (confirmed) {
        try {
          await resetWorkshopCache();
          showToast('Workshop cache cleared! Steam will reinstall UE4SS on next launch.', 'success');
          const { loadDependencies } = await import('../modsView');
          await loadDependencies();
        } catch (e: any) {
          showToast('Failed to reset workshop cache: ' + e, 'error');
        }
      }
    });
  }

  // Installer Modal buttons
  const installBtn = document.getElementById('install-btn');
  if (installBtn) {
    installBtn.addEventListener('click', () => {
      handleInstall();
    });
  }

  const modalCloseX = document.getElementById('modal-close-x');
  const modalCancel = document.getElementById('modal-cancel');
  if (modalCloseX && modalCancel) {
    modalCloseX.onclick = closeInstallModal;
    modalCancel.onclick = closeInstallModal;
  }

  const modalConfirm = document.getElementById('modal-confirm');
  if (modalConfirm) {
    modalConfirm.addEventListener('click', () => {
      handleInstallConfirm();
    });
  }

  // Console Modal launcher
  const consoleBtn = document.getElementById('console-btn');
  if (consoleBtn) {
    consoleBtn.addEventListener('click', () => {
      openConsoleModal();
    });
  }

  // Global overlay click listeners
  document.querySelectorAll('.modal-overlay').forEach((overlay) => {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        overlay.classList.remove('visible');
        if (overlay.id === 'install-modal') {
          closeInstallModal();
        }
      }
    });
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      document.querySelectorAll('.modal-overlay').forEach((overlay) => {
        if (overlay.classList.contains('visible')) {
          overlay.classList.remove('visible');
          if (overlay.id === 'install-modal') {
            closeInstallModal();
          }
        }
      });
    }
  });
}
