import { readModFile, saveModFile, listModFiles } from '../api';
import { getState, updateState } from '../state';
import { showToast } from './toast';
import { escapeHtml } from '../utils/helpers';
import { highlightText } from '../utils/syntax';
import { marked } from 'marked';

let findMatches: { index: number; length: number }[] = [];
let findCurrentMatch = -1;
export let _originalContent: string | null = null;

export function clearOriginalContent(): void {
  _originalContent = null;
}


// Remember last opened file for each mod (modId -> filePath)
const _lastFilePerMod: Record<string, string> = {};

export async function openConfigEditor(modId: string): Promise<void> {
  updateState({ activeTab: 'editor', editorModId: modId, editorSelectedFile: null });
  switchTab('editor');
  renderEditorModTree();
  await loadEditorData(modId);
  // Restore last opened file or open the first one
  const lastFile = _lastFilePerMod[modId];
  if (lastFile) {
    const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(lastFile)}"]`) as HTMLElement | null;
    if (item) { item.click(); return; }
  }
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) firstFile.click();
}

export function switchTab(tab: 'mods' | 'editor' | 'library'): void {
  updateState({ activeTab: tab });
  document.querySelectorAll('.sidebar-tab').forEach(b => b.classList.remove('active'));
  const tabBtn = document.querySelector(`.sidebar-tab[data-tab="${tab}"]`);
  if (tabBtn) tabBtn.classList.add('active');

  document.getElementById('mods-view')!.style.display = tab === 'mods' ? '' : 'none';
  document.getElementById('editor-view')!.style.display = tab === 'editor' ? 'flex' : 'none';
  const libView = document.getElementById('library-view');
  if (libView) libView.style.display = tab === 'library' ? 'flex' : 'none';

  if (tab === 'editor') renderEditorModTree();
}

export function populateEditorModSelect(): void {
  // Legacy: keep the hidden select in sync (used by handleEditorModChange)
  const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
  const state = getState();
  select.innerHTML = '<option value="">Select a mod...</option>' +
    state.allMods
      .filter(m => m.type !== 'pak' && m.type !== 'logicmods' && m.nexusAuthor !== 'UE4SS Native Mod')
      .map(m => `<option value="${m.id}">${escapeHtml(m.name)}</option>`).join('');
  renderEditorModTree();
}

export function renderEditorModTree(): void {
  const tree = document.getElementById('editor-mod-tree');
  if (!tree) return;

  const state = getState();
  const currentModId = state.editorModId;

  // Filter out native mods and pak/logicmods
  const editableMods = state.allMods.filter(m =>
    m.type !== 'pak' &&
    m.type !== 'logicmods' &&
    m.nexusAuthor !== 'UE4SS Native Mod'
  );

  const ue4ssMods = editableMods.filter(m => m.type === 'ue4ss');
  const palSchemaMods = editableMods.filter(m => m.type === 'palschema');

  function renderSection(label: string, mods: typeof editableMods, sectionId: string): string {
    if (mods.length === 0) return '';
    const items = mods.map(m => `
      <div class="editor-mod-item${m.id === currentModId ? ' active' : ''}" data-mod-id="${m.id}" title="${escapeHtml(m.name)}">
        <span class="editor-mod-item-icon">📄</span>
        <span style="overflow:hidden;text-overflow:ellipsis;">${escapeHtml(m.name)}</span>
      </div>
    `).join('');
    return `
      <div class="editor-mod-section" id="editor-section-${sectionId}">
        <div class="editor-mod-section-header" data-section="${sectionId}">
          <span class="editor-mod-section-chevron">▾</span>
          ${label}
          <span style="margin-left:auto;font-size:9px;opacity:0.5">${mods.length}</span>
        </div>
        <div class="editor-mod-section-list">${items}</div>
      </div>
    `;
  }

  tree.innerHTML =
    renderSection('UE4SS', ue4ssMods, 'ue4ss') +
    renderSection('PalSchema', palSchemaMods, 'palschema');

  if (editableMods.length === 0) {
    tree.innerHTML = '<div style="padding:12px;font-size:11px;color:var(--text-muted)">No editable mods</div>';
    return;
  }

  // Section collapse toggle
  tree.querySelectorAll('.editor-mod-section-header').forEach(header => {
    header.addEventListener('click', () => {
      const section = header.closest('.editor-mod-section')!;
      section.classList.toggle('collapsed');
    });
  });

  // Mod item click
  tree.querySelectorAll('.editor-mod-item').forEach(item => {
    item.addEventListener('click', async () => {
      const modId = (item as HTMLElement).dataset.modId!;
      await switchEditorMod(modId);
    });
  });
}

