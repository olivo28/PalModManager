import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';

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

  ['dragenter', 'dragover', 'dragleave', 'drop'].forEach(eventName => {
    container.addEventListener(eventName, preventDefaults, false);
  });

  function preventDefaults(e: Event) {
    e.preventDefault();
    e.stopPropagation();
  }

  container.addEventListener('dragenter', () => {
    overlay.classList.add('drag-over');
  }, false);

  container.addEventListener('dragover', () => {
    overlay.classList.add('drag-over');
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
    renderWorkspace();
    showToast('Staged workspace cleared', 'info');
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
  const inputs = ['packer-meta-name', 'packer-meta-version', 'packer-meta-author'];
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

  if (mode === 'list') {
    listBtn?.classList.add('active');
    treeBtn?.classList.remove('active');
    if (listTable) listTable.style.display = 'table';
    if (treeView) treeView.style.display = 'none';
  } else {
    listBtn?.classList.remove('active');
    treeBtn?.classList.add('active');
    if (listTable) listTable.style.display = 'none';
    if (treeView) treeView.style.display = 'block';
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
    return `
      <tr data-index="${index}">
        <td title="${escapeHtml(file.sourcePath)}" style="max-width: 250px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
          <strong>${escapeHtml(filename)}</strong>
          <div style="font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis;">${escapeHtml(file.sourcePath)}</div>
        </td>
        <td style="white-space: nowrap;">${formatBytes(file.size)}</td>
        <td>
          <input type="text" class="packer-input-target" value="${escapeHtml(file.targetPath)}" data-index="${index}" />
        </td>
        <td>
          <button class="packer-remove-file-btn" data-index="${index}" title="Remove file">✕</button>
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
      // Stash this override
      targetOverrides.set(stagedFiles[idx].sourcePath, val);
    });
  });

  // Add listeners to remove buttons
  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);
      
      // If we removed all files originating from a specific source path, remove that source path
      // Note: for folders, we scan recursively. We can check if any stagedFile still contains the sourcePath.
      const stillHasSource = stagedFiles.some(f => f.sourcePath === removedFile.sourcePath || f.sourcePath.startsWith(removedFile.sourcePath));
      if (!stillHasSource) {
        sourcePaths = sourcePaths.filter(p => p !== removedFile.sourcePath && !removedFile.sourcePath.startsWith(p));
      }
      targetOverrides.delete(removedFile.sourcePath);

      renderWorkspace();
    });
  });
}

function renderTreeMode(): void {
  const container = document.getElementById('packer-tree-view');
  if (!container) return;

  const root: any = { name: 'root', isDir: true, children: {} };

  stagedFiles.forEach((file, index) => {
    const parts = file.targetPath.replace(/\\/g, '/').split('/').filter(p => p.trim() !== '');
    let current = root;

    parts.forEach((part, i) => {
      const isLast = i === parts.length - 1;
      if (isLast) {
        current.children[part] = { name: part, isDir: false, file, index };
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

  container.querySelectorAll('.packer-remove-file-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const idx = parseInt((e.currentTarget as HTMLButtonElement).dataset.index || '0');
      const removedFile = stagedFiles[idx];
      stagedFiles.splice(idx, 1);
      targetOverrides.delete(removedFile.sourcePath);
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
        </div>
      `;
      html += renderTreeHtml(child, depth + 1, nextPath);
    } else {
      const sizeStr = formatBytes(child.file.size);
      html += `
        <div class="packer-tree-node packer-tree-file" style="padding-left: ${depth * 16}px;" draggable="true" data-index="${child.index}">
          <span class="packer-tree-icon">📄</span>
          <span class="packer-tree-name" title="Source: ${escapeHtml(child.file.sourcePath)}">${escapeHtml(child.name)}</span>
          <span class="packer-tree-size">${sizeStr}</span>
          <button class="packer-remove-file-btn" data-index="${child.index}" title="Remove file" style="margin-left: 8px; padding: 2px 6px; font-size: 11px;">✕</button>
        </div>
      `;
    }
  });
  return html;
}

function setupTreeDragAndDropHandlers(container: HTMLElement): void {
  let draggedIndex: number | null = null;

  container.querySelectorAll('.packer-tree-file').forEach(node => {
    node.addEventListener('dragstart', (e) => {
      const dragEvent = e as DragEvent;
      draggedIndex = parseInt((node as HTMLElement).dataset.index || '0');
      if (dragEvent.dataTransfer) {
        dragEvent.dataTransfer.effectAllowed = 'move';
        dragEvent.dataTransfer.setData('text/plain', draggedIndex.toString());
      }
      (node as HTMLElement).style.opacity = '0.5';
    });

    node.addEventListener('dragend', () => {
      (node as HTMLElement).style.opacity = '1';
      draggedIndex = null;
    });
  });

  container.querySelectorAll('.packer-tree-dir').forEach(node => {
    node.addEventListener('dragover', (e) => {
      e.preventDefault();
      const dragEvent = e as DragEvent;
      if (dragEvent.dataTransfer) {
        dragEvent.dataTransfer.dropEffect = 'move';
      }
      (node as HTMLElement).classList.add('drag-over');
    });

    node.addEventListener('dragenter', (e) => {
      e.preventDefault();
      (node as HTMLElement).classList.add('drag-over');
    });

    node.addEventListener('dragleave', () => {
      (node as HTMLElement).classList.remove('drag-over');
    });

    node.addEventListener('drop', async (e) => {
      e.preventDefault();
      (node as HTMLElement).classList.remove('drag-over');

      const targetFolderPath = (node as HTMLElement).dataset.path || '';
      
      if (draggedIndex !== null && draggedIndex >= 0 && draggedIndex < stagedFiles.length) {
        const file = stagedFiles[draggedIndex];
        const filename = file.sourcePath.split(/[/\\]/).pop() || file.relativePath;
        const newTarget = targetFolderPath ? `${targetFolderPath}/${filename}` : filename;

        file.targetPath = newTarget.replace(/\\/g, '/');
        targetOverrides.set(file.sourcePath, file.targetPath);
        
        showToast(`Moved ${filename} to ${targetFolderPath || 'root'}`, 'success');
        renderWorkspace();
      }
    });
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

    const metadata: ModMetadata = {
      name,
      version,
      description,
      author,
      modType
    };

    showToast(`Packaging mod to ${format.toUpperCase()}... Please wait.`, 'info');

    const result = await invoke<string>('pack_mod', {
      files: stagedFiles,
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
  
  // Re-build Map overrides
  targetOverrides = new Map(Object.entries(project.targetPathsOverride || {}));

  // Restore metadata
  if (project.metadata) {
    const m = project.metadata;
    (document.getElementById('packer-meta-name') as HTMLInputElement).value = m.name || '';
    (document.getElementById('packer-meta-version') as HTMLInputElement).value = m.version || '1.0.0';
    (document.getElementById('packer-meta-author') as HTMLInputElement).value = m.author || '';
    (document.getElementById('packer-meta-type') as HTMLSelectElement).value = m.modType || '';
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
  const formatSelect = document.getElementById('packer-format-select') as HTMLSelectElement;
  const format = formatSelect?.value || 'zip';

  const metadata: ModMetadata | null = metaName ? {
    name: metaName,
    version: metaVersion || '1.0.0',
    author: metaAuthor,
    modType: metaType,
    description: metaDesc
  } : null;

  // Convert targetOverrides Map to Record/Object for serialization
  const overridesRecord: Record<string, string> = {};
  targetOverrides.forEach((v, k) => {
    overridesRecord[k] = v;
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
