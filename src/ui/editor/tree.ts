import { listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { showConfirm } from '../confirm';
import { confirmDiscardOrSave, _lastFilePerMod, loadFileContent, loadEditorData } from './viewer';

export function renderEditorModTree(): void {
  const tree = document.getElementById('editor-mod-tree');
  if (!tree) return;

  const state = getState();
  const currentModId = state.editorModId;

  const editableMods = state.allMods.filter(m =>
    m.type !== 'pak' &&
    m.type !== 'logicmods' &&
    m.nexusAuthor !== 'UE4SS Native Mod'
  );

  const ue4ssMods = editableMods.filter(m => m.type === 'ue4ss').sort((a, b) => a.name.localeCompare(b.name));
  const palSchemaMods = editableMods.filter(m => m.type === 'palschema').sort((a, b) => a.name.localeCompare(b.name));
  const hybridMods = editableMods.filter(m => m.type === 'hybrid').sort((a, b) => a.name.localeCompare(b.name));

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
    renderSection('PalSchema', palSchemaMods, 'palschema') +
    renderSection('Hybrid', hybridMods, 'hybrid');

  if (editableMods.length === 0) {
    tree.innerHTML = `<div style="padding:12px;font-size:11px;color:var(--text-muted)">${escapeHtml(t('editor.no_editable_mods'))}</div>`;
    return;
  }

  tree.querySelectorAll('.editor-mod-section-header').forEach(header => {
    header.addEventListener('click', () => {
      const section = header.closest('.editor-mod-section')!;
      section.classList.toggle('collapsed');
    });
  });

  tree.querySelectorAll('.editor-mod-item').forEach(item => {
    item.addEventListener('click', async () => {
      const modId = (item as HTMLElement).dataset.modId!;
      await switchEditorMod(modId);
    });
  });
}

export async function switchEditorMod(modId: string): Promise<void> {
  const state = getState();
  const currentModId = state.editorModId;
  if (currentModId === modId) return;

  if (currentModId && state.editorSelectedFile) {
    _lastFilePerMod[currentModId] = state.editorSelectedFile;
  }

  const proceed = await confirmDiscardOrSave();
  if (!proceed) return;

  updateState({ editorModId: modId, editorSelectedFile: null });

  const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
  if (select) select.value = modId;

  const mod = getState().allMods.find(m => m.id === modId);
  const nameEl = document.getElementById('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';

  document.querySelectorAll('.editor-mod-item').forEach(el => {
    el.classList.toggle('active', (el as HTMLElement).dataset.modId === modId);
  });

  await loadEditorData(modId);

  const lastFile = _lastFilePerMod[modId];
  if (lastFile) {
    const item = document.querySelector(`.editor-file-item[data-path="${CSS.escape(lastFile)}"]`) as HTMLElement | null;
    if (item) { item.click(); return; }
  }
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) firstFile.click();
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

const _collapsedFolders: Set<string> = new Set();

function getFileIcon(ext: string): string {
  const lower = ext.toLowerCase();
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'ico', 'bmp', 'svg'].includes(lower)) return 'IMG';
  if (lower === 'json' || lower === 'jsonc') return '{ }';
  if (lower === 'lua') return 'LUA';
  if (lower === 'txt' || lower === 'md') return 'TXT';
  if (lower === 'cfg' || lower === 'ini' || lower === 'conf') return 'CFG';
  if (lower === 'toml') return 'TOML';
  if (lower === 'yaml' || lower === 'yml') return 'YML';
  if (lower === 'py') return 'PY';
  if (lower === 'xml' || lower === 'html') return 'XML';
  if (lower.startsWith('bak')) return 'BAK';
  if (lower === 'log') return 'LOG';
  return '--';
}

