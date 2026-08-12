import { invoke } from '@tauri-apps/api/core';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { stagedFiles, sourcePaths, targetOverrides, virtualFolders, backupPaths, format, activeProjectName, savedProjects, setStagedFiles, setSourcePaths, setVirtualFolders, setBackupPaths, setActiveProject, setSavedProjects, renderWorkspace, escapeHtml } from './mod';
import { scanAndBuildStagedFiles } from './staging';
import { clearMetadataForm } from './rendering';

import { ModMetadata, PackerProject } from './mod';

export async function loadProjectsList(): Promise<void> {
  try {
    const list = await invoke<PackerProject[]>('load_packer_projects');
    setSavedProjects(list);
    renderProjectsHub();
  } catch (err) {
    console.error('Failed to load packer projects:', err);
  }
}

export function renderProjectsHub(): void {
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

  html += `
    <div class="packer-project-card new-placeholder" id="packer-hub-create-card">
      <div class="packer-project-card-icon">+</div>
      <div class="packer-project-card-title">New Project</div>
      <div class="packer-project-card-meta">Create a blank stash</div>
    </div>
  `;

  grid.innerHTML = html;

  grid.querySelectorAll('.packer-project-card:not(.new-placeholder)').forEach(card => {
    card.addEventListener('dblclick', () => {
      const name = (card as HTMLElement).dataset.name || '';
      loadSelectedProject(name);
    });
  });

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

  document.getElementById('packer-hub-create-card')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });
}

export function showProjectsHub(): void {
  document.getElementById('packer-projects-hub')!.style.display = 'flex';
  document.getElementById('packer-workspace-view')!.style.display = 'none';
  loadProjectsList();
}

export function openNewProjectWorkspace(): void {
  setActiveProject('');
  setStagedFiles([]);
  setSourcePaths([]);
  targetOverrides.clear();
  setVirtualFolders([]);
  backupPaths.clear();
  clearMetadataForm();

  const nameInput = document.getElementById('packer-project-name') as HTMLInputElement;
  if (nameInput) nameInput.value = '';

  document.getElementById('packer-workspace-title')!.textContent = 'New Project / Staging Area';
  document.getElementById('packer-projects-hub')!.style.display = 'none';
  document.getElementById('packer-workspace-view')!.style.display = 'flex';
  renderWorkspace();
}

export function loadSelectedProject(name: string): void {
  const project = savedProjects.find(p => p.name === name);
  if (!project) return;

  setActiveProject(name);
  setSourcePaths([...project.sourcePaths]);

  targetOverrides.clear();
  const vFolders: string[] = [];
  const bPaths: Map<string, string> = new Map();

  if (project.targetPathsOverride) {
    Object.entries(project.targetPathsOverride).forEach(([k, v]) => {
      if (k.startsWith('__VIRTUAL_DIR__:')) {
        vFolders.push(k.substring('__VIRTUAL_DIR__:'.length));
        targetOverrides.set(k, v);
      } else if (k.startsWith('__SKIP_ORIGINAL__:')) {
        bPaths.set(k.substring('__SKIP_ORIGINAL__:'.length), v);
      } else {
        targetOverrides.set(k, v);
      }
    });
  }

  setVirtualFolders(vFolders);
  setBackupPaths(bPaths);

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

  scanAndBuildStagedFiles().then(() => {
    showToast(`Loaded project '${name}' and scanned folders`, 'info');
  });
}

export async function saveCurrentProject(): Promise<void> {
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

    setActiveProject(projName);
    document.getElementById('packer-workspace-title')!.textContent = `Project: ${projName}`;
    showToast(res, 'success');
  } catch (err: any) {
    showToast(`Failed to save project: ${err}`, 'error');
  }
}

export async function deleteProjectByName(name: string): Promise<void> {
  try {
    const res = await invoke<string>('delete_packer_project', { projectName: name });
    if (activeProjectName === name) {
      setActiveProject('');
      setStagedFiles([]);
      setSourcePaths([]);
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
export { savedProjects, activeProjectName };
