/**
 * dbView.ts
 * DB Inspector & Editor — Browse and edit the live PMM database state.
 * Read: db_get_all | Write: db_write_record
 */
import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';
import { escapeHtml } from '../utils/helpers';

// ─── Types ────────────────────────────────────────────────────────────────────

type DbTable = 'mods' | 'profiles' | 'settings';

interface ModInfo {
  id: string;
  name: string;
  type: string;
  enabled: boolean;
  version: string;
  installDate: string;
  gamePath: string;
  nexusAuthor: string | null;
}

interface Profile {
  id: string;
  name: string;
  createdAt: string;
  installedModIds: string[];
  enabledModIds: string[];
}

interface AppSettings {
  gamePath: string;
  programPath: string;
  hideNativeMods: boolean | null;
  debugConsole: boolean | null;
  customDataPath: string | null;
}

interface DbSnapshot {
  mods: ModInfo[];
  profiles: Profile[];
  currentProfileId: string;
  settings: AppSettings;
}

// ─── State ────────────────────────────────────────────────────────────────────

let _snapshot: DbSnapshot | null = null;
let _activeTable: DbTable = 'mods';
let _selectedRecordType: 'mod' | 'profile' | 'settings' | null = null;
let _selectedRecordId: string = '';

// ─── Entry Point ──────────────────────────────────────────────────────────────

export async function renderDbView(): Promise<void> {
  const container = document.getElementById('db-view');
  if (!container) return;

  container.innerHTML = `
    <div class="db-view-container">
      <div class="db-toolbar">
        <span class="db-toolbar-title">🗄 DB Inspector</span>
        <div class="db-tab-group">
          <button class="db-tab-btn active" data-dbtab="mods">Mods</button>
          <button class="db-tab-btn" data-dbtab="profiles">Profiles</button>
          <button class="db-tab-btn" data-dbtab="settings">Settings</button>
        </div>
        <button id="db-refresh-btn" class="db-action-btn" title="Reload from database">↻ Refresh</button>
      </div>

      <div class="db-split">
        <div class="db-grid-panel" id="db-grid-panel">
          <div class="db-loading">Loading database…</div>
        </div>
        <div class="db-inspector-panel">
          <div class="db-inspector-toolbar">
            <span class="db-inspector-label">JSON Inspector</span>
            <span id="db-json-status" class="db-json-status"></span>
            <button id="db-save-btn" class="db-save-btn" disabled>Save Record</button>
          </div>
          <textarea
            id="db-json-editor"
            class="db-json-editor"
            spellcheck="false"
            placeholder="Select a record to inspect its raw JSON…"
          ></textarea>
        </div>
      </div>
    </div>
  `;

  setupDbEventListeners();
  await loadSnapshot();
}

// ─── Event Listeners ──────────────────────────────────────────────────────────

function setupDbEventListeners(): void {
  document.querySelectorAll('.db-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = (btn as HTMLElement).dataset.dbtab as DbTable;
      switchDbTable(tab);
    });
  });

  document.getElementById('db-refresh-btn')?.addEventListener('click', loadSnapshot);

  const editor = document.getElementById('db-json-editor') as HTMLTextAreaElement | null;
  editor?.addEventListener('input', onJsonEditorInput);

  document.getElementById('db-save-btn')?.addEventListener('click', handleSaveRecord);
}

// ─── Data Loading ─────────────────────────────────────────────────────────────

async function loadSnapshot(): Promise<void> {
  const panel = document.getElementById('db-grid-panel');
  if (panel) panel.innerHTML = '<div class="db-loading">Loading database…</div>';

  try {
    _snapshot = await invoke<DbSnapshot>('db_get_all');
    clearInspector();
    renderCurrentTable();
  } catch (e) {
    showToast(`DB load failed: ${e}`, 'error');
    if (panel) panel.innerHTML = `<div class="db-error">❌ Failed to load database: ${escapeHtml(String(e))}</div>`;
  }
}

// ─── Table Rendering ──────────────────────────────────────────────────────────

function switchDbTable(tab: DbTable): void {
  _activeTable = tab;
  document.querySelectorAll('.db-tab-btn').forEach(b => {
    b.classList.toggle('active', (b as HTMLElement).dataset.dbtab === tab);
  });
  clearInspector();
  renderCurrentTable();
}