function renderNodeHTML(node: FileTreeNode): string {
  const childrenArray = Array.from(node.children.values()).sort((a, b) => {
    if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  const state = getState();
  const currentSelected = state.editorSelectedFile;

  return childrenArray.map(child => {
    if (child.isFolder) {
      const isCollapsed = _collapsedFolders.has(child.path);
      return `
      <div class="editor-tree-folder${isCollapsed ? ' collapsed' : ''}" data-folder-path="${escapeHtml(child.path)}">
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
      const isSelected = currentSelected === child.path;
      const isBak = ext.toLowerCase().startsWith('bak');

      let actionButtons = '';
      if (isBak) {
        actionButtons += `
          <button class="editor-file-action-btn editor-file-diff-btn" data-path="${escapeHtml(child.path)}" title="${escapeHtml(t('editor.btn_diff') || 'Compare Diff')}">
            🔍
          </button>
        `;
      }
      actionButtons += `
        <button class="editor-file-action-btn editor-file-delete-btn" data-path="${escapeHtml(child.path)}" title="${escapeHtml(t('editor.btn_delete_file') || 'Delete File')}">
          🗑️
        </button>
      `;

      return `
      <div class="editor-file-item${isSelected ? ' selected' : ''}" data-path="${escapeHtml(child.path)}" data-ext="${escapeHtml(ext)}">
        <span class="editor-file-icon">${icon}</span>
        <span class="editor-file-name" title="${escapeHtml(child.name)}">${escapeHtml(child.name)}</span>
        <span class="editor-file-dirty-dot" title="${escapeHtml(t('editor.unsaved_changes') || 'Unsaved changes')}">●</span>
        <div class="editor-file-actions">
          ${actionButtons}
        </div>
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

  tree.querySelectorAll('.editor-tree-folder-header').forEach(header => {
    header.addEventListener('click', (e) => {
      e.stopPropagation();
      const folder = header.closest('.editor-tree-folder') as HTMLElement | null;
      if (folder) {
        folder.classList.toggle('collapsed');
        const folderPath = folder.dataset.folderPath;
        if (folderPath) {
          if (folder.classList.contains('collapsed')) {
            _collapsedFolders.add(folderPath);
          } else {
            _collapsedFolders.delete(folderPath);
          }
        }
      }
    });
  });

  // Diff Action Buttons
  tree.querySelectorAll('.editor-file-diff-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const path = (btn as HTMLElement).dataset.path;
      const state = getState();
      if (state.editorModId && path) {
        const { openEditorDiffModal } = await import('./diffModal');
        await openEditorDiffModal(state.editorModId, path);
      }
    });
  });

  // Delete Action Buttons
  tree.querySelectorAll('.editor-file-delete-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const path = (btn as HTMLElement).dataset.path;
      const state = getState();
      if (!state.editorModId || !path) return;

      const title = t('editor.confirm_delete_title') || 'Delete File';
      const body = (t('editor.confirm_delete_body') || 'Are you sure you want to permanently delete **{file}**?').replace('{file}', path);

      const confirmed = await showConfirm(title, body);
      if (!confirmed) return;

      try {
        const { deleteModFile } = await import('../../api');
        const { showToast } = await import('../toast');
        const res = await deleteModFile(state.editorModId, path);
        if (res.success) {
          showToast(t('editor.toast_deleted') || 'File deleted successfully', 'success');
          if (state.editorSelectedFile === path) {
            updateState({ editorSelectedFile: null });
            const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
            if (editorContent) {
              editorContent.value = '';
              editorContent.disabled = true;
            }
            const codeEl = document.getElementById('editor-highlight-code');
            if (codeEl) codeEl.innerHTML = '';
            const pathEl = document.getElementById('editor-file-path');
            if (pathEl) pathEl.textContent = '';
          }
          await refreshEditorFileTree(state.editorModId);
        }
      } catch (err) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
      }
    });
  });

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
      if (state.editorModId) _lastFilePerMod[state.editorModId] = path;
      await loadFileContent(path);
    });
  });
}

export function populateEditorModSelect(): void {
  const select = document.getElementById('editor-mod-select') as HTMLSelectElement;
  if (!select) return;
  const state = getState();
  select.innerHTML = '<option value="">Select a mod...</option>' +
    state.allMods
      .filter(m => m.type !== 'pak' && m.type !== 'logicmods' && m.nexusAuthor !== 'UE4SS Native Mod')
      .map(m => `<option value="${m.id}">${escapeHtml(m.name)}</option>`).join('');
  renderEditorModTree();
}

export async function refreshEditorFileTree(modId: string): Promise<void> {
  try {
    const files = await listModFiles(modId);
    updateState({ editorFiles: files });
    renderFileTree(files);
  } catch (err) {
    console.error('Failed to refresh editor file tree:', err);
  }
}
