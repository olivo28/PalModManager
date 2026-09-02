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
  const seenFolders = new Set<string>();

  const addFolder = (folderPath: string, explicitType?: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other') => {
    if (!folderPath) return;
    const normalized = folderPath.replace(/\\/g, '/');
    const lower = normalized.toLowerCase();

    // Check if this folder or an ancestor folder is already registered
    if (seenFolders.has(lower)) return;
    for (const seen of seenFolders) {
      if (lower.startsWith(seen + '/') || seen.startsWith(lower + '/')) {
        return;
      }
    }
    seenFolders.add(lower);

    let compType: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other' = explicitType || 'other';
    let label = t('detail.comp_other_folder');
    let buttonLabel = t('detail.btn_open_folder');

    if (lower.includes('palschema/mods') || (lower.includes('palschema') && !lower.endsWith('.pak'))) {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.comp_palschema_folder');
    } else if (lower.includes('nativemods/ue4ss') || lower.includes('ue4ss/mods') || (lower.includes('ue4ss') && !lower.endsWith('.pak'))) {
      compType = 'ue4ss';
      label = t('detail.comp_ue4ss_folder');
      buttonLabel = t('detail.comp_ue4ss_folder');
    } else if (lower.includes('logicmods') || (lower.endsWith('.pak') && lower.includes('logicmods'))) {
      compType = 'logicmods';
      label = t('detail.comp_logicmods_folder');
      buttonLabel = t('detail.comp_logicmods_folder');
    } else if (lower.endsWith('.pak') || lower.includes('content/paks') || lower.includes('~mods')) {
      compType = 'pak';
      label = t('detail.comp_pak_folder');
      buttonLabel = t('detail.comp_pak_folder');
    } else if (mod.type === 'ue4ss') {
      compType = 'ue4ss';
      label = t('detail.comp_ue4ss_folder');
      buttonLabel = t('detail.comp_ue4ss_folder');
    } else if (mod.type === 'palschema') {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.comp_palschema_folder');
    } else if (mod.type === 'pak') {
      compType = 'pak';
      label = t('detail.comp_pak_folder');
      buttonLabel = t('detail.comp_pak_folder');
    } else if (mod.type === 'logicmods') {
      compType = 'logicmods';
      label = t('detail.comp_logicmods_folder');
      buttonLabel = t('detail.comp_logicmods_folder');
    }

    components.push({ type: compType, label, buttonLabel, path: folderPath });
  };

  // 1. Add primary path
  if (primaryPath) {
    addFolder(primaryPath);
  }

  // 2. Add extra files ONLY if they represent distinct external directories / companion packages
  if (mod.extraFiles && Array.isArray(mod.extraFiles)) {
    for (const f of mod.extraFiles) {
      if (!f) continue;
      const fNorm = f.replace(/\\/g, '/');
      const fLower = fNorm.toLowerCase();

      // If this file is an internal child of primaryPath, skip it
      if (primaryPath) {
        const primNorm = primaryPath.replace(/\\/g, '/').toLowerCase();
        if (fLower.startsWith(primNorm + '/') || fLower === primNorm) {
          continue;
        }
      }

      // If it's a standalone companion .pak or .json outside the primary folder
      if (fLower.endsWith('.pak')) {
        addFolder(f, fLower.includes('logicmods') ? 'logicmods' : 'pak');
      } else if (fLower.includes('swapjson') || fLower.includes('alterconfig')) {
        const parentDir = f.substring(0, Math.max(f.lastIndexOf('/'), f.lastIndexOf('\\')));
        addFolder(parentDir || f, 'other');
      } else if (fLower.includes('palschema')) {
        addFolder(f, 'palschema');
      } else if (fLower.includes('ue4ss') || fLower.includes('nativemods')) {
        addFolder(f, 'ue4ss');
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
