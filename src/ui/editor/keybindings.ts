import { getState, updateState } from '../../state';
import { handleEditorSave, handleEditorPreview, syncHighlight, loadEditorData } from './viewer';
import { switchEditorMod, renderEditorModTree } from './tree';
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

  const state = getState();
  if (state.editorModId !== modId) {
    await switchEditorMod(modId);
  }

  const normalizedPath = filePath.replace(/\\/g, '/');

  setTimeout(() => {
    const fileItem = document.querySelector(`.editor-file-item[data-path="${CSS.escape(normalizedPath)}"]`) as HTMLElement | null;
    if (fileItem) {
      if (getState().editorSelectedFile !== normalizedPath) {
        fileItem.click();
      }
    }

    setTimeout(() => {
      const editorContent = editorDom.elMaybe('editor-content');
      if (editorContent) {
        const text = editorContent.value;
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
          editorContent.scrollTop = Math.max(0, (lineNumber - 5) * lineHeight);
          syncHighlight();
        }
      }
    }, 250);
  }, 250);
}
