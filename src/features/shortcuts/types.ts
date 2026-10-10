export type ShortcutScope = 'global' | 'mods' | 'editor' | 'library' | 'scanner' | 'build';
export type ShortcutCategory = 'general' | 'navigation' | 'mods' | 'editor';

export interface ShortcutDefinition {
  id: string;               // Unique ID: e.g. 'mod.toggle', 'editor.save', 'nav.mods'
  labelKey: string;         // i18n key for the action label
  descriptionKey?: string;  // i18n key for action description
  category: ShortcutCategory;
  defaultKey: string;       // Default key string: e.g. 'Space', 'Ctrl+S', 'Ctrl+1', 'Delete'
  scope: ShortcutScope;     // Scope where this shortcut is active
  allowInInput?: boolean;   // If true, fires even when an input or editor has focus (default: false)
  run: () => void | Promise<void>; // Execution handler
}

export interface ShortcutBinding {
  id: string;
  key: string;              // Current bound key (empty string if unassigned)
}

export type ShortcutBindingsMap = Record<string, string>;
