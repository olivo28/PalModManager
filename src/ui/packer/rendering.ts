import { stagedFiles, sourcePaths, targetOverrides, backupPaths, viewMode, virtualFolders, setVirtualFolders, setSourcePaths, escapeHtml } from './mod';
import { toggleSkipFile, StagedFile } from './staging';
import { getState } from '../../state';
import { showPrompt, showConfirm } from '../confirm';
import { t } from '../../utils/i18n';
import { packerDom } from '../../framework';

export async function renderWorkspace(): Promise<void> {
  const ws = packerDom.elMaybe('packer-workspace-view');
  if (!ws || ws.style.display === 'none') return;

  const noFilesPlaceholder = packerDom.elMaybe('packer-empty-state');
  const filesContainer = packerDom.elMaybe('packer-files-container');

  if (filesContainer) filesContainer.style.display = 'block';

  if (stagedFiles.length === 0) {
    if (noFilesPlaceholder) noFilesPlaceholder.style.display = 'flex';
    const listArea = packerDom.elMaybe('packer-list-table');
    const treeArea = packerDom.elMaybe('packer-tree-view');
    if (listArea) listArea.style.display = 'none';
    if (treeArea) treeArea.style.display = 'none';
    updateBuildButtonState();
    return;
  }

  if (noFilesPlaceholder) noFilesPlaceholder.style.display = 'none';

  const listTab = packerDom.elMaybe('packer-view-list-btn');
  const treeTab = packerDom.elMaybe('packer-view-tree-btn');
  const listArea = packerDom.elMaybe('packer-list-table');
  const treeArea = packerDom.elMaybe('packer-tree-view');

  if (viewMode === 'list') {
    if (listTab) listTab.classList.add('active');
    if (treeTab) treeTab.classList.remove('active');
    if (listArea) listArea.style.display = 'table';
    if (treeArea) treeArea.style.display = 'none';
    renderListMode();
  } else {
    if (listTab) listTab.classList.remove('active');
    if (treeTab) treeTab.classList.add('active');
    if (listArea) listArea.style.display = 'none';
    if (treeArea) treeArea.style.display = 'block';
    await renderTreeMode();
  }

  updateBuildButtonState();
}

interface ListTreeNode {
  name: string;
  path: string;
  isFolder: boolean;
  children: Map<string, ListTreeNode>;
  file?: StagedFile;
  fileIndex?: number;
  totalSize: number;
  fileCount: number;
}

const collapsedTableFolders = new Set<string>();

