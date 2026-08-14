import { cleanConflictDlls } from '../api';
import { showToast } from './toast';
import { loadDependencies } from './modsView';

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
        <strong>Fatal DLL Conflict Detected:</strong> Leftover Nexus DLLs (<code>${dllNames}</code>) found in <code>Pal/Binaries/Win64</code> while using Steam Workshop UE4SS. Palworld will crash on launch!
      </div>
    </div>
    <div class="conflict-banner-actions">
      <button id="clean-conflict-dlls-btn" class="conflict-btn-action" title="Quarantine leftover DLLs safely">🧹 Clean Conflict DLLs</button>
      <button id="dismiss-conflict-banner-btn" class="conflict-btn-dismiss" title="Dismiss this warning for this session">✕</button>
    </div>
  `;

  mainElement.insertBefore(bannerElement, mainElement.firstChild);

  const cleanBtn = document.getElementById('clean-conflict-dlls-btn');
  cleanBtn?.addEventListener('click', async () => {
    try {
      if (cleanBtn) cleanBtn.textContent = 'Cleaning...';
      const removed = await cleanConflictDlls();
      showToast(`Cleaned ${removed.length} conflicting DLL(s). Files moved to quarantine folder.`, 'success');
      removeConflictBanner();
      await loadDependencies();
    } catch (e: any) {
      showToast(`Failed to clean conflict DLLs: ${e}`, 'error');
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
