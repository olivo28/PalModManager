import type { ShortcutDefinition, ShortcutCategory } from './types';
import { getState, updateState } from '../../state';
import { navigateTo } from '../../ui/tabManager';
import { openSettingsModal } from '../../ui/modal';
import { showToast } from '../../ui/toast';
import { showConfirm } from '../../ui/confirm';
import { t } from '../../utils/i18n';
import { settingsDom } from '../../framework';

const registry = new Map<string, ShortcutDefinition>();
let _scaleSaveTimer: ReturnType<typeof setTimeout> | null = null;

/**
 * Adjust the global application UI scale and persist the update.
 */
export function adjustUiScale(delta: number): void {
  const current = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--ui-scale')) || 1.0;
  let next = delta === 0 ? 1.0 : Math.round((current + delta) * 100) / 100;
  if (delta > 0) next = Math.min(1.4, next);
  else if (delta < 0) next = Math.max(0.85, next);

  if (next !== current) {
    document.documentElement.style.setProperty('--ui-scale', next.toString());
    import('../../ui/editor/monaco/instance').then(m => m.updateMonacoScale(next)).catch(() => {});
    const slider = settingsDom.elMaybe('settings-ui-scale');
    const badge = settingsDom.elMaybe('settings-ui-scale-val');
    if (slider) slider.value = next.toString();
    if (badge) badge.textContent = `${Math.round(next * 100)}%`;

    const percent = Math.round(next * 100);
    showToast(t('toasts.scale_level', { percent }) || `UI Scale: ${percent}%`, 'info');

    if (_scaleSaveTimer) clearTimeout(_scaleSaveTimer);
    _scaleSaveTimer = setTimeout(async () => {
      try {
        const { setUiScale } = await import('../../api');
        const updated = await setUiScale(next);
        updateState({ currentSettings: updated });
      } catch (err) {
        console.error('[Shortcuts:Registry] Failed to persist uiScale:', err);
      }
    }, 400);
  }
}

/**
 * Register a single shortcut definition into the catalog.
 */
export function registerShortcut(def: ShortcutDefinition): void {
  registry.set(def.id, def);
}

/**
 * Retrieve a specific shortcut definition by ID.
 */
export function getShortcut(id: string): ShortcutDefinition | undefined {
  return registry.get(id);
}

/**
 * Retrieve all registered shortcut definitions.
 */
export function getAllShortcuts(): ShortcutDefinition[] {
  return Array.from(registry.values());
}

/**
 * Retrieve all shortcuts filtered by category.
 */
export function getShortcutsByCategory(category: ShortcutCategory): ShortcutDefinition[] {
  return getAllShortcuts().filter(s => s.category === category);
}

/**
 * Register the default PalModManager catalog of actions.
 */
