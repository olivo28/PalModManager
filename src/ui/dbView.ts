import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';
import { escapeHtml } from '../utils/helpers';
import { t } from '../utils/i18n';
import { dbDom } from '../framework';
import {
  getUsmapSummary,
  searchUsmapEntries,
  getUsmapFullStructDetails,
  getUsmapEnumInfo,
  syncMappingsNow,
  type UsmapSummaryData,
  type UsmapSearchItem,
  type UsmapSearchResult,
  type UsmapStructFullDetails,
} from '../api';

// ─── Types ────────────────────────────────────────────────────────────────────

type DbTable = 'mods' | 'profiles' | 'settings' | 'usmap';

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
let _selectedRecordType: 'mod' | 'profile' | 'settings' | 'usmap_struct' | 'usmap_enum' | 'usmap_name' | null = null;
let _selectedRecordId: string = '';

// USMAP State
let _usmapSummary: UsmapSummaryData | null = null;
let _usmapQuery: string = '';
let _usmapCategory: 'all' | 'structs' | 'enums' | 'names' = 'all';
let _usmapPage: number = 0;
const USMAP_PAGE_SIZE = 40;
let _usmapSearchResult: UsmapSearchResult | null = null;
let _selectedUsmapItem: UsmapSearchItem | null = null;
let _selectedUsmapStructDetails: UsmapStructFullDetails | null = null;
let _selectedUsmapEnumValues: string[] | null = null;
let _usmapInspectorMode: 'visual' | 'json' = 'visual';

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

// ─── Event Listeners ──────────────────────────────────────────────────────────

function setupDbEventListeners(): void {
  document.querySelectorAll('.db-tab-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const tab = (btn as HTMLElement).dataset.dbtab as DbTable;
      switchDbTable(tab);
    });
  });

  dbDom.elMaybe('db-refresh-btn')?.addEventListener('click', async () => {
    if (_activeTable === 'usmap') {
      await loadUsmapData();
    } else {
      await loadSnapshot();
    }
  });

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
  if (tab === 'usmap') {
    loadUsmapData();
  } else {
    renderCurrentTable();
  }
}

