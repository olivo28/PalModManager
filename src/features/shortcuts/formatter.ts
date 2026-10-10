import { loadCustomBindings } from './storage';
import { getShortcut } from './registry';

const IS_MAC = typeof navigator !== 'undefined' && /(Mac|iPhone|iPod|iPad)/i.test(navigator.platform);

/**
 * Normalizes a raw key representation to a standardized format (e.g. 'Ctrl+Shift+S').
 */
export function normalizeKeyCombination(combination: string): string {
  if (!combination) return '';
  const parts = combination.split('+').map(p => p.trim());
  const modifiers: string[] = [];
  let mainKey = '';

  for (const part of parts) {
    const lower = part.toLowerCase();
    if (lower === 'ctrl' || lower === 'control') modifiers.push('Ctrl');
    else if (lower === 'alt' || lower === 'option') modifiers.push('Alt');
    else if (lower === 'shift') modifiers.push('Shift');
    else if (lower === 'meta' || lower === 'cmd' || lower === 'command') modifiers.push('Cmd');
    else {
      // Normalize specific key tokens
      if (lower === ' ' || lower === 'space' || lower === 'spacebar') mainKey = 'Space';
      else if (lower === 'esc' || lower === 'escape') mainKey = 'Esc';
      else if (lower === 'arrowup' || lower === 'up') mainKey = 'Up';
      else if (lower === 'arrowdown' || lower === 'down') mainKey = 'Down';
      else if (lower === 'arrowleft' || lower === 'left') mainKey = 'Left';
      else if (lower === 'arrowright' || lower === 'right') mainKey = 'Right';
      else if (lower === 'delete' || lower === 'del') mainKey = 'Delete';
      else if (lower === 'enter' || lower === 'return') mainKey = 'Enter';
      else if (part.length === 1) mainKey = part.toUpperCase();
      else mainKey = part.charAt(0).toUpperCase() + part.slice(1);
    }
  }

  // Canonical modifier ordering: Ctrl -> Alt -> Shift -> Cmd
  const ordered: string[] = [];
  if (modifiers.includes('Ctrl')) ordered.push('Ctrl');
  if (modifiers.includes('Alt')) ordered.push('Alt');
  if (modifiers.includes('Shift')) ordered.push('Shift');
  if (modifiers.includes('Cmd')) ordered.push('Cmd');
  if (mainKey) ordered.push(mainKey);

  return ordered.join('+');
}

/**
 * Converts a KeyboardEvent into a normalized key combination string.
 * Returns null if the event is only a modifier key pressed alone.
 */
export function eventToKeyCombination(e: KeyboardEvent): string | null {
  // If the pressed key is a solitary modifier, don't generate a combination yet
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
    return null;
  }

  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Cmd');

  let keyName = e.key;
  if (keyName === ' ') keyName = 'Space';
  else if (keyName === 'Escape') keyName = 'Esc';
  else if (keyName === 'ArrowUp') keyName = 'Up';
  else if (keyName === 'ArrowDown') keyName = 'Down';
  else if (keyName === 'ArrowLeft') keyName = 'Left';
  else if (keyName === 'ArrowRight') keyName = 'Right';
  else if (keyName.length === 1) keyName = keyName.toUpperCase();

  parts.push(keyName);
  return parts.join('+');
}

/**
 * Format combination for human display, adjusting for macOS symbols when relevant.
 */
export function formatKeyDisplay(keyStr: string): string {
  if (!keyStr) return '';
  if (!IS_MAC) return keyStr;

  return keyStr
    .replace(/\bCtrl\b/g, '⌃')
    .replace(/\bAlt\b/g, '⌥')
    .replace(/\bShift\b/g, '⇧')
    .replace(/\bCmd\b/g, '⌘')
    .replace(/\+/g, ' ');
}

/**
 * Retrieves the effective key for an action (custom binding or default).
 */
export function getEffectiveKey(actionId: string): string {
  const custom = loadCustomBindings();
  if (Object.prototype.hasOwnProperty.call(custom, actionId)) {
    return custom[actionId];
  }
  const def = getShortcut(actionId);
  return def ? def.defaultKey : '';
}

/**
 * Get human-readable formatted display string for an action shortcut.
 */
export function getShortcutDisplay(actionId: string): string {
  const key = getEffectiveKey(actionId);
  return key ? formatKeyDisplay(key) : '';
}

/**
 * Generates an HTML badge string to inject into context menus or toolbars.
 * Returns an empty string if no shortcut is bound.
 */
export function getShortcutBadgeHtml(actionId: string): string {
  const display = getShortcutDisplay(actionId);
  if (!display) return '';
  return `<span class="ctx-shortcut" data-shortcut-id="${actionId}">${display}</span>`;
}