function renderListTreeRows(node: ListTreeNode, depth: number, isWorkshop: boolean): string {
  const children = Array.from(node.children.values()).sort((a, b) => {
    if (a.isFolder !== b.isFolder) return a.isFolder ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  let html = '';

  children.forEach(child => {
    if (child.isFolder) {
      const isCollapsed = collapsedTableFolders.has(child.path);
      const indent = depth * 20 + 12;

      html += `
        <tr class="packer-table-folder-row" data-folder-path="${escapeHtml(child.path)}">
          <td style="padding-left: ${indent}px !important;">
            <div style="display: flex; align-items: center; gap: 6px; cursor: pointer; user-select: none;">
              <span class="editor-tree-chevron" style="display: inline-block; font-size: 8.5px; transition: transform 0.15s ease; ${isCollapsed ? 'transform: rotate(-90deg);' : ''}">▾</span>
              <span style="font-size: 13px;">📁</span>
              <strong style="color: var(--text-primary); font-size: 12px;">${escapeHtml(child.name)}</strong>
              <span style="font-size: 10px; color: var(--text-muted); opacity: 0.7;">(${child.totalSize > 0 ? formatBytes(child.totalSize) : ''})</span>
            </div>
          </td>
          <td style="white-space: nowrap; font-family: monospace; font-size: 11px; color: var(--text-muted);">${formatBytes(child.totalSize)}</td>
          <td>
            <div style="font-family: 'Fira Code', monospace; font-size: 10.5px; color: var(--text-muted); opacity: 0.5;">
              ${escapeHtml(child.path)}/
            </div>
          </td>
          <td></td>
        </tr>
      `;

      if (!isCollapsed) {
        html += renderListTreeRows(child, depth + 1, isWorkshop);
      }
    } else {
      const file = child.file!;
      const index = child.fileIndex!;
      const isSkipped = file.targetPath === '__SKIP__';
      const displayPath = isSkipped ? (backupPaths.get(file.sourcePath) || file.relativePath) : file.targetPath;

      const cleanTarget = displayPath.replace(/\\/g, '/');
      const lower = cleanTarget.toLowerCase();
      let routeBadge = 'FILE';
      let destPreview = cleanTarget;

      if (
        lower.startsWith('ue4ss/') ||
        lower.includes('/ue4ss/') ||
        lower.startsWith('mods/') ||
        lower.includes('/scripts/') ||
        lower.includes('/dlls/') ||
        lower.endsWith('.lua') ||
        lower.endsWith('enabled.txt')
      ) {
        routeBadge = 'UE4SS';
        let sub = cleanTarget;
        if (lower.startsWith('ue4ss/')) sub = cleanTarget.substring(6);
        else if (lower.startsWith('mods/')) sub = cleanTarget.substring(5);
        destPreview = (isWorkshop ? `Mods/NativeMods/UE4SS/Mods/` : `Pal/Binaries/Win64/ue4ss/Mods/`) + sub;
      } else if (
        lower.startsWith('palschema/') ||
        lower.startsWith('mods/palschema/') ||
        lower.includes('/palschema/') ||
        lower.includes('/blueprints/') ||
        lower.includes('/items/') ||
        lower.includes('/raw/') ||
        lower.includes('/translations/')
      ) {
        routeBadge = 'PALSCHEMA';
        let sub = cleanTarget;
        if (lower.startsWith('palschema/')) sub = cleanTarget.substring(10);
        else if (lower.startsWith('mods/palschema/')) sub = cleanTarget.substring(15);
        destPreview = (isWorkshop ? `Mods/NativeMods/UE4SS/Mods/PalSchema/mods/` : `Pal/Binaries/Win64/ue4ss/PalSchema/mods/`) + sub;
      } else if (
        lower.startsWith('pal/content/paks/logicmods/') ||
        lower.includes('/logicmods/')
      ) {
        routeBadge = 'LOGICMODS';
        const fn = cleanTarget.split('/').pop() || cleanTarget;
        destPreview = `Pal/Content/Paks/LogicMods/${fn}`;
      } else if (
        lower.startsWith('pal/content/paks/~mods/') ||
        lower.startsWith('pal/content/paks/') ||
        lower.endsWith('.pak') ||
        lower.endsWith('.ucas') ||
        lower.endsWith('.utoc')
      ) {
        routeBadge = 'PAK';
        const fn = cleanTarget.split('/').pop() || cleanTarget;
        destPreview = `Pal/Content/Paks/~mods/${fn}`;
      }

      let fileIcon = '📄';
      const ext = child.name.split('.').pop()?.toLowerCase();
      if (ext === 'lua') fileIcon = '📜';
      else if (ext === 'json' || ext === 'jsonc') fileIcon = '⚙️';
      else if (ext === 'pak' || ext === 'ucas' || ext === 'utoc') fileIcon = '📦';
      else if (ext === 'dll') fileIcon = '🧩';
      else if (ext === 'txt' || ext === 'ini' || ext === 'cfg') fileIcon = '📝';

      const indent = depth * 20 + 12;

      html += `
        <tr data-index="${index}" class="${isSkipped ? 'skipped' : ''}">
          <td title="${escapeHtml(file.sourcePath)}" style="padding-left: ${indent}px !important; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
            <div style="display: flex; align-items: center; gap: 6px; overflow: hidden;">
              <span style="font-size: 14px; opacity: 0.85; flex-shrink: 0;">${fileIcon}</span>
              <div style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                <strong style="color: var(--text-primary); font-size: 12px; ${isSkipped ? 'text-decoration: line-through; opacity: 0.6;' : ''}">${escapeHtml(child.name)}</strong>
              </div>
            </div>
          </td>
          <td style="white-space: nowrap; font-family: monospace; font-size: 11px; color: var(--text-muted);">${formatBytes(file.size)}</td>
          <td>
            <input type="text" class="packer-input-target" value="${escapeHtml(displayPath)}" data-index="${index}" ${isSkipped ? 'disabled' : ''} />
            ${!isSkipped ? `
              <div class="packer-route-preview" title="${escapeHtml(destPreview.replace(/\//g, '\\'))}">
                <span class="packer-route-badge ${routeBadge.toLowerCase()}">${routeBadge}</span>
                <span class="packer-route-arrow">→</span>
                <span class="packer-route-path">${escapeHtml(destPreview.replace(/\//g, '\\'))}</span>
              </div>
            ` : ''}
          </td>
          <td style="text-align: right; width: 70px;">
            <div style="display: flex; align-items: center; justify-content: flex-end; gap: 6px;">
              <button class="packer-table-action-btn packer-skip-file-btn skip ${isSkipped ? 'active' : ''}" data-index="${index}" title="${isSkipped ? escapeHtml(t('packer.btn_include_file_title')) : escapeHtml(t('packer.btn_skip_file_title'))}">
                ${isSkipped ? '↩️' : '🚫'}
              </button>
              <button class="packer-table-action-btn packer-remove-file-btn remove" data-index="${index}" title="${escapeHtml(t('packer.btn_remove_file_title'))}">
                ✕
              </button>
            </div>
          </td>
        </tr>
      `;
    }
  });

  return html;
}

function renderListMode(): void {
  const container = packerDom.elMaybe('packer-files-body');
  if (!container) return;

  const state = getState();
  const isWorkshop = state.currentProfile?.dependency_mode === 'workshop';

  const root: ListTreeNode = {
    name: '',
    path: '',
    isFolder: true,
    children: new Map(),
    totalSize: 0,
    fileCount: 0,
  };

  virtualFolders.forEach(vf => {
    const parts = vf.replace(/\\/g, '/').split('/').filter(p => p.trim() !== '');
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const curPath = parts.slice(0, i + 1).join('/');
      if (!current.children.has(part)) {
        current.children.set(part, {
          name: part,
          path: curPath,
          isFolder: true,
          children: new Map(),
          totalSize: 0,
          fileCount: 0,
        });
      }
      current = current.children.get(part)!;
    }
  });

  stagedFiles.forEach((file, index) => {
    const isSkipped = file.targetPath === '__SKIP__';
    const displayPath = isSkipped ? (backupPaths.get(file.sourcePath) || file.relativePath) : file.targetPath;
    const clean = displayPath.replace(/\\/g, '/');
    const parts = clean.split('/').filter(p => p.trim() !== '');
    let current = root;

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const curPath = parts.slice(0, i + 1).join('/');

      if (!current.children.has(part)) {
        current.children.set(part, {
          name: part,
          path: curPath,
          isFolder: !isLast,
          children: new Map(),
          file: isLast ? file : undefined,
          fileIndex: isLast ? index : undefined,
          totalSize: 0,
          fileCount: 0,
        });
      }
      current = current.children.get(part)!;
      current.totalSize += file.size;
      if (isLast) current.fileCount += 1;
    }
  });

  container.innerHTML = renderListTreeRows(root, 0, isWorkshop);

  container.querySelectorAll('.packer-table-folder-row').forEach(row => {
    row.addEventListener('click', () => {
      const folderPath = (row as HTMLElement).dataset.folderPath || '';
      if (collapsedTableFolders.has(folderPath)) {
        collapsedTableFolders.delete(folderPath);
      } else {
        collapsedTableFolders.add(folderPath);
      }
      renderListMode();
    });
  });

  container.querySelectorAll('.packer-input-target').forEach(input => {
    input.addEventListener('change', (e) => {
      const idx = parseInt((e.target as HTMLInputElement).dataset.index || '0');
      const val = (e.target as HTMLInputElement).value.trim();
      stagedFiles[idx].targetPath = val;
      targetOverrides.set(stagedFiles[idx].sourcePath, val);
    });
  });

  container.querySelectorAll('.packer-skip-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      toggleSkipFile(idx);
    });
  });

  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);

      const stillHasSource = stagedFiles.some(f => f.sourcePath === removedFile.sourcePath || f.sourcePath.startsWith(removedFile.sourcePath));
      if (!stillHasSource) {
        setSourcePaths(sourcePaths.filter(p => p !== removedFile.sourcePath && !removedFile.sourcePath.startsWith(p)));
      }
      targetOverrides.delete(removedFile.sourcePath);
      backupPaths.delete(removedFile.sourcePath);

      renderWorkspace();
    });
  });
}