function renderCurrentTable(): void {
  if (_activeTable === 'usmap') {
    renderUsmapView();
    return;
  }

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

// ─── USMAP Schema Explorer ───────────────────────────────────────────────────

let _usmapSearchDebounceTimer: any = null;

async function loadUsmapData(): Promise<void> {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (panel) panel.innerHTML = `<div class="db-loading">${escapeHtml(t('db.loading') || 'Loading USMAP reflection schema…')}</div>`;

  try {
    _usmapSummary = await getUsmapSummary();
    _usmapSearchResult = await searchUsmapEntries(_usmapQuery, _usmapCategory, _usmapPage, USMAP_PAGE_SIZE);
    renderUsmapView();
  } catch (e) {
    showToast(`Failed to load USMAP: ${e}`, 'error');
    if (panel) panel.innerHTML = `<div class="db-error">❌ ${escapeHtml(String(e))}</div>`;
  }
}

function renderUsmapView(): void {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (!panel) return;

  const s = _usmapSummary;
  const res = _usmapSearchResult;
  const isLoaded = s && s.loaded;

  const totalPages = res ? Math.ceil(res.totalItems / USMAP_PAGE_SIZE) || 1 : 1;

  panel.innerHTML = `
    <div style="padding: 12px; border-bottom: 1px solid var(--border); background: var(--bg-card); display: flex; flex-direction: column; gap: 10px;">
      <!-- USMAP Summary Banner -->
      <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 13px; font-weight: 700; color: var(--accent);">⚡ Palworld USMAP Reflection Schema</span>
          <span style="font-size: 11px; padding: 2px 8px; background: rgba(0, 210, 255, 0.12); border: 1px solid rgba(0, 210, 255, 0.3); border-radius: 12px; color: var(--accent); font-weight: 600;">
            ${escapeHtml(s?.gameVersion ? `v${s.gameVersion}` : 'Loaded')}
          </span>
        </div>
        <button id="db-usmap-sync-btn" class="btn btn-secondary btn-xs" style="display: flex; align-items: center; gap: 6px; font-size: 11px; padding: 4px 10px;">
          <span>🔄</span> <span>${escapeHtml(t('settings.btn_sync_now') || 'Sync Mappings')}</span>
        </button>
      </div>

      <!-- Stats counter -->
      <div style="display: flex; gap: 12px; font-size: 11px; color: var(--text-muted); flex-wrap: wrap;">
        <span>📦 <b>${s?.totalStructs?.toLocaleString() || '0'}</b> Structs & Classes</span>
        <span>🏷️ <b>${s?.totalEnums?.toLocaleString() || '0'}</b> Enums</span>
        <span>📝 <b>${s?.totalNames?.toLocaleString() || '0'}</b> FNames</span>
      </div>

      <!-- Search & Filters -->
      <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 2px;">
        <input
          type="text"
          id="db-usmap-search-input"
          class="input-text"
          placeholder="${escapeHtml(t('db.usmap_search_placeholder') || 'Search Structs, Classes, Enums, Properties...')}"
          value="${escapeHtml(_usmapQuery)}"
          style="flex: 1; min-width: 180px; font-size: 11.5px; padding: 6px 10px;"
        />
        <div style="display: flex; gap: 4px;">
          <button class="btn btn-xs db-usmap-cat-btn ${_usmapCategory === 'all' ? 'btn-primary' : 'btn-secondary'}" data-cat="all">All</button>
          <button class="btn btn-xs db-usmap-cat-btn ${_usmapCategory === 'structs' ? 'btn-primary' : 'btn-secondary'}" data-cat="structs">Structs</button>
          <button class="btn btn-xs db-usmap-cat-btn ${_usmapCategory === 'enums' ? 'btn-primary' : 'btn-secondary'}" data-cat="enums">Enums</button>
          <button class="btn btn-xs db-usmap-cat-btn ${_usmapCategory === 'names' ? 'btn-primary' : 'btn-secondary'}" data-cat="names">FNames</button>
        </div>
      </div>
    </div>

    <!-- Items Grid Table -->
    ${!isLoaded ? `<div class="db-empty">USMAP file is not loaded. Click "Sync Mappings" above.</div>` : `
      <div style="flex: 1; overflow-y: auto;">
        ${!res || res.items.length === 0 ? `<div class="db-empty">${escapeHtml(t('common.no_results') || 'No matching entries found in USMAP schema')}</div>` : `
          <table class="db-grid-table">
            <thead>
              <tr>
                <th style="width: 80px;">Type</th>
                <th>Name</th>
                <th>Details / Super</th>
                <th style="width: 90px; text-align: right;">Count</th>
              </tr>
            </thead>
            <tbody>
              ${res.items.map(item => {
                const isSelected = _selectedUsmapItem?.name === item.name;
                let typeBadge = '';
                if (item.category === 'struct') {
                  typeBadge = `<span class="scanner-conflict-type" style="background: rgba(0, 210, 255, 0.15); color: var(--accent); border-color: rgba(0, 210, 255, 0.3);">Struct</span>`;
                } else if (item.category === 'enum') {
                  typeBadge = `<span class="scanner-conflict-type" style="background: rgba(255, 170, 0, 0.15); color: var(--warning); border-color: rgba(255, 170, 0, 0.3);">Enum</span>`;
                } else {
                  typeBadge = `<span class="scanner-conflict-type" style="background: rgba(150, 150, 150, 0.15); color: var(--text-muted); border-color: var(--border);">FName</span>`;
                }

                return `
                  <tr class="db-row ${isSelected ? 'selected' : ''}" data-name="${escapeHtml(item.name)}" data-cat="${item.category}" style="cursor: pointer;">
                    <td class="db-cell">${typeBadge}</td>
                    <td class="db-cell db-cell-name" style="font-weight: 600; font-family: monospace;">${escapeHtml(item.name)}</td>
                    <td class="db-cell" style="font-size: 11px; color: var(--text-muted);">${escapeHtml(item.preview)}</td>
                    <td class="db-cell db-cell-mono" style="text-align: right;">${item.category === 'struct' ? `${item.propertyCount} props` : item.category === 'enum' ? `${item.enumValuesCount} vals` : '—'}</td>
                  </tr>
                `;
              }).join('')}
            </tbody>
          </table>
        `}
      </div>

      <!-- Pagination Footer -->
      <div style="padding: 8px 14px; border-top: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center; background: var(--bg-card); font-size: 11px;">
        <span style="color: var(--text-muted);">
          Showing <b>${res?.items.length || 0}</b> of <b>${res?.totalItems.toLocaleString() || 0}</b> items
        </span>
        <div style="display: flex; align-items: center; gap: 8px;">
          <button id="db-usmap-prev-btn" class="btn btn-secondary btn-xs" ${_usmapPage === 0 ? 'disabled' : ''}>◀ Prev</button>
          <span>Page <b>${_usmapPage + 1}</b> / ${totalPages}</span>
          <button id="db-usmap-next-btn" class="btn btn-secondary btn-xs" ${_usmapPage + 1 >= totalPages ? 'disabled' : ''}>Next ▶</button>
        </div>
      </div>
    `}
  `;

  // Setup Event Listeners for USMAP View
  panel.querySelector('#db-usmap-sync-btn')?.addEventListener('click', async () => {
    showToast('Synchronizing USMAP reflection schema...', 'info');
    try {
      await syncMappingsNow();
      showToast('USMAP synchronized successfully!', 'success');
      await loadUsmapData();
    } catch (e) {
      showToast(`Sync failed: ${e}`, 'error');
    }
  });

  const searchInput = panel.querySelector<HTMLInputElement>('#db-usmap-search-input');
  searchInput?.addEventListener('input', () => {
    clearTimeout(_usmapSearchDebounceTimer);
    _usmapSearchDebounceTimer = setTimeout(async () => {
      _usmapQuery = searchInput.value;
      _usmapPage = 0;
      _usmapSearchResult = await searchUsmapEntries(_usmapQuery, _usmapCategory, _usmapPage, USMAP_PAGE_SIZE);
      renderUsmapView();
    }, 250);
  });

  panel.querySelectorAll('.db-usmap-cat-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      _usmapCategory = (btn as HTMLElement).dataset.cat as any;
      _usmapPage = 0;
      _usmapSearchResult = await searchUsmapEntries(_usmapQuery, _usmapCategory, _usmapPage, USMAP_PAGE_SIZE);
      renderUsmapView();
    });
  });

  panel.querySelector('#db-usmap-prev-btn')?.addEventListener('click', async () => {
    if (_usmapPage > 0) {
      _usmapPage -= 1;
      _usmapSearchResult = await searchUsmapEntries(_usmapQuery, _usmapCategory, _usmapPage, USMAP_PAGE_SIZE);
      renderUsmapView();
    }
  });

  panel.querySelector('#db-usmap-next-btn')?.addEventListener('click', async () => {
    if (_usmapPage + 1 < totalPages) {
      _usmapPage += 1;
      _usmapSearchResult = await searchUsmapEntries(_usmapQuery, _usmapCategory, _usmapPage, USMAP_PAGE_SIZE);
      renderUsmapView();
    }
  });

  panel.querySelectorAll('.db-row[data-name]').forEach(row => {
    row.addEventListener('click', async () => {
      const name = (row as HTMLElement).dataset.name!;
      const cat = (row as HTMLElement).dataset.cat as any;
      const item = _usmapSearchResult?.items.find(i => i.name === name);
      if (item) {
        panel.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
        await selectUsmapItem(item);
      }
    });
  });
}

