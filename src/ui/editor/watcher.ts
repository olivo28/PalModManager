import { listen } from '@tauri-apps/api/event';
import { getState, updateState } from '../../state';
import { loadFileContent, _originalContent } from './viewer';
import { renderEditorModTree } from './tree';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';

let isSettingUpWatcher = false;
let debounceTimeout: any = null;
let watcherSuppressedUntil = 0;

export function suppressWatcherRefresh(durationMs: number = 1000): void {
  watcherSuppressedUntil = Date.now() + durationMs;
}

export async function setupEditorFsWatcher(): Promise<void> {
  if (isSettingUpWatcher) return;
  isSettingUpWatcher = true;

  try {
    await listen<{ paths: string[] }>('fs:file-changed', async (event) => {
      if (Date.now() < watcherSuppressedUntil) {
        return;
      }

      const state = getState();
      const changedPaths = event.payload.paths || [];
      if (changedPaths.length === 0) return;

      const norm = (p: string) => p.replace(/\\/g, '/').toLowerCase();
      const normChanged = changedPaths.map(norm);

      // 1. If currently in Config Editor or viewing a mod
      if (state.editorModId) {
        // Refresh mod files from disk and re-render file tree
        const { listModFiles } = await import('../../api');
        const { renderFileTree } = await import('./tree');
        try {
          const latestFiles = await listModFiles(state.editorModId);
          updateState({ editorFiles: latestFiles });
          renderFileTree(latestFiles);

          // Restore selection highlight on file tree if selected file still exists
          if (state.editorSelectedFile) {
            const currentSelected = state.editorSelectedFile;
            const normLatest = latestFiles.map(f => f.replace(/\\/g, '/'));
            const normSelected = currentSelected.replace(/\\/g, '/');
            if (!normLatest.includes(normSelected)) {
              // File was deleted on disk!
              updateState({ editorSelectedFile: null });
              const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
              const editorPath = document.getElementById('editor-file-path');
              const editorStatus = document.getElementById('editor-status');
              const codeEl = document.getElementById('editor-highlight-code');
              if (editorContent) { editorContent.value = ''; editorContent.disabled = true; }
              if (editorPath) editorPath.textContent = '';
              if (editorStatus) editorStatus.textContent = '';
              if (codeEl) codeEl.innerHTML = '';
              showToast(t('toasts.file_deleted_externally', { file: currentSelected }) || `File "${currentSelected}" was deleted on disk`, 'info');
            } else {
              const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(currentSelected)}"]`) as HTMLElement | null;
              if (item) item.classList.add('selected');
            }
          }
        } catch (e) {
          console.error('Failed to refresh editor file tree on change:', e);
        }

        // If the open file itself changed on disk, reload its content
        if (state.editorSelectedFile) {
          let selectedNorm = norm(state.editorSelectedFile);
          // Strip [UE4SS] folder/ or [PalSchema] folder/ prefix if present
          selectedNorm = selectedNorm.replace(/^\[(ue4ss|palschema)\]\s*[^/]+\//, '');

          const isCurrentFileChanged = normChanged.some(p => {
            const cleanP = p.replace(/^\[(ue4ss|palschema)\]\s*[^/]+\//, '');
            return cleanP.endsWith('/' + selectedNorm) || cleanP.endsWith(selectedNorm) || selectedNorm.endsWith(cleanP) || cleanP === selectedNorm;
          });

          if (isCurrentFileChanged) {
            const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
            const hasLocalChanges = editorContent && _originalContent !== null && editorContent.value !== _originalContent;

            if (!hasLocalChanges) {
              await loadFileContent(state.editorSelectedFile);
              const editorStatus = document.getElementById('editor-status');
              if (editorStatus) {
                editorStatus.textContent = 'Auto-reloaded from disk';
                setTimeout(() => {
                  if (editorStatus.textContent === 'Auto-reloaded from disk') editorStatus.textContent = '';
                }, 2500);
              }
            } else {
              showToast(t('toasts.file_modified_externally', { file: state.editorSelectedFile }), 'info');
            }
          }
        }

        // Refresh left mod sidebar in editor
        renderEditorModTree();
      }

      // 2. Debounce refresh for mods view
      clearTimeout(debounceTimeout);
      debounceTimeout = setTimeout(async () => {
        if (Date.now() < watcherSuppressedUntil) return;
        const currentState = getState();
        if (currentState.activeTab === 'mods') {
          const { loadMods } = await import('../modsView');
          loadMods();
        }
      }, 500);
    });
  } catch (err) {
    console.error('Failed to setup fs watcher listener:', err);
  }
}