async function switchEditorMod(modId: string): Promise<void> {
  const state = getState();
  const currentModId = state.editorModId;
  if (currentModId === modId) return;

  // Save last file for current mod before switching
  if (currentModId && state.editorSelectedFile) {
    _lastFilePerMod[currentModId] = state.editorSelectedFile;
  }

  const proceed = await confirmDiscardOrSave();
  if (!proceed) return;

  updateState({ editorModId: modId, editorSelectedFile: null });

  // Update hidden select value (legacy compat)
  const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
  if (select) select.value = modId;

  // Update toolbar mod name
  const mod = getState().allMods.find(m => m.id === modId);
  const nameEl = document.getElementById('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';

  // Update active class in tree WITHOUT re-rendering (avoids flash)
  document.querySelectorAll('.editor-mod-item').forEach(el => {
    el.classList.toggle('active', (el as HTMLElement).dataset.modId === modId);
  });

  await loadEditorData(modId);

  // Restore last opened file or open first
  const lastFile = _lastFilePerMod[modId];
  if (lastFile) {
    const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(lastFile)}"]`) as HTMLElement | null;
    if (item) { item.click(); return; }
  }
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) firstFile.click();
}

async function loadEditorData(modId: string): Promise<void> {
  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);

  const editorModSelect = document.getElementById('editor-mod-select') as HTMLSelectElement;
  const editorFileTree = document.getElementById('editor-file-tree')!;
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorPath = document.getElementById('editor-file-path')!;
  const editorStatus = document.getElementById('editor-status')!;

  // Clear content area immediately (no layout impact — same size)
  editorPath.textContent = '';
  editorStatus.textContent = '';
  editorContent.value = '';
  editorContent.disabled = true;
  _originalContent = null;
  // Clear highlight layer so old code doesn't show during async fetch
  const codeEl = document.getElementById('editor-highlight-code');
  if (codeEl) codeEl.innerHTML = '';

  // Update toolbar mod name display
  const nameEl = document.getElementById('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';

  if (mod) {
    editorModSelect.value = modId;
    try {
      // Fetch files BEFORE touching the file tree DOM to avoid the collapse→expand jump
      const files = await listModFiles(modId);
      updateState({ editorFiles: files, editorSelectedFile: null });
      // Now swap the file tree in one synchronous paint — no intermediate empty state
      renderFileTree(files);
    } catch (e) {
      editorFileTree.innerHTML = '<div class="editor-file-error">Error loading files</div>';
    }
  }
}

interface FileTreeNode {
  name: string;
  path: string;
  isFolder: boolean;
  children: Map<string, FileTreeNode>;
}

function buildFileTree(files: string[]): FileTreeNode {
  const root: FileTreeNode = {
    name: '',
    path: '',
    isFolder: true,
    children: new Map(),
  };

  const filtered = files.filter(f => {
    const name = f.replace(/^.*[/\\]/, '');
    return !name.startsWith('.') && name.toLowerCase() !== 'enabled.txt';
  });

  for (const f of filtered) {
    const parts = f.split(/[/\\]/);
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      if (!current.children.has(part)) {
        current.children.set(part, {
          name: part,
          path: parts.slice(0, i + 1).join('/'),
          isFolder: !isLast,
          children: new Map(),
        });
      }
      current = current.children.get(part)!;
    }
  }

  return root;
}

function getFileIcon(ext: string): string {
  const lower = ext.toLowerCase();
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'ico', 'bmp', 'svg'].includes(lower)) return 'IMG';
  if (lower === 'json' || lower === 'jsonc') return '{ }';
  if (lower === 'lua') return 'LUA';
  if (lower === 'txt' || lower === 'md') return 'TXT';
  if (lower === 'cfg' || lower === 'ini') return 'CFG';
  if (lower === 'py') return 'PY';
  if (lower === 'xml' || lower === 'html') return 'XML';
  return '--';
}

