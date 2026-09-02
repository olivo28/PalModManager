import { getState, updateState } from '../../state';
import { handleEditorSave, handleEditorPreview, syncHighlight, loadEditorData } from './viewer';
import { switchEditorMod, renderEditorModTree, revealAndSelectFile } from './tree';
import { openFind, closeFind } from './search';
import { confirmDiscardOrSave } from './unsaved';
import { editorDom, mainDom } from '../../framework';

export function setupEditorKeybindings(): void {
  const editorContent = editorDom.elMaybe('editor-content');
  const highlightEl = editorDom.elMaybe('editor-highlight');

  document.addEventListener('keydown', (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
      const editorView = editorDom.elMaybe('editor-view');
      if (editorView && editorView.style.display !== 'none') {
        e.preventDefault();
        openFind();
      }
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      const editorView = editorDom.elMaybe('editor-view');
      if (editorView && editorView.style.display !== 'none') {
        e.preventDefault();
        handleEditorSave();
      }
    }
  });

  if (editorContent) {
    const updateCursorPosition = () => {
      const pos = editorContent.selectionStart;
      const val = editorContent.value;
      const textBefore = val.substring(0, pos);
      const lines = textBefore.split('\n');
      const lineNum = lines.length;
      const colNum = lines[lines.length - 1].length + 1;
      const cursorEl = editorDom.elMaybe('editor-cursor-pos');
      if (cursorEl) {
        cursorEl.textContent = `Ln ${lineNum}, Col ${colNum}`;
      }
    };

    editorContent.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        e.preventDefault();
        const start = editorContent.selectionStart;
        const end = editorContent.selectionEnd;
        editorContent.value = editorContent.value.substring(0, start) + '  ' + editorContent.value.substring(end);
        editorContent.selectionStart = editorContent.selectionEnd = start + 2;
        syncHighlight();
        updateCursorPosition();
      } else if (e.key === 'Enter') {
        const start = editorContent.selectionStart;
        const val = editorContent.value;
        const textBefore = val.substring(0, start);
        const currentLine = textBefore.split('\n').pop() || '';
        const matchIndent = currentLine.match(/^(\s+)/);
        let indent = matchIndent ? matchIndent[1] : '';

        const trimmed = currentLine.trim();
        if (trimmed.endsWith('{') || trimmed.endsWith('[') || trimmed.endsWith('(') || trimmed.endsWith('then') || trimmed.endsWith('do')) {
          indent += '  ';
        }

        if (indent.length > 0) {
          e.preventDefault();
          const end = editorContent.selectionEnd;
          editorContent.value = val.substring(0, start) + '\n' + indent + val.substring(end);
          editorContent.selectionStart = editorContent.selectionEnd = start + 1 + indent.length;
          syncHighlight();
          updateCursorPosition();
        }
      }
    });

    editorContent.addEventListener('input', () => {
      syncHighlight();
      updateCursorPosition();
    });

    editorContent.addEventListener('click', updateCursorPosition);
    editorContent.addEventListener('keyup', updateCursorPosition);

    editorContent.addEventListener('scroll', () => {
      if (highlightEl) {
        highlightEl.scrollTop = editorContent.scrollTop;
        highlightEl.scrollLeft = editorContent.scrollLeft;
      }
      const gutter = editorDom.elMaybe('editor-gutter');
      if (gutter) gutter.scrollTop = editorContent.scrollTop;
    });
  }

  if (highlightEl && editorContent) {
    highlightEl.addEventListener('scroll', () => {
      editorContent.scrollTop = highlightEl.scrollTop;
      editorContent.scrollLeft = highlightEl.scrollLeft;
      const gutter = editorDom.elMaybe('editor-gutter');
      if (gutter) gutter.scrollTop = highlightEl.scrollTop;
    });
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
}

export async function openFileAtLine(modId: string, filePath: string, lineNumber: number): Promise<void> {
  const { navigateTo } = await import('../tabManager');
  navigateTo('editor');

  const normalizedPath = filePath.replace(/\\/g, '/').replace(/^\/+/, '').trim();
  const state = getState();

  if (state.editorModId !== modId) {
    await switchEditorMod(modId, normalizedPath);
  } else {
    revealAndSelectFile(normalizedPath);
  }

  jumpToLineInEditor(lineNumber);
}

export function jumpToLineInEditor(lineNumber: number, retryCount = 0): void {
  const editorContent = editorDom.elMaybe('editor-content') as HTMLTextAreaElement | null;
  if (!editorContent) return;

  const text = editorContent.value;
  if (!text && retryCount < 10) {
    setTimeout(() => jumpToLineInEditor(lineNumber, retryCount + 1), 60);
    return;
  }

  const lines = text.split('\n');
  if (lineNumber > 0 && lineNumber <= lines.length) {
    let charIndex = 0;
    for (let i = 0; i < lineNumber - 1; i++) {
      charIndex += lines[i].length + 1;
    }
    const lineText = lines[lineNumber - 1];
    editorContent.focus();
    editorContent.setSelectionRange(charIndex, charIndex + lineText.length);

    const lineHeight = 19;
    const targetScrollTop = Math.max(0, (lineNumber - 6) * lineHeight);
    editorContent.scrollTop = targetScrollTop;

    const highlight = editorDom.elMaybe('editor-highlight');
    if (highlight) highlight.scrollTop = targetScrollTop;
    const gutter = editorDom.elMaybe('editor-gutter');
    if (gutter) gutter.scrollTop = targetScrollTop;

    syncHighlight(true);
  }
}
