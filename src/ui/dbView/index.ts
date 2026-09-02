import { invoke } from '@tauri-apps/api/core';
import { dbDom } from '../../framework';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import type { DbSnapshot, DbTable } from './types';
import { dbState } from './state';
import { renderModsTable, renderProfilesTable, renderSettingsTable } from './tables';
import { selectRecord, clearInspector, onJsonEditorInput, handleSaveRecord } from './inspector';
import { loadUsmapData, renderUsmapView } from './usmap';

export * from './types';
export * from './state';
export * from './tables';
export * from './inspector';
export * from './usmap';

export async function renderDbView(): Promise<void> {
  const container = dbDom.elMaybe('db-view');
  if (!container) return;

  container.innerHTML = `
    <div class="db-view-container">
      <div class="db-toolbar">
        <span class="db-toolbar-title">${escapeHtml(t('db.title'))}</span>
        <div class="db-tab-group">
          <button class="db-tab-btn active" data-dbtab="mods">${escapeHtml(t('db.tab_mods'))}</button>
          <button class="db-tab-btn" data-dbtab="profiles">${escapeHtml(t('db.tab_profiles'))}</button>
          <button class="db-tab-btn" data-dbtab="settings">${escapeHtml(t('db.tab_settings'))}</button>
          <button class="db-tab-btn" data-dbtab="usmap">⚡ ${escapeHtml(t('db.tab_usmap') || 'USMAP Schema')}</button>
        </div>
        <button id="db-refresh-btn" class="db-action-btn" title="${escapeHtml(t('db.btn_refresh_title'))}">${escapeHtml(t('db.btn_refresh'))}</button>
      </div>

      <div class="db-split">
        <div class="db-grid-panel" id="db-grid-panel">
          <div class="db-loading">${escapeHtml(t('db.loading'))}</div>
        </div>
        <div class="db-inspector-panel">
          <div class="db-inspector-toolbar">
            <span id="db-inspector-title" class="db-inspector-label">${escapeHtml(t('db.inspector_title'))}</span>
            <div id="db-inspector-actions" style="display: flex; align-items: center; gap: 8px;">
              <span id="db-json-status" class="db-json-status"></span>
              <button id="db-save-btn" class="db-save-btn" disabled>${escapeHtml(t('db.btn_save'))}</button>
            </div>
          </div>
          <div id="db-inspector-custom-container" style="display: none; flex: 1; overflow-y: auto; padding: 12px;"></div>
          <textarea
            id="db-json-editor"
            class="db-json-editor"
            spellcheck="false"
            placeholder="${escapeHtml(t('db.editor_placeholder'))}"
          ></textarea>
        </div>
      </div>
    </div>
  `;

  setupDbEventListeners();
  await loadSnapshot();
}

function setupDbEventListeners(): void {
  document.querySelectorAll('.db-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = (btn as HTMLElement).dataset.dbtab as DbTable;
      switchDbTable(tab);
    });
  });

  dbDom.elMaybe('db-refresh-btn')?.addEventListener('click', async () => {
    if (dbState.activeTable === 'usmap') {
      await loadUsmapData();
    } else {
      await loadSnapshot();
    }
  });

  const editor = dbDom.elMaybe('db-json-editor');
  editor?.addEventListener('input', onJsonEditorInput);

  dbDom.elMaybe('db-save-btn')?.addEventListener('click', handleSaveRecord);
}

export async function loadSnapshot(): Promise<void> {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (panel) panel.innerHTML = '<div class="db-loading">Loading database…</div>';

  try {
    dbState.snapshot = await invoke<DbSnapshot>('db_get_all');
    clearInspector();
    renderCurrentTable();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    if (panel) panel.innerHTML = `<div class="db-error">❌ ${escapeHtml(t('toasts.export_failed', { error: String(e) }))}</div>`;
  }
}

export function switchDbTable(tab: DbTable): void {
  dbState.activeTable = tab;
  document.querySelectorAll('.db-tab-btn').forEach(b => {
    b.classList.toggle('active', (b as HTMLElement).dataset.dbtab === tab);
  });
  
  const split = document.querySelector('.db-split');
  if (split) {
    split.classList.toggle('usmap-mode', tab === 'usmap');
    split.classList.remove('has-selection');
  }

  clearInspector();
  if (tab === 'usmap') {
    loadUsmapData();
  } else {
    renderCurrentTable();
  }
}

export function renderCurrentTable(): void {
  if (dbState.activeTable === 'usmap') {
    renderUsmapView();
    return;
  }

  if (!dbState.snapshot) return;
  const panel = document.getElementById('db-grid-panel');
  if (!panel) return;

  if (dbState.activeTable === 'mods') {
    panel.innerHTML = renderModsTable(dbState.snapshot.mods);
    panel.querySelectorAll('.db-row[data-id]').forEach(row => {
      row.addEventListener('click', () => {
        const id = (row as HTMLElement).dataset.id!;
        const mod = dbState.snapshot!.mods.find(m => m.id === id);
        if (mod) selectRecord('mod', id, mod);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  } else if (dbState.activeTable === 'profiles') {
    panel.innerHTML = renderProfilesTable(dbState.snapshot.profiles, dbState.snapshot.currentProfileId);
    panel.querySelectorAll('.db-row[data-id]').forEach(row => {
      row.addEventListener('click', () => {
        const id = (row as HTMLElement).dataset.id!;
        const profile = dbState.snapshot!.profiles.find(p => p.id === id);
        if (profile) selectRecord('profile', id, profile);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  } else if (dbState.activeTable === 'settings') {
    panel.innerHTML = renderSettingsTable(dbState.snapshot.settings);
    panel.querySelectorAll('.db-row-settings').forEach(row => {
      row.addEventListener('click', () => {
        selectRecord('settings', '', dbState.snapshot!.settings);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  }
}