function renderCurrentTable(): void {
  if (!_snapshot) return;
  const panel = document.getElementById('db-grid-panel');
  if (!panel) return;

  if (_activeTable === 'mods') {
    panel.innerHTML = renderModsTable(_snapshot.mods);
    panel.querySelectorAll('.db-row[data-id]').forEach(row => {
      row.addEventListener('click', () => {
        const id = (row as HTMLElement).dataset.id!;
        const mod = _snapshot!.mods.find(m => m.id === id);
        if (mod) selectRecord('mod', id, mod);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  } else if (_activeTable === 'profiles') {
    panel.innerHTML = renderProfilesTable(_snapshot.profiles, _snapshot.currentProfileId);
    panel.querySelectorAll('.db-row[data-id]').forEach(row => {
      row.addEventListener('click', () => {
        const id = (row as HTMLElement).dataset.id!;
        const profile = _snapshot!.profiles.find(p => p.id === id);
        if (profile) selectRecord('profile', id, profile);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  } else if (_activeTable === 'settings') {
    panel.innerHTML = renderSettingsTable(_snapshot.settings);
    panel.querySelectorAll('.db-row-settings').forEach(row => {
      row.addEventListener('click', () => {
        selectRecord('settings', '', _snapshot!.settings);
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
      });
    });
  }
}

function renderModsTable(mods: ModInfo[]): string {
  if (!mods.length) return '<div class="db-empty">No mods in database.</div>';

  const typeColor: Record<string, string> = {
    ue4ss: 'var(--type-ue4ss)',
    palschema: 'var(--type-palschema)',
    pak: 'var(--type-pak)',
    logicmods: 'var(--type-logicmods)',
    hybrid: 'var(--type-hybrid)',
  };

  const rows = mods.map(m => `
    <tr class="db-row" data-id="${escapeHtml(m.id)}" title="${escapeHtml(m.id)}">
      <td class="db-cell db-cell-name">${escapeHtml(m.name)}</td>
      <td class="db-cell"><span class="db-type-badge" style="color:${typeColor[m.type] ?? 'var(--text-muted)'}">${escapeHtml(m.type)}</span></td>
      <td class="db-cell"><span class="db-status-dot ${m.enabled ? 'on' : 'off'}"></span></td>
      <td class="db-cell db-cell-mono">${escapeHtml(m.version)}</td>
      <td class="db-cell db-cell-date">${escapeHtml(m.installDate?.split('T')[0] ?? '')}</td>
    </tr>
  `).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Type</th>
          <th>On</th>
          <th>Version</th>
          <th>Installed</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function renderProfilesTable(profiles: Profile[], currentId: string): string {
  if (!profiles.length) return '<div class="db-empty">No profiles in database.</div>';

  const rows = profiles.map(p => `
    <tr class="db-row ${p.id === currentId ? 'db-row-active' : ''}" data-id="${escapeHtml(p.id)}" title="${escapeHtml(p.id)}">
      <td class="db-cell db-cell-name">
        ${escapeHtml(p.name)}
        ${p.id === currentId ? '<span class="db-active-badge">Active</span>' : ''}
      </td>
      <td class="db-cell db-cell-mono">${p.installedModIds?.length ?? 0} installed</td>
      <td class="db-cell db-cell-mono">${p.enabledModIds?.length ?? 0} enabled</td>
      <td class="db-cell db-cell-date">${escapeHtml(p.createdAt?.split('T')[0] ?? '')}</td>
    </tr>
  `).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Installed</th>
          <th>Enabled</th>
          <th>Created</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function renderSettingsTable(settings: AppSettings): string {
  const rows = Object.entries(settings).map(([key, val]) => `
    <tr class="db-row db-row-settings" title="Click to edit settings">
      <td class="db-cell db-cell-key">${escapeHtml(key)}</td>
      <td class="db-cell db-cell-mono db-cell-val">${escapeHtml(val === null || val === undefined ? 'null' : String(val))}</td>
    </tr>
  `).join('');

  return `
    <div style="padding: 8px 12px; font-size: 11px; color: var(--text-muted);">Click a row to open full settings JSON in the inspector.</div>
    <table class="db-grid-table">
      <thead><tr><th>Key</th><th>Value</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

// ─── JSON Inspector ───────────────────────────────────────────────────────────

function selectRecord(type: 'mod' | 'profile' | 'settings', id: string, data: unknown): void {
  _selectedRecordType = type;
  _selectedRecordId = id;

  const editor = document.getElementById('db-json-editor') as HTMLTextAreaElement | null;
  if (editor) {
    editor.value = JSON.stringify(data, null, 2);
    editor.disabled = false;
  }
  updateJsonStatus(true);
}

function clearInspector(): void {
  _selectedRecordType = null;
  _selectedRecordId = '';
  const editor = document.getElementById('db-json-editor') as HTMLTextAreaElement | null;
  if (editor) {
    editor.value = '';
    editor.disabled = true;
  }
  updateJsonStatus(null);
}

function onJsonEditorInput(): void {
  const editor = document.getElementById('db-json-editor') as HTMLTextAreaElement | null;
  if (!editor) return;
  try {
    JSON.parse(editor.value);
    updateJsonStatus(true);
  } catch {
    updateJsonStatus(false);
  }
}

function updateJsonStatus(valid: boolean | null): void {
  const status = document.getElementById('db-json-status');
  const saveBtn = document.getElementById('db-save-btn') as HTMLButtonElement | null;
  if (!status || !saveBtn) return;

  if (valid === null) {
    status.textContent = '';
    status.className = 'db-json-status';
    saveBtn.disabled = true;
  } else if (valid) {
    status.textContent = '✓ Valid JSON';
    status.className = 'db-json-status valid';
    saveBtn.disabled = _selectedRecordType === null;
  } else {
    status.textContent = '✗ Invalid JSON';
    status.className = 'db-json-status invalid';
    saveBtn.disabled = true;
  }
}

// ─── Save Handler ─────────────────────────────────────────────────────────────

async function handleSaveRecord(): Promise<void> {
  if (!_selectedRecordType) return;

  const editor = document.getElementById('db-json-editor') as HTMLTextAreaElement | null;
  if (!editor) return;

  let parsed: unknown;
  try {
    parsed = JSON.parse(editor.value);
  } catch {
    showToast('Cannot save: JSON is invalid', 'error');
    return;
  }

  try {
    await invoke('db_write_record', {
      recordType: _selectedRecordType,
      recordId: _selectedRecordId,
      json: JSON.stringify(parsed),
    });

    showToast('Record saved successfully', 'success');

    // Ask if user wants to re-scan mods
    const { showConfirm } = await import('./confirm');
    const doRescan = await showConfirm(
      'Record saved. Do you want to re-scan mods to reflect the changes in the main list?'
    );
    if (doRescan) {
      const { loadMods } = await import('./modsView');
      await loadMods();
      showToast('Mods re-scanned', 'info');
    }

    // Reload snapshot so the grid reflects changes
    await loadSnapshot();
  } catch (e) {
    showToast(`Save failed: ${e}`, 'error');
  }
}
