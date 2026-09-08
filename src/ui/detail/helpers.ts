import { t } from '../../utils/i18n';
import type { ModInfo } from '../../types';
import type { ModComponentFolder } from './types';

export function formatDisplayPath(fullPath: string): string {
  if (!fullPath) return '';
  const normalized = fullPath.replace(/\//g, '\\');
  const lower = normalized.toLowerCase();
  const palIdx = lower.lastIndexOf('\\pal\\');
  if (palIdx !== -1) {
    return normalized.substring(palIdx + 1);
  }
  if (lower.startsWith('pal\\')) {
    return normalized;
  }
  return normalized;
}

export function getModComponentFolders(mod: ModInfo): ModComponentFolder[] {
  const primaryPath = (mod.enabled ? mod.gamePath : mod.disabledPath) || '';
  const components: ModComponentFolder[] = [];
  const folderMap = new Map<string, ModComponentFolder>();

  const registerItem = (itemPath: string, explicitType?: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other') => {
    if (!itemPath) return;
    const normalized = itemPath.replace(/\\/g, '/');
    const lower = normalized.toLowerCase();

    const isPakFile = lower.endsWith('.pak');
    const isLogicMods = lower.includes('logicmods');

    // If it's a pak file, resolve its folder directory
    let folderDir = normalized;
    let fileName: string | null = null;
    if (isPakFile) {
      const lastSlash = normalized.lastIndexOf('/');
      if (lastSlash !== -1) {
        folderDir = normalized.substring(0, lastSlash);
        fileName = normalized.substring(lastSlash + 1);
      }
    }

    const folderLower = folderDir.toLowerCase();

    // Determine type
    let compType: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other' = explicitType || 'other';
    if (isLogicMods) {
      compType = 'logicmods';
    } else if (isPakFile || folderLower.includes('content/paks') || folderLower.includes('~mods')) {
      compType = 'pak';
    } else if (folderLower.includes('palschema/mods') || (folderLower.includes('palschema') && !folderLower.endsWith('.pak'))) {
      compType = 'palschema';
    } else if (folderLower.includes('nativemods/ue4ss') || folderLower.includes('ue4ss/mods') || (folderLower.includes('ue4ss') && !folderLower.endsWith('.pak'))) {
      compType = 'ue4ss';
    } else if (mod.type === 'ue4ss') {
      compType = 'ue4ss';
    } else if (mod.type === 'palschema') {
      compType = 'palschema';
    } else if (mod.type === 'pak') {
      compType = 'pak';
    } else if (mod.type === 'logicmods') {
      compType = 'logicmods';
    }

    // Check if folder is already registered in folderMap
    let existing = folderMap.get(folderLower);
    if (!existing) {
      for (const [fKey, c] of folderMap.entries()) {
        if (folderLower === fKey || folderLower.startsWith(fKey + '/') || fKey.startsWith(folderLower + '/')) {
          existing = c;
          break;
        }
      }
    }

    if (existing) {
      if (fileName && (!existing.files || !existing.files.includes(fileName))) {
        existing.files = existing.files || [];
        existing.files.push(fileName);
      }
      return;
    }

    // Determine clean label and button label
    let label = t('detail.comp_other_folder');
    let buttonLabel = t('detail.btn_open_folder');

    const folderName = folderDir.split('/').filter(Boolean).pop() || '';
    if (compType === 'pak') {
      label = `Paks (${folderName || '~mods'})`;
      buttonLabel = folderName || '~mods';
    } else if (compType === 'logicmods') {
      label = `LogicMods (${folderName || 'LogicMods'})`;
      buttonLabel = folderName || 'LogicMods';
    } else if (compType === 'ue4ss') {
      label = `UE4SS: ${folderName}`;
      buttonLabel = `UE4SS (${folderName})`;
    } else if (compType === 'palschema') {
      label = `PalSchema: ${folderName}`;
      buttonLabel = `PalSchema (${folderName})`;
    } else {
      label = folderName || t('detail.comp_other_folder');
      buttonLabel = folderName || t('detail.btn_open_folder');
    }

    const newComp: ModComponentFolder = {
      type: compType,
      label,
      buttonLabel,
      path: folderDir,
      files: fileName ? [fileName] : undefined,
    };

    folderMap.set(folderLower, newComp);
    components.push(newComp);
  };

  // 1. Add primary path
  if (primaryPath) {
    registerItem(primaryPath);
  }

  // 2. Add extra files
  if (mod.extraFiles && Array.isArray(mod.extraFiles)) {
    for (const f of mod.extraFiles) {
      if (!f) continue;
      const fNorm = f.replace(/\\/g, '/');
      const fLower = fNorm.toLowerCase();

      // If this file is an internal child of primaryPath and not a pak, skip
      if (primaryPath && !fLower.endsWith('.pak')) {
        const primNorm = primaryPath.replace(/\\/g, '/').toLowerCase();
        if (fLower.startsWith(primNorm + '/') || fLower === primNorm) {
          continue;
        }
      }

      if (fLower.endsWith('.pak')) {
        registerItem(f, fLower.includes('logicmods') ? 'logicmods' : 'pak');
      } else if (fLower.includes('swapjson') || fLower.includes('alterconfig')) {
        const parentDir = f.substring(0, Math.max(f.lastIndexOf('/'), f.lastIndexOf('\\')));
        registerItem(parentDir || f, 'other');
      } else if (fLower.includes('palschema')) {
        registerItem(f, 'palschema');
      } else if (fLower.includes('ue4ss') || fLower.includes('nativemods')) {
        registerItem(f, 'ue4ss');
      }
    }
  }

  const typeOrder: Record<string, number> = {
    ue4ss: 1,
    palschema: 2,
    pak: 3,
    logicmods: 4,
    other: 5,
  };

  components.sort((a, b) => (typeOrder[a.type] ?? 99) - (typeOrder[b.type] ?? 99));

  if (components.length === 1) {
    components[0].buttonLabel = t('detail.btn_open_folder');
  }

  return components;
}
