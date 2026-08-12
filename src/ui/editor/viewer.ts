import { readModFile, saveModFile, listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { highlightText } from '../../utils/syntax';
import { marked } from 'marked';
import { confirmDiscardOrSave } from './unsaved';
import { renderFileTree } from './tree';
import { resetFindMatches, getFindMatchesText } from './search';

export let _originalContent: string | null = null;
export const _lastFilePerMod: Record<string, string> = {};

export function clearOriginalContent(): void {
  _originalContent = null;
}

export function syncHighlight(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const codeEl = document.getElementById('editor-highlight-code')!;
  const state = getState();
  const ext = state.editorSelectedFile ? state.editorSelectedFile.split('.').pop() || '' : '';
  const text = editorContent.value;

  const gutter = document.getElementById('editor-gutter');
  if (gutter) {
    const lines = text.split('\n').length;
    let html = '';
    for (let i = 1; i <= lines; i++) {
      html += `${i}<br/>`;
    }
    gutter.innerHTML = html;
  }

  // Handle Find/Search matches highlighting dynamically
  const processedText = getFindMatchesText(text);

  const highlighted = highlightText(processedText.text, ext);
  let result = highlighted;

  if (processedText.hasMatches) {
    for (let i = 0; i < processedText.count; i++) {
      result = result
        .replace('\x00START' + i + '\x00', '<mark class="find-match">')
        .replace('\x00END' + i + '\x00', '</mark>');
    }
  }

  codeEl.innerHTML = result + '\n';
}

export async function loadFileContent(filePath: string): Promise<void> {
  const state = getState();
  if (!state.editorModId) return;

  resetFindMatches();

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorPath = document.getElementById('editor-file-path')!;
  const editorStatus = document.getElementById('editor-status')!;
  const formatBtn = document.getElementById('editor-format-btn') as HTMLButtonElement;
  const previewBtn = document.getElementById('editor-preview-btn') as HTMLButtonElement;
  const preview = document.getElementById('editor-preview')!;
  const highlight = document.getElementById('editor-highlight')!;
  const gutter = document.getElementById('editor-gutter')!;

  editorContent.disabled = true;
  editorStatus.textContent = '';
  updateState({ editorPreviewMode: false });
  preview.style.display = 'none';
  highlight.style.display = '';
  editorContent.style.display = '';
  gutter.style.display = 'block';
  previewBtn.style.display = 'none';
  previewBtn.textContent = 'Preview';

  try {
    const result = await readModFile(state.editorModId, filePath);
    if (!result.content) {
      editorPath.textContent = 'No content available';
      editorContent.value = '';
      editorContent.disabled = true;
      formatBtn.style.display = 'none';
      _originalContent = null;
      gutter.style.display = 'none';
    } else if (result.configType === 'image') {
      editorPath.textContent = result.path || filePath;
      preview.innerHTML = `
        <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;padding:20px;box-sizing:border-box;background:var(--bg-secondary);">
          <img src="${result.content}" style="max-width:100%;max-height:80vh;object-fit:contain;border-radius:4px;box-shadow:0 4px 16px rgba(0,0,0,0.4);" />
          <div style="margin-top:12px;font-size:11px;color:var(--text-muted);">${escapeHtml(filePath)}</div>
        </div>`;
      preview.style.display = 'block';
      highlight.style.display = 'none';
      editorContent.style.display = 'none';
      gutter.style.display = 'none';
      editorContent.value = '';
      _originalContent = null;
      formatBtn.style.display = 'none';
      previewBtn.style.display = 'none';
      editorStatus.textContent = '';
      return;
    } else {
      editorPath.textContent = result.path || filePath;
      editorContent.value = result.content;
      _originalContent = result.content;
      editorContent.disabled = false;
      formatBtn.style.display = result.configType === 'json' || result.configType === 'jsonc' ? '' : 'none';
      if (filePath.endsWith('.md')) {
        previewBtn.style.display = '';
        preview.style.display = 'none';
        highlight.style.display = '';
        editorContent.style.display = '';
        gutter.style.display = 'block';
        previewBtn.textContent = 'Preview';
        updateState({ editorPreviewMode: false });
      }
    }
    editorStatus.textContent = '';
  } catch (e) {
    editorPath.textContent = 'Error: ' + e;
    editorContent.value = '';
    editorContent.disabled = true;
    _originalContent = null;
    editorStatus.textContent = '';
  }
  syncHighlight();
}

export function stripJsonComments(jsonc: string): string {
  return jsonc.replace(/\\"|"(?:\\"|[^"])*"|(\/\/.*|\/\*[\s\S]*?\*\/)/g, (m, g) => g ? "" : m);
}

export async function handleEditorSave(): Promise<void> {
  const state = getState();
  if (!state.editorModId || !state.editorSelectedFile) return;
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorStatus = document.getElementById('editor-status')!;
  const saveBtn = document.getElementById('editor-save-btn') as HTMLButtonElement;

  const content = editorContent.value;
  const isJson = state.editorSelectedFile.endsWith('.json') || state.editorSelectedFile.endsWith('.jsonc');

  if (isJson) {
    try {
      const cleanContent = state.editorSelectedFile.endsWith('.jsonc')
        ? stripJsonComments(content)
        : content;
      JSON.parse(cleanContent);
    } catch (e) {
      editorStatus.textContent = 'Invalid JSON: ' + (e as Error).message;
      return;
    }
  }

  saveBtn.disabled = true;
  editorStatus.textContent = 'Saving...';

  try {
    await saveModFile(state.editorModId, state.editorSelectedFile, content);
    _originalContent = content;
    editorStatus.textContent = 'Saved!';
    showToast('File saved', 'success');

    setTimeout(() => { editorStatus.textContent = ''; }, 2000);
  } catch (e) {
    editorStatus.textContent = 'Error: ' + e;
    showToast('Failed to save: ' + e, 'error');
  } finally {
    saveBtn.disabled = false;
  }
}

export function handleEditorFormat(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorStatus = document.getElementById('editor-status')!;
  const state = getState();
  const isJsonc = state.editorSelectedFile?.endsWith('.jsonc');

  try {
    const raw = editorContent.value;
    const clean = isJsonc ? stripJsonComments(raw) : raw;
    const parsed = JSON.parse(clean);
    editorContent.value = JSON.stringify(parsed, null, 2);
    editorStatus.textContent = isJsonc ? 'Formatted (Comments removed)' : 'Formatted';
    syncHighlight();
    setTimeout(() => { editorStatus.textContent = ''; }, 2000);
  } catch (e) {
    editorStatus.textContent = 'Invalid JSON: ' + (e as Error).message;
  }
}

export async function handleEditorPreview(): Promise<void> {
  const state = getState();
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const highlight = document.getElementById('editor-highlight')!;
  const preview = document.getElementById('editor-preview')!;
  const previewBtn = document.getElementById('editor-preview-btn') as HTMLButtonElement;
  const gutter = document.getElementById('editor-gutter')!;
  const mode = state.editorPreviewMode;

  if (mode) {
    preview.style.display = 'none';
    highlight.style.display = '';
    editorContent.style.display = '';
    gutter.style.display = 'block';
    editorContent.disabled = false;
    previewBtn.textContent = 'Preview';
    previewBtn.classList.remove('active');
    updateState({ editorPreviewMode: false });
  } else {
    preview.innerHTML = await marked.parse(editorContent.value);
    preview.style.display = 'block';
    highlight.style.display = 'none';
    editorContent.style.display = 'none';
    gutter.style.display = 'none';
    previewBtn.textContent = 'Edit';
    previewBtn.classList.add('active');
    updateState({ editorPreviewMode: true });
  }
}

export async function loadEditorData(modId: string): Promise<void> {
  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);

  const editorModSelect = document.getElementById('editor-mod-select') as HTMLSelectElement;
  const editorFileTree = document.getElementById('editor-file-tree')!;
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorPath = document.getElementById('editor-file-path')!;
  const editorStatus = document.getElementById('editor-status')!;

  editorPath.textContent = '';
  editorStatus.textContent = '';
  editorContent.value = '';
  editorContent.disabled = true;
  _originalContent = null;

  const codeEl = document.getElementById('editor-highlight-code');
  if (codeEl) codeEl.innerHTML = '';

  const nameEl = document.getElementById('editor-current-mod-name');
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
