import type { ShortcutBindingsMap } from './types';

const STORAGE_KEY = 'pmm-shortcuts-bindings';

/**
 * Load user custom bindings from localStorage.
 */
export function loadCustomBindings(): ShortcutBindingsMap {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return typeof parsed === 'object' && parsed !== null ? parsed : {};
  } catch (err) {
    console.error('[Shortcuts:Storage] Failed to read custom bindings from localStorage:', err);
    return {};
  }
}

/**
 * Save a custom binding for a specific shortcut ID.
 */
export function saveCustomBinding(id: string, key: string): void {
  try {
    const bindings = loadCustomBindings();
    bindings[id] = key;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(bindings));
  } catch (err) {
    console.error(`[Shortcuts:Storage] Failed to save binding for ${id}:`, err);
  }
}

/**
 * Reset a single custom binding to its default by deleting it from storage.
 */
export function resetCustomBinding(id: string): void {
  try {
    const bindings = loadCustomBindings();
    delete bindings[id];
    localStorage.setItem(STORAGE_KEY, JSON.stringify(bindings));
  } catch (err) {
    console.error(`[Shortcuts:Storage] Failed to reset binding for ${id}:`, err);
  }
}

/**
 * Clear all custom user bindings, restoring factory defaults.
 */
export function resetAllCustomBindings(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (err) {
    console.error('[Shortcuts:Storage] Failed to clear custom bindings:', err);
  }
}
