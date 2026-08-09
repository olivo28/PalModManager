import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';
import { showConfirm, showPrompt } from './confirm';

interface StagedFile {
  sourcePath: string;
  relativePath: string;
  size: number;
  targetPath: string;
}

interface ModMetadata {
  name: string;
  version: string;
  description: string;
  author: string;
  modType: string;
  nexusModId?: number | null;
}

interface PackerProject {
  name: string;
  metadata: ModMetadata | null;
  sourcePaths: string[];
  targetPathsOverride: Record<string, string>;
  format: string;
}

let stagedFiles: StagedFile[] = [];
let sourcePaths: string[] = [];
let targetOverrides: Map<string, string> = new Map();
let virtualFolders: string[] = [];
let backupPaths: Map<string, string> = new Map();

let viewMode: 'list' | 'tree' = 'list';
let savedProjects: PackerProject[] = [];
let activeProjectName: string = '';

export function initPackerView(): void {
  setupPackerDragAndDrop();
  setupPackerEventListeners();
  loadProjectsList();
}

function setupPackerDragAndDrop(): void {
  const container = document.getElementById('build-view');
  const overlay = document.getElementById('packer-drag-overlay');
  if (!container || !overlay) return;

  // Only preventDefault — do NOT stopPropagation, otherwise internal tree
  // node dragover/drop listeners never fire and internal DnD is broken.
  ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
    container.addEventListener(eventName, (e: Event) => { e.preventDefault(); }, false);
  });

  container.addEventListener('dragenter', () => {
    // Only show the "drop files here" overlay for real OS file drags, not
    // internal element reorders (which set data-internal-drag on body).
    if (!document.body.hasAttribute('data-internal-drag')) {
      overlay.classList.add('drag-over');
    }
  }, false);

  container.addEventListener('dragover', () => {
    if (!document.body.hasAttribute('data-internal-drag')) {
      overlay.classList.add('drag-over');
    }
  }, false);

  container.addEventListener('dragleave', () => {
    overlay.classList.remove('drag-over');
  }, false);
}

export async function addStagedPaths(paths: string[]): Promise<void> {
  // Push new source paths
  paths.forEach(p => {
    if (!sourcePaths.includes(p)) {
      sourcePaths.push(p);
    }
  });

  await scanAndBuildStagedFiles();
  showToast(`Added ${paths.length} path(s) to project`, 'success');
}

// Scans current source paths and updates stagedFiles
async function scanAndBuildStagedFiles(): Promise<void> {
  if (sourcePaths.length === 0) {
    stagedFiles = [];
    renderWorkspace();
    return;
  }

  try {
    const files = await invoke<StagedFile[]>('scan_paths_for_packing', { paths: sourcePaths });
    
    // Apply overrides or default relative paths
    files.forEach(f => {
      if (targetOverrides.has(f.sourcePath)) {
        f.targetPath = targetOverrides.get(f.sourcePath) || f.targetPath;
      }
    });

    stagedFiles = files;
    renderWorkspace();
  } catch (err: any) {
    console.error(err);
    showToast(`Error scanning paths: ${err}`, 'error');
  }
}

