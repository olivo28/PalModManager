import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';
import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';
import { dbDom } from '../framework';

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
  language?: string | null;
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
        </div>
        <button id="db-refresh-btn" class="db-action-btn" title="${escapeHtml(t('db.btn_refresh_title'))}">${escapeHtml(t('db.btn_refresh'))}</button>
      </div>

      <div class="db-split">
        <div class="db-grid-panel" id="db-grid-panel">
          <div class="db-loading">${escapeHtml(t('db.loading'))}</div>
        </div>
        <div class="db-inspector-panel">
          <div class="db-inspector-toolbar">
            <span class="db-inspector-label">${escapeHtml(t('db.inspector_title'))}</span>
            <span id="db-json-status" class="db-json-status"></span>
            <button id="db-save-btn" class="db-save-btn" disabled>${escapeHtml(t('db.btn_save'))}</button>
          </div>
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

// ─── Event Listeners ──────────────────────────────────────────────────────────

function setupDbEventListeners(): void {
  document.querySelectorAll('.db-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = (btn as HTMLElement).dataset.dbtab as DbTable;
      switchDbTable(tab);
    });
  });

  dbDom.elMaybe('db-refresh-btn')?.addEventListener('click', loadSnapshot);

  const editor = dbDom.elMaybe('db-json-editor');
  editor?.addEventListener('input', onJsonEditorInput);

  dbDom.elMaybe('db-save-btn')?.addEventListener('click', handleSaveRecord);
}

// ─── Data Loading ─────────────────────────────────────────────────────────────

