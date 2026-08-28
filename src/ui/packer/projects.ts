import { invoke } from '@tauri-apps/api/core';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { stagedFiles, sourcePaths, targetOverrides, virtualFolders, backupPaths, activeProjectName, savedProjects, setStagedFiles, setSourcePaths, setVirtualFolders, setBackupPaths, setActiveProject, setSavedProjects, renderWorkspace, escapeHtml } from './mod';
import { scanAndBuildStagedFiles } from './staging';
import { clearMetadataForm } from './rendering';
import { packerDom } from '../../framework';

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
  const grid = packerDom.elMaybe('packer-projects-grid');
  if (!grid) return;

  let html = savedProjects.map(proj => {
    const typeBadge = proj.metadata?.modType ? `<span class="mod-type-badge" style="font-size:10px; padding:2px 6px; margin-top:6px;">${escapeHtml(proj.metadata.modType)}</span>` : '';
    return `
      <div class="packer-project-card" data-name="${escapeHtml(proj.name)}">
        <div class="packer-project-card-icon">📁</div>
        <div class="packer-project-card-title">${escapeHtml(proj.name)}</div>
        <div class="packer-project-card-meta">${escapeHtml(t('common.version'))}: ${escapeHtml(proj.metadata?.version || '1.0.0')}</div>
        ${typeBadge}
        <div class="packer-project-card-actions">
          <button class="packer-project-card-btn primary" data-action="open" data-name="${escapeHtml(proj.name)}">${escapeHtml(t('common.edit'))}</button>
          <button class="packer-project-card-btn danger" data-action="delete" data-name="${escapeHtml(proj.name)}">${escapeHtml(t('common.delete'))}</button>
        </div>
      </div>
    `;
  }).join('');

  html += `
    <div class="packer-project-card new-placeholder" id="packer-hub-create-card">
      <div class="packer-project-card-icon">+</div>
      <div class="packer-project-card-title">${escapeHtml(t('packer.btn_new_project'))}</div>
      <div class="packer-project-card-meta">${escapeHtml(t('packer.project_meta_type'))}</div>
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

  packerDom.elMaybe('packer-hub-create-card')?.addEventListener('click', () => {
    openNewProjectWorkspace();
  });
}

export function showProjectsHub(): void {
  packerDom.el('packer-projects-hub').style.display = 'flex';
  packerDom.el('packer-workspace-view').style.display = 'none';
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

  const nameInput = packerDom.elMaybe('packer-project-name');
  if (nameInput) nameInput.value = '';

  packerDom.el('packer-workspace-title').textContent = t('packer.workspace_title_new');
  packerDom.el('packer-projects-hub').style.display = 'none';
  packerDom.el('packer-workspace-view').style.display = 'flex';
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
    packerDom.el('packer-meta-name').value = m.name || '';
    packerDom.el('packer-meta-version').value = m.version || '1.0.0';
    packerDom.el('packer-meta-author').value = m.author || '';
    packerDom.el('packer-meta-type').value = m.modType || '';
    packerDom.el('packer-meta-nexus-id').value = m.nexusModId ? String(m.nexusModId) : '';
    packerDom.el('packer-meta-desc').value = m.description || '';
  } else {
    clearMetadataForm();
  }

  const formatSelect = packerDom.elMaybe('packer-format-select');
  if (formatSelect) {
    formatSelect.value = project.format || 'zip';
  }

  const nameInput = packerDom.elMaybe('packer-project-name');
  if (nameInput) nameInput.value = name;

  packerDom.el('packer-workspace-title').textContent = t('packer.workspace_title_project', { name });
  packerDom.el('packer-projects-hub').style.display = 'none';
  packerDom.el('packer-workspace-view').style.display = 'flex';

  scanAndBuildStagedFiles().then(() => {
    showToast(t('packer.toast_loaded_project', { name }), 'info');
  });
}

export async function saveCurrentProject(): Promise<void> {
  const nameInput = packerDom.elMaybe('packer-project-name');
  let projName = nameInput?.value.trim();

  if (!projName) {
    const modName = packerDom.elMaybe('packer-meta-name')?.value.trim();
    if (modName) {
      projName = modName;
    } else {
      showToast(t('packer.toast_enter_project_name'), 'warning');
      return;
    }
  }

  const metaName = packerDom.elMaybe('packer-meta-name')?.value.trim();
  const metaVersion = packerDom.elMaybe('packer-meta-version')?.value.trim();
  const metaAuthor = packerDom.elMaybe('packer-meta-author')?.value.trim();
  const metaType = packerDom.elMaybe('packer-meta-type')?.value;
  const metaDesc = packerDom.elMaybe('packer-meta-desc')?.value.trim();
  const metaNexusIdStr = packerDom.elMaybe('packer-meta-nexus-id')?.value.trim();
  const metaNexusId = metaNexusIdStr ? parseInt(metaNexusIdStr, 10) : null;
  const formatSelect = packerDom.elMaybe('packer-format-select');
  const format = formatSelect?.value || 'zip';

  const metadata: ModMetadata | null = metaName ? {
    name: metaName,
    version: metaVersion || '1.0.0',
    author: metaAuthor || '',
    modType: metaType || '',
    description: metaDesc || '',
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
    packerDom.el('packer-workspace-title').textContent = `Project: ${projName}`;
    showToast(t('packer.toast_saved_project'), 'success');
  } catch (err: any) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
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
    showToast(t('packer.toast_deleted_project'), 'success');
  } catch (err: any) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}
export { savedProjects, activeProjectName };
