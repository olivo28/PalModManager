import { getAllShortcuts } from './registry';
import { getEffectiveKey, eventToKeyCombination } from './formatter';
import { getState } from '../../state';
import type { ShortcutScope } from './types';

let _dispatcherAttached = false;

/**
 * Determines the current active application scope.
 */
export function getCurrentActiveScope(): ShortcutScope {
  const activeTab = getState().activeTab;
  switch (activeTab) {
    case 'editor':
      return 'editor';
    case 'library':
      return 'library';
    case 'build':
      return 'build';
    case 'scanner':
      return 'scanner';
    case 'mods':
    default:
      return 'mods';
  }
}

/**
 * Tests whether an event target is an interactive text input element.
 */
function isEditableElement(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  if (tag === 'input' || tag === 'textarea' || tag === 'select') return true;
  if (target.isContentEditable) return true;
  if (target.closest('.monaco-editor')) return true;
  return false;
}

/**
 * Handles incoming keydown events and dispatches them to matching shortcuts.
 */
export function handleKeydown(e: KeyboardEvent): void {
  // Ignore repeat events if modifier or solitary key
  const combination = eventToKeyCombination(e);
  if (!combination) return;

  const currentScope = getCurrentActiveScope();
  const isInput = isEditableElement(e.target);

  const shortcuts = getAllShortcuts();
  for (const def of shortcuts) {
    const boundKey = getEffectiveKey(def.id);
    if (!boundKey) continue;

    // Compare normalized combinations
    if (boundKey.toLowerCase() === combination.toLowerCase()) {
      // Scope validation: must be 'global' or match current active scope
      if (def.scope !== 'global' && def.scope !== currentScope) {
        continue;
      }

      // Input protection: skip if inside text input unless explicitly allowed
      if (isInput && !def.allowInInput) {
        continue;
      }

      // Execute matched action
      e.preventDefault();
      e.stopPropagation();
      try {
        def.run();
      } catch (err) {
        console.error(`[Shortcuts:Dispatcher] Error executing action '${def.id}':`, err);
      }
      return;
    }
  }
}

/**
 * Attaches the global keyboard shortcuts dispatcher listener.
 */
export function setupShortcutsDispatcher(): void {
  if (_dispatcherAttached) return;
  window.addEventListener('keydown', handleKeydown, { capture: true });
  _dispatcherAttached = true;
}
