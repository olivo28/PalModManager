import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../toast';
import { showConfirm, showPrompt } from '../confirm';
import { StagedFile, addStagedPaths, scanAndBuildStagedFiles, toggleSkipFile, autoStructureWorkspace } from './staging';
import { renderWorkspace, renderListMode, renderTreeMode, renderTreeHtml, formatBytes, clearMetadataForm, updateBuildButtonState } from './rendering';
import { setupPackerDragAndDrop, setupTreeDragAndDropHandlers } from './dragDrop';
export interface ModMetadata {
  name: string;
  version: string;
  description: string;
  author: string;
  modType: string;
  nexusModId?: number | null;
}

export interface PackerProject {
  name: string;
  metadata: ModMetadata | null;
  sourcePaths: string[];
  targetPathsOverride: Record<string, string>;
  format: string;
}

import { loadProjectsList, renderProjectsHub, showProjectsHub, openNewProjectWorkspace, loadSelectedProject, saveCurrentProject, deleteProjectByName } from './projects';
export { addStagedPaths };
export let stagedFiles: StagedFile[] = [];
export let sourcePaths: string[] = [];
export let targetOverrides: Map<string, string> = new Map();
export let virtualFolders: string[] = [];
export let backupPaths: Map<string, string> = new Map();

export let viewMode: 'list' | 'tree' = 'list';
export let savedProjects: PackerProject[] = [];
export let activeProjectName: string = '';

// State Setters to avoid read-only bindings across files
export function setStagedFiles(files: StagedFile[]): void {
  stagedFiles.length = 0;
  stagedFiles.push(...files);
}
export function setSourcePaths(paths: string[]): void {
  sourcePaths.length = 0;
  sourcePaths.push(...paths);
}
export function setVirtualFolders(folders: string[]): void {
  virtualFolders.length = 0;
  virtualFolders.push(...folders);
}
export function setBackupPaths(paths: Map<string, string>): void {
  backupPaths.clear();
  paths.forEach((v, k) => backupPaths.set(k, v));
}
export function setViewMode(mode: 'list' | 'tree'): void { viewMode = mode; }
export function setSavedProjects(projects: PackerProject[]): void {
  savedProjects.length = 0;
  savedProjects.push(...projects);
}
export function setActiveProject(name: string): void { activeProjectName = name; }

export function initPackerView(): void {
  setupPackerDragAndDrop();
  setupPackerEventListeners();
  loadProjectsList();
}

function setupPackerEventListeners(): void {
  document.getElementById('packer-workspace-back-btn')?.addEventListener('click', () => {
    showProjectsHub();
  });

  document.getElementById('packer-hub-new-btn')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });

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

  document.getElementById('packer-add-folder-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        directory: true,
        title: 'Select Folder to Add to Project'
      });
      if (selected) {
        const paths = Array.isArray(selected) ? [selected] : [selected as any];
        await addStagedPaths(paths);
      }
    } catch (err) {
      console.error(err);
    }
  });

  document.getElementById('packer-new-virtual-folder-btn')?.addEventListener('click', async () => {
    const folderPath = await showPrompt("Enter virtual folder path (e.g. 'Mods/MyMod'):");
    if (folderPath && folderPath.trim()) {
      const cleaned = folderPath.trim().replace(/\\/g, '/');
      if (!virtualFolders.includes(cleaned)) {
        virtualFolders.push(cleaned);
        targetOverrides.set(`__VIRTUAL_DIR__:${cleaned}`, '__VIRTUAL_DIR__');
        renderWorkspace();
      }
    }
  });

  document.getElementById('packer-clear-btn')?.addEventListener('click', async () => {
    const confirmed = await showConfirm('Clear current staging area? unsaved overrides will be lost.');
    if (confirmed) {
      stagedFiles = [];
      sourcePaths = [];
      targetOverrides.clear();
      virtualFolders = [];
      backupPaths.clear();
      renderWorkspace();
    }
  });

  document.getElementById('packer-autostruct-btn')?.addEventListener('click', () => {
    autoStructureWorkspace();
  });

  document.getElementById('packer-project-save-btn')?.addEventListener('click', () => {
    saveCurrentProject();
  });

  document.getElementById('packer-build-btn')?.addEventListener('click', async () => {
    if (stagedFiles.length === 0) {
      showToast('No files staged to pack.', 'warning');
      return;
    }

    const formatSelect = document.getElementById('packer-format-select') as HTMLSelectElement;
    const format = formatSelect?.value || 'zip';

    const metaName = (document.getElementById('packer-meta-name') as HTMLInputElement)?.value.trim();
    const metaVersion = (document.getElementById('packer-meta-version') as HTMLInputElement)?.value.trim() || '1.0.0';
    const metaAuthor = (document.getElementById('packer-meta-author') as HTMLInputElement)?.value.trim();
    const metaType = (document.getElementById('packer-meta-type') as HTMLSelectElement)?.value;
    const metaDesc = (document.getElementById('packer-meta-desc') as HTMLTextAreaElement)?.value.trim();
    const metaNexusIdStr = (document.getElementById('packer-meta-nexus-id') as HTMLInputElement)?.value.trim();
    const metaNexusId = metaNexusIdStr ? parseInt(metaNexusIdStr, 10) : null;

    const metadata = metaName ? {
      name: metaName,
      version: metaVersion,
      author: metaAuthor,
      modType: metaType,
      description: metaDesc,
      nexusModId: isNaN(metaNexusId as any) ? null : metaNexusId
    } : null;

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const destPath = await save({
        title: 'Save packed mod archive',
        filters: [{ name: 'Mod Archive', extensions: [format] }],
        defaultPath: metaName ? `${metaName}_v${metaVersion}.${format}` : `packed_mod.${format}`
      });

      if (!destPath) return;

      const btn = document.getElementById('packer-build-btn') as HTMLButtonElement;
      btn.disabled = true;
      btn.textContent = 'Packing...';
      showToast('Packing mod archive, please wait...', 'info');

      const overridesRecord: Record<string, string> = {};
      targetOverrides.forEach((v, k) => { overridesRecord[k] = v; });
      backupPaths.forEach((v, k) => { overridesRecord[`__SKIP_ORIGINAL__:${k}`] = v; });
      virtualFolders.forEach(vf => { overridesRecord[`__VIRTUAL_DIR__:${vf}`] = '__VIRTUAL_DIR__'; });

      const res = await invoke<string>('pack_staged_mod', {
        destPath,
        metadata,
        sourcePaths,
        targetPathsOverride: overridesRecord,
        format
      });

      showToast(res, 'success');
    } catch (err: any) {
      console.error(err);
      showToast(`Failed to pack mod: ${err}`, 'error');
    } finally {
      const btn = document.getElementById('packer-build-btn') as HTMLButtonElement;
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = '📦 Package Mod';
      }
    }
  });

  const listTab = document.getElementById('packer-view-list-btn');
  const treeTab = document.getElementById('packer-view-tree-btn');

  if (listTab && treeTab) {
    listTab.addEventListener('click', () => {
      viewMode = 'list';
      renderWorkspace();
    });
    treeTab.addEventListener('click', () => {
      viewMode = 'tree';
      renderWorkspace();
    });
  }

  // Automatically update build button disabled state on form changes
  const inputs = ['packer-meta-name', 'packer-meta-version', 'packer-meta-author', 'packer-meta-nexus-id'];
  inputs.forEach(id => {
    document.getElementById(id)?.addEventListener('input', updateBuildButtonState);
  });
  document.getElementById('packer-meta-type')?.addEventListener('change', updateBuildButtonState);
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export { renderWorkspace };
