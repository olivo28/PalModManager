import { listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { escapeHtml } from '../../utils/helpers';
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
    tree.innerHTML = '<div style="padding:12px;font-size:11px;color:var(--text-muted)">No editable mods</div>';
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

  tree.querySelectorAll('.editor-tree-folder-header').forEach(header => {
    header.addEventListener('click', (e) => {
      e.stopPropagation();
      const folder = header.closest('.editor-tree-folder')!;
      folder.classList.toggle('collapsed');
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
