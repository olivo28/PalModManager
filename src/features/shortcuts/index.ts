export * from './types';
export * from './storage';
export * from './formatter';
export * from './registry';
export * from './dispatcher';

import { initializeDefaultShortcuts } from './registry';
import { setupShortcutsDispatcher } from './dispatcher';

/**
 * Master initializer for the PalModManager Centralized Shortcuts Hub subsystem.
 */
export function initShortcutsHub(): void {
  initializeDefaultShortcuts();
  setupShortcutsDispatcher();
}
