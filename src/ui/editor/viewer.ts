import { readModFile, saveModFile, listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { escapeHtml, formatBytes } from '../../utils/helpers';
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
import { triggerWorkspaceScan, loadWorkspaceDiagnosticsForMod } from './monaco/problemsPanel';
import { initEditorStatusBar, updateStatusBarModInfo, setStatusBarMode } from './monaco/statusBar';
import { renderPakExplorer } from './pakExplorer';
import { renderInlineUAssetInspector } from './uassetInline';

export interface FileBuffer {
  current: string;
  original: string;
  isDirty: boolean;
}

export const _fileBufferCache: Map<string, FileBuffer> = new Map();

export function getDirtyBufferCount(): number {
  let count = 0;
  for (const buf of _fileBufferCache.values()) {
    if (buf.isDirty) count++;
  }
  return count;
}

export function clearBufferCache(): void {
  _fileBufferCache.clear();
  updateUnsavedIndicator();
}

export let _originalContent: string | null = null;
export const _lastFilePerMod: Record<string, string> = {};

export function clearOriginalContent(): void {
  _originalContent = null;
  updateUnsavedIndicator();
}

export function clearEditorContent(): void {
  _originalContent = null;
  clearBufferCache();
  const editorPath = editorDom.elMaybe('editor-file-path');
  if (editorPath) editorPath.textContent = '';
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

  setMonacoFile('empty.txt', '');
  updateUnsavedIndicator();
}

export function updateUnsavedIndicator(): void {
  const currentText = getMonacoContent();
  const state = getState();
  const selectedPath = state.editorSelectedFile || getCurrentMonacoFilePath();

  if (selectedPath && _originalContent !== null) {
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    const isDirty = normalize(currentText) !== normalize(_originalContent);
    _fileBufferCache.set(selectedPath, {
      current: currentText,
      original: _originalContent,
      isDirty,
    });
  }

  // Update tree dirty classes for all cached files
  for (const [path, buf] of _fileBufferCache.entries()) {
    const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(path)}"]`);
    if (item) {
      item.classList.toggle('dirty', buf.isDirty);
    }
  }

  const dirtyCount = getDirtyBufferCount();
  const saveBtn = editorDom.elMaybe('editor-save-btn');
  const saveBtnText = document.getElementById('editor-save-btn-text');
  const revertBtn = editorDom.elMaybe('editor-revert-btn');
  const revertBtnText = document.getElementById('editor-revert-btn-text');

  if (dirtyCount > 0) {
    if (saveBtn) {
      saveBtn.style.display = 'inline-flex';
      saveBtn.classList.add('dirty');
      if (saveBtnText) {
        saveBtnText.textContent = dirtyCount > 1
          ? (t('editor.btn_save_all', { count: dirtyCount }) || `Save All (${dirtyCount})`)
          : (t('editor.btn_save') || 'Save');
      }
    }
    if (revertBtn) {
      revertBtn.style.display = 'inline-flex';
      if (revertBtnText) {
        revertBtnText.textContent = dirtyCount > 1
          ? (t('editor.btn_revert_all', { count: dirtyCount }) || `Revert All (${dirtyCount})`)
          : (t('editor.btn_revert') || 'Revert');
      }
    }
  } else {
    if (saveBtn) {
      saveBtn.style.display = 'none';
      saveBtn.classList.remove('dirty');
    }
    if (revertBtn) {
      revertBtn.style.display = 'none';
    }
  }
}

export function renderEditorBreadcrumbs(modName: string, filePath: string): string {
  const normalized = filePath.replace(/\\/g, '/').replace(/^\/+/, '');
  const parts = normalized.split('/').filter(Boolean);
  if (parts.length === 0) {
    return `<div class="editor-bc-crumb"><span class="editor-bc-mod">📦 ${escapeHtml(modName)}</span></div>`;
  }
  const fileName = parts.pop()!;
  const isPak = fileName.toLowerCase().endsWith('.pak');
  const cleanFileName = isPak ? fileName.replace(/^\[Pak\]\s*/i, '') : fileName;
  const fileIcon = isPak ? '📦' : '📄';
  const segmentsHtml = [
    `<span class="editor-bc-mod">📦 ${escapeHtml(modName)}</span>`,
    ...parts.map(p => `<span class="editor-bc-sep">›</span><span class="editor-bc-part">📁 ${escapeHtml(p)}</span>`),
    `<span class="editor-bc-sep">›</span><span class="editor-bc-file">${fileIcon} ${escapeHtml(cleanFileName)}</span>`,
  ].join('');
  return `<div class="editor-bc-crumb">${segmentsHtml}</div>`;
}

