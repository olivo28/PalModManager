import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../toast';
import { showConfirm, showPrompt } from '../confirm';
import { t } from '../../utils/i18n';
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
  routes?: Array<{ zipPath: string; routeType: string }>;
}

export interface PackerProject {
  name: string;
  metadata: ModMetadata | null;
  sourcePaths: string[];
  targetPathsOverride: Record<string, string>;
  format: string;
}

import { loadProjectsList, renderProjectsHub, showProjectsHub, openNewProjectWorkspace, loadSelectedProject, saveCurrentProject, deleteProjectByName } from './projects';
import { packerDom, bus, type PackerDomMap } from '../../framework';
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
  packerDom.elMaybe('packer-workspace-back-btn')?.addEventListener('click', () => {
    showProjectsHub();
  });

  packerDom.elMaybe('packer-hub-new-btn')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });

  packerDom.elMaybe('packer-add-files-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: true,
        directory: false,
        title: t('packer.dialog_select_files_title')
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        await addStagedPaths(paths);
      }
    } catch (err) {
      console.error(err);
    }
  });

  packerDom.elMaybe('packer-add-folder-btn')?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        directory: true,
        title: t('packer.dialog_select_folder_title')
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected as any];
        await addStagedPaths(paths);
      }
    } catch (err) {
      console.error(err);
    }
  });

  packerDom.elMaybe('packer-new-virtual-folder-btn')?.addEventListener('click', async () => {
    const input = await showPrompt(t('packer.prompt_virtual_folder'));
    if (input && input.trim()) {
      const cleaned = input.trim().replace(/\\/g, '/');
      if (!virtualFolders.includes(cleaned)) {
        virtualFolders.push(cleaned);
        targetOverrides.set(`__VIRTUAL_DIR__:${cleaned}`, '__VIRTUAL_DIR__');
        renderWorkspace();
      }
    }
  });

  packerDom.elMaybe('packer-clear-btn')?.addEventListener('click', async () => {
    const confirmed = await showConfirm(t('packer.confirm_clear_staging'));
    if (confirmed) {
      stagedFiles = [];
      sourcePaths = [];
      targetOverrides.clear();
      virtualFolders = [];
      backupPaths.clear();
      renderWorkspace();
    }
  });

  packerDom.elMaybe('packer-autostruct-btn')?.addEventListener('click', () => {
    autoStructureWorkspace();
  });

  packerDom.elMaybe('packer-project-save-btn')?.addEventListener('click', () => {
    saveCurrentProject();
  });

  packerDom.elMaybe('packer-build-btn')?.addEventListener('click', async () => {
    if (stagedFiles.length === 0) {
      showToast(t('packer.toast_no_files_staged'), 'warning');
      return;
    }

    const formatSelect = packerDom.elMaybe('packer-format-select') as HTMLSelectElement;
    const format = formatSelect?.value || 'zip';

    const metaName = (packerDom.elMaybe('packer-meta-name') as HTMLInputElement)?.value.trim();
    const metaVersion = (packerDom.elMaybe('packer-meta-version') as HTMLInputElement)?.value.trim() || '1.0.0';
    const metaAuthor = (packerDom.elMaybe('packer-meta-author') as HTMLInputElement)?.value.trim();
    const metaType = (packerDom.elMaybe('packer-meta-type') as HTMLSelectElement)?.value;
    const metaDesc = (packerDom.elMaybe('packer-meta-desc') as HTMLTextAreaElement)?.value.trim();
    const metaNexusIdStr = (packerDom.elMaybe('packer-meta-nexus-id') as HTMLInputElement)?.value.trim();
    const metaNexusId = metaNexusIdStr ? parseInt(metaNexusIdStr, 10) : null;

    // Compute routes based on staged files
    const activeFiles = stagedFiles.filter(f => f.targetPath !== '__SKIP__');
    const computedRoutes: Array<{ zipPath: string; routeType: string }> = [];

    activeFiles.forEach(f => {
      const cleanTarget = f.targetPath.replace(/\\/g, '/');
      const lower = cleanTarget.toLowerCase();
      let routeType = 'passthrough';

      if (
        lower.startsWith('palschema/') ||
        lower.startsWith('mods/palschema/') ||
        lower.includes('/palschema/') ||
        lower.includes('/blueprints/') ||
        lower.includes('/items/') ||
        lower.includes('/raw/') ||
        lower.includes('/translations/')
      ) {
        routeType = 'palschema';
      } else if (
        lower.startsWith('ue4ss/') ||
        lower.startsWith('mods/') ||
        lower.includes('/scripts/') ||
        lower.endsWith('.lua') ||
        lower.endsWith('enabled.txt')
      ) {
        routeType = 'ue4ss';
      } else if (
        lower.startsWith('pal/content/paks/logicmods/') ||
        lower.includes('/logicmods/')
      ) {
        routeType = 'logicmods';
      } else if (
        lower.startsWith('pal/content/paks/~mods/') ||
        lower.startsWith('pal/content/paks/') ||
        lower.endsWith('.pak') ||
        lower.endsWith('.ucas') ||
        lower.endsWith('.utoc')
      ) {
        routeType = 'pak';
      }

      computedRoutes.push({
        zipPath: cleanTarget,
        routeType
      });
    });

    const metadata: ModMetadata | null = metaName ? {
      name: metaName,
      version: metaVersion,
      author: metaAuthor,
      modType: metaType,
      description: metaDesc,
      nexusModId: isNaN(metaNexusId as any) ? null : metaNexusId,
      routes: computedRoutes.length > 0 ? computedRoutes : undefined
    } : null;

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const destPath = await save({
        title: t('packer.dialog_save_archive_title'),
        filters: [{ name: 'Mod Archive', extensions: [format] }],
        defaultPath: metaName ? `${metaName}_v${metaVersion}.${format}` : `packed_mod.${format}`
      });

      if (!destPath) return;

      const btn = packerDom.elMaybe('packer-build-btn') as HTMLButtonElement;
      if (btn) {
        btn.disabled = true;
        btn.textContent = '...';
      }
      showToast(t('packer.toast_packing_wait'), 'info');

      const filesToPack = activeFiles.map(f => ({
        sourcePath: f.sourcePath,
        relativePath: f.relativePath,
        size: f.size,
        targetPath: f.targetPath
      }));

      const res = await invoke<string>('pack_mod', {
        files: filesToPack,
        metadata,
        outputPath: destPath,
        format
      });

      bus.emit('project:packed', {
        outputPath: destPath,
        format,
        modName: metadata?.name || packerDom.elMaybe('packer-project-name')?.value.trim() || 'Mod'
      });

      showToast(t('packer.toast_pack_success'), 'success');
    } catch (err: any) {
      console.error(err);
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    } finally {
      const btn = packerDom.elMaybe('packer-build-btn');
      if (btn) {
        btn.disabled = false;
        btn.innerHTML = `📦 ${escapeHtml(t('packer.btn_package_mod'))}`;
      }
    }
  });

  const listTab = packerDom.elMaybe('packer-view-list-btn');
  const treeTab = packerDom.elMaybe('packer-view-tree-btn');

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
  const inputs: (keyof PackerDomMap)[] = ['packer-meta-name', 'packer-meta-version', 'packer-meta-author', 'packer-meta-nexus-id'];
  inputs.forEach(id => {
    packerDom.elMaybe(id)?.addEventListener('input', updateBuildButtonState);
  });
  packerDom.elMaybe('packer-meta-type')?.addEventListener('change', updateBuildButtonState);
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
