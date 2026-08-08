/**
 * tabManager.ts
 * Central tab router. Wraps the legacy switchTab from editorView.ts and extends it
 * with the 'db' tab — editorView.ts is NOT modified.
 */
import { switchTab } from './editorView';
import { updateState } from '../state';

export type AppTab = 'mods' | 'load' | 'editor' | 'library' | 'build' | 'scanner' | 'db';

const ALL_PANELS: { id: string; tab: AppTab; display: string }[] = [
  { id: 'mods-view',     tab: 'mods',    display: '' },
  { id: 'load-view',     tab: 'load',    display: 'flex' },
  { id: 'editor-view',   tab: 'editor',  display: 'flex' },
  { id: 'library-view',  tab: 'library', display: 'flex' },
  { id: 'build-view',    tab: 'build',   display: 'flex' },
  { id: 'scanner-view',  tab: 'scanner', display: 'flex' },
  { id: 'db-view',       tab: 'db',      display: 'flex' },
];

/**
 * Navigate to any application tab.
 * For legacy tabs (mods/editor/library/build/scanner), delegates to editorView.switchTab.
 * For the new 'db' and 'load' tabs, handles panel visibility directly.
 */
export function navigateTo(tab: AppTab): void {
  if (tab === 'db' || tab === 'load') {
    updateState({ activeTab: tab as any });

    // Update sidebar active button
    document.querySelectorAll('.sidebar-tab').forEach(b => b.classList.remove('active'));
    const tabBtn = document.querySelector(`.sidebar-tab[data-tab="${tab}"]`);
    if (tabBtn) tabBtn.classList.add('active');

    // Show/hide panels
    ALL_PANELS.forEach(({ id, tab: panelTab, display }) => {
      const el = document.getElementById(id);
      if (el) el.style.display = panelTab === tab ? display : 'none';
    });

    if (tab === 'load') {
      import('./loadView').then(m => m.renderLoadView());
    }
  } else {
    // Always hide custom panels first
    const dbPanel = document.getElementById('db-view');
    if (dbPanel) dbPanel.style.display = 'none';
    const loadPanel = document.getElementById('load-view');
    if (loadPanel) loadPanel.style.display = 'none';
    // Delegate to the existing router for all known legacy tabs
    switchTab(tab);
  }
}
