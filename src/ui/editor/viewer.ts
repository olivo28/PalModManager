import { readModFile, saveModFile, listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { marked } from 'marked';
import { confirmDiscardOrSave } from './unsaved';
import { renderFileTree, refreshEditorFileTree } from './tree';
import { editorDom, bus } from '../../framework';
import {
  initMonacoEditor,
  getMonacoEditor,
  setMonacoFile,
  getMonacoContent,
  getCurrentMonacoFilePath,
} from './monaco/instance';

export let _originalContent: string | null = null;
export const _lastFilePerMod: Record<string, string> = {};

export function clearOriginalContent(): void {
  _originalContent = null;
  updateUnsavedIndicator();
}

export function clearEditorContent(): void {
  _originalContent = null;
  const editorPath = editorDom.elMaybe('editor-file-path');
  if (editorPath) editorPath.textContent = '';
  const editorStatus = editorDom.elMaybe('editor-status');
  if (editorStatus) editorStatus.textContent = '';
  const formatBtn = editorDom.elMaybe('editor-format-btn');
  if (formatBtn) formatBtn.style.display = 'none';
  const previewBtn = editorDom.elMaybe('editor-preview-btn');
  if (previewBtn) previewBtn.style.display = 'none';
  const diffBtn = editorDom.elMaybe('editor-diff-btn');
  if (diffBtn) diffBtn.style.display = 'none';
  const restoreBtn = editorDom.elMaybe('editor-restore-btn');
  if (restoreBtn) restoreBtn.style.display = 'none';
  const preview = editorDom.elMaybe('editor-preview');
  if (preview) preview.style.display = 'none';
  const healthBadge = editorDom.elMaybe('editor-health-badge');
  if (healthBadge) healthBadge.style.display = 'none';

  setMonacoFile('empty.txt', '');
  updateUnsavedIndicator();
}

export function updateUnsavedIndicator(): void {
  const currentText = getMonacoContent();
  const isDirty = _originalContent !== null && (() => {
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    return normalize(currentText) !== normalize(_originalContent);
  })();

  const state = getState();
  const selectedPath = state.editorSelectedFile;
  if (selectedPath) {
    const activeItem = document.querySelector(`.editor-file-item[data-path="${CSS.escape(selectedPath)}"]`);
    if (activeItem) {
      activeItem.classList.toggle('dirty', isDirty);
    }
  }

  const saveBtn = editorDom.elMaybe('editor-save-btn');
  if (saveBtn) {
    saveBtn.classList.toggle('dirty', isDirty);
  }
}

export async function loadFileContent(filePath: string): Promise<void> {
  const state = getState();
  if (!state.editorModId) return;

  const editorPath = editorDom.el('editor-file-path');
  const editorStatus = editorDom.el('editor-status');
  const formatBtn = editorDom.el('editor-format-btn');
  const previewBtn = editorDom.el('editor-preview-btn');
  const diffBtn = editorDom.elMaybe('editor-diff-btn');
  const restoreBtn = editorDom.elMaybe('editor-restore-btn');
  const preview = editorDom.el('editor-preview');
  const monacoContainer = editorDom.elMaybe('editor-monaco-container');
  const healthBadge = editorDom.elMaybe('editor-health-badge');

  editorStatus.textContent = '';
  updateState({ editorPreviewMode: false });
  preview.style.display = 'none';
  if (monacoContainer) monacoContainer.style.display = '';
  previewBtn.style.display = 'none';
  previewBtn.textContent = 'Preview';

  const isBak = filePath.toLowerCase().includes('.bak');
  if (diffBtn) diffBtn.style.display = isBak ? '' : 'none';
  if (restoreBtn) restoreBtn.style.display = isBak ? '' : 'none';
  if (healthBadge) healthBadge.style.display = '';

  try {
    const result = await readModFile(state.editorModId, filePath);
    if (!result.content) {
      editorPath.textContent = t('editor.no_content_available');
      _originalContent = null;
      if (monacoContainer) monacoContainer.style.display = 'none';
      if (formatBtn) formatBtn.style.display = 'none';
      if (diffBtn) diffBtn.style.display = 'none';
      if (restoreBtn) restoreBtn.style.display = 'none';
    } else if (result.configType === 'image') {
      editorPath.textContent = result.path || filePath;
      preview.innerHTML = `
        <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;padding:20px;box-sizing:border-box;background:var(--bg-secondary);">
          <img src="${result.content}" style="max-width:100%;max-height:80vh;object-fit:contain;border-radius:4px;box-shadow:0 4px 16px rgba(0,0,0,0.4);" />
          <div style="margin-top:12px;font-size:11px;color:var(--text-muted);">${escapeHtml(filePath)}</div>
        </div>`;
      preview.style.display = 'block';
      if (monacoContainer) monacoContainer.style.display = 'none';
      _originalContent = null;
      if (formatBtn) formatBtn.style.display = 'none';
      if (previewBtn) previewBtn.style.display = 'none';
      if (diffBtn) diffBtn.style.display = 'none';
      if (restoreBtn) restoreBtn.style.display = 'none';
      editorStatus.textContent = '';
      return;
    } else {
      editorPath.textContent = result.path || filePath;
      _originalContent = result.content;

      // Initialize Monaco Editor and set file
      setMonacoFile(filePath, result.content);
      const editor = getMonacoEditor();

      if (editor) {
        editor.onDidChangeCursorPosition((e) => {
          const cursorPosEl = editorDom.elMaybe('editor-cursor-pos');
          if (cursorPosEl) {
            cursorPosEl.textContent = `Ln ${e.position.lineNumber}, Col ${e.position.column}`;
          }
        });

        editor.onDidChangeModelContent(() => {
          updateUnsavedIndicator();
        });
      }

      formatBtn.style.display = (result.configType === 'json' || result.configType === 'jsonc') && !isBak ? '' : 'none';
      if (filePath.endsWith('.md')) {
        previewBtn.style.display = '';
        preview.style.display = 'none';
        if (monacoContainer) monacoContainer.style.display = '';
        previewBtn.textContent = t('editor.btn_preview');
        updateState({ editorPreviewMode: false });
      }
    }
    editorStatus.textContent = '';
  } catch (e) {
    editorPath.textContent = 'Error: ' + e;
    _originalContent = null;
    editorStatus.textContent = '';
  }

  updateUnsavedIndicator();
}

export function stripJsonComments(jsonc: string): string {
  return jsonc.replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => (g ? '' : m));
}

