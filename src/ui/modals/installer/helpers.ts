import { updateState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import {
  _onInstallCompleteCallback,
  _lastInstallSuccess,
  setPendingUpdateModId,
  setPendingBatchPaths,
  setBatchItems,
  setLastInstallSuccess,
  setInstallModalCallback
} from './state';
import { installerDom } from '../../../framework';

export function showInstallModal(): void {
  const modal = installerDom.elMaybe('install-modal');
  if (modal) modal.classList.add('visible');
  const content = installerDom.elMaybe('modal-content');
  if (content) {
    content.innerHTML = `
      <div style="padding: 40px; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 180px; gap: 12px;">
        <div class="loading-spinner"></div>
        <span style="font-size: 12px; font-weight: 600; color: var(--text-secondary);">${escapeHtml(t('installer.status_analyzing'))}</span>
      </div>
    `;
  }
  const statusEl = installerDom.elMaybe('modal-status');
  if (statusEl) {
    statusEl.textContent = '';
  }
}

export function closeInstallModal(): void {
  const modal = installerDom.elMaybe('install-modal');
  if (modal) modal.classList.remove('visible');
  updateState({ currentAnalysis: null });
  setPendingUpdateModId(null);
  setPendingBatchPaths([]);
  setBatchItems([]);

  const content = installerDom.elMaybe('modal-content');
  if (content) {
    content.innerHTML = '';
  }
  const statusEl = installerDom.elMaybe('modal-status');
  if (statusEl) {
    statusEl.textContent = '';
  }

  const retryBtn = installerDom.elMaybe('modal-install-deps-retry');
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  const confirmBtn = installerDom.elMaybe('modal-confirm');
  const cancelBtn = installerDom.elMaybe('modal-cancel');
  if (confirmBtn) {
    confirmBtn.style.display = '';
    confirmBtn.textContent = t('installer.btn_install');
    confirmBtn.disabled = false;
  }
  if (cancelBtn) {
    cancelBtn.textContent = t('common.cancel');
    cancelBtn.disabled = false;
  }

  // Restore modal size to default
  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '750px';
  }

  if (_onInstallCompleteCallback) {
    const cb = _onInstallCompleteCallback;
    setInstallModalCallback(null);
    cb(_lastInstallSuccess);
  }
  setLastInstallSuccess(false);
}

export function setModalStatus(status: string): void {
  const statusEl = installerDom.elMaybe('modal-status');
  if (statusEl) statusEl.textContent = status;
}

export function getCleanNameFromFilename(filename: string): string {
  const stem = filename.substring(0, filename.lastIndexOf('.')) || filename;
  const words = stem.split(/\s+/);

  let idIndex = -1;
  for (let i = words.length - 1; i >= 0; i--) {
    const word = words[i].replace(/[()]/g, '');
    if (/^\d+$/.test(word)) {
      const num = parseInt(word, 10);
      if (!(num >= 2020 && num <= 2038)) {
        idIndex = i;
        break;
      }
    }
  }

  let cleanWords = words;
  if (idIndex !== -1) {
    cleanWords = words.slice(0, idIndex);
  } else {
    for (let i = 0; i < words.length; i++) {
      const word = words[i];
      if (/^\d{4}-\d{2}-\d{2}/.test(word) || (word.includes('-') && word.length > 6 && /^\d/.test(word))) {
        cleanWords = words.slice(0, i);
        break;
      }
    }
  }

  const clean: string[] = [];
  for (const word of cleanWords) {
    const lower = word.toLowerCase().replace(/[()]/g, '');
    if (["gamepass", "steam", "gdk", "xbox", "singleplayer", "sp"].includes(lower)) {
      continue;
    }
    clean.push(word);
  }

  const result = clean.join(' ').trim();
  const finalResult = result.replace(/[-\s_]+$/, '').trim();
  return finalResult.length < 2 ? stem.trim() : finalResult;
}
