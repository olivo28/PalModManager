import { cleanConflictDlls } from '../api';
import { showToast } from './toast';
import { loadDependencies } from './modsView';
import { t } from '../utils/i18n';

let bannerElement: HTMLElement | null = null;

export function renderConflictBanner(conflictingDlls: string[]): void {
  removeConflictBanner();

  if (!conflictingDlls || conflictingDlls.length === 0) {
    return;
  }

  const appElement = document.getElementById('app');
  if (!appElement) return;

  const mainElement = appElement.querySelector('main');
  if (!mainElement) return;

  bannerElement = document.createElement('div');
  bannerElement.id = 'global-conflict-banner';
  bannerElement.className = 'global-conflict-banner';

  const dllNames = conflictingDlls.join(', ');

  bannerElement.innerHTML = `
    <div class="conflict-banner-content">
      <span class="conflict-banner-icon">⚠️</span>
      <div class="conflict-banner-text">
        <strong>${t('dependencies.banner_conflict_title')}</strong> ${t('dependencies.banner_conflict_desc', { dlls: dllNames })}
      </div>
    </div>
    <div class="conflict-banner-actions">
      <button id="clean-conflict-dlls-btn" class="conflict-btn-action" title="${t('dependencies.banner_clean_btn_title')}">${t('dependencies.banner_clean_btn')}</button>
      <button id="dismiss-conflict-banner-btn" class="conflict-btn-dismiss" title="${t('dependencies.banner_dismiss_title')}">✕</button>
    </div>
  `;

  mainElement.insertBefore(bannerElement, mainElement.firstChild);

  const cleanBtn = document.getElementById('clean-conflict-dlls-btn');
  cleanBtn?.addEventListener('click', async () => {
    try {
      if (cleanBtn) cleanBtn.textContent = 'Cleaning...';
      const removed = await cleanConflictDlls();
      showToast(t('toasts.dlls_cleaned', { count: removed.length }), 'success');
      removeConflictBanner();
      await loadDependencies();
    } catch (e: any) {
      showToast(t('toasts.clean_conflict_dlls_failed', { error: String(e) }), 'error');
      if (cleanBtn) cleanBtn.textContent = '🧹 Clean Conflict DLLs';
    }
  });

  const dismissBtn = document.getElementById('dismiss-conflict-banner-btn');
  dismissBtn?.addEventListener('click', () => {
    removeConflictBanner();
  });
}

export function removeConflictBanner(): void {
  if (bannerElement && bannerElement.parentNode) {
    bannerElement.parentNode.removeChild(bannerElement);
  }
  bannerElement = null;
}
