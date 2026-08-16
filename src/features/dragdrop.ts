import { getCurrentWebview } from '@tauri-apps/api/webview';
import { analyzeZip, checkModExistsCommand } from '../api';
import { getState, updateState } from '../state';
import { showInstallModal, setModalStatus, renderInstallPreview, closeInstallModal, renderBatchInstallPreview } from '../ui/modal';
import { showToast } from '../ui/toast';
import { t } from '../utils/i18n';

export function setupDragAndDrop(): void {
  const overlay = document.getElementById('drop-overlay')!;
  if (!overlay) return;

  // ─── Internal DOM drag detection ───────────────────────────────────────────
  // When the user drags a DOM element (load order item, packer tree node, etc.)
  // the Tauri webview fires its own drag events as if files are being dropped
  // from the OS. We mark internal drags on document.body so the Tauri handler
  // can skip them and avoid raising the blocking file-drop overlay.
  document.addEventListener('dragstart', () => {
    document.body.setAttribute('data-internal-drag', 'true');
  }, true);
  document.addEventListener('dragend', () => {
    document.body.removeAttribute('data-internal-drag');
  }, true);

  try {
    const webview = getCurrentWebview();
    webview.onDragDropEvent((event) => {
      const payload = event.payload;

      // If a DOM element drag is active, this is an internal reorder — ignore it.
      if (document.body.hasAttribute('data-internal-drag')) return;

      if (payload.type === 'enter') {
        const tab = getState().activeTab;
        if (tab !== 'editor' && tab !== 'build' && !getState().isDraggingCard) {
          overlay.classList.add('visible');
        } else if (tab === 'build') {
          const packerOverlay = document.getElementById('packer-drag-overlay');
          if (packerOverlay) packerOverlay.classList.add('drag-over');
        }
      } else if (payload.type === 'leave') {
        overlay.classList.remove('visible');
        const packerOverlay = document.getElementById('packer-drag-overlay');
        if (packerOverlay) packerOverlay.classList.remove('drag-over');
      } else if (payload.type === 'drop') {
        overlay.classList.remove('visible');
        const packerOverlay = document.getElementById('packer-drag-overlay');
        if (packerOverlay) packerOverlay.classList.remove('drag-over');

        const tab = getState().activeTab;
        if (tab === 'editor') return;

        const paths = payload.paths;
        if (!paths || paths.length === 0) return;

        if (tab === 'build') {
          import('../ui/packer/mod').then(mod => {
            mod.addStagedPaths(paths);
          });
          return;
        }


        const archives: string[] = [];
        let skipped = 0;

        for (const path of paths) {
          const lower = path.toLowerCase();
          if (lower.endsWith('.zip') || lower.endsWith('.rar') || lower.endsWith('.7z')) {
            archives.push(path);
          } else {
            skipped++;
          }
        }

        if (skipped > 0) {
          showToast(t('toasts.skipped_unsupported_files', { count: skipped }), 'info');
        }

        if (archives.length === 0) return;

        if (getState().activeTab === 'library') {
          handleImportToLibrary(archives);
          return;
        }

        if (archives.length === 1) {
          handleInstallFromPath(archives[0]);
        } else {
          renderBatchInstallPreview(archives);
        }
      }
    }).catch(err => {
      console.error("Failed to setup Tauri drag and drop event listener:", err);
    });
  } catch (e) {
    console.error("Failed to fetch getCurrentWebview:", e);
  }
}

async function handleImportToLibrary(archives: string[]): Promise<void> {
  const { copyToLibraryCommand } = await import('../api');
  const { loadLibrary } = await import('../ui/modsView');
  let added = 0;
  for (const path of archives) {
    try {
      await copyToLibraryCommand(path);
      added++;
    } catch (e) {
      console.error('Failed to add to library:', e);
    }
  }
  showToast(t('toasts.added_to_library', { count: added }), 'success');
  loadLibrary();
}

async function handleInstallFromPath(zipPath: string): Promise<void> {
  try {
    showInstallModal();
    setModalStatus(t('installer.status_analyzing'));
    const analysis = await analyzeZip(zipPath);

    let existingMod: { id: string; name: string; version: string } | null = null;
    try {
      const checkResult = await checkModExistsCommand(zipPath);
      if (checkResult.exists && checkResult.modInfo) {
        existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name, version: checkResult.modInfo.version };
      }
    } catch { }

    updateState({ currentAnalysis: analysis });
    renderInstallPreview(analysis, existingMod);
  } catch (e) {
    closeInstallModal();
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}