async function selectUsmapItem(item: UsmapSearchItem): Promise<void> {
  _selectedUsmapItem = item;
  _selectedRecordType = item.category === 'struct' ? 'usmap_struct' : item.category === 'enum' ? 'usmap_enum' : 'usmap_name';
  _selectedRecordId = item.name;

  if (item.category === 'struct') {
    _selectedUsmapStructDetails = await getUsmapFullStructDetails(item.name);
    _selectedUsmapEnumValues = null;
  } else if (item.category === 'enum') {
    _selectedUsmapEnumValues = await getUsmapEnumInfo(item.name);
    _selectedUsmapStructDetails = null;
  } else {
    _selectedUsmapStructDetails = null;
    _selectedUsmapEnumValues = null;
  }

  renderUsmapInspector();
}

function renderUsmapInspector(): void {
  const customContainer = document.getElementById('db-inspector-custom-container');
  const jsonEditor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = document.getElementById('db-inspector-title');
  const inspectorActions = document.getElementById('db-inspector-actions');
  if (!customContainer || !jsonEditor) return;

  if (!_selectedUsmapItem) {
    customContainer.style.display = 'none';
    jsonEditor.style.display = 'block';
    if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title');
    clearInspector();
    return;
  }

  const item = _selectedUsmapItem;
  if (inspectorTitle) {
    inspectorTitle.textContent = `${item.category.toUpperCase()}: ${item.name}`;
  }

  if (inspectorActions) {
    inspectorActions.innerHTML = `
      <div style="display: flex; gap: 4px;">
        <button id="db-inspector-toggle-visual" class="btn btn-xs ${_usmapInspectorMode === 'visual' ? 'btn-primary' : 'btn-secondary'}">Schema</button>
        <button id="db-inspector-toggle-json" class="btn btn-xs ${_usmapInspectorMode === 'json' ? 'btn-primary' : 'btn-secondary'}">JSON</button>
      </div>
    `;

    document.getElementById('db-inspector-toggle-visual')?.addEventListener('click', () => {
      _usmapInspectorMode = 'visual';
      renderUsmapInspector();
    });
    document.getElementById('db-inspector-toggle-json')?.addEventListener('click', () => {
      _usmapInspectorMode = 'json';
      renderUsmapInspector();
    });
  }

  const rawJsonData = _selectedUsmapStructDetails || _selectedUsmapEnumValues || item;

  if (_usmapInspectorMode === 'json') {
    customContainer.style.display = 'none';
    jsonEditor.style.display = 'block';
    jsonEditor.value = JSON.stringify(rawJsonData, null, 2);
    jsonEditor.disabled = false;
    updateJsonStatus(true);
    return;
  }

  // Visual Mode
  jsonEditor.style.display = 'none';
  customContainer.style.display = 'block';

  if (item.category === 'struct') {
    const details = _selectedUsmapStructDetails;
    const hierarchy = details?.inheritanceChain || [item.name];

    customContainer.innerHTML = `
      <div style="display: flex; flex-direction: column; gap: 14px;">
        <!-- Struct Header & Inheritance Breadcrumbs -->
        <div style="background: var(--bg-primary); padding: 12px 14px; border-radius: var(--card-radius); border: 1px solid var(--border);">
          <div style="font-size: 10px; color: var(--text-muted); font-weight: 700; text-transform: uppercase; margin-bottom: 6px;">Inheritance Hierarchy</div>
          <div style="display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: 11.5px; font-family: monospace;">
            ${hierarchy.map((cls, idx) => `
              <span style="color: ${idx === 0 ? 'var(--accent)' : 'var(--text-primary)'}; font-weight: ${idx === 0 ? '700' : '500'}; background: rgba(255,255,255,0.04); padding: 2px 8px; border-radius: 4px; border: 1px solid var(--border);">
                ${escapeHtml(cls)}
              </span>
              ${idx < hierarchy.length - 1 ? '<span style="color: var(--text-muted);">←</span>' : ''}
            `).join('')}
          </div>
          <div style="display: flex; gap: 14px; margin-top: 10px; font-size: 11px; color: var(--text-muted);">
            <span>Direct Properties: <b>${details?.properties.length || 0}</b></span>
            <span>Total with Ancestors: <b>${details?.totalPropertiesWithAncestors || details?.properties.length || 0}</b></span>
          </div>
        </div>

        <!-- Properties Table -->
        <div>
          <div style="font-size: 12px; font-weight: 700; margin-bottom: 8px; color: var(--text-primary); display: flex; justify-content: space-between; align-items: center;">
            <span>Properties Layout (${details?.properties.length || 0})</span>
          </div>

          ${!details || details.properties.length === 0 ? `
            <div style="color: var(--text-muted); font-size: 11px; padding: 10px; background: var(--bg-primary); border-radius: var(--card-radius);">
              No direct serialized properties in this struct (inherits properties from parent classes).
            </div>
          ` : `
            <table class="db-grid-table" style="font-size: 11px;">
              <thead>
                <tr>
                  <th style="width: 40px;">#</th>
                  <th>Property Name</th>
                  <th>Type</th>
                  <th>Details / Inner</th>
                  <th style="width: 40px; text-align: center;">Dim</th>
                </tr>
              </thead>
              <tbody>
                ${details.properties.map(p => `
                  <tr>
                    <td class="db-cell db-cell-mono" style="color: var(--text-muted);">${p.index}</td>
                    <td class="db-cell db-cell-mono" style="color: var(--accent); font-weight: 600;">${escapeHtml(p.name)}</td>
                    <td class="db-cell"><span style="color: var(--warning); font-family: monospace;">${escapeHtml(p.typeName)}</span></td>
                    <td class="db-cell" style="font-family: monospace; color: var(--text-muted); font-size: 10.5px;">
                      ${escapeHtml(p.structType || p.enumType || p.innerType || '—')}
                    </td>
                    <td class="db-cell db-cell-mono" style="text-align: center;">${p.arrayDim > 1 ? p.arrayDim : '1'}</td>
                  </tr>
                `).join('')}
              </tbody>
            </table>
          `}
        </div>
      </div>
    `;
  } else if (item.category === 'enum') {
    const vals = _selectedUsmapEnumValues || [];
    customContainer.innerHTML = `
      <div style="display: flex; flex-direction: column; gap: 12px;">
        <div style="background: var(--bg-primary); padding: 12px 14px; border-radius: var(--card-radius); border: 1px solid var(--border);">
          <div style="font-size: 13px; font-weight: 700; color: var(--warning); font-family: monospace;">enum class ${escapeHtml(item.name)}</div>
          <div style="font-size: 11px; color: var(--text-muted); margin-top: 4px;">Total Enum Values: <b>${vals.length}</b></div>
        </div>

        <table class="db-grid-table" style="font-size: 11px;">
          <thead>
            <tr>
              <th style="width: 50px;">Index</th>
              <th>Enum Identifier</th>
            </tr>
          </thead>
          <tbody>
            ${vals.map((v, idx) => `
              <tr>
                <td class="db-cell db-cell-mono" style="color: var(--text-muted);">${idx}</td>
                <td class="db-cell db-cell-mono" style="color: var(--text-primary); font-weight: 600;">${escapeHtml(v)}</td>
              </tr>
            `).join('')}
          </tbody>
        </table>
      </div>
    `;
  } else {
    customContainer.innerHTML = `
      <div style="background: var(--bg-primary); padding: 16px; border-radius: var(--card-radius); border: 1px solid var(--border);">
        <div style="font-size: 13px; font-weight: 700; color: var(--text-primary); font-family: monospace;">FName: "${escapeHtml(item.name)}"</div>
        <div style="font-size: 11px; color: var(--text-muted); margin-top: 8px;">
          Global string table token used across Palworld engine reflection and property serializations.
        </div>
      </div>
    `;
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
  _selectedUsmapItem = null;

  const customContainer = document.getElementById('db-inspector-custom-container');
  const editor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = document.getElementById('db-inspector-title');
  const inspectorActions = document.getElementById('db-inspector-actions');

  if (customContainer) customContainer.style.display = 'none';
  if (editor) {
    editor.style.display = 'block';
    const maskedData = maskSensitiveData(data);
    editor.value = JSON.stringify(maskedData, null, 2);
    editor.disabled = false;
  }

  if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title');
  if (inspectorActions) {
    inspectorActions.innerHTML = `
      <span id="db-json-status" class="db-json-status"></span>
      <button id="db-save-btn" class="db-save-btn" disabled>${escapeHtml(t('db.btn_save'))}</button>
    `;
    dbDom.elMaybe('db-save-btn')?.addEventListener('click', handleSaveRecord);
  }

  updateJsonStatus(true);
}

function clearInspector(): void {
  _selectedRecordType = null;
  _selectedRecordId = '';
  _selectedUsmapItem = null;

  const customContainer = document.getElementById('db-inspector-custom-container');
  const editor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = document.getElementById('db-inspector-title');

  if (customContainer) customContainer.style.display = 'none';
  if (editor) {
    editor.style.display = 'block';
    editor.value = '';
    editor.disabled = true;
  }
  if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title');
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
