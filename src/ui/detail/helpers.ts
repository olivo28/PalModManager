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
  const allPaths: string[] = [];
  const primaryPath = mod.enabled ? mod.gamePath : mod.disabledPath;
  if (primaryPath) allPaths.push(primaryPath);
  if (mod.extraFiles && Array.isArray(mod.extraFiles)) {
    for (const f of mod.extraFiles) {
      if (f && !allPaths.includes(f)) {
        allPaths.push(f);
      }
    }
  }

  const components: ModComponentFolder[] = [];

  for (const p of allPaths) {
    const lower = p.toLowerCase().replace(/\\/g, '/');
    let compType: 'ue4ss' | 'palschema' | 'pak' | 'logicmods' | 'other' = 'other';
    let label = t('detail.comp_other_folder');
    let buttonLabel = t('detail.btn_open_folder');

    if (lower.includes('palschema/mods') || (lower.includes('palschema') && !lower.endsWith('.pak'))) {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.comp_palschema_folder');
    } else if (lower.includes('ue4ss/mods') || (lower.includes('ue4ss') && !lower.endsWith('.pak'))) {
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
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'palschema') {
      compType = 'palschema';
      label = t('detail.comp_palschema_folder');
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'pak') {
      compType = 'pak';
      label = t('detail.comp_pak_folder');
      buttonLabel = t('detail.btn_open_folder');
    } else if (mod.type === 'logicmods') {
      compType = 'logicmods';
      label = t('detail.comp_logicmods_folder');
      buttonLabel = t('detail.btn_open_folder');
    }

    if (!components.some(c => c.path === p)) {
      components.push({ type: compType, label, buttonLabel, path: p });
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