function renderNodeHTML(node: FileTreeNode): string {
  const childrenArray = Array.from(node.children.values()).sort((a, b) => {
    if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  return childrenArray.map(child => {
    if (child.isFolder) {
      return `
      <div class="editor-tree-folder">
        <div class="editor-tree-folder-header">
          <span class="editor-tree-chevron">▾</span>
          <span class="editor-file-icon" style="color:var(--accent);">📁</span>
          <span class="editor-folder-name">${escapeHtml(child.name)}</span>
        </div>
        <div class="editor-tree-folder-children">
          ${renderNodeHTML(child)}
        </div>
      </div>`;
    } else {
      const ext = child.name.split('.').pop() || '';
      const icon = getFileIcon(ext);
      return `
      <div class="editor-file-item" data-path="${escapeHtml(child.path)}" data-ext="${escapeHtml(ext)}">
        <span class="editor-file-icon">${icon}</span>
        <span class="editor-file-name" title="${escapeHtml(child.name)}">${escapeHtml(child.name)}</span>
      </div>`;
    }
  }).join('');
}

export function renderFileTree(files: string[]): void {
  const tree = document.getElementById('editor-file-tree')!;
  const rootNode = buildFileTree(files);

  if (rootNode.children.size === 0) {
    tree.innerHTML = '<div class="editor-file-empty">No editable files found</div>';
    return;
  }

  tree.innerHTML = renderNodeHTML(rootNode);

  // Folder toggle handlers
  tree.querySelectorAll('.editor-tree-folder-header').forEach(header => {
    header.addEventListener('click', (e) => {
      e.stopPropagation();
      const folder = header.closest('.editor-tree-folder')!;
      folder.classList.toggle('collapsed');
    });
  });

  // File item click handlers
  tree.querySelectorAll('.editor-file-item').forEach(item => {
    item.addEventListener('click', async (e) => {
      e.stopPropagation();
      const path = (item as HTMLElement).dataset.path!;
      const state = getState();
      if (state.editorSelectedFile === path) return;

      const proceed = await confirmDiscardOrSave();
      if (!proceed) return;

      tree.querySelectorAll('.editor-file-item').forEach(el => el.classList.remove('selected'));
      item.classList.add('selected');
      updateState({ editorSelectedFile: path });
      // Remember last opened file for this mod
      if (state.editorModId) _lastFilePerMod[state.editorModId] = path;
      await loadFileContent(path);
    });
  });
}

function syncHighlight(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const codeEl = document.getElementById('editor-highlight-code')!;
  const state = getState();
  const ext = state.editorSelectedFile ? state.editorSelectedFile.split('.').pop() || '' : '';
  let text = editorContent.value;

  if (findMatches.length > 0) {
    const parts: string[] = [];
    let lastEnd = 0;
    for (let i = 0; i < findMatches.length; i++) {
      const m = findMatches[i];
      parts.push(text.slice(lastEnd, m.index));
      parts.push('\x00START' + i + '\x00' + text.slice(m.index, m.index + m.length) + '\x00END' + i + '\x00');
      lastEnd = m.index + m.length;
    }
    parts.push(text.slice(lastEnd));
    text = parts.join('');
  }

  const highlighted = highlightText(text, ext);

  let result = highlighted;
  if (findMatches.length > 0) {
    for (let i = 0; i < findMatches.length; i++) {
      result = result
        .replace('\x00START' + i + '\x00', '<mark class="find-match"')
        .replace('\x00END' + i + '\x00', '</mark>');
    }
  }

  codeEl.innerHTML = result + '\n';
}

async function loadFileContent(filePath: string): Promise<void> {
  const state = getState();
  if (!state.editorModId) return;

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const editorPath = document.getElementById('editor-file-path')!;
  const editorStatus = document.getElementById('editor-status')!;
  const formatBtn = document.getElementById('editor-format-btn') as HTMLButtonElement;
  const previewBtn = document.getElementById('editor-preview-btn') as HTMLButtonElement;
  const preview = document.getElementById('editor-preview')!;
  const highlight = document.getElementById('editor-highlight')!;

  editorContent.disabled = true;
  editorStatus.textContent = '';
  updateState({ editorPreviewMode: false });
  preview.style.display = 'none';
  highlight.style.display = '';
  editorContent.style.display = '';
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
        preview.innerHTML = await marked.parse(result.content);
        preview.style.display = 'block';
        highlight.style.display = 'none';
        editorContent.style.display = 'none';
        previewBtn.textContent = 'Edit';
        updateState({ editorPreviewMode: true });
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
      JSON.parse(content);
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
  try {
    const parsed = JSON.parse(editorContent.value);
    editorContent.value = JSON.stringify(parsed, null, 2);
    editorStatus.textContent = 'Formatted';
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
  const mode = state.editorPreviewMode;

  if (mode) {
    preview.style.display = 'none';
    highlight.style.display = '';
    editorContent.style.display = '';
    editorContent.disabled = false;
    previewBtn.textContent = 'Preview';
    previewBtn.classList.remove('active');
    updateState({ editorPreviewMode: false });
  } else {
    preview.innerHTML = await marked.parse(editorContent.value);
    preview.style.display = 'block';
    highlight.style.display = 'none';
    editorContent.style.display = 'none';
    previewBtn.textContent = 'Edit';
    previewBtn.classList.add('active');
    updateState({ editorPreviewMode: true });
  }
}

function openFind(): void {
  const findBar = document.getElementById('editor-find-bar')!;
  const findInput = document.getElementById('editor-find-input') as HTMLInputElement;
  findBar.style.display = 'flex';
  findInput.value = '';
  findInput.focus();
  updateFindMatches();
}

export function closeFind(): void {
  const findBar = document.getElementById('editor-find-bar')!;
  findBar.style.display = 'none';
  clearFindHighlights();
}

function clearFindHighlights(): void {
  findMatches = [];
  findCurrentMatch = -1;
  document.getElementById('editor-find-count')!.textContent = '';
  syncHighlight();
}

function updateFindMatches(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const findInput = document.getElementById('editor-find-input') as HTMLInputElement;
  const countEl = document.getElementById('editor-find-count')!;
  const text = editorContent.value;
  const query = findInput.value;

  findMatches = [];
  findCurrentMatch = -1;

  if (!query) {
    countEl.textContent = '';
    syncHighlight();
    return;
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  let idx = 0;
  while ((idx = lowerText.indexOf(lowerQuery, idx)) !== -1) {
    findMatches.push({ index: idx, length: query.length });
    idx += query.length;
  }

  if (findMatches.length > 0) {
    findCurrentMatch = 0;
    scrollToMatch(0);
  }
  countEl.textContent = findMatches.length > 0
    ? `${findCurrentMatch + 1} of ${findMatches.length}`
    : 'No matches';
  syncHighlight();
}

function scrollToMatch(idx: number): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  if (idx < 0 || idx >= findMatches.length) return;
  findCurrentMatch = idx;
  const match = findMatches[idx];
  const text = editorContent.value;
  const before = text.substring(0, match.index);
  const lineNum = before.split('\n').length;
  const lines = text.split('\n');
  let pos = 0;
  for (let i = 0; i < lines.length; i++) {
    if (pos + lines[i].length >= match.index) {
      const lineStart = lines.slice(0, i).join('\n').length + (i > 0 ? 1 : 0);
      editorContent.focus();
      editorContent.setSelectionRange(match.index, match.index + match.length);
      const lineHeight = 20;
      editorContent.scrollTop = Math.max(0, (i - 3) * lineHeight);
      break;
    }
    pos += lines[i].length + 1;
  }
  document.getElementById('editor-find-count')!.textContent =
    `${findCurrentMatch + 1} of ${findMatches.length}`;
  syncHighlight();
}

function findNext(): void {
  if (findMatches.length === 0) return;
  const next = (findCurrentMatch + 1) % findMatches.length;
  scrollToMatch(next);
}

function findPrev(): void {
  if (findMatches.length === 0) return;
  const prev = (findCurrentMatch - 1 + findMatches.length) % findMatches.length;
  scrollToMatch(prev);
}

export function setupEditorFindHandlers(): void {
  document.getElementById('editor-find-input')!.addEventListener('input', updateFindMatches);
  document.getElementById('editor-find-next')!.addEventListener('click', findNext);
  document.getElementById('editor-find-prev')!.addEventListener('click', findPrev);
  document.getElementById('editor-find-close')!.addEventListener('click', closeFind);
  document.getElementById('editor-find-input')!.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.shiftKey ? findPrev() : findNext();
    }
    if (e.key === 'Escape') {
      closeFind();
    }
  });
}