async function renderTreeMode(): Promise<void> {
  const container = packerDom.elMaybe('packer-tree-view');
  if (!container) return;

  const root: any = { name: 'root', isDir: true, children: {} };

  virtualFolders.forEach(folderPath => {
    const parts = folderPath.replace(/\\/g, '/').split('/').filter(p => p.trim() !== '');
    let current = root;
    parts.forEach(part => {
      if (!current.children[part]) {
        current.children[part] = { name: part, isDir: true, children: {} };
      }
      current = current.children[part];
    });
  });

  stagedFiles.forEach((file, index) => {
    const isSkipped = file.targetPath === '__SKIP__';
    const treePath = isSkipped ? (backupPaths.get(file.sourcePath) || file.relativePath) : file.targetPath;
    const parts = treePath.replace(/\\/g, '/').split('/').filter(p => p.trim() !== '');
    let current = root;

    parts.forEach((part, i) => {
      const isLast = i === parts.length - 1;
      if (isLast) {
        current.children[part] = { name: part, isDir: false, file, index, isSkipped };
      } else {
        if (!current.children[part]) {
          current.children[part] = { name: part, isDir: true, children: {} };
        }
        current = current.children[part];
      }
    });
  });

  container.innerHTML = `<div class="packer-tree-root">${renderTreeHtml(root, 0, '')}</div>`;

  const { setupTreeDragAndDropHandlers } = await import('./dragDrop');
  setupTreeDragAndDropHandlers(container);

  container.querySelectorAll('.packer-rename-dir-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const oldPath = (btn as HTMLElement).dataset.path || '';
      const newPath = await showPrompt(t('packer.prompt_rename_dir'), oldPath);
      if (newPath && newPath.trim() !== oldPath) {
        const cleaned = newPath.trim().replace(/\\/g, '/');
        stagedFiles.forEach(f => {
          if (f.targetPath === oldPath) {
            f.targetPath = cleaned;
            targetOverrides.set(f.sourcePath, cleaned);
          } else if (f.targetPath.startsWith(oldPath + '/')) {
            f.targetPath = cleaned + f.targetPath.substring(oldPath.length);
            targetOverrides.set(f.sourcePath, f.targetPath);
          }
        });

        const updatedVirtual = virtualFolders.map(vf => {
          if (vf === oldPath) return cleaned;
          if (vf.startsWith(oldPath + '/')) {
            return cleaned + vf.substring(oldPath.length);
          }
          return vf;
        });
        setVirtualFolders(updatedVirtual);

        targetOverrides.forEach((val, key) => {
          if (key.startsWith('__VIRTUAL_DIR__:')) {
            const path = key.substring('__VIRTUAL_DIR__:'.length);
            if (path === oldPath || path.startsWith(oldPath + '/')) {
              targetOverrides.delete(key);
            }
          }
        });
        virtualFolders.forEach(vf => {
          targetOverrides.set(`__VIRTUAL_DIR__:${vf}`, '__VIRTUAL_DIR__');
        });
        renderWorkspace();
      }
    });
  });

  container.querySelectorAll('.packer-add-subdir-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const parentPath = (btn as HTMLElement).dataset.path || '';
      const subName = await showPrompt(t('packer.prompt_new_subdir', { path: parentPath }));
      if (subName && subName.trim()) {
        const cleanedSub = subName.trim().replace(/\\/g, '/');
        const nextPath = `${parentPath}/${cleanedSub}`;
        if (!virtualFolders.includes(nextPath)) {
          virtualFolders.push(nextPath);
          targetOverrides.set(`__VIRTUAL_DIR__:${nextPath}`, '__VIRTUAL_DIR__');
          renderWorkspace();
        }
      }
    });
  });

  container.querySelectorAll('.packer-remove-dir-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const oldPath = (btn as HTMLElement).dataset.path || '';
      const confirmed = await showConfirm(t('packer.confirm_remove_folder', { path: oldPath }));
      if (confirmed) {
        const remainingFiles = stagedFiles.filter(f => {
          const match = f.targetPath === oldPath || f.targetPath.startsWith(oldPath + '/');
          if (match) {
            targetOverrides.delete(f.sourcePath);
            backupPaths.delete(f.sourcePath);
          }
          return !match;
        });
        // Mutate stagedFiles in place
        stagedFiles.length = 0;
        stagedFiles.push(...remainingFiles);

        setSourcePaths(sourcePaths.filter(sp => {
          return stagedFiles.some(sf => sf.sourcePath === sp || sf.sourcePath.startsWith(sp + '/') || sf.sourcePath.startsWith(sp + '\\'));
        }));

        const remainingVirtual = virtualFolders.filter(vf => {
          const match = vf === oldPath || vf.startsWith(oldPath + '/');
          if (match) {
            targetOverrides.delete(`__VIRTUAL_DIR__:${vf}`);
          }
          return !match;
        });
        setVirtualFolders(remainingVirtual);

        renderWorkspace();
      }
    });
  });

  container.querySelectorAll('.packer-rename-file-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index || '0');
      const file = stagedFiles[idx];
      const isSkipped = file.targetPath === '__SKIP__';
      const currentVal = isSkipped ? (backupPaths.get(file.sourcePath) || file.relativePath) : file.targetPath;
      const newVal = await showPrompt(t('packer.prompt_rename_file'), currentVal);
      if (newVal && newVal.trim() !== currentVal) {
        const cleaned = newVal.trim().replace(/\\/g, '/');
        if (isSkipped) {
          backupPaths.set(file.sourcePath, cleaned);
        } else {
          file.targetPath = cleaned;
          targetOverrides.set(file.sourcePath, cleaned);
        }
        renderWorkspace();
      }
    });
  });

  container.querySelectorAll('.packer-skip-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index || '0');
      toggleSkipFile(idx);
    });
  });

  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);

      const stillHasSource = stagedFiles.some(f => f.sourcePath === removedFile.sourcePath || f.sourcePath.startsWith(removedFile.sourcePath));
      if (!stillHasSource) {
        setSourcePaths(sourcePaths.filter(p => p !== removedFile.sourcePath && !removedFile.sourcePath.startsWith(p)));
      }
      targetOverrides.delete(removedFile.sourcePath);
      backupPaths.delete(removedFile.sourcePath);
      renderWorkspace();
    });
  });
}