export async function handleEditorSave(): Promise<void> {
  const state = getState();
  const filePath = state.editorSelectedFile || getCurrentMonacoFilePath();
  if (!state.editorModId || !filePath) return;

  const editorStatus = editorDom.el('editor-status');
  const saveBtn = editorDom.el('editor-save-btn');

  const content = getMonacoContent();
  const isJson = filePath.endsWith('.json') || filePath.endsWith('.jsonc');

  if (isJson) {
    try {
      const cleanContent = filePath.endsWith('.jsonc') ? stripJsonComments(content) : content;
      JSON.parse(cleanContent);
    } catch (e) {
      editorStatus.textContent = t('editor.status_invalid_json', { error: (e as Error).message });
      return;
    }
  }

  saveBtn.disabled = true;
  editorStatus.textContent = t('editor.status_saving');

  try {
    const { suppressWatcherRefresh } = await import('./watcher');
    suppressWatcherRefresh(3000);

    await saveModFile(state.editorModId, filePath, content);
    _originalContent = content;
    updateUnsavedIndicator();
    editorStatus.textContent = t('editor.status_saved');
    bus.emit('editor:saved', { filePath });
    showToast(t('editor.toast_saved'), 'success');

    await refreshEditorFileTree(state.editorModId);

    setTimeout(() => {
      editorStatus.textContent = '';
    }, 2000);
  } catch (e) {
    editorStatus.textContent = String(e);
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    saveBtn.disabled = false;
  }
}

export async function handleEditorFormat(): Promise<void> {
  const editor = getMonacoEditor();
  if (!editor) return;

  const editorStatus = editorDom.el('editor-status');
  const state = getState();
  const isJsonc = state.editorSelectedFile?.endsWith('.jsonc');

  try {
    const action = editor.getAction('editor.action.formatDocument');
    if (action) {
      await action.run();
      editorStatus.textContent = isJsonc ? t('editor.status_formatted_clean') : t('editor.status_formatted');
      updateUnsavedIndicator();
      setTimeout(() => {
        editorStatus.textContent = '';
      }, 2000);
    } else {
      // Fallback JSON format
      const raw = getMonacoContent();
      const clean = isJsonc ? stripJsonComments(raw) : raw;
      const parsed = JSON.parse(clean);
      editor.setValue(JSON.stringify(parsed, null, 2));
      editorStatus.textContent = isJsonc ? t('editor.status_formatted_clean') : t('editor.status_formatted');
      updateUnsavedIndicator();
      setTimeout(() => {
        editorStatus.textContent = '';
      }, 2000);
    }
  } catch (e) {
    editorStatus.textContent = t('editor.status_invalid_json', { error: (e as Error).message });
  }
}

export async function handleEditorPreview(): Promise<void> {
  const state = getState();
  const monacoContainer = editorDom.elMaybe('editor-monaco-container');
  const preview = editorDom.el('editor-preview');
  const previewBtn = editorDom.el('editor-preview-btn');
  const mode = state.editorPreviewMode;

  if (mode) {
    preview.style.display = 'none';
    if (monacoContainer) monacoContainer.style.display = '';
    previewBtn.textContent = t('editor.btn_preview');
    previewBtn.classList.remove('active');
    updateState({ editorPreviewMode: false });
  } else {
    const content = getMonacoContent();
    preview.innerHTML = await marked.parse(content);
    preview.style.display = 'block';
    if (monacoContainer) monacoContainer.style.display = 'none';
    previewBtn.textContent = t('editor.btn_edit');
    previewBtn.classList.add('active');
    updateState({ editorPreviewMode: true });
  }
}

export async function loadEditorData(modId: string): Promise<void> {
  const state = getState();
  const mod = state.allMods.find((m) => m.id === modId);

  const editorModSelect = editorDom.el('editor-mod-select');
  const editorFileTree = editorDom.el('editor-file-tree');
  const editorPath = editorDom.el('editor-file-path');
  const editorStatus = editorDom.el('editor-status');

  editorPath.textContent = '';
  editorStatus.textContent = '';
  _originalContent = null;

  const nameEl = editorDom.elMaybe('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';

  if (mod) {
    editorModSelect.value = modId;
    try {
      const files = await listModFiles(modId);
      updateState({ editorFiles: files, editorSelectedFile: null });
      renderFileTree(files);
    } catch (e) {
      editorFileTree.innerHTML = '<div class="editor-file-error">Error loading files</div>';
    }
  }
}

export { confirmDiscardOrSave };