export async function handleEditorModChange(): Promise<void> {
  const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
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
    loadEditorData(modId);
  }
}


export function setupEditorKeybindings(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const highlightEl = document.getElementById('editor-highlight')!;

  document.addEventListener('keydown', (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
      const editorView = document.getElementById('editor-view')!;
      if (editorView.style.display !== 'none') {
        e.preventDefault();
        openFind();
      }
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      const editorView = document.getElementById('editor-view')!;
      if (editorView.style.display !== 'none') {
        e.preventDefault();
        handleEditorSave();
      }
    }
  });

  editorContent.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = editorContent.selectionStart;
      const end = editorContent.selectionEnd;
      editorContent.value = editorContent.value.substring(0, start) + '  ' + editorContent.value.substring(end);
      editorContent.selectionStart = editorContent.selectionEnd = start + 2;
      syncHighlight();
    }
  });
  editorContent.addEventListener('input', () => {
    syncHighlight();
  });

  editorContent.addEventListener('scroll', () => {
    highlightEl.scrollTop = editorContent.scrollTop;
    highlightEl.scrollLeft = editorContent.scrollLeft;
  });
  highlightEl.addEventListener('scroll', () => {
    editorContent.scrollTop = highlightEl.scrollTop;
    editorContent.scrollLeft = highlightEl.scrollLeft;
  });

  document.getElementById('editor-preview-btn')!.addEventListener('click', handleEditorPreview);
}