export function renderTreeHtml(node: any, depth = 0, currentPath = ''): string {
  let html = '';
  const keys = Object.keys(node.children || {}).sort((a, b) => {
    const na = node.children[a];
    const nb = node.children[b];
    if (na.isDir && !nb.isDir) return -1;
    if (!na.isDir && nb.isDir) return 1;
    return a.localeCompare(b);
  });

  keys.forEach(key => {
    const child = node.children[key];
    if (child.isDir) {
      const nextPath = currentPath ? `${currentPath}/${key}` : key;
      html += `
        <div class="packer-tree-node packer-tree-dir" style="padding-left: ${depth * 16}px;" data-path="${escapeHtml(nextPath)}">
          <span class="packer-tree-icon">📁</span>
          <span class="packer-tree-name">${escapeHtml(child.name)}</span>
          <div class="packer-tree-actions">
            <button class="packer-tree-action-btn packer-rename-dir-btn" data-path="${escapeHtml(nextPath)}" title="${escapeHtml(t('packer.btn_rename_folder_title'))}">✏️</button>
            <button class="packer-tree-action-btn packer-add-subdir-btn" data-path="${escapeHtml(nextPath)}" title="${escapeHtml(t('packer.btn_add_subfolder_title'))}">➕</button>
            <button class="packer-tree-action-btn danger packer-remove-dir-btn" data-path="${escapeHtml(nextPath)}" title="${escapeHtml(t('packer.btn_remove_folder_title'))}">✕</button>
          </div>
        </div>
      `;
      html += renderTreeHtml(child, depth + 1, nextPath);
    } else {
      const sizeStr = formatBytes(child.file.size);
      html += `
        <div class="packer-tree-node packer-tree-file ${child.isSkipped ? 'skipped' : ''}" style="padding-left: ${depth * 16}px;" data-index="${child.index}">
          <span class="packer-tree-icon">📄</span>
          <span class="packer-tree-name" title="${escapeHtml(t('packer.source_prefix_label', { path: child.file.sourcePath }))}" style="${child.isSkipped ? 'text-decoration: line-through;' : ''}">${escapeHtml(child.name)}</span>
          <span class="packer-tree-size">${sizeStr}</span>
          <div class="packer-tree-actions">
            <button class="packer-tree-action-btn packer-rename-file-btn" data-index="${child.index}" title="${escapeHtml(t('packer.btn_rename_file_title'))}">✏️</button>
            <button class="packer-tree-action-btn packer-skip-file-btn" data-index="${child.index}" title="${child.isSkipped ? escapeHtml(t('packer.btn_include_file_title')) : escapeHtml(t('packer.btn_skip_file_title'))}">${child.isSkipped ? '↩️' : '🚫'}</button>
            <button class="packer-tree-action-btn danger packer-remove-file-btn" data-index="${child.index}" title="${escapeHtml(t('packer.btn_remove_file_title'))}">✕</button>
          </div>
        </div>
      `;
    }
  });

  return html;
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

export function clearMetadataForm(): void {
  const metaName = packerDom.elMaybe('packer-meta-name');
  const metaVersion = packerDom.elMaybe('packer-meta-version');
  const metaAuthor = packerDom.elMaybe('packer-meta-author');
  const metaNexusId = packerDom.elMaybe('packer-meta-nexus-id');
  const metaType = packerDom.elMaybe('packer-meta-type');
  const metaDesc = packerDom.elMaybe('packer-meta-desc');

  if (metaName) metaName.value = '';
  if (metaVersion) metaVersion.value = '1.0.0';
  if (metaAuthor) metaAuthor.value = '';
  if (metaNexusId) metaNexusId.value = '';
  if (metaType) metaType.value = '';
  if (metaDesc) metaDesc.value = '';
}

export function updateBuildButtonState(): void {
  const name = packerDom.elMaybe('packer-meta-name')?.value.trim();
  const version = packerDom.elMaybe('packer-meta-version')?.value.trim();
  const type = packerDom.elMaybe('packer-meta-type')?.value;
  const buildBtn = packerDom.elMaybe('packer-build-btn');
  
  if (buildBtn) {
    buildBtn.disabled = !name || !version || !type || stagedFiles.length === 0;
  }
}

export { renderListMode, renderTreeMode };