export function initializeDefaultShortcuts(): void {
  if (registry.size > 0) return; // Already initialized

  // 1. MODS MANAGEMENT
  registerShortcut({
    id: 'mod.toggle',
    labelKey: 'shortcuts.action.mod_toggle',
    descriptionKey: 'shortcuts.desc.mod_toggle',
    category: 'mods',
    defaultKey: 'Space',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length === 0) return;

      const { enableMod, disableMod } = await import('../../api');
      const { loadMods } = await import('../../ui/mods/loader');

      if (selected.length === 1) {
        const modId = selected[0];
        const mod = getState().allMods.find(m => m.id === modId);
        if (!mod) return;
        try {
          if (mod.enabled) await disableMod(modId);
          else await enableMod(modId);
          await loadMods();
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else {
        // Multi-selection: toggle bulk
        const anyDisabled = selected.some(id => !getState().allMods.find(m => m.id === id)?.enabled);
        try {
          for (const id of selected) {
            if (anyDisabled) await enableMod(id);
            else await disableMod(id);
          }
          await loadMods();
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    }
  });

  registerShortcut({
    id: 'mod.details',
    labelKey: 'shortcuts.action.mod_details',
    descriptionKey: 'shortcuts.desc.mod_details',
    category: 'mods',
    defaultKey: 'Enter',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length !== 1) return;
      const { openDetailPanel } = await import('../../ui/detailPanel');
      openDetailPanel(selected[0]);
    }
  });

  registerShortcut({
    id: 'mod.edit_config',
    labelKey: 'shortcuts.action.mod_edit_config',
    descriptionKey: 'shortcuts.desc.mod_edit_config',
    category: 'mods',
    defaultKey: 'Ctrl+E',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length !== 1) return;
      const { openConfigEditor } = await import('../../ui/editorView');
      openConfigEditor(selected[0]);
    }
  });

  registerShortcut({
    id: 'mod.open_folder',
    labelKey: 'shortcuts.action.mod_open_folder',
    descriptionKey: 'shortcuts.desc.mod_open_folder',
    category: 'mods',
    defaultKey: 'Ctrl+O',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length !== 1) return;
      const { openModFolder } = await import('../../api');
      openModFolder(selected[0]);
    }
  });

  registerShortcut({
    id: 'mod.check_updates',
    labelKey: 'shortcuts.action.mod_check_updates',
    descriptionKey: 'shortcuts.desc.mod_check_updates',
    category: 'mods',
    defaultKey: 'Ctrl+U',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length !== 1) return;
      const modId = selected[0];
      const mod = getState().allMods.find(m => m.id === modId);
      if (!mod) return;
      showToast(t('toasts.checking_updates', { name: mod.name }), 'info');
      const { refreshNexusCache } = await import('../../api');
      try {
        const updated = await refreshNexusCache(modId);
        const idx = getState().allMods.findIndex(m => m.id === modId);
        if (idx >= 0) {
          const newMods = [...getState().allMods];
          newMods[idx] = updated;
          updateState({ allMods: newMods });
          const { renderModsView } = await import('../../ui/mods/renderer');
          renderModsView();
        }
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    }
  });

  registerShortcut({
    id: 'mod.remove',
    labelKey: 'shortcuts.action.mod_remove',
    descriptionKey: 'shortcuts.desc.mod_remove',
    category: 'mods',
    defaultKey: 'Delete',
    scope: 'mods',
    run: async () => {
      const selected = Array.from(getState().selectedModIds);
      if (selected.length === 0) return;

      if (selected.length === 1) {
        const modId = selected[0];
        const mod = getState().allMods.find(m => m.id === modId);
        if (!mod) return;
        const confirmed = await showConfirm(t('dialogs.confirm_remove_mod', { name: mod.name }));
        if (!confirmed) return;
        const { removeMod } = await import('../../api');
        const { closeDetailPanel } = await import('../../ui/detailPanel');
        const { loadMods } = await import('../../ui/mods/loader');
        const { loadProfiles } = await import('../../ui/mods/profiles');
        const { loadDependencies } = await import('../../ui/mods/dependencies');
        try {
          await removeMod(modId);
          closeDetailPanel();
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          showToast(t('toasts.mod_removed'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else {
        // Multi-selection removal
        const confirmed = await showConfirm(t('dialogs.confirm_remove_multiple_mods', { count: selected.length }) || `Remove ${selected.length} mods?`);
        if (!confirmed) return;
        const { removeMod } = await import('../../api');
        const { closeDetailPanel } = await import('../../ui/detailPanel');
        const { loadMods } = await import('../../ui/mods/loader');
        const { loadProfiles } = await import('../../ui/mods/profiles');
        const { loadDependencies } = await import('../../ui/mods/dependencies');
        try {
          for (const id of selected) await removeMod(id);
          closeDetailPanel();
          await Promise.all([loadMods(), loadProfiles(), loadDependencies(true)]);
          showToast(t('toasts.mod_removed'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    }
  });

  registerShortcut({
    id: 'mods.new_folder',
    labelKey: 'shortcuts.action.new_folder',
    descriptionKey: 'shortcuts.desc.new_folder',
    category: 'mods',
    defaultKey: 'Ctrl+Shift+N',
    scope: 'mods',
    run: () => {
      import('../../framework').then(({ mainDom }) => mainDom.elMaybe('new-folder-btn')?.click());
    }
  });

  registerShortcut({
    id: 'mods.install',
    labelKey: 'shortcuts.action.install_archive',
    descriptionKey: 'shortcuts.desc.install_archive',
    category: 'mods',
    defaultKey: 'Ctrl+I',
    scope: 'mods',
    run: () => {
      import('../../framework').then(({ mainDom }) => mainDom.elMaybe('install-btn')?.click());
    }
  });

  registerShortcut({
    id: 'mods.install_nexus_id',
    labelKey: 'shortcuts.action.install_nexus_id',
    descriptionKey: 'shortcuts.desc.install_nexus_id',
    category: 'mods',
    defaultKey: 'Ctrl+Shift+I',
    scope: 'mods',
    run: async () => {
      const { showPrompt } = await import('../../ui/confirm');
      const input = await showPrompt(t('context.prompt_nexus_id_title'));
      if (!input || !input.trim()) return;
      const trimmed = input.trim();
      let targetId: number | null = null;
      if (/^\d+$/.test(trimmed)) {
        targetId = parseInt(trimmed, 10);
      } else if (trimmed.includes('/mods/')) {
        const match = trimmed.match(/\/mods\/(\d+)/);
        if (match) targetId = parseInt(match[1], 10);
      }
      if (targetId) {
        const { openModDetails } = await import('../../ui/discovery/detailsModal');
        openModDetails(targetId);
      }
    }
  });

  registerShortcut({
    id: 'mods.rescan',
    labelKey: 'shortcuts.action.rescan_mods',
    descriptionKey: 'shortcuts.desc.rescan_mods',
    category: 'mods',
    defaultKey: 'F5',
    scope: 'mods',
    run: () => {
      import('../../framework').then(({ mainDom }) => mainDom.elMaybe('scan-btn')?.click());
    }
  });

  registerShortcut({
    id: 'mods.check_updates_all',
    labelKey: 'shortcuts.action.check_updates_all',
    descriptionKey: 'shortcuts.desc.check_updates_all',
    category: 'mods',
    defaultKey: 'Ctrl+Shift+U',
    scope: 'mods',
    run: () => {
      import('../../framework').then(({ mainDom }) => mainDom.elMaybe('check-updates-btn')?.click());
    }
  });

  registerShortcut({
    id: 'mods.export_json',
    labelKey: 'shortcuts.action.export_json',
    descriptionKey: 'shortcuts.desc.export_json',
    category: 'mods',
    defaultKey: 'Ctrl+Shift+E',
    scope: 'mods',
    run: () => {
      import('../../framework').then(({ mainDom }) => mainDom.elMaybe('export-json-btn')?.click());
    }
  });

  // 2. CONFIG EDITOR
  registerShortcut({
    id: 'editor.save',
    labelKey: 'shortcuts.action.editor_save',
    descriptionKey: 'shortcuts.desc.editor_save',
    category: 'editor',
    defaultKey: 'Ctrl+S',
    scope: 'editor',
    allowInInput: true,
    run: async () => {
      const { handleEditorSave } = await import('../../ui/editorView');
      await handleEditorSave();
    }
  });

  registerShortcut({
    id: 'editor.find',
    labelKey: 'shortcuts.action.editor_find',
    descriptionKey: 'shortcuts.desc.editor_find',
    category: 'editor',
    defaultKey: 'Ctrl+F',
    scope: 'editor',
    allowInInput: true,
    run: async () => {
      const { getMonacoEditor } = await import('../../ui/editor/monaco/instance');
      const monaco = getMonacoEditor();
      if (monaco) {
        monaco.focus();
        monaco.trigger('keyboard', 'actions.find', null);
      }
    }
  });

  // 3. NAVIGATION
  registerShortcut({
    id: 'nav.tab_mods',
    labelKey: 'shortcuts.action.nav_mods',
    descriptionKey: 'shortcuts.desc.nav_mods',
    category: 'navigation',
    defaultKey: 'Ctrl+1',
    scope: 'global',
    run: () => navigateTo('mods')
  });

  registerShortcut({
    id: 'nav.tab_editor',
    labelKey: 'shortcuts.action.nav_editor',
    descriptionKey: 'shortcuts.desc.nav_editor',
    category: 'navigation',
    defaultKey: 'Ctrl+2',
    scope: 'global',
    run: () => navigateTo('editor')
  });

  registerShortcut({
    id: 'nav.tab_library',
    labelKey: 'shortcuts.action.nav_library',
    descriptionKey: 'shortcuts.desc.nav_library',
    category: 'navigation',
    defaultKey: 'Ctrl+3',
    scope: 'global',
    run: () => navigateTo('library')
  });

  registerShortcut({
    id: 'nav.tab_build',
    labelKey: 'shortcuts.action.nav_build',
    descriptionKey: 'shortcuts.desc.nav_build',
    category: 'navigation',
    defaultKey: 'Ctrl+4',
    scope: 'global',
    run: () => navigateTo('build')
  });

  registerShortcut({
    id: 'nav.tab_scanner',
    labelKey: 'shortcuts.action.nav_scanner',
    descriptionKey: 'shortcuts.desc.nav_scanner',
    category: 'navigation',
    defaultKey: 'Ctrl+5',
    scope: 'global',
    run: () => navigateTo('scanner')
  });

  // 4. GENERAL
  registerShortcut({
    id: 'general.settings',
    labelKey: 'shortcuts.action.general_settings',
    descriptionKey: 'shortcuts.desc.general_settings',
    category: 'general',
    defaultKey: 'Ctrl+,',
    scope: 'global',
    run: () => openSettingsModal()
  });

  registerShortcut({
    id: 'general.shortcuts',
    labelKey: 'shortcuts.action.general_shortcuts',
    descriptionKey: 'shortcuts.desc.general_shortcuts',
    category: 'general',
    defaultKey: 'F1',
    scope: 'global',
    run: async () => {
      const { openSettingsToTab } = await import('../../ui/modals/settings/modal');
      openSettingsToTab('shortcuts');
    }
  });

  registerShortcut({
    id: 'general.zoom_in',
    labelKey: 'shortcuts.action.zoom_in',
    descriptionKey: 'shortcuts.desc.zoom_in',
    category: 'general',
    defaultKey: 'Ctrl+=',
    scope: 'global',
    run: () => adjustUiScale(0.05)
  });

  registerShortcut({
    id: 'general.zoom_out',
    labelKey: 'shortcuts.action.zoom_out',
    descriptionKey: 'shortcuts.desc.zoom_out',
    category: 'general',
    defaultKey: 'Ctrl+-',
    scope: 'global',
    run: () => adjustUiScale(-0.05)
  });

  registerShortcut({
    id: 'general.zoom_reset',
    labelKey: 'shortcuts.action.zoom_reset',
    descriptionKey: 'shortcuts.desc.zoom_reset',
    category: 'general',
    defaultKey: 'Ctrl+0',
    scope: 'global',
    run: () => adjustUiScale(0)
  });
}
