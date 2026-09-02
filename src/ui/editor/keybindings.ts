import { getState, updateState } from '../../state';
import { handleEditorSave, handleEditorPreview, handleEditorFormat, loadEditorData } from './viewer';
import { switchEditorMod, renderEditorModTree, revealAndSelectFile } from './tree';
import { confirmDiscardOrSave } from './unsaved';
import { editorDom, mainDom } from '../../framework';
import { getMonacoEditor } from './monaco/instance';

export function setupEditorKeybindings(): void {
  document.addEventListener('keydown', (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      const editorView = editorDom.elMaybe('editor-view');
      if (editorView && editorView.style.display !== 'none') {
        e.preventDefault();
        handleEditorSave();
      }
    }
  });

  const saveBtn = editorDom.elMaybe('editor-save-btn');
  if (saveBtn) {
    saveBtn.addEventListener('click', handleEditorSave);
  }

  const formatBtn = editorDom.elMaybe('editor-format-btn');
  if (formatBtn) {
    formatBtn.addEventListener('click', handleEditorFormat);
  }

  const previewBtn = editorDom.elMaybe('editor-preview-btn');
  if (previewBtn) {
    previewBtn.addEventListener('click', handleEditorPreview);
  }

  const diffBtn = editorDom.elMaybe('editor-diff-btn');
  if (diffBtn) {
    diffBtn.addEventListener('click', async () => {
      const state = getState();
      if (state.editorModId && state.editorSelectedFile) {
        const { openEditorDiffModal } = await import('./diffModal');
        await openEditorDiffModal(state.editorModId, state.editorSelectedFile);
      }
    });
  }

  const restoreBtn = editorDom.elMaybe('editor-restore-btn');
  if (restoreBtn) {
    restoreBtn.addEventListener('click', async () => {
      const state = getState();
      if (!state.editorModId || !state.editorSelectedFile) return;

      const backupPath = state.editorSelectedFile;
      let targetPath = backupPath;
      if (targetPath.endsWith('.bak')) {
        targetPath = targetPath.slice(0, -4);
      } else if (targetPath.endsWith('.bak1') || targetPath.endsWith('.bak2')) {
        targetPath = targetPath.slice(0, -5);
      }

      const { showConfirm } = await import('../confirm');
      const { t } = await import('../../utils/i18n');
      const title = t('editor.confirm_restore_title') || 'Restore Backup';
      const body = (t('editor.confirm_restore_body') || 'Are you sure you want to restore **{backup}** over **{target}**?\nA backup of the current file will be preserved.')
        .replace('{backup}', backupPath)
        .replace('{target}', targetPath);

      const confirmed = await showConfirm(title, body);
      if (!confirmed) return;

      try {
        const { restoreModBackup } = await import('../../api');
        const { showToast } = await import('../toast');
        const { refreshEditorFileTree } = await import('./tree');
        const { loadFileContent } = await import('./viewer');

        const res = await restoreModBackup(state.editorModId, backupPath);
        if (res.success) {
          showToast(t('editor.toast_restored') || 'Backup restored successfully', 'success');
          await refreshEditorFileTree(state.editorModId);
          await loadFileContent(targetPath);
        }
      } catch (err) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
      }
    });
  }
}

export async function handleEditorModChange(): Promise<void> {
  const select = editorDom.elMaybe('editor-mod-select');
  if (!select) return;
  const modId = select.value;
  const state = getState();
  const currentModId = state.editorModId;

  if (currentModId === modId) return;

  const proceed = await confirmDiscardOrSave();
  if (!proceed) {
    select.value = currentModId || '';
    return;
  }

  if (modId) {
    updateState({ editorModId: modId, editorSelectedFile: null });
    await loadEditorData(modId);
  }
}

export function switchTab(tab: 'mods' | 'editor' | 'library' | 'build' | 'scanner'): void {
  updateState({ activeTab: tab as any });
  document.querySelectorAll('.sidebar-tab').forEach(b => b.classList.remove('active'));
  const tabBtn = document.querySelector(`.sidebar-tab[data-tab="${tab}"]`);
  if (tabBtn) tabBtn.classList.add('active');

  const modsView = mainDom.elMaybe('mods-view');
  if (modsView) modsView.style.display = tab === 'mods' ? '' : 'none';

  const editorView = editorDom.elMaybe('editor-view');
  if (editorView) editorView.style.display = tab === 'editor' ? 'flex' : 'none';

  const libView = document.getElementById('library-view');
  if (libView) libView.style.display = tab === 'library' ? 'flex' : 'none';

  const buildView = document.getElementById('build-view');
  if (buildView) buildView.style.display = tab === 'build' ? 'flex' : 'none';

  const scannerView = document.getElementById('scanner-view');
  if (scannerView) scannerView.style.display = tab === 'scanner' ? 'flex' : 'none';

  if (tab === 'editor') {
    renderEditorModTree();
  }
  if (tab === 'build') {
    import('../packer/projects').then(m => m.showProjectsHub());
  }
}

export async function openFileAtLine(modId: string, filePath: string, lineNumber: number): Promise<void> {
  const { navigateTo } = await import('../tabManager');
  navigateTo('editor');

  const normalizedPath = filePath.replace(/\\/g, '/').replace(/^\/+/, '').trim();
  const state = getState();

  // Find exact mod ID if display name or folder was passed
  const targetMod = state.allMods.find(m => 
    m.id === modId || 
    m.name.toLowerCase() === modId.toLowerCase() || 
    (m as any).folderName?.toLowerCase() === modId.toLowerCase()
  );
  const resolvedModId = targetMod ? targetMod.id : modId;

  if (state.editorModId !== resolvedModId) {
    await switchEditorMod(resolvedModId, normalizedPath, lineNumber);
  } else {
    await revealAndSelectFile(normalizedPath, lineNumber);
  }

  jumpToLineInEditor(lineNumber);
}

export function jumpToLineInEditor(lineNumber: number, retryCount = 0): void {
  const editor = getMonacoEditor();
  if (!editor) {
    if (retryCount < 15) {
      setTimeout(() => jumpToLineInEditor(lineNumber, retryCount + 1), 40);
    }
    return;
  }

  const model = editor.getModel();
  if (!model) {
    if (retryCount < 15) {
      setTimeout(() => jumpToLineInEditor(lineNumber, retryCount + 1), 40);
    }
    return;
  }

  const maxLine = model.getLineCount();
  const targetLine = Math.min(Math.max(1, lineNumber), maxLine);

  editor.revealLineInCenter(targetLine);
  editor.setPosition({ lineNumber: targetLine, column: 1 });
  editor.focus();

  // Highlight the line briefly with a flash decoration
  const monaco = (window as any).monaco;
  if (monaco) {
    const decorations = editor.createDecorationsCollection([
      {
        range: new monaco.Range(targetLine, 1, targetLine, model.getLineMaxColumn(targetLine)),
        options: {
          isWholeLine: true,
          className: 'monaco-line-jump-highlight',
        },
      },
    ]);
    setTimeout(() => decorations.clear(), 2000);
  }
}
