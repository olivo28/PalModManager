import { listModFiles } from '../../api';
import { getState, updateState } from '../../state';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { showConfirm } from '../confirm';
import { confirmDiscardOrSave, _lastFilePerMod, loadFileContent, loadEditorData, clearEditorContent, _fileBufferCache, _originalContent, clearBufferCache } from './viewer';
import { getMonacoContent } from './monaco/instance';
import { editorDom } from '../../framework';

export function renderEditorModTree(): void {
  const tree = editorDom.elMaybe('editor-mod-tree');
  if (!tree) return;

  const state = getState();
  const currentModId = state.editorModId;

  const editableMods = state.allMods.filter(m =>
    m.type !== 'pak' &&
    m.type !== 'logicmods' &&
    m.nexusAuthor !== 'UE4SS Native Mod'
  );

  // If currently active mod was deleted or removed from disk
  if (currentModId && !editableMods.some(m => m.id === currentModId)) {
    if (editableMods.length > 0) {
      setTimeout(() => switchEditorMod(editableMods[0].id), 0);
    } else {
      updateState({ editorModId: null, editorSelectedFile: null, editorFiles: [] });
      const fileTree = editorDom.elMaybe('editor-file-tree');
      if (fileTree) fileTree.innerHTML = '';
      const nameEl = editorDom.elMaybe('editor-current-mod-name');
      if (nameEl) nameEl.textContent = '';
      clearEditorContent();
    }
  }

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

export async function switchEditorMod(modId: string, targetFile?: string, lineNumber?: number): Promise<void> {
  const state = getState();
  const currentModId = state.editorModId;
  if (currentModId === modId && !targetFile) return;

  if (currentModId && state.editorSelectedFile && currentModId !== modId) {
    _lastFilePerMod[currentModId] = state.editorSelectedFile;
  }

  const proceed = await confirmDiscardOrSave();
  if (!proceed) return;
  clearBufferCache();

  updateState({ editorModId: modId, editorSelectedFile: null });

  const select = editorDom.elMaybe('editor-mod-select');
  if (select) select.value = modId;

  const mod = getState().allMods.find(m => m.id === modId);
  const nameEl = editorDom.elMaybe('editor-current-mod-name');
  if (nameEl) nameEl.textContent = mod?.name || '';

  document.querySelectorAll('.editor-mod-item').forEach(el => {
    el.classList.toggle('active', (el as HTMLElement).dataset.modId === modId);
  });

  await loadEditorData(modId);

  // 1. If a specific target file was requested, prioritize revealing it
  if (targetFile && await revealAndSelectFile(targetFile, lineNumber)) {
    return;
  }

  // 2. Prioritize last opened file for this mod
  const lastFile = _lastFilePerMod[modId];
  if (lastFile && await revealAndSelectFile(lastFile)) {
    return;
  }

  // 3. Fallback to best configuration / script file
  const bestFile = findBestConfigFile(getState().editorFiles || []);
  if (bestFile && await revealAndSelectFile(bestFile)) {
    return;
  }

  // 4. Fallback to first file in the tree
  const firstFile = document.querySelector('.editor-file-item') as HTMLElement | null;
  if (firstFile) {
    const path = firstFile.dataset.path;
    if (path) await revealAndSelectFile(path);
  }
}

export function findBestConfigFile(files: string[]): string | null {
  if (!files || files.length === 0) return null;

  const validFiles = files.filter(f => {
    const name = f.replace(/^.*[/\\]/, '').toLowerCase();
    return !name.startsWith('.') && name !== 'enabled.txt' && !name.endsWith('.bak');
  });

  if (validFiles.length === 0) return files[0] || null;

  // 1. Exact or near-exact config / settings file matches
  const exactConfig = validFiles.find(f => {
    const leaf = f.replace(/^.*[/\\]/, '').toLowerCase();
    return (
      leaf === 'config.lua' ||
      leaf === 'config.json' ||
      leaf === 'config.jsonc' ||
      leaf === 'config.ini' ||
      leaf === 'config.cfg' ||
      leaf === 'settings.json' ||
      leaf === 'settings.lua' ||
      leaf === 'options.json' ||
      leaf === 'alterconfig.json' ||
      leaf === 'swapjson.json'
    );
  });
  if (exactConfig) return exactConfig;

  // 2. Any file with "config", "settings", "options", "params" in the filename
  const namedConfig = validFiles.find(f => {
    const leaf = f.replace(/^.*[/\\]/, '').toLowerCase();
    return leaf.includes('config') || leaf.includes('setting') || leaf.includes('option') || leaf.includes('param');
  });
  if (namedConfig) return namedConfig;

  // 3. Main script entry points (main.lua, init.lua, mod.lua)
  const mainScript = validFiles.find(f => {
    const leaf = f.replace(/^.*[/\\]/, '').toLowerCase();
    return leaf === 'main.lua' || leaf === 'init.lua' || leaf === 'mod.lua' || leaf === 'index.js';
  });
  if (mainScript) return mainScript;

  // 4. Any JSON / JSONC / INI / CFG files
  const dataFile = validFiles.find(f => {
    const lower = f.toLowerCase();
    return lower.endsWith('.jsonc') || lower.endsWith('.json') || lower.endsWith('.ini') || lower.endsWith('.cfg');
  });
  if (dataFile) return dataFile;

  // 5. Any Lua scripts
  const luaScript = validFiles.find(f => f.toLowerCase().endsWith('.lua'));
  if (luaScript) return luaScript;

  // 6. First valid file
  return validFiles[0];
}

export async function revealAndSelectFile(filePath: string, lineNumber?: number): Promise<boolean> {
  if (!filePath) return false;
  const tree = editorDom.elMaybe('editor-file-tree');
  if (!tree) return false;

  const normalizedTarget = filePath.replace(/\\/g, '/').replace(/^\/+/, '');

  // 1. Expand all parent folders of this file path
  const parts = normalizedTarget.split('/');
  for (let i = 1; i < parts.length; i++) {
    const folderPath = parts.slice(0, i).join('/');
    _collapsedFolders.delete(folderPath);
    const folderEl = tree.querySelector(`.editor-tree-folder[data-folder-path="${CSS.escape(folderPath)}"]`);
    if (folderEl) {
      folderEl.classList.remove('collapsed');
    }
  }

  // 2. Query exact match, normalized match, or fuzzy/suffix match
  let fileItem = tree.querySelector(`.editor-file-item[data-path="${CSS.escape(filePath)}"]`) as HTMLElement | null;
  if (!fileItem) {
    fileItem = tree.querySelector(`.editor-file-item[data-path="${CSS.escape(normalizedTarget)}"]`) as HTMLElement | null;
  }
  if (!fileItem) {
    const allItems = Array.from(tree.querySelectorAll('.editor-file-item')) as HTMLElement[];
    const targetLower = normalizedTarget.toLowerCase();
    fileItem = allItems.find(el => {
      const p = (el.dataset.path || '').replace(/\\/g, '/').replace(/^\/+/, '').toLowerCase();
      return p === targetLower || p.endsWith('/' + targetLower) || targetLower.endsWith('/' + p);
    }) || null;
  }

  if (fileItem) {
    const actualPath = fileItem.dataset.path || normalizedTarget;
    tree.querySelectorAll('.editor-file-item').forEach(el => el.classList.remove('selected'));
    fileItem.classList.add('selected');
    fileItem.scrollIntoView({ block: 'nearest', behavior: 'smooth' });

    updateState({ editorSelectedFile: actualPath });
    const state = getState();
    if (state.editorModId) _lastFilePerMod[state.editorModId] = actualPath;

    await loadFileContent(actualPath, lineNumber);
    return true;
  }

  // Direct load fallback
  await loadFileContent(normalizedTarget, lineNumber);
  return true;
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
  const tree = editorDom.el('editor-file-tree');
  const rootNode = buildFileTree(files);

  const state = getState();
  const currentMod = state.allMods.find(m => m.id === state.editorModId);
  const modName = currentMod ? currentMod.name : '';

  const headerHtml = `
    <div class="editor-file-tree-header">
      <div class="editor-file-tree-title-row">
        <span class="editor-file-tree-mod-name" title="${escapeHtml(modName)}">${escapeHtml(modName)}</span>
        <div class="editor-file-tree-actions">
          <button id="editor-new-file-btn" class="editor-tree-icon-btn" title="${escapeHtml(t('editor.btn_new_file') || 'New File')}">📄+</button>
          <button id="editor-new-folder-btn" class="editor-tree-icon-btn" title="${escapeHtml(t('editor.btn_new_folder') || 'New Folder')}">📁+</button>
        </div>
      </div>
    </div>
  `;

  if (rootNode.children.size === 0) {
    tree.innerHTML = headerHtml + `<div class="editor-file-empty">${escapeHtml(t('editor.no_editable_files') || 'No editable files found')}</div>`;
  } else {
    tree.innerHTML = headerHtml + `<div class="editor-file-tree-content">${renderNodeHTML(rootNode)}</div>`;
  }

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
            const editorContent = editorDom.elMaybe('editor-content');
            if (editorContent) {
              editorContent.value = '';
              editorContent.disabled = true;
            }
            const codeEl = editorDom.elMaybe('editor-highlight-code');
            if (codeEl) codeEl.innerHTML = '';
            const pathEl = editorDom.elMaybe('editor-file-path');
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

      // Save current in-memory buffer before switching files
      const currentPath = state.editorSelectedFile;
      if (currentPath && _originalContent !== null) {
        const currentContent = getMonacoContent();
        const normalize = (str: string) => str.replace(/\r\n/g, '\n');
        const isDirty = normalize(currentContent) !== normalize(_originalContent);
        _fileBufferCache.set(currentPath, {
          current: currentContent,
          original: _originalContent,
          isDirty,
        });
      }

      tree.querySelectorAll('.editor-file-item').forEach(el => el.classList.remove('selected'));
      item.classList.add('selected');
      updateState({ editorSelectedFile: path });
      if (state.editorModId) _lastFilePerMod[state.editorModId] = path;
      await loadFileContent(path);
    });
  });

  // New File action
  const newFileBtn = editorDom.elMaybe('editor-new-file-btn');
  if (newFileBtn) {
    newFileBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const currentModId = getState().editorModId;
      if (!currentModId) return;
      const promptMsg = t('editor.prompt_new_file') || 'Enter relative path for new file (e.g. scripts/subsystem.lua):';
      const relPath = window.prompt(promptMsg);
      if (!relPath || !relPath.trim()) return;

      try {
        const { createModFile } = await import('../../api');
        const { showToast } = await import('../toast');
        const createdPath = await createModFile(currentModId, relPath.trim());
        showToast(`File created: ${createdPath}`, 'success');
        await refreshEditorFileTree(currentModId);
        await revealAndSelectFile(createdPath);
      } catch (err: any) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
      }
    });
  }

  // New Folder action
  const newFolderBtn = editorDom.elMaybe('editor-new-folder-btn');
  if (newFolderBtn) {
    newFolderBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const currentModId = getState().editorModId;
      if (!currentModId) return;
      const promptMsg = t('editor.prompt_new_folder') || 'Enter relative path for new folder (e.g. scripts/spawners):';
      const relPath = window.prompt(promptMsg);
      if (!relPath || !relPath.trim()) return;

      try {
        const { createEditorFolder } = await import('../../api');
        const { showToast } = await import('../toast');
        const createdPath = await createEditorFolder(currentModId, relPath.trim());
        showToast(`Folder created: ${createdPath}`, 'success');
        await refreshEditorFileTree(currentModId);
      } catch (err: any) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
      }
    });
  }

}

export function populateEditorModSelect(): void {
  const select = editorDom.elMaybe('editor-mod-select');
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
