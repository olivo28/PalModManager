import { openSettingsModal, closeSettingsModal, handleDataPathChange, handleSettingsBrowse, handleSaveSettings } from './settings';
import { handleInstall, closeInstallModal, handleInstallConfirm } from './installer';
import { openConsoleModal } from './console';
import { openAboutModal, closeAboutModal, setupAboutModal } from './about';
import { mainDom, settingsDom, installerDom } from '../../framework';

export { openSettingsModal, closeSettingsModal, handleDataPathChange, handleSettingsBrowse, handleSaveSettings, _tempCustomDataPath } from './settings';
export { showInstallModal, closeInstallModal, setModalStatus, getCleanNameFromFilename, showFileTreeModal, renderInstallPreview, renderBatchInstallPreview, handleInstallConfirm as handleConfirmInstall, handleInstall, openInstallModalForZip, setInstallModalCallback, _pendingUpdateModId, _pendingBatchPaths } from './installer';

export { openWorkshopModal, refreshWorkshopUI } from './workshop';
export { openConsoleModal, pushToLogBuffer, _logBuffer } from './console';
export { openAboutModal, closeAboutModal, setupAboutModal } from './about';

export function setupModalListeners(): void {
  setupAboutModal();

  // Settings & Global navigation triggers
  mainDom.elMaybe('settings-btn')?.addEventListener('click', openSettingsModal);
  settingsDom.elMaybe('settings-modal-close-x')?.addEventListener('click', closeSettingsModal);
  settingsDom.elMaybe('settings-cancel')?.addEventListener('click', closeSettingsModal);
  settingsDom.elMaybe('settings-browse-btn')?.addEventListener('click', handleSettingsBrowse);
  settingsDom.elMaybe('settings-data-path-select')?.addEventListener('change', handleDataPathChange);
  settingsDom.elMaybe('settings-save')?.addEventListener('click', handleSaveSettings);

  // Safety Backup & Troubleshooting buttons
  settingsDom.elMaybe('safety-restore-btn')?.addEventListener('click', async () => {
    const { showConfirm } = await import('../confirm');
    const { showToast } = await import('../toast');
    const { restoreSafetyBackup } = await import('../../api');
    const { t } = await import('../../utils/i18n');
    const confirmed = await showConfirm(
      t('settings.safety_restore_dialog_title'),
      t('settings.safety_restore_dialog_body'),
      t('settings.safety_restore_btn_confirm'),
      t('common.cancel')
    );
    if (confirmed) {
      try {
        showToast(t('toasts.backup_restoring'), 'info');
        await restoreSafetyBackup();
        showToast(t('toasts.backup_restored'), 'success');
        const { loadDependencies, loadMods } = await import('../modsView');
        await loadDependencies();
        await loadMods();
        const { refreshSafetyBackupStatus } = await import('./settings');
        refreshSafetyBackupStatus();
      } catch (e: any) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    }
  });

  settingsDom.elMaybe('safety-backup-now-btn')?.addEventListener('click', async () => {
    const { showToast } = await import('../toast');
    const { triggerSafetyBackup } = await import('../../api');
    const { t } = await import('../../utils/i18n');
    try {
      showToast(t('toasts.backup_creating'), 'info');
      await triggerSafetyBackup();
      showToast(t('toasts.backup_created', { path: 'Pre-PMM' }), 'success');
      const { refreshSafetyBackupStatus } = await import('./settings');
      refreshSafetyBackupStatus();
    } catch (e: any) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    }
  });

  settingsDom.elMaybe('reset-workshop-cache-btn')?.addEventListener('click', async () => {
    const { showConfirm } = await import('../confirm');
    const { showToast } = await import('../toast');
    const { resetWorkshopCache } = await import('../../api');
    const { t } = await import('../../utils/i18n');
    const confirmed = await showConfirm(
      t('settings.safety_reset_ws_title'),
      t('settings.safety_reset_ws_body'),
      t('settings.safety_reset_ws_btn_confirm'),
      t('common.cancel')
    );
    if (confirmed) {
      try {
        await resetWorkshopCache();
        showToast(t('toasts.workshop_state_updated'), 'success');
        const { loadDependencies } = await import('../modsView');
        await loadDependencies();
      } catch (e: any) {
        showToast(t('toasts.export_failed', { error: String(e) }), 'error');
      }
    }
  });

  // Installer Modal buttons
  mainDom.elMaybe('install-btn')?.addEventListener('click', handleInstall);
  installerDom.elMaybe('modal-close-x')?.addEventListener('click', closeInstallModal);
  installerDom.elMaybe('modal-cancel')?.addEventListener('click', closeInstallModal);
  installerDom.elMaybe('modal-confirm')?.addEventListener('click', handleInstallConfirm);

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
}
