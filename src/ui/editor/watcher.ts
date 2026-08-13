import { listen } from '@tauri-apps/api/event';
import { getState } from '../../state';
import { loadFileContent, _originalContent } from './viewer';
import { renderEditorModTree } from './tree';
import { showToast } from '../toast';

let isSettingUpWatcher = false;
let debounceTimeout: any = null;

export async function setupEditorFsWatcher(): Promise<void> {
  if (isSettingUpWatcher) return;
  isSettingUpWatcher = true;

  try {
    await listen<{ paths: string[] }>('fs:file-changed', async (event) => {
      const state = getState();
      const changedPaths = event.payload.paths || [];
      if (changedPaths.length === 0) return;

      const norm = (p: string) => p.replace(/\\/g, '/').toLowerCase();
      const normChanged = changedPaths.map(norm);

      // 1. If currently in Config Editor or viewing a mod
      if (state.editorModId) {
        // If the open file itself changed on disk
        if (state.editorSelectedFile) {
          const selectedNorm = norm(state.editorSelectedFile);
          const isCurrentFileChanged = normChanged.some(p => p.endsWith('/' + selectedNorm) || p.endsWith(selectedNorm) || selectedNorm.endsWith(p));

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
              showToast(`File "${state.editorSelectedFile}" was modified externally`, 'info');
            }
          }
        }

        // Refresh file tree in editor
        renderEditorModTree();
      }

      // 2. Debounce refresh for mods view
      clearTimeout(debounceTimeout);
      debounceTimeout = setTimeout(async () => {
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
