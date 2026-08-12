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