function setupPackerEventListeners(): void {
  // Back button
  document.getElementById('packer-workspace-back-btn')?.addEventListener('click', () => {
    showProjectsHub();
  });

  // Create New Project button (hub)
  document.getElementById('packer-hub-new-btn')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });

  // Add Files button
  document.getElementById('packer-add-files-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: true,
        directory: false,
        title: 'Select Files to Add to Project'
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        await addStagedPaths(paths);
      }
    } catch (err) {
      console.error(err);
    }
  });

  // Add Folder button
  document.getElementById('packer-add-folder-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        directory: true,
        title: 'Select Folder to Add to Project'
      });
      if (selected) {
        await addStagedPaths([selected]);
      }
    } catch (err) {
      console.error(err);
    }
  });

  // Clear button
  document.getElementById('packer-clear-btn')?.addEventListener('click', () => {
    stagedFiles = [];
    sourcePaths = [];
    targetOverrides.clear();
    virtualFolders = [];
    backupPaths.clear();
    renderWorkspace();
    showToast('Staged workspace cleared', 'info');
  });

  // New Virtual Folder button
  document.getElementById('packer-new-virtual-folder-btn')?.addEventListener('click', async () => {
    const folderName = await showPrompt('Enter virtual folder path (e.g. Mods/MyMod/Scripts):');
    if (folderName) {
      const cleaned = folderName.replace(/\\/g, '/').trim();
      if (cleaned) {
        if (!virtualFolders.includes(cleaned)) {
          virtualFolders.push(cleaned);
          targetOverrides.set(`__VIRTUAL_DIR__:${cleaned}`, '__VIRTUAL_DIR__');
          renderWorkspace();
          showToast(`Virtual folder '${cleaned}' created`, 'success');
        }
      }
    }
  });

  // Auto-Structure button
  document.getElementById('packer-autostruct-btn')?.addEventListener('click', () => {
    autoStructureWorkspace();
  });

  // Build button
  document.getElementById('packer-build-btn')?.addEventListener('click', async () => {
    await buildModPackage();
  });

  // View Mode Toggles
  document.getElementById('packer-view-list-btn')?.addEventListener('click', () => {
    setViewMode('list');
  });

  document.getElementById('packer-view-tree-btn')?.addEventListener('click', () => {
    setViewMode('tree');
  });

  // Save project button
  document.getElementById('packer-project-save-btn')?.addEventListener('click', async () => {
    await saveCurrentProject();
  });

  // Delete project button (workspace)
  document.getElementById('packer-workspace-delete-btn')?.addEventListener('click', async () => {
    if (activeProjectName) {
      await deleteProjectByName(activeProjectName);
    }
  });

  // Automatically update build button disabled state on form changes
  const inputs = ['packer-meta-name', 'packer-meta-version', 'packer-meta-author', 'packer-meta-nexus-id'];
  inputs.forEach(id => {
    document.getElementById(id)?.addEventListener('input', updateBuildButtonState);
  });
  document.getElementById('packer-meta-type')?.addEventListener('change', updateBuildButtonState);
}

function setViewMode(mode: 'list' | 'tree'): void {
  viewMode = mode;
  
  const listBtn = document.getElementById('packer-view-list-btn');
  const treeBtn = document.getElementById('packer-view-tree-btn');
  const listTable = document.getElementById('packer-list-table');
  const treeView = document.getElementById('packer-tree-view');

  const newFolderBtn = document.getElementById('packer-new-virtual-folder-btn');

  if (mode === 'list') {
    listBtn?.classList.add('active');
    treeBtn?.classList.remove('active');
    if (listTable) listTable.style.display = 'table';
    if (treeView) treeView.style.display = 'none';
    if (newFolderBtn) newFolderBtn.style.display = 'none';
  } else {
    listBtn?.classList.remove('active');
    treeBtn?.classList.add('active');
    if (listTable) listTable.style.display = 'none';
    if (treeView) treeView.style.display = 'block';
    if (newFolderBtn) newFolderBtn.style.display = 'inline-block';
  }

  renderWorkspace();
}

function updateBuildButtonState(): void {
  const name = (document.getElementById('packer-meta-name') as HTMLInputElement)?.value.trim();
  const version = (document.getElementById('packer-meta-version') as HTMLInputElement)?.value.trim();
  const type = (document.getElementById('packer-meta-type') as HTMLSelectElement)?.value;
  const buildBtn = document.getElementById('packer-build-btn') as HTMLButtonElement | null;
  
  if (buildBtn) {
    buildBtn.disabled = !name || !version || !type || stagedFiles.length === 0;
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function renderWorkspace(): void {
  const emptyState = document.getElementById('packer-empty-state');

  if (stagedFiles.length === 0) {
    const listBody = document.getElementById('packer-files-body');
    const treeBody = document.getElementById('packer-tree-view');
    if (listBody) listBody.innerHTML = '';
    if (treeBody) treeBody.innerHTML = '';
    if (emptyState) emptyState.style.display = 'flex';
    updateBuildButtonState();
    return;
  }

  if (emptyState) contradicts(emptyState);

  if (viewMode === 'list') {
    renderListMode();
  } else {
    renderTreeMode();
  }

  updateBuildButtonState();
}

function contradicts(el: HTMLElement): void {
  el.style.display = 'none';
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

  // Add listeners to target path inputs
  container.querySelectorAll('.packer-input-target').forEach(input => {
    input.addEventListener('change', (e) => {
      const idx = parseInt((e.target as HTMLInputElement).dataset.index || '0');
      const val = (e.target as HTMLInputElement).value.trim();
      stagedFiles[idx].targetPath = val;
      targetOverrides.set(stagedFiles[idx].sourcePath, val);
    });
  });

  // Add listeners to skip buttons
  container.querySelectorAll('.packer-skip-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      toggleSkipFile(idx);
    });
  });

  // Add listeners to remove buttons
  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);
      
      const stillHasSource = stagedFiles.some(f => f.sourcePath === removedFile.sourcePath || f.sourcePath.startsWith(removedFile.sourcePath));
      if (!stillHasSource) {
        sourcePaths = sourcePaths.filter(p => p !== removedFile.sourcePath && !removedFile.sourcePath.startsWith(p));
      }
      targetOverrides.delete(removedFile.sourcePath);
      backupPaths.delete(removedFile.sourcePath);

      renderWorkspace();
    });
  });
}