export function hasUnsavedChanges(): boolean {
  if (_originalContent === null) return false;
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
  if (!editorContent) return false;
  
  const normalize = (str: string) => str.replace(/\r\n/g, '\n');
  return normalize(editorContent.value) !== normalize(_originalContent);
}


export async function confirmDiscardOrSave(): Promise<boolean> {
  if (!hasUnsavedChanges()) return true;

  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const current = editorContent.value;
  const original = _originalContent || '';

  const choice = await showUnsavedChangesModal(original, current);

  if (choice === 'save') {
    await handleEditorSave();
    _originalContent = null;
    return true;
  } else if (choice === 'discard') {
    _originalContent = null;
    return true;
  }

  return false;
}

function generateLineDiff(original: string, current: string): string {
  const origLines = original.split('\n');
  const currLines = current.split('\n');
  
  interface DiffItem {
    type: 'added' | 'deleted' | 'unchanged';
    text: string;
  }
  
  const rawDiff: DiffItem[] = [];
  let i = 0, j = 0;
  
  while (i < origLines.length || j < currLines.length) {
    if (i < origLines.length && j < currLines.length) {
      if (origLines[i] === currLines[j]) {
        rawDiff.push({ type: 'unchanged', text: origLines[i] });
        i++; j++;
      } else {
        let foundMatch = false;
        for (let look = 1; look < 5; look++) {
          if (i + look < origLines.length && origLines[i + look] === currLines[j]) {
            for (let d = 0; d < look; d++) {
              rawDiff.push({ type: 'deleted', text: origLines[i + d] });
            }
            i += look;
            foundMatch = true;
            break;
          }
          if (j + look < currLines.length && origLines[i] === currLines[j + look]) {
            for (let a = 0; a < look; a++) {
              rawDiff.push({ type: 'added', text: currLines[j + a] });
            }
            j += look;
            foundMatch = true;
            break;
          }
        }
        if (!foundMatch) {
          rawDiff.push({ type: 'deleted', text: origLines[i] });
          rawDiff.push({ type: 'added', text: currLines[j] });
          i++; j++;
        }
      }
    } else if (i < origLines.length) {
      rawDiff.push({ type: 'deleted', text: origLines[i] });
      i++;
    } else if (j < currLines.length) {
      rawDiff.push({ type: 'added', text: currLines[j] });
      j++;
    }
  }

  const contextSize = 2;
  const showFlags = new Array(rawDiff.length).fill(false);
  
  for (let k = 0; k < rawDiff.length; k++) {
    if (rawDiff[k].type !== 'unchanged') {
      showFlags[k] = true;
      for (let c = 1; c <= contextSize; c++) {
        if (k - c >= 0) showFlags[k - c] = true;
      }
      for (let c = 1; c <= contextSize; c++) {
        if (k + c < rawDiff.length) showFlags[k + c] = true;
      }
    }
  }

  const htmlLines: string[] = [];
  let inCollapse = false;
  let collapsedCount = 0;

  for (let k = 0; k < rawDiff.length; k++) {
    if (showFlags[k]) {
      if (inCollapse) {
        htmlLines.push(`
          <div class="diff-line-collapsed" style="color:var(--text-muted);font-family:monospace;padding:6px 12px;background:rgba(0,0,0,0.15);border-top:1px dashed var(--border);border-bottom:1px dashed var(--border);font-size:10px;text-align:center;user-select:none;">
            --- Colapsadas ${collapsedCount} líneas sin cambios ---
          </div>
        `);
        inCollapse = false;
        collapsedCount = 0;
      }

      const item = rawDiff[k];
      if (item.type === 'unchanged') {
        htmlLines.push(`<div class="diff-line unchanged" style="color:var(--text-muted);font-family:monospace;white-space:pre-wrap;padding:2px 8px;">  ${escapeHtml(item.text)}</div>`);
      } else if (item.type === 'deleted') {
        htmlLines.push(`<div class="diff-line deleted" style="background:rgba(232, 17, 35, 0.15);color:#f1707b;font-family:monospace;white-space:pre-wrap;padding:2px 8px;">- ${escapeHtml(item.text)}</div>`);
      } else if (item.type === 'added') {
        htmlLines.push(`<div class="diff-line added" style="background:rgba(16, 124, 65, 0.15);color:#57cf84;font-family:monospace;white-space:pre-wrap;padding:2px 8px;">+ ${escapeHtml(item.text)}</div>`);
      }
    } else {
      inCollapse = true;
      collapsedCount++;
    }
  }

  if (inCollapse) {
    htmlLines.push(`
      <div class="diff-line-collapsed" style="color:var(--text-muted);font-family:monospace;padding:6px 12px;background:rgba(0,0,0,0.15);border-top:1px dashed var(--border);border-bottom:1px dashed var(--border);font-size:10px;text-align:center;user-select:none;">
        --- Colapsadas ${collapsedCount} líneas sin cambios ---
      </div>
    `);
  }

  return `<div style="max-height:280px;overflow-y:auto;border:1px solid var(--border);border-radius:4px;background:var(--bg-secondary);padding:4px;font-size:11px;line-height:1.4;">${htmlLines.join('')}</div>`;
}