async function loadSnapshot(): Promise<void> {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (panel) panel.innerHTML = '<div class="db-loading">Loading database…</div>';

  try {
    _snapshot = await invoke<DbSnapshot>('db_get_all');
    clearInspector();
    renderCurrentTable();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
    if (panel) panel.innerHTML = `<div class="db-error">❌ ${escapeHtml(t('toasts.export_failed', { error: String(e) }))}</div>`;
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
  if (!mods.length) return `<div class="db-empty">${escapeHtml(t('db.empty_mods'))}</div>`;

  const typeColor: Record<string, string> = {
    ue4ss: 'var(--type-ue4ss)',
    palschema: 'var(--type-palschema)',
    pak: 'var(--type-pak)',
    logicmods: 'var(--type-logicmods)',
    hybrid: 'var(--type-hybrid)',
  };

  const rows = mods.map(m => {
    const typeLabel = m.type.toLowerCase() === 'hybrid' ? t('card.type_hybrid') : m.type.toUpperCase();
    return `
    <tr class="db-row" data-id="${escapeHtml(m.id)}" title="${escapeHtml(m.id)}">
      <td class="db-cell db-cell-name">${escapeHtml(m.name)}</td>
      <td class="db-cell"><span class="db-type-badge" style="color:${typeColor[m.type] ?? 'var(--text-muted)'}">${escapeHtml(typeLabel)}</span></td>
      <td class="db-cell"><span class="db-status-dot ${m.enabled ? 'on' : 'off'}"></span></td>
      <td class="db-cell db-cell-mono">${escapeHtml(m.version)}</td>
      <td class="db-cell db-cell-date">${escapeHtml(m.installDate?.split('T')[0] ?? '')}</td>
    </tr>
  `;
  }).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>${escapeHtml(t('card.table_col_name'))}</th>
          <th>${escapeHtml(t('card.table_col_type'))}</th>
          <th>${escapeHtml(t('common.on'))}</th>
          <th>${escapeHtml(t('card.table_col_version'))}</th>
          <th>${escapeHtml(t('detail.installed_label'))}</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function renderProfilesTable(profiles: any[], currentId: string): string {
  if (!profiles.length) return `<div class="db-empty">${escapeHtml(t('db.empty_profiles'))}</div>`;

  const rows = profiles.map(p => {
    const installed = p.installed_mod_ids ?? p.installedModIds ?? [];
    const enabled = p.enabled_mod_ids ?? p.enabledModIds ?? [];
    const created = p.created_at ?? p.createdAt ?? '';

    return `
    <tr class="db-row ${p.id === currentId ? 'db-row-active' : ''}" data-id="${escapeHtml(p.id)}" title="${escapeHtml(p.id)}">
      <td class="db-cell db-cell-name">
        ${escapeHtml(p.name)}
        ${p.id === currentId ? `<span class="db-active-badge">${escapeHtml(t('profiles.active_badge'))}</span>` : ''}
      </td>
      <td class="db-cell db-cell-mono">${installed.length} ${escapeHtml(t('detail.installed_label')).toLowerCase()}</td>
      <td class="db-cell db-cell-mono">${enabled.length} ${escapeHtml(t('common.enabled')).toLowerCase()}</td>
      <td class="db-cell db-cell-date">${escapeHtml(created.split('T')[0] ?? '')}</td>
    </tr>
  `;
  }).join('');

  return `
    <table class="db-grid-table">
      <thead>
        <tr>
          <th>${escapeHtml(t('db.col_profile_name'))}</th>
          <th>${escapeHtml(t('detail.installed_label'))}</th>
          <th>${escapeHtml(t('common.enabled'))}</th>
          <th>${escapeHtml(t('db.col_created'))}</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

function formatSettingValue(key: string, val: unknown): string {
  if (val === null || val === undefined) return 'null';
  if (typeof val === 'boolean' || typeof val === 'number') return String(val);
  if (typeof val === 'string') {
    const lower = key.toLowerCase();
    if (lower.includes('token') || lower.includes('secret') || lower === 'apikey') {
      return '•••••••••••••••• (Encrypted)';
    }
    return val;
  }
  if (Array.isArray(val)) {
    if (val.length === 0) return '[]';
    return `[ ${val.length} item${val.length === 1 ? '' : 's'} ]`;
  }
  if (typeof val === 'object') {
    const acc = val as any;
    if (acc.name || acc.username) {
      return `{ user: "${acc.name || acc.username}", id: ${acc.userId || acc.user_id || '?'}, ... }`;
    }
    const keys = Object.keys(val as object);
    if (keys.length === 0) return '{}';
    return `{ ${keys.length} field${keys.length === 1 ? '' : 's'} }`;
  }
  return String(val);
}

function maskSensitiveData(data: unknown): unknown {
  if (!data || typeof data !== 'object') return data;
  const clone = JSON.parse(JSON.stringify(data));

  function recurse(o: any) {
    if (!o || typeof o !== 'object') return;
    for (const k of Object.keys(o)) {
      const lower = k.toLowerCase();
      if (typeof o[k] === 'string' && (lower.includes('token') || lower.includes('secret') || lower === 'apikey')) {
        const val = o[k];
        if (val.length > 10) {
          o[k] = `[ENCRYPTED: ${val.slice(0, 4)}••••••••${val.slice(-4)}]`;
        } else {
          o[k] = '••••••••••••••••';
        }
      } else if (typeof o[k] === 'object') {
        recurse(o[k]);
      }
    }
  }

  recurse(clone);
  return clone;
}

function renderSettingsTable(settings: AppSettings): string {
  const rows = Object.entries(settings).map(([key, val]) => `
    <tr class="db-row db-row-settings" title="${escapeHtml(t('db.click_to_edit'))}">
      <td class="db-cell db-cell-key">${escapeHtml(key)}</td>
      <td class="db-cell db-cell-mono db-cell-val">${escapeHtml(formatSettingValue(key, val))}</td>
    </tr>
  `).join('');

  return `
    <div style="padding: 8px 12px; font-size: 11px; color: var(--text-muted);">${escapeHtml(t('db.settings_hint'))}</div>
    <table class="db-grid-table">
      <thead><tr><th>${escapeHtml(t('db.col_key'))}</th><th>${escapeHtml(t('db.col_val'))}</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;
}

// ─── JSON Inspector ───────────────────────────────────────────────────────────

function selectRecord(type: 'mod' | 'profile' | 'settings', id: string, data: unknown): void {
  _selectedRecordType = type;
  _selectedRecordId = id;

  const maskedData = maskSensitiveData(data);
  const editor = dbDom.elMaybe('db-json-editor');
  if (editor) {
    editor.value = JSON.stringify(maskedData, null, 2);
    editor.disabled = false;
  }
  updateJsonStatus(true);
}

function clearInspector(): void {
  _selectedRecordType = null;
  _selectedRecordId = '';
  const editor = dbDom.elMaybe('db-json-editor');
  if (editor) {
    editor.value = '';
    editor.disabled = true;
  }
  updateJsonStatus(null);
}

function onJsonEditorInput(): void {
  const editor = dbDom.elMaybe('db-json-editor');
  if (!editor) return;
  try {
    JSON.parse(editor.value);
    updateJsonStatus(true);
  } catch {
    updateJsonStatus(false);
  }
}

function updateJsonStatus(valid: boolean | null): void {
  const status = dbDom.elMaybe('db-json-status');
  const saveBtn = dbDom.elMaybe('db-save-btn');
  if (!status || !saveBtn) return;

  if (valid === null) {
    status.textContent = '';
    status.className = 'db-json-status';
    saveBtn.disabled = true;
  } else if (valid) {
    status.textContent = t('db.valid_json');
    status.className = 'db-json-status valid';
    saveBtn.disabled = _selectedRecordType === null;
  } else {
    status.textContent = t('db.invalid_json');
    status.className = 'db-json-status invalid';
    saveBtn.disabled = true;
  }
}

// ─── Save Handler ─────────────────────────────────────────────────────────────

async function handleSaveRecord(): Promise<void> {
  if (!_selectedRecordType) return;

  const editor = dbDom.elMaybe('db-json-editor');
  if (!editor) return;

  let parsed: unknown;
  try {
    parsed = JSON.parse(editor.value);
  } catch {
    showToast(t('toasts.export_failed', { error: 'Invalid JSON' }), 'error');
    return;
  }

  try {
    await invoke('db_write_record', {
      recordType: _selectedRecordType,
      recordId: _selectedRecordId,
      json: JSON.stringify(parsed),
    });

    showToast(t('toasts.settings_saved'), 'success');

    if (_selectedRecordType === 'settings') {
      const { getSettings } = await import('../api');
      const updatedSettings = await getSettings();
      const { updateState } = await import('../state');
      updateState({ currentSettings: updatedSettings });
      if (updatedSettings.language) {
        const { initI18n } = await import('../utils/i18n');
        initI18n(updatedSettings.language);
      }
    }

    // Ask if user wants to re-scan mods
    const { showConfirm } = await import('./confirm');
    const doRescan = await showConfirm(
      t('db.confirm_rescan')
    );
    if (doRescan) {
      const { loadMods } = await import('./modsView');
      await loadMods();
      showToast(t('dependencies.up_to_date'), 'info');
    }

    // Reload snapshot so the grid reflects changes
    await loadSnapshot();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}
