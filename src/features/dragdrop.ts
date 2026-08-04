import { getCurrentWebview } from '@tauri-apps/api/webview';
import { analyzeZip, checkModExistsCommand } from '../api';
import { getState, updateState } from '../state';
import { showInstallModal, setModalStatus, renderInstallPreview, closeInstallModal, renderBatchInstallPreview } from '../ui/modal';
import { showToast } from '../ui/toast';

export function setupDragAndDrop(): void {
  const overlay = document.getElementById('drop-overlay')!;
  if (!overlay) return;

  try {
    const webview = getCurrentWebview();
    webview.onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === 'enter') {
        if (getState().activeTab !== 'editor' && !getState().isDraggingCard) {
          overlay.classList.add('visible');
        }
      } else if (payload.type === 'leave') {
        overlay.classList.remove('visible');
      } else if (payload.type === 'drop') {
        overlay.classList.remove('visible');
        if (getState().activeTab === 'editor') return;

        const paths = payload.paths;
        if (!paths || paths.length === 0) return;

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
          showToast(`Skipped ${skipped} non-supported file(s)`, 'info');
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
  showToast(`Added ${added} mod(s) to library`, 'success');
  loadLibrary();
}

async function handleInstallFromPath(zipPath: string): Promise<void> {
  try {
    showInstallModal();
    setModalStatus('Analyzing zip file...');
    const analysis = await analyzeZip(zipPath);

    let existingMod: { id: string; name: string } | null = null;
    try {
      const checkResult = await checkModExistsCommand(zipPath);
      if (checkResult.exists && checkResult.modInfo) {
        existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name };
      }
    } catch {}

    updateState({ currentAnalysis: analysis });
    renderInstallPreview(analysis, existingMod);
  } catch (e) {
    closeInstallModal();
    showToast('Failed to analyze zip: ' + e, 'error');
  }
}