export async function loadFileContent(filePath: string, lineNumber?: number): Promise<void> {
  const state = getState();
  if (!state.editorModId) return;

  // Synchronize previous active file into buffer cache before switching files
  const prevPath = state.editorSelectedFile || getCurrentMonacoFilePath();
  if (prevPath && prevPath !== filePath && _originalContent !== null) {
    const currentText = getMonacoContent();
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    const isDirty = normalize(currentText) !== normalize(_originalContent);
    _fileBufferCache.set(prevPath, {
      current: currentText,
      original: _originalContent,
      isDirty,
    });
  }

  const editorPath = editorDom.el('editor-file-path');
  const formatBtn = editorDom.elMaybe('editor-format-btn');
  const previewBtn = editorDom.elMaybe('editor-preview-btn');
  const diffBtn = editorDom.elMaybe('editor-diff-btn');
  const restoreBtn = editorDom.elMaybe('editor-restore-btn');
  const preview = editorDom.elMaybe('editor-preview');
  const monacoContainer = editorDom.elMaybe('editor-monaco-container');

  updateState({ editorPreviewMode: false, editorSelectedFile: filePath });
  if (state.editorModId) {
    _lastFilePerMod[state.editorModId] = filePath;
  }

  // Ensure file tree selection reflects the loaded file
  const tree = editorDom.elMaybe('editor-file-tree');
  if (tree) {
    const currentActive = tree.querySelector('.editor-file-item.selected');
    if (!currentActive || currentActive.getAttribute('data-path') !== filePath) {
      tree.querySelectorAll('.editor-file-item').forEach(el => el.classList.remove('selected'));
      try {
        const targetItem = tree.querySelector(`.editor-file-item[data-path="${CSS.escape(filePath)}"]`);
        if (targetItem) {
          targetItem.classList.add('selected');
        }
      } catch { }
    }
  }

  if (preview) preview.style.display = 'none';
  if (monacoContainer) monacoContainer.style.display = '';
  if (previewBtn) {
    previewBtn.style.display = 'none';
    previewBtn.textContent = 'Preview';
  }

  const isBak = filePath.toLowerCase().includes('.bak');
  if (diffBtn) diffBtn.style.display = isBak ? '' : 'none';
  if (restoreBtn) restoreBtn.style.display = isBak ? '' : 'none';

  const currentMod = state.allMods.find(m => m.id === state.editorModId);
  const modDisplayName = currentMod?.name || state.editorModId;

  try {
    const cached = _fileBufferCache.get(filePath);
    if (cached) {
      editorPath.innerHTML = renderEditorBreadcrumbs(modDisplayName, filePath);
      _originalContent = cached.original;
      setMonacoFile(filePath, cached.current);
      setStatusBarMode('code');
    } else {
      const result = await readModFile(state.editorModId, filePath);
      const confType = (result.configType || '').toLowerCase();
      const isUasset = ['uasset', 'uexp', 'ubulk', 'uptnl'].includes(confType);
      const isPak = confType === 'pak';
      const isBinary = result.isBinary || isUasset || isPak;

      if (isBinary) {
        editorPath.innerHTML = renderEditorBreadcrumbs(modDisplayName, result.path || filePath);
        if (preview) {
          preview.style.display = 'block';
          const assetName = (result.path || filePath).replace(/^.*[/\\]/, '');

          if (isPak) {
            const cleanPakName = assetName.replace(/^\[Pak\]\s*/i, '');
            const rawPakPath = result.path || filePath;
            const loadPakView = () => {
              renderPakExplorer(preview, {
                modId: state.editorModId || '',
                pakFileName: cleanPakName,
                pakPath: rawPakPath,
                fileSize: result.fileSize,
                onInspectAsset: (assetInternalPath: string) => {
                  renderInlineUAssetInspector(preview, {
                    modId: state.editorModId || null,
                    pakPath: rawPakPath,
                    assetInternalPath,
                    onBack: () => loadPakView(),
                  });
                },
              });
            };
            loadPakView();
          } else if (isUasset) {
            renderInlineUAssetInspector(preview, {
              modId: state.editorModId || null,
              pakPath: null,
              assetInternalPath: filePath,
            });
            setStatusBarMode('binary', {
              size: result.fileSize ? formatBytes(result.fileSize) : undefined,
              lang: 'Asset',
            });
          } else {
            const fileSizeFormatted = result.fileSize ? formatBytes(result.fileSize) : '';
            preview.innerHTML = `
              <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;padding:40px 20px;box-sizing:border-box;background:var(--bg-secondary);text-align:center;">
                <div style="font-size:48px;margin-bottom:12px;opacity:0.85;">⚡</div>
                <div style="font-size:16px;font-weight:700;color:var(--text-primary);margin-bottom:6px;">${escapeHtml(assetName)}</div>
                <div style="font-size:12px;color:var(--text-secondary);margin-bottom:4px;">${escapeHtml(t('editor.binary_asset_title') || 'Unreal Engine Binary Asset')}</div>
                <div style="font-size:11px;color:var(--text-muted);margin-bottom:18px;max-width:440px;line-height:1.5;">${escapeHtml(t('editor.binary_asset_desc') || 'Binary compiled asset files (.uasset/.pak) cannot be edited directly as plain text.')}${fileSizeFormatted ? ` (${fileSizeFormatted})` : ''}</div>
              </div>`;
            setStatusBarMode('binary', { size: fileSizeFormatted, lang: 'Binary' });
          }
        }
        if (monacoContainer) monacoContainer.style.display = 'none';
        _originalContent = null;
        if (formatBtn) formatBtn.style.display = 'none';
        if (previewBtn) previewBtn.style.display = 'none';
        if (diffBtn) diffBtn.style.display = 'none';
        if (restoreBtn) restoreBtn.style.display = 'none';
        return;
      } else if (!result.content) {
        editorPath.innerHTML = `<span class="editor-bc-empty">${escapeHtml(t('editor.no_content_available'))}</span>`;
        _originalContent = null;
        if (monacoContainer) monacoContainer.style.display = 'none';
        if (formatBtn) formatBtn.style.display = 'none';
        if (diffBtn) diffBtn.style.display = 'none';
        if (restoreBtn) restoreBtn.style.display = 'none';
      } else if (result.configType === 'image') {
        editorPath.innerHTML = renderEditorBreadcrumbs(modDisplayName, result.path || filePath);
        if (preview) {
          preview.innerHTML = `
            <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;padding:20px;box-sizing:border-box;background:var(--bg-secondary);">
              <img src="${result.content}" style="max-width:100%;max-height:80vh;object-fit:contain;border-radius:4px;box-shadow:0 4px 16px rgba(0,0,0,0.4);" />
              <div style="margin-top:12px;font-size:11px;color:var(--text-muted);">${escapeHtml(filePath)}</div>
            </div>`;
          preview.style.display = 'block';
        }
        if (monacoContainer) monacoContainer.style.display = 'none';
        _originalContent = null;
        if (formatBtn) formatBtn.style.display = 'none';
        if (previewBtn) previewBtn.style.display = 'none';
        if (diffBtn) diffBtn.style.display = 'none';
        if (restoreBtn) restoreBtn.style.display = 'none';
        setStatusBarMode('binary', {
          size: result.fileSize ? formatBytes(result.fileSize) : undefined,
          lang: 'Image',
        });
        return;
      } else {
        editorPath.innerHTML = renderEditorBreadcrumbs(modDisplayName, result.path || filePath);
        _originalContent = result.content;
        setMonacoFile(filePath, result.content);
        setStatusBarMode('code');
        _fileBufferCache.set(filePath, {
          current: result.content,
          original: result.content,
          isDirty: false,
        });
      }
    }

    if (lineNumber && lineNumber > 0) {
      const { jumpToLineInEditor } = await import('./keybindings');
      jumpToLineInEditor(lineNumber);
    }
  } catch (err) {
    editorPath.textContent = 'Error loading file content';
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

  const saveBtn = editorDom.elMaybe('editor-save-btn');
  const dirtyEntries = Array.from(_fileBufferCache.entries()).filter(([_, b]) => b.isDirty);

  // If active file has edits in Monaco, sync it into dirtyEntries
  const currentContent = getMonacoContent();
  if (_originalContent !== null) {
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    const isCurDirty = normalize(currentContent) !== normalize(_originalContent);
    _fileBufferCache.set(filePath, {
      current: currentContent,
      original: _originalContent,
      isDirty: isCurDirty,
    });
  }

  const updatedDirty = Array.from(_fileBufferCache.entries()).filter(([_, b]) => b.isDirty);

  if (updatedDirty.length > 1) {
    // Multi-file batch save (Save All)
    if (saveBtn) saveBtn.disabled = true;
    try {
      const { suppressWatcherRefresh } = await import('./watcher');
      suppressWatcherRefresh(3000);

      // Validate JSON files first
      for (const [p, buf] of updatedDirty) {
        if (p.endsWith('.json') || p.endsWith('.jsonc')) {
          try {
            const clean = p.endsWith('.jsonc') ? stripJsonComments(buf.current) : buf.current;
            JSON.parse(clean);
          } catch (e) {
            showToast(`${p}: ${t('editor.status_invalid_json', { error: (e as Error).message })}`, 'error');
            if (saveBtn) saveBtn.disabled = false;
            return;
          }
        }
      }

      for (const [p, buf] of updatedDirty) {
        await saveModFile(state.editorModId, p, buf.current);
        buf.original = buf.current;
        buf.isDirty = false;
        bus.emit('editor:saved', { filePath: p });
      }

      _originalContent = getMonacoContent();
      updateUnsavedIndicator();
      showToast(t('editor.toast_saved_all', { count: updatedDirty.length }) || `Saved ${updatedDirty.length} files`, 'success');
      await refreshEditorFileTree(state.editorModId);
    } catch (e) {
      showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    } finally {
      if (saveBtn) saveBtn.disabled = false;
    }
    return;
  }

  // Single file save
  const content = currentContent;
  const isJson = filePath.endsWith('.json') || filePath.endsWith('.jsonc');

  if (isJson) {
    try {
      const cleanContent = filePath.endsWith('.jsonc') ? stripJsonComments(content) : content;
      JSON.parse(cleanContent);
    } catch (e) {
      showToast(t('editor.status_invalid_json', { error: (e as Error).message }), 'error');
      return;
    }
  }

  if (saveBtn) saveBtn.disabled = true;

  try {
    const { suppressWatcherRefresh } = await import('./watcher');
    suppressWatcherRefresh(3000);

    await saveModFile(state.editorModId, filePath, content);
    _originalContent = content;
    if (_fileBufferCache.has(filePath)) {
      const b = _fileBufferCache.get(filePath)!;
      b.original = content;
      b.current = content;
      b.isDirty = false;
    }
    updateUnsavedIndicator();
    bus.emit('editor:saved', { filePath });
    showToast(t('editor.toast_saved'), 'success');

    await refreshEditorFileTree(state.editorModId);
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    if (saveBtn) saveBtn.disabled = false;
  }
}

export async function handleEditorRevert(): Promise<void> {
  const state = getState();
  const currentPath = state.editorSelectedFile || getCurrentMonacoFilePath();

  // Flush active Monaco buffer into cache first
  if (currentPath && _originalContent !== null) {
    const currentText = getMonacoContent();
    const normalize = (str: string) => str.replace(/\r\n/g, '\n');
    const isCurDirty = normalize(currentText) !== normalize(_originalContent);
    _fileBufferCache.set(currentPath, {
      current: currentText,
      original: _originalContent,
      isDirty: isCurDirty,
    });
  }

  const updatedDirty = Array.from(_fileBufferCache.entries()).filter(([_, b]) => b.isDirty);
  if (updatedDirty.length === 0) return;

  const { showConfirm } = await import('../confirm');

  if (updatedDirty.length > 1) {
    const title = t('editor.confirm_revert_all_title') || 'Discard All Unsaved Changes?';
    const body = (t('editor.confirm_revert_all_body') || 'Are you sure you want to discard unsaved changes across **{count} files**? All modifications will be lost.').replace('{count}', String(updatedDirty.length));
    const confirmed = await showConfirm(title, body);
    if (!confirmed) return;

    for (const [_, buf] of updatedDirty) {
      buf.current = buf.original;
      buf.isDirty = false;
    }

    if (currentPath && _fileBufferCache.has(currentPath)) {
      const activeBuf = _fileBufferCache.get(currentPath)!;
      _originalContent = activeBuf.original;
      setMonacoFile(currentPath, activeBuf.original);
    }

    updateUnsavedIndicator();
    showToast(t('editor.toast_reverted_all', { count: updatedDirty.length }) || `Reverted ${updatedDirty.length} files`, 'info');
  } else {
    const [path, buf] = updatedDirty[0];
    const fileName = path.replace(/^.*[/\\]/, '');
    const title = t('editor.confirm_revert_title') || 'Discard Unsaved Changes?';
    const body = (t('editor.confirm_revert_body') || 'Are you sure you want to discard unsaved changes in **{file}**? All modifications will be lost.').replace('{file}', fileName);
    const confirmed = await showConfirm(title, body);
    if (!confirmed) return;

    buf.current = buf.original;
    buf.isDirty = false;

    if (currentPath === path) {
      _originalContent = buf.original;
      setMonacoFile(path, buf.original);
    }

    updateUnsavedIndicator();
    showToast(t('editor.toast_reverted') || 'Changes discarded', 'info');
  }
}

export async function handleEditorFormat(): Promise<void> {
  const editor = getMonacoEditor();
  if (!editor) return;

  const state = getState();
  const isJsonc = state.editorSelectedFile?.endsWith('.jsonc');

  try {
    const action = editor.getAction('editor.action.formatDocument');
    if (action) {
      await action.run();
      updateUnsavedIndicator();
    } else {
      // Fallback JSON format
      const raw = getMonacoContent();
      const clean = isJsonc ? stripJsonComments(raw) : raw;
      const parsed = JSON.parse(clean);
      editor.setValue(JSON.stringify(parsed, null, 2));
      updateUnsavedIndicator();
    }
  } catch (e) {
    showToast(t('editor.status_invalid_json', { error: (e as Error).message }), 'error');
  }
}

export async function handleEditorPreview(): Promise<void> {
  const state = getState();
  const monacoContainer = editorDom.elMaybe('editor-monaco-container');
  const preview = editorDom.elMaybe('editor-preview');
  const previewBtn = editorDom.elMaybe('editor-preview-btn');
  const mode = state.editorPreviewMode;

  if (!preview) return;

  if (mode) {
    preview.style.display = 'none';
    if (monacoContainer) monacoContainer.style.display = '';
    if (previewBtn) {
      previewBtn.textContent = t('editor.btn_preview');
      previewBtn.classList.remove('active');
    }
    updateState({ editorPreviewMode: false });
  } else {
    const content = getMonacoContent();
    preview.innerHTML = await marked.parse(content);
    preview.style.display = 'block';
    if (monacoContainer) monacoContainer.style.display = 'none';
    if (previewBtn) {
      previewBtn.textContent = t('editor.btn_edit');
      previewBtn.classList.add('active');
    }
    updateState({ editorPreviewMode: true });
  }
}

export async function loadEditorData(modId: string): Promise<void> {
  const state = getState();
  const mod = state.allMods.find((m) => m.id === modId);

  const editorFileTree = editorDom.el('editor-file-tree');
  const editorPath = editorDom.el('editor-file-path');

  editorPath.textContent = '';
  _originalContent = null;

  initEditorStatusBar();
  const nameEl = editorDom.elMaybe('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';
  updateStatusBarModInfo(mod?.name || '');
  loadWorkspaceDiagnosticsForMod(modId);

  if (mod) {
    try {
      const files = await listModFiles(modId);
      updateState({ editorFiles: files, editorSelectedFile: null });
      renderFileTree(files);
      setTimeout(() => {
        triggerWorkspaceScan(false).catch(() => {});
      }, 300);
    } catch (e) {
      editorFileTree.innerHTML = '<div class="editor-file-error">Error loading files</div>';
    }
  }
}

export { confirmDiscardOrSave };