function toggleSkipFile(index: number): void {
  const file = stagedFiles[index];
  if (file.targetPath === '__SKIP__') {
    const restored = backupPaths.get(file.sourcePath) || file.relativePath;
    file.targetPath = restored === '__SKIP__' ? file.relativePath : restored;
    targetOverrides.set(file.sourcePath, file.targetPath);
  } else {
    backupPaths.set(file.sourcePath, file.targetPath);
    file.targetPath = '__SKIP__';
    targetOverrides.set(file.sourcePath, '__SKIP__');
  }
  renderWorkspace();
}

function renderTreeMode(): void {
  const container = document.getElementById('packer-tree-view');
  if (!container) return;

  const root: any = { name: 'root', isDir: true, children: {} };

  // 1. Add virtual empty folders
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

  // 2. Add staged files
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

  setupTreeDragAndDropHandlers(container);

  // 1. Rename Dir listener
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

        virtualFolders = virtualFolders.map(vf => {
          if (vf === oldPath) return cleaned;
          if (vf.startsWith(oldPath + '/')) {
            return cleaned + vf.substring(oldPath.length);
          }
          return vf;
        });

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

  // 2. Add Subdir listener
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

  // 3. Remove Dir listener
  container.querySelectorAll('.packer-remove-dir-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const oldPath = (btn as HTMLElement).dataset.path || '';
      const confirmed = await showConfirm(`Are you sure you want to remove folder '${oldPath}' and all its contents from staging?`);
      if (confirmed) {
        stagedFiles = stagedFiles.filter(f => {
          const match = f.targetPath === oldPath || f.targetPath.startsWith(oldPath + '/');
          if (match) {
            targetOverrides.delete(f.sourcePath);
            backupPaths.delete(f.sourcePath);
          }
          return !match;
        });

        sourcePaths = sourcePaths.filter(sp => {
          return stagedFiles.some(sf => sf.sourcePath === sp || sf.sourcePath.startsWith(sp + '/') || sf.sourcePath.startsWith(sp + '\\'));
        });

        virtualFolders = virtualFolders.filter(vf => {
          const match = vf === oldPath || vf.startsWith(oldPath + '/');
          if (match) {
            targetOverrides.delete(`__VIRTUAL_DIR__:${vf}`);
          }
          return !match;
        });

        renderWorkspace();
      }
    });
  });

  // 4. Rename File listener
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

  // 5. Skip File listener
  container.querySelectorAll('.packer-skip-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index || '0');
      toggleSkipFile(idx);
    });
  });

  // 6. Remove File listener
  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);

      const stillHasSource = stagedFiles.some(f => f.sourcePath === removedFile.sourcePath || f.sourcePath.startsWith(removedFile.sourcePath));
      if (!stillHasSource) {
        sourcePaths = sourcePaths.filter(p => p !== removedFile.sourcePath && !removedFile.sourcePath.startsWith(p));
      }
      targetOverrides.delete(removedFile.sourcePath);
      backupPaths.delete(removedFile.sourcePath);
      renderWorkspace();
    });
  });
}

