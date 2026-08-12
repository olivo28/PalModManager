import { stagedFiles, sourcePaths, targetOverrides, backupPaths, viewMode, virtualFolders, setVirtualFolders, setSourcePaths, renderWorkspace, escapeHtml } from './mod';
import { toggleSkipFile } from './staging';
import { showPrompt, showConfirm } from '../confirm';

export async function renderWorkspace(): Promise<void> {
  const ws = document.getElementById('packer-workspace-view');
  if (!ws || ws.style.display === 'none') return;

  const noFilesPlaceholder = document.getElementById('packer-empty-state');
  const filesContainer = document.getElementById('packer-files-container');

  if (filesContainer) filesContainer.style.display = 'block';

  if (stagedFiles.length === 0) {
    if (noFilesPlaceholder) noFilesPlaceholder.style.display = 'flex';
    const listArea = document.getElementById('packer-list-table');
    const treeArea = document.getElementById('packer-tree-view');
    if (listArea) listArea.style.display = 'none';
    if (treeArea) treeArea.style.display = 'none';
    updateBuildButtonState();
    return;
  }

  if (noFilesPlaceholder) noFilesPlaceholder.style.display = 'none';

  const listTab = document.getElementById('packer-view-list-btn');
  const treeTab = document.getElementById('packer-view-tree-btn');
  const listArea = document.getElementById('packer-list-table');
  const treeArea = document.getElementById('packer-tree-view');

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

function renderListMode(): void {
  const container = document.getElementById('packer-files-body');
  if (!container) return;

  container.innerHTML = stagedFiles.map((file, index) => {
    const filename = file.sourcePath.split(/[/\\]/).pop() || file.relativePath;
    const isSkipped = file.targetPath === '__SKIP__';
    const displayPath = isSkipped ? (backupPaths.get(file.sourcePath) || file.relativePath) : file.targetPath;

    return `
      <tr data-index="${index}" class="${isSkipped ? 'skipped' : ''}">
        <td title="${escapeHtml(file.sourcePath)}" style="max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
          <strong style="${isSkipped ? 'text-decoration: line-through;' : ''}">${escapeHtml(filename)}</strong>
          <div style="font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis;">${escapeHtml(file.sourcePath)}</div>
        </td>
        <td style="white-space: nowrap;">${formatBytes(file.size)}</td>
        <td>
          <input type="text" class="packer-input-target" value="${escapeHtml(displayPath)}" data-index="${index}" ${isSkipped ? 'disabled' : ''} />
        </td>
        <td>
          <div style="display: flex; align-items: center; gap: 4px;">
            <button class="packer-skip-file-btn" data-index="${index}" title="${isSkipped ? 'Include file' : 'Skip/Omit file'}">${isSkipped ? '↩️' : '🚫'}</button>
            <button class="packer-remove-file-btn" data-index="${index}" title="Remove file">✕</button>
          </div>
        </td>
      </tr>
    `;
  }).join('');

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
  const container = document.getElementById('packer-tree-view');
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
      const newPath = await showPrompt(`Rename/Move folder to (e.g. 'Mods/MyNewMod'):`, oldPath);
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
      const subName = await showPrompt(`Enter name for new subfolder inside '${parentPath}':`);
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
      const confirmed = await showConfirm(`Are you sure you want to remove folder '${oldPath}' and all its contents from staging?`);
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
      const newVal = await showPrompt(`Enter new target path for file:`, currentVal);
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
            <button class="packer-tree-action-btn packer-rename-dir-btn" data-path="${escapeHtml(nextPath)}" title="Rename/Move folder">✏️</button>
            <button class="packer-tree-action-btn packer-add-subdir-btn" data-path="${escapeHtml(nextPath)}" title="Add subfolder">➕</button>
            <button class="packer-tree-action-btn danger packer-remove-dir-btn" data-path="${escapeHtml(nextPath)}" title="Remove folder">✕</button>
          </div>
        </div>
      `;
      html += renderTreeHtml(child, depth + 1, nextPath);
    } else {
      const sizeStr = formatBytes(child.file.size);
      html += `
        <div class="packer-tree-node packer-tree-file ${child.isSkipped ? 'skipped' : ''}" style="padding-left: ${depth * 16}px;" data-index="${child.index}">
          <span class="packer-tree-icon">📄</span>
          <span class="packer-tree-name" title="Source: ${escapeHtml(child.file.sourcePath)}" style="${child.isSkipped ? 'text-decoration: line-through;' : ''}">${escapeHtml(child.name)}</span>
          <span class="packer-tree-size">${sizeStr}</span>
          <div class="packer-tree-actions">
            <button class="packer-tree-action-btn packer-rename-file-btn" data-index="${child.index}" title="Rename/Move file">✏️</button>
            <button class="packer-tree-action-btn packer-skip-file-btn" data-index="${child.index}" title="${child.isSkipped ? 'Include file' : 'Skip/Omit file'}">${child.isSkipped ? '↩️' : '🚫'}</button>
            <button class="packer-tree-action-btn danger packer-remove-file-btn" data-index="${child.index}" title="Remove file">✕</button>
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
  const metaName = document.getElementById('packer-meta-name') as HTMLInputElement | null;
  const metaVersion = document.getElementById('packer-meta-version') as HTMLInputElement | null;
  const metaAuthor = document.getElementById('packer-meta-author') as HTMLInputElement | null;
  const metaNexusId = document.getElementById('packer-meta-nexus-id') as HTMLInputElement | null;
  const metaType = document.getElementById('packer-meta-type') as HTMLSelectElement | null;
  const metaDesc = document.getElementById('packer-meta-desc') as HTMLTextAreaElement | null;

  if (metaName) metaName.value = '';
  if (metaVersion) metaVersion.value = '1.0.0';
  if (metaAuthor) metaAuthor.value = '';
  if (metaNexusId) metaNexusId.value = '';
  if (metaType) metaType.value = '';
  if (metaDesc) metaDesc.value = '';
}

export function updateBuildButtonState(): void {
  const name = (document.getElementById('packer-meta-name') as HTMLInputElement | null)?.value.trim();
  const version = (document.getElementById('packer-meta-version') as HTMLInputElement | null)?.value.trim();
  const type = (document.getElementById('packer-meta-type') as HTMLSelectElement | null)?.value;
  const buildBtn = document.getElementById('packer-build-btn') as HTMLButtonElement | null;
  
  if (buildBtn) {
    buildBtn.disabled = !name || !version || !type || stagedFiles.length === 0;
  }
}

export { renderListMode, renderTreeMode };
