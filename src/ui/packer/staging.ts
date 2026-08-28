import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { packerDom } from '../../framework';
import { stagedFiles, sourcePaths, targetOverrides, backupPaths, renderWorkspace, setStagedFiles, setSourcePaths } from './mod';

export async function addStagedPaths(paths: string[]): Promise<void> {
  const currentSources = [...sourcePaths];
  paths.forEach(p => {
    if (!currentSources.includes(p)) {
      currentSources.push(p);
    }
  });
  setSourcePaths(currentSources);

  await scanAndBuildStagedFiles();
  showToast(t('packer.toast_added_paths', { count: paths.length }), 'success');
}

export async function scanAndBuildStagedFiles(): Promise<void> {
  if (sourcePaths.length === 0) {
    setStagedFiles([]);
    renderWorkspace();
    return;
  }

  try {
    const files = await invoke<any[]>('scan_paths_for_packing', { paths: sourcePaths });
    
    files.forEach(f => {
      f.targetPath = f.targetPath.replace(/\\/g, '/');
      if (targetOverrides.has(f.sourcePath)) {
        f.targetPath = targetOverrides.get(f.sourcePath) || f.targetPath;
      }
    });

    setStagedFiles(files);
    renderWorkspace();
  } catch (err: any) {
    console.error(err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export function toggleSkipFile(index: number): void {
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

export function autoStructureWorkspace(): void {
  const modNameInput = packerDom.elMaybe('packer-meta-name') as HTMLInputElement | null;
  const modTypeSelect = packerDom.elMaybe('packer-meta-type') as HTMLSelectElement | null;

  const rawModName = modNameInput?.value.trim();
  const modType = modTypeSelect?.value;

  if (!rawModName) {
    showToast(t('packer.toast_fill_mod_name'), 'warning');
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
  showToast(t('packer.toast_auto_structured'), 'success');
}

export { stagedFiles, sourcePaths, targetOverrides, backupPaths };
export interface StagedFile {
  sourcePath: string;
  relativePath: string;
  size: number;
  targetPath: string;
}