function renderTreeHtml(node: any, depth = 0, currentPath = ''): string {
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

function setupTreeDragAndDropHandlers(container: HTMLElement): void {
  // ── Pointer-event based DnD (HTML5 drag API is broken in Tauri/WebView2) ──
  let draggedIndex: number | null = null;
  let draggedDirPath: string | null = null;
  let ghost: HTMLElement | null = null;
  let activeDropTarget: HTMLElement | null = null;

  function clearDropHighlights(): void {
    container.querySelectorAll('.packer-tree-dir.drag-over').forEach(n => n.classList.remove('drag-over'));
    const root = container.querySelector('.packer-tree-root');
    if (root) root.classList.remove('drag-over');
  }

  function createGhost(label: string): HTMLElement {
    const g = document.createElement('div');
    g.style.cssText = `
      position: fixed; pointer-events: none; z-index: 9998;
      background: rgba(0,188,255,0.15); border: 1px solid var(--accent);
      color: var(--text-primary); font-size: 12px; padding: 4px 10px;
      border-radius: 6px; backdrop-filter: blur(6px); white-space: nowrap;
      box-shadow: 0 4px 16px rgba(0,0,0,0.4);
    `;
    g.textContent = `📦 ${label}`;
    document.body.appendChild(g);
    return g;
  }

  function getDropTarget(e: PointerEvent): { folderPath: string | null; isRoot: boolean } {
    // Check if pointer is over a dir node
    const dirNodes = Array.from(container.querySelectorAll('.packer-tree-dir')) as HTMLElement[];
    for (const node of dirNodes) {
      const rect = node.getBoundingClientRect();
      if (e.clientX >= rect.left && e.clientX <= rect.right &&
          e.clientY >= rect.top && e.clientY <= rect.bottom) {
        return { folderPath: node.dataset.path || '', isRoot: false };
      }
    }
    // Check root
    const root = container.querySelector('.packer-tree-root') as HTMLElement;
    if (root) {
      const rect = root.getBoundingClientRect();
      if (e.clientX >= rect.left && e.clientX <= rect.right &&
          e.clientY >= rect.top && e.clientY <= rect.bottom) {
        return { folderPath: null, isRoot: true };
      }
    }
    return { folderPath: null, isRoot: false };
  }

  // ── Attach pointerdown on each file node's icon/name (not action buttons) ──
  container.querySelectorAll('.packer-tree-file').forEach(node => {
    const fileNode = node as HTMLElement;
    fileNode.style.cursor = 'grab';

    fileNode.addEventListener('pointerdown', (e: PointerEvent) => {
      // Ignore clicks on action buttons
      if ((e.target as HTMLElement).closest('.packer-tree-actions')) return;
      e.preventDefault();

      draggedIndex = parseInt(fileNode.dataset.index || '0', 10);
      draggedDirPath = null;

      const filename = stagedFiles[draggedIndex]?.sourcePath.split(/[/\\]/).pop() || 'file';
      ghost = createGhost(filename);
      ghost.style.left = (e.clientX + 12) + 'px';
      ghost.style.top = (e.clientY + 8) + 'px';

      fileNode.style.opacity = '0.4';
      container.setPointerCapture(e.pointerId);
    });
  });

  // ── Attach pointerdown on each dir node ──
  container.querySelectorAll('.packer-tree-dir').forEach(node => {
    const dirNode = node as HTMLElement;
    dirNode.style.cursor = 'grab';

    dirNode.addEventListener('pointerdown', (e: PointerEvent) => {
      if ((e.target as HTMLElement).closest('.packer-tree-actions')) return;
      e.preventDefault();

      draggedDirPath = dirNode.dataset.path || '';
      draggedIndex = null;

      const dirname = draggedDirPath.split('/').pop() || draggedDirPath;
      ghost = createGhost('📁 ' + dirname);
      ghost.style.left = (e.clientX + 12) + 'px';
      ghost.style.top = (e.clientY + 8) + 'px';

      dirNode.style.opacity = '0.4';
      container.setPointerCapture(e.pointerId);
    });
  });

  // ── Shared pointermove on container (pointer is captured) ──
  container.addEventListener('pointermove', (e: PointerEvent) => {
    if (!ghost) return;
    e.preventDefault();

    ghost.style.left = (e.clientX + 12) + 'px';
    ghost.style.top = (e.clientY + 8) + 'px';

    // Highlight drop target
    clearDropHighlights();
    const { folderPath, isRoot } = getDropTarget(e);
    if (folderPath !== null) {
      const dirNodes = Array.from(container.querySelectorAll('.packer-tree-dir')) as HTMLElement[];
      const target = dirNodes.find(n => n.dataset.path === folderPath);
      if (target) { target.classList.add('drag-over'); activeDropTarget = target; }
    } else if (isRoot) {
      const root = container.querySelector('.packer-tree-root') as HTMLElement;
      if (root) { root.classList.add('drag-over'); activeDropTarget = root; }
    } else {
      activeDropTarget = null;
    }
  });

  // ── Shared pointerup ──
  container.addEventListener('pointerup', (e: PointerEvent) => {
    if (!ghost) return;
    e.preventDefault();

    ghost.remove();
    ghost = null;
    clearDropHighlights();
    activeDropTarget = null;

    // Restore opacity of all nodes
    container.querySelectorAll('.packer-tree-file, .packer-tree-dir').forEach(n => {
      (n as HTMLElement).style.opacity = '1';
    });

    const { folderPath, isRoot } = getDropTarget(e);

    if (draggedIndex !== null && draggedIndex >= 0 && draggedIndex < stagedFiles.length) {
      const file = stagedFiles[draggedIndex];
      const filename = file.sourcePath.split(/[/\\]/).pop() || file.relativePath;

      if (folderPath !== null) {
        // Dropped on a specific folder
        const newTarget = `${folderPath}/${filename}`;
        file.targetPath = newTarget.replace(/\\/g, '/');
        targetOverrides.set(file.sourcePath, file.targetPath);
        showToast(`Moved ${filename} → ${folderPath}`, 'success');
        renderWorkspace();
      } else if (isRoot) {
        // Dropped on root
        file.targetPath = filename;
        targetOverrides.set(file.sourcePath, filename);
        showToast(`Moved ${filename} → root`, 'success');
        renderWorkspace();
      }
    } else if (draggedDirPath !== null) {
      const oldPath = draggedDirPath;

      if (folderPath !== null) {
        // Dropped on another folder
        if (folderPath === oldPath || folderPath.startsWith(oldPath + '/')) {
          showToast('Cannot move folder inside itself', 'warning');
        } else {
          const dirname = oldPath.split('/').pop() || oldPath;
          const newPath = `${folderPath}/${dirname}`;
          moveDirPath(oldPath, newPath);
          showToast(`Moved 📁 ${dirname} → ${folderPath}`, 'success');
          renderWorkspace();
        }
      } else if (isRoot) {
        // Dropped on root
        const dirname = oldPath.split('/').pop() || oldPath;
        moveDirPath(oldPath, dirname);
        showToast(`Moved 📁 ${dirname} → root`, 'success');
        renderWorkspace();
      }
    }

    draggedIndex = null;
    draggedDirPath = null;
  });

  container.addEventListener('pointercancel', () => {
    if (ghost) { ghost.remove(); ghost = null; }
    clearDropHighlights();
    container.querySelectorAll('.packer-tree-file, .packer-tree-dir').forEach(n => {
      (n as HTMLElement).style.opacity = '1';
    });
    draggedIndex = null;
    draggedDirPath = null;
    activeDropTarget = null;
  });
}

/** Moves all staged files and virtual folders from oldPath to newPath prefix */
function moveDirPath(oldPath: string, newPath: string): void {
  stagedFiles.forEach(f => {
    if (f.targetPath === oldPath) {
      f.targetPath = newPath;
      targetOverrides.set(f.sourcePath, newPath);
    } else if (f.targetPath.startsWith(oldPath + '/')) {
      f.targetPath = newPath + f.targetPath.substring(oldPath.length);
      targetOverrides.set(f.sourcePath, f.targetPath);
    }
  });

  virtualFolders = virtualFolders.map(vf => {
    if (vf === oldPath) return newPath;
    if (vf.startsWith(oldPath + '/')) return newPath + vf.substring(oldPath.length);
    return vf;
  });

  // Clean up stale virtual dir overrides for old path
  targetOverrides.forEach((_val, key) => {
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
}


function autoStructureWorkspace(): void {
  const modNameInput = document.getElementById('packer-meta-name') as HTMLInputElement;
  const modTypeSelect = document.getElementById('packer-meta-type') as HTMLSelectElement;

  const rawModName = modNameInput?.value.trim();
  const modType = modTypeSelect?.value;

  if (!rawModName) {
    showToast('Please fill in the Mod Name first to determine folder structure', 'warning');
    return;
  }

  const modName = rawModName.replace(/[^a-zA-Z0-9_]/g, '');

  stagedFiles.forEach(file => {
    const filename = file.sourcePath.split(/[/\\]/).pop() || '';
    const ext = filename.split('.').pop()?.toLowerCase();

    let target = file.targetPath;

    if (modType === 'ue4ss') {
      if (ext === 'lua') {
        if (filename.toLowerCase() === 'main.lua') {
          target = `Mods/${modName}/Scripts/main.lua`;
        } else {
          const lowerPath = file.relativePath.toLowerCase();
          const scriptsIndex = lowerPath.indexOf('scripts/');
          if (scriptsIndex !== -1) {
            target = `Mods/${modName}/${file.relativePath.substring(scriptsIndex)}`;
          } else {
            target = `Mods/${modName}/Scripts/${filename}`;
          }
        }
      } else if (ext === 'dll') {
        target = `Mods/${modName}/${filename}`;
      } else {
        target = `Mods/${modName}/${filename}`;
      }
    } else if (modType === 'palschema') {
      if (ext === 'json' || ext === 'jsonc') {
        target = `Mods/PalSchema/mods/${filename}`;
      } else {
        target = `Mods/PalSchema/mods/${filename}`;
      }
    } else if (modType === 'pak') {
      if (ext === 'pak') {
        target = `Pal/Content/Paks/~mods/${filename}`;
      } else {
        target = filename;
      }
    } else if (modType === 'logicmods') {
      if (ext === 'pak') {
        target = `Pal/Content/Paks/LogicMods/${filename}`;
      } else {
        target = filename;
      }
    } else if (modType === 'hybrid') {
      if (ext === 'lua') {
        target = `Mods/${modName}/Scripts/${filename}`;
      } else if (ext === 'pak') {
        target = `Pal/Content/Paks/~mods/${filename}`;
      } else if (ext === 'json' || ext === 'jsonc') {
        target = `Mods/PalSchema/mods/${filename}`;
      } else {
        target = filename;
      }
    }
    
    target = target.replace(/\\/g, '/');
    file.targetPath = target;
    targetOverrides.set(file.sourcePath, target);
  });

  renderWorkspace();
  showToast('Workspace structured automatically based on mod type', 'success');
}

async function buildModPackage(): Promise<void> {
  const name = (document.getElementById('packer-meta-name') as HTMLInputElement)?.value.trim();
  const version = (document.getElementById('packer-meta-version') as HTMLInputElement)?.value.trim();
  const description = (document.getElementById('packer-meta-desc') as HTMLTextAreaElement)?.value.trim();
  const author = (document.getElementById('packer-meta-author') as HTMLInputElement)?.value.trim();
  const modType = (document.getElementById('packer-meta-type') as HTMLSelectElement)?.value;
  const nexusIdStr = (document.getElementById('packer-meta-nexus-id') as HTMLInputElement)?.value.trim();
  const nexusModId = nexusIdStr ? parseInt(nexusIdStr, 10) : null;
  const formatSelect = document.getElementById('packer-format-select') as HTMLSelectElement;
  const format = formatSelect?.value || 'zip';

  if (!name || !version || !modType) {
    showToast('Name, Version, and Mod Type are required.', 'warning');
    return;
  }

  if (stagedFiles.length === 0) {
    showToast('Workspace is empty. Add files first.', 'warning');
    return;
  }

  try {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const defaultFilename = `${name.replace(/[^a-zA-Z0-9_-]/g, '_')}_v${version}.${format}`;
    const outputLocation = await save({
      defaultPath: defaultFilename,
      filters: [{
        name: format.toUpperCase() + ' Archive',
        extensions: [format]
      }],
      title: 'Save Packaged Mod'
    });

    if (!outputLocation) {
      return;
    }

    const activeFiles = stagedFiles.filter(f => f.targetPath !== '__SKIP__');
    if (activeFiles.length === 0) {
      showToast('All files are skipped/omitted. Nothing to package.', 'warning');
      return;
    }

    const routes = activeFiles.map(f => {
      let routeType = 'passthrough';
      const pathLower = f.targetPath.toLowerCase();
      if (pathLower.includes('ue4ss/mods/') || pathLower.includes('ue4ss/scripts/') || pathLower.endsWith('.lua') || pathLower.endsWith('.dll')) {
        routeType = 'ue4ss';
      } else if (pathLower.includes('palschema/mods/') || pathLower.includes('palschema/')) {
        routeType = 'palschema';
      } else if (pathLower.endsWith('.pak')) {
        if (pathLower.includes('logicmods')) {
          routeType = 'logicmods';
        } else {
          routeType = 'pak';
        }
      }
      return {
        zipPath: f.targetPath,
        routeType
      };
    });

    const metadata: ModMetadata = {
      name,
      version,
      description,
      author,
      modType,
      nexusModId: isNaN(nexusModId as any) ? null : nexusModId,
      routes
    };

    showToast(`Packaging mod to ${format.toUpperCase()}... Please wait.`, 'info');

    const result = await invoke<string>('pack_mod', {
      files: activeFiles,
      metadata,
      outputPath: outputLocation,
      format
    });

    showToast(result, 'success');
  } catch (err: any) {
    console.error(err);
    showToast(`Packaging failed: ${err}`, 'error');
  }
}

// PROJECTS HUB MANAGEMENT
async function loadProjectsList(): Promise<void> {
  try {
    const list = await invoke<PackerProject[]>('load_packer_projects');
    savedProjects = list;
    renderProjectsHub();
  } catch (err) {
    console.error('Failed to load packer projects:', err);
  }
}

function renderProjectsHub(): void {
  const grid = document.getElementById('packer-projects-grid');
  if (!grid) return;

  let html = savedProjects.map(proj => {
    const typeBadge = proj.metadata?.modType ? `<span class="mod-type-badge" style="font-size:10px; padding:2px 6px; margin-top:6px;">${escapeHtml(proj.metadata.modType)}</span>` : '';
    return `
      <div class="packer-project-card" data-name="${escapeHtml(proj.name)}">
        <div class="packer-project-card-icon">📁</div>
        <div class="packer-project-card-title">${escapeHtml(proj.name)}</div>
        <div class="packer-project-card-meta">Version: ${escapeHtml(proj.metadata?.version || '1.0.0')}</div>
        ${typeBadge}
        <div class="packer-project-card-actions">
          <button class="packer-project-card-btn primary" data-action="open" data-name="${escapeHtml(proj.name)}">Open</button>
          <button class="packer-project-card-btn danger" data-action="delete" data-name="${escapeHtml(proj.name)}">Delete</button>
        </div>
      </div>
    `;
  }).join('');

  // Add Create New placeholder card
  html += `
    <div class="packer-project-card new-placeholder" id="packer-hub-create-card">
      <div class="packer-project-card-icon">+</div>
      <div class="packer-project-card-title">New Project</div>
      <div class="packer-project-card-meta">Create a blank stash</div>
    </div>
  `;

  grid.innerHTML = html;

  // Double click project card to open
  grid.querySelectorAll('.packer-project-card:not(.new-placeholder)').forEach(card => {
    card.addEventListener('dblclick', () => {
      const name = (card as HTMLElement).dataset.name || '';
      loadSelectedProject(name);
    });
  });

  // Action button click handlers
  grid.querySelectorAll('.packer-project-card-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const action = (btn as HTMLElement).dataset.action;
      const name = (btn as HTMLElement).dataset.name || '';
      if (action === 'open') {
        loadSelectedProject(name);
      } else if (action === 'delete') {
        deleteProjectByName(name);
      }
    });
  });

  // Create card listener
  document.getElementById('packer-hub-create-card')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });
}