function showUnsavedChangesModal(original: string, current: string): Promise<'save' | 'discard' | 'cancel'> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '2000';
    overlay.style.position = 'fixed';
    overlay.style.top = '0';
    overlay.style.left = '0';
    overlay.style.right = '0';
    overlay.style.bottom = '0';
    overlay.style.background = 'rgba(0,0,0,0.6)';
    overlay.style.backdropFilter = 'blur(4px)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';
    
    const diffHtml = generateLineDiff(original, current);
    
    overlay.innerHTML = `
      <div class="modal" style="width: 750px; max-width: 90vw;">
        <div class="modal-header">
          <h3>Unsaved Changes</h3>
          <button class="modal-close-btn" id="unsaved-close-x">✕</button>
        </div>
        <div class="modal-body" style="gap:12px;padding:20px;">
          <div style="font-size:13px;color:var(--text-muted);">
            You have unsaved changes in this file. Review the changes below:
          </div>
          ${diffHtml}
        </div>
        <div class="modal-footer" style="padding:16px 20px;">
          <button id="unsaved-discard" class="btn-secondary" style="background:#a80000;color:white;border-color:#a80000;cursor:pointer;">Discard Changes</button>
          <button id="unsaved-cancel" class="btn-secondary" style="cursor:pointer;">Cancel</button>
          <button id="unsaved-save" class="btn-primary" style="cursor:pointer;">Save & Continue</button>
        </div>
      </div>
    `;
    
    document.body.appendChild(overlay);
    
    const cleanUp = () => {
      document.body.removeChild(overlay);
    };
    
    document.getElementById('unsaved-close-x')!.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    document.getElementById('unsaved-cancel')!.addEventListener('click', () => {
      cleanUp();
      resolve('cancel');
    });
    document.getElementById('unsaved-discard')!.addEventListener('click', () => {
      cleanUp();
      resolve('discard');
    });
    document.getElementById('unsaved-save')!.addEventListener('click', () => {
      cleanUp();
      resolve('save');
    });
  });
}