function showProjectsHub(): void {
  document.getElementById('packer-projects-hub')!.style.display = 'flex';
  document.getElementById('packer-workspace-view')!.style.display = 'none';
  loadProjectsList();
}

function openNewProjectWorkspace(): void {
  activeProjectName = '';
  stagedFiles = [];
  sourcePaths = [];
  targetOverrides.clear();
  virtualFolders = [];
  backupPaths.clear();
  clearMetadataForm();
  
  const nameInput = document.getElementById('packer-project-name') as HTMLInputElement;
  if (nameInput) nameInput.value = '';

  document.getElementById('packer-workspace-title')!.textContent = 'New Project / Staging Area';
  document.getElementById('packer-projects-hub')!.style.display = 'none';
  document.getElementById('packer-workspace-view')!.style.display = 'flex';
  renderWorkspace();
}

function loadSelectedProject(name: string): void {
  const project = savedProjects.find(p => p.name === name);
  if (!project) return;

  activeProjectName = name;
  sourcePaths = [...project.sourcePaths];
  
  // Re-build Map overrides, virtual folders, and skipped files backup paths
  targetOverrides = new Map();
  virtualFolders = [];
  backupPaths = new Map();

  if (project.targetPathsOverride) {
    Object.entries(project.targetPathsOverride).forEach(([k, v]) => {
      if (k.startsWith('__VIRTUAL_DIR__:')) {
        virtualFolders.push(k.substring('__VIRTUAL_DIR__:'.length));
        targetOverrides.set(k, v);
      } else if (k.startsWith('__SKIP_ORIGINAL__:')) {
        backupPaths.set(k.substring('__SKIP_ORIGINAL__:'.length), v);
      } else {
        targetOverrides.set(k, v);
      }
    });
  }

  // Restore metadata
  if (project.metadata) {
    const m = project.metadata;
    (document.getElementById('packer-meta-name') as HTMLInputElement).value = m.name || '';
    (document.getElementById('packer-meta-version') as HTMLInputElement).value = m.version || '1.0.0';
    (document.getElementById('packer-meta-author') as HTMLInputElement).value = m.author || '';
    (document.getElementById('packer-meta-type') as HTMLSelectElement).value = m.modType || '';
    (document.getElementById('packer-meta-nexus-id') as HTMLInputElement).value = m.nexusModId ? String(m.nexusModId) : '';
    (document.getElementById('packer-meta-desc') as HTMLTextAreaElement).value = m.description || '';
  } else {
    clearMetadataForm();
  }

  const formatSelect = document.getElementById('packer-format-select') as HTMLSelectElement;
  if (formatSelect) {
    formatSelect.value = project.format || 'zip';
  }

  const nameInput = document.getElementById('packer-project-name') as HTMLInputElement;
  if (nameInput) nameInput.value = name;

  document.getElementById('packer-workspace-title')!.textContent = `Project: ${name}`;
  document.getElementById('packer-projects-hub')!.style.display = 'none';
  document.getElementById('packer-workspace-view')!.style.display = 'flex';
  
  // Automatically trigger directory re-scanning
  scanAndBuildStagedFiles().then(() => {
    showToast(`Loaded project '${name}' and scanned folders`, 'info');
  });
}

async function saveCurrentProject(): Promise<void> {
  const nameInput = document.getElementById('packer-project-name') as HTMLInputElement;
  let projName = nameInput?.value.trim();

  if (!projName) {
    const modName = (document.getElementById('packer-meta-name') as HTMLInputElement)?.value.trim();
    if (modName) {
      projName = modName;
    } else {
      showToast('Please enter a Project Name to save.', 'warning');
      return;
    }
  }

  const metaName = (document.getElementById('packer-meta-name') as HTMLInputElement)?.value.trim();
  const metaVersion = (document.getElementById('packer-meta-version') as HTMLInputElement)?.value.trim();
  const metaAuthor = (document.getElementById('packer-meta-author') as HTMLInputElement)?.value.trim();
  const metaType = (document.getElementById('packer-meta-type') as HTMLSelectElement)?.value;
  const metaDesc = (document.getElementById('packer-meta-desc') as HTMLTextAreaElement)?.value.trim();
  const metaNexusIdStr = (document.getElementById('packer-meta-nexus-id') as HTMLInputElement)?.value.trim();
  const metaNexusId = metaNexusIdStr ? parseInt(metaNexusIdStr, 10) : null;
  const formatSelect = document.getElementById('packer-format-select') as HTMLSelectElement;
  const format = formatSelect?.value || 'zip';

  const metadata: ModMetadata | null = metaName ? {
    name: metaName,
    version: metaVersion || '1.0.0',
    author: metaAuthor,
    modType: metaType,
    description: metaDesc,
    nexusModId: isNaN(metaNexusId as any) ? null : metaNexusId
  } : null;

  // Convert targetOverrides Map to Record/Object for serialization
  const overridesRecord: Record<string, string> = {};
  targetOverrides.forEach((v, k) => {
    overridesRecord[k] = v;
  });
  backupPaths.forEach((v, k) => {
    overridesRecord[`__SKIP_ORIGINAL__:${k}`] = v;
  });
  virtualFolders.forEach(vf => {
    overridesRecord[`__VIRTUAL_DIR__:${vf}`] = '__VIRTUAL_DIR__';
  });

  try {
    const res = await invoke<string>('save_packer_project', {
      projectName: projName,
      metadata,
      sourcePaths,
      targetPathsOverride: overridesRecord,
      format
    });

    activeProjectName = projName;
    document.getElementById('packer-workspace-title')!.textContent = `Project: ${projName}`;
    showToast(res, 'success');
  } catch (err: any) {
    showToast(`Failed to save project: ${err}`, 'error');
  }
}

async function deleteProjectByName(name: string): Promise<void> {
  try {
    const res = await invoke<string>('delete_packer_project', { projectName: name });
    if (activeProjectName === name) {
      activeProjectName = '';
      stagedFiles = [];
      sourcePaths = [];
      targetOverrides.clear();
      clearMetadataForm();
      showProjectsHub();
    } else {
      loadProjectsList();
    }
    showToast(res, 'success');
  } catch (err: any) {
    showToast(`Failed to delete project: ${err}`, 'error');
  }
}

function clearMetadataForm(): void {
  (document.getElementById('packer-meta-name') as HTMLInputElement).value = '';
  (document.getElementById('packer-meta-version') as HTMLInputElement).value = '1.0.0';
  (document.getElementById('packer-meta-author') as HTMLInputElement).value = '';
  (document.getElementById('packer-meta-nexus-id') as HTMLInputElement).value = '';
  (document.getElementById('packer-meta-type') as HTMLSelectElement).value = '';
  (document.getElementById('packer-meta-desc') as HTMLTextAreaElement).value = '';
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
