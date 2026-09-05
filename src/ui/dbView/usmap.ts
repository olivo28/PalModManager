import { dbDom } from '../../framework';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import {
  getUsmapSummary,
  searchUsmapEntries,
  getUsmapFullStructDetails,
  getUsmapEnumInfo,
  syncMappingsNow,
  type UsmapSearchItem,
} from '../../api';
import { dbState, USMAP_PAGE_SIZE } from './state';
import { updateJsonStatus, clearInspector } from './inspector';

let _usmapSearchDebounceTimer: any = null;

export async function loadUsmapData(): Promise<void> {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (panel) panel.innerHTML = `<div class="db-loading">${escapeHtml(t('db.loading') || 'Loading USMAP reflection schema…')}</div>`;

  try {
    dbState.usmapSummary = await getUsmapSummary();
    dbState.usmapSearchResult = await searchUsmapEntries(dbState.usmapQuery, dbState.usmapCategory, dbState.usmapPage, USMAP_PAGE_SIZE);
    renderUsmapView();
  } catch (e) {
    showToast(`Failed to load USMAP: ${e}`, 'error');
    if (panel) panel.innerHTML = `<div class="db-error">❌ ${escapeHtml(String(e))}</div>`;
  }
}

export function renderUsmapView(): void {
  const panel = dbDom.elMaybe('db-grid-panel');
  if (!panel) return;

  const s = dbState.usmapSummary;
  const verStr = s?.gameVersion
    ? (s.gameVersion.startsWith('v') ? s.gameVersion : `v${s.gameVersion}`)
    : 'Loaded';

  // If container already exists, just update table and footer in-place to avoid re-rendering input
  const existingContainer = panel.querySelector('.db-usmap-container');
  if (existingContainer) {
    updateUsmapTableAndFooter();
    return;
  }

  panel.innerHTML = `
    <div class="db-usmap-container">
      <!-- Fixed USMAP Header Banner -->
      <div class="db-usmap-header">
        <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
          <div style="display: flex; align-items: center; gap: 8px;">
            <span style="font-size: 13px; font-weight: 700; color: var(--accent);">⚡ Palworld USMAP Reflection Schema</span>
            <span style="font-size: 11px; padding: 2px 8px; background: rgba(0, 210, 255, 0.12); border: 1px solid rgba(0, 210, 255, 0.3); border-radius: 12px; color: var(--accent); font-weight: 600;">
              ${escapeHtml(verStr)}
            </span>
          </div>
          <button id="db-usmap-sync-btn" class="btn btn-secondary btn-xs" style="display: flex; align-items: center; gap: 6px; font-size: 11px; padding: 4px 10px;">
            <span>🔄</span> <span>${escapeHtml(t('db.btn_sync_usmap') || 'Sync Mappings')}</span>
          </button>
        </div>

        <!-- Stats counter -->
        <div style="display: flex; gap: 14px; font-size: 11.5px; color: var(--text-muted); flex-wrap: wrap;">
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
            value="${escapeHtml(dbState.usmapQuery)}"
            style="flex: 1; min-width: 180px; font-size: 11.5px; padding: 6px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary);"
          />
          <div style="display: flex; gap: 4px;">
            <button class="btn btn-xs db-usmap-cat-btn ${dbState.usmapCategory === 'all' ? 'btn-primary' : 'btn-secondary'}" data-cat="all">${escapeHtml(t('db.usmap_cat_all') || 'All')}</button>
            <button class="btn btn-xs db-usmap-cat-btn ${dbState.usmapCategory === 'structs' ? 'btn-primary' : 'btn-secondary'}" data-cat="structs">${escapeHtml(t('db.usmap_cat_structs') || 'Structs')}</button>
            <button class="btn btn-xs db-usmap-cat-btn ${dbState.usmapCategory === 'enums' ? 'btn-primary' : 'btn-secondary'}" data-cat="enums">${escapeHtml(t('db.usmap_cat_enums') || 'Enums')}</button>
            <button class="btn btn-xs db-usmap-cat-btn ${dbState.usmapCategory === 'names' ? 'btn-primary' : 'btn-secondary'}" data-cat="names">${escapeHtml(t('db.usmap_cat_names') || 'FNames')}</button>
          </div>
        </div>
      </div>

      <!-- Scrollable Items Grid Table in the Middle -->
      <div id="db-usmap-table-wrap" class="db-usmap-table-wrap"></div>

      <!-- Fixed Pagination Footer -->
      <div id="db-usmap-footer" class="db-usmap-footer"></div>
    </div>
  `;

  // Setup Event Listeners for Header Controls
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
      dbState.usmapQuery = searchInput.value;
      dbState.usmapPage = 0;
      dbState.usmapSearchResult = await searchUsmapEntries(dbState.usmapQuery, dbState.usmapCategory, dbState.usmapPage, USMAP_PAGE_SIZE);
      updateUsmapTableAndFooter();
    }, 80);
  });

  panel.querySelectorAll('.db-usmap-cat-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      panel.querySelectorAll('.db-usmap-cat-btn').forEach(b => {
        b.classList.remove('btn-primary');
        b.classList.add('btn-secondary');
      });
      btn.classList.remove('btn-secondary');
      btn.classList.add('btn-primary');

      dbState.usmapCategory = (btn as HTMLElement).dataset.cat as any;
      dbState.usmapPage = 0;
      dbState.usmapSearchResult = await searchUsmapEntries(dbState.usmapQuery, dbState.usmapCategory, dbState.usmapPage, USMAP_PAGE_SIZE);
      updateUsmapTableAndFooter();
    });
  });

  updateUsmapTableAndFooter();
}

function updateUsmapTableAndFooter(): void {
  const panel = dbDom.elMaybe('db-grid-panel');
  const tableWrap = dbDom.elMaybe('db-usmap-table-wrap');
  const footerWrap = dbDom.elMaybe('db-usmap-footer');
  if (!panel || !tableWrap || !footerWrap) return;

  const s = dbState.usmapSummary;
  const res = dbState.usmapSearchResult;
  const isLoaded = s && s.loaded;
  const totalPages = res ? Math.ceil(res.totalItems / USMAP_PAGE_SIZE) || 1 : 1;

  if (!isLoaded) {
    tableWrap.innerHTML = `<div class="db-empty">USMAP file is not loaded. Click "Sync Mappings" above.</div>`;
    footerWrap.innerHTML = '';
    return;
  }

  if (!res || res.items.length === 0) {
    tableWrap.innerHTML = `<div class="db-empty">${escapeHtml(t('db.no_results') || t('common.no_results') || 'No matching entries found in USMAP schema')}</div>`;
    footerWrap.innerHTML = `
      <span style="color: var(--text-muted);">Showing <b>0</b> items</span>
      <div style="display: flex; align-items: center; gap: 8px;">
        <button class="btn btn-secondary btn-xs" disabled>◀ Prev</button>
        <span>Page <b>1</b> / 1</span>
        <button class="btn btn-secondary btn-xs" disabled>Next ▶</button>
      </div>
    `;
    return;
  }

  tableWrap.innerHTML = `
    <table class="db-grid-table usmap-table">
      <thead>
        <tr>
          <th style="width: 80px;">Type</th>
          <th>Name / Symbol</th>
          <th>Details / Super</th>
          <th style="width: 110px; text-align: right;">Count</th>
          <th style="width: 45px; text-align: center;"></th>
        </tr>
      </thead>
      <tbody>
        ${res.items.map(item => {
          const isSelected = dbState.selectedUsmapItem?.name === item.name;
          let typeBadge = '';
          if (item.category === 'struct') {
            typeBadge = `<span style="padding: 2px 7px; border-radius: 4px; font-size: 9.5px; font-weight: 700; background: rgba(56, 189, 248, 0.15); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.35); text-transform: uppercase;">Struct</span>`;
          } else if (item.category === 'enum') {
            typeBadge = `<span style="padding: 2px 7px; border-radius: 4px; font-size: 9.5px; font-weight: 700; background: rgba(245, 158, 11, 0.15); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.35); text-transform: uppercase;">Enum</span>`;
          } else {
            typeBadge = `<span style="padding: 2px 7px; border-radius: 4px; font-size: 9.5px; font-weight: 700; background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.35); text-transform: uppercase;">FName</span>`;
          }

          return `
            <tr class="db-row ${isSelected ? 'selected' : ''}" data-name="${escapeHtml(item.name)}" data-cat="${item.category}" style="cursor: pointer;">
              <td class="db-cell">${typeBadge}</td>
              <td class="db-cell db-cell-name">${escapeHtml(item.name)}</td>
              <td class="db-cell" style="font-size: 11px; color: var(--text-muted); font-family: 'Consolas', monospace;">${escapeHtml(item.preview)}</td>
              <td class="db-cell db-cell-mono" style="text-align: right; font-size: 11px;">${item.category === 'struct' ? `${item.propertyCount} props` : item.category === 'enum' ? `${item.enumValuesCount} vals` : '—'}</td>
              <td class="db-cell" style="text-align: center; padding: 4px 6px;">
                <button class="btn btn-secondary btn-xs db-row-copy-btn" data-copy="${escapeHtml(item.name)}" title="Copy Name" style="padding: 2px 6px; font-size: 10px; border-radius: 4px;">📋</button>
              </td>
            </tr>
          `;
        }).join('')}
      </tbody>
    </table>
  `;

  footerWrap.innerHTML = `
    <span style="color: var(--text-muted);">
      Showing <b>${res.items.length}</b> of <b>${res.totalItems.toLocaleString()}</b> items
    </span>
    <div style="display: flex; align-items: center; gap: 8px;">
      <button id="db-usmap-prev-btn" class="btn btn-secondary btn-xs" ${dbState.usmapPage === 0 ? 'disabled' : ''}>◀ Prev</button>
      <span>Page <b>${dbState.usmapPage + 1}</b> / ${totalPages}</span>
      <button id="db-usmap-next-btn" class="btn btn-secondary btn-xs" ${dbState.usmapPage + 1 >= totalPages ? 'disabled' : ''}>Next ▶</button>
    </div>
  `;

  // Attach Table Event Listeners
  footerWrap.querySelector('#db-usmap-prev-btn')?.addEventListener('click', async () => {
    if (dbState.usmapPage > 0) {
      dbState.usmapPage -= 1;
      dbState.usmapSearchResult = await searchUsmapEntries(dbState.usmapQuery, dbState.usmapCategory, dbState.usmapPage, USMAP_PAGE_SIZE);
      updateUsmapTableAndFooter();
    }
  });

  footerWrap.querySelector('#db-usmap-next-btn')?.addEventListener('click', async () => {
    if (dbState.usmapPage + 1 < totalPages) {
      dbState.usmapPage += 1;
      dbState.usmapSearchResult = await searchUsmapEntries(dbState.usmapQuery, dbState.usmapCategory, dbState.usmapPage, USMAP_PAGE_SIZE);
      updateUsmapTableAndFooter();
    }
  });

  tableWrap.querySelectorAll('.db-row-copy-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const val = (btn as HTMLElement).dataset.copy;
      if (val) {
        navigator.clipboard.writeText(val);
        showToast(`Copied: ${val}`, 'info');
      }
    });
  });

  tableWrap.querySelectorAll('.db-row[data-name]').forEach(row => {
    row.addEventListener('click', async () => {
      const name = (row as HTMLElement).dataset.name!;
      const item = dbState.usmapSearchResult?.items.find(i => i.name === name);
      if (item) {
        tableWrap.querySelectorAll('.db-row').forEach(r => r.classList.remove('selected'));
        row.classList.add('selected');
        await selectUsmapItem(item);
      }
    });
  });
}

export async function selectUsmapItem(item: UsmapSearchItem): Promise<void> {
  dbState.selectedUsmapItem = item;
  dbState.selectedRecordType = item.category === 'struct' ? 'usmap_struct' : item.category === 'enum' ? 'usmap_enum' : 'usmap_name';
  dbState.selectedRecordId = item.name;

  const split = document.querySelector('.db-split');
  if (split) split.classList.add('has-selection');

  if (item.category === 'struct') {
    dbState.selectedUsmapStructDetails = await getUsmapFullStructDetails(item.name);
    dbState.selectedUsmapEnumValues = null;
  } else if (item.category === 'enum') {
    dbState.selectedUsmapEnumValues = await getUsmapEnumInfo(item.name);
    dbState.selectedUsmapStructDetails = null;
  } else {
    dbState.selectedUsmapStructDetails = null;
    dbState.selectedUsmapEnumValues = null;
  }

  renderUsmapInspector();
}

export function closeUsmapInspector(): void {
  dbState.selectedUsmapItem = null;
  dbState.selectedRecordId = null;
  const split = document.querySelector('.db-split');
  if (split) split.classList.remove('has-selection');
  document.querySelectorAll('.db-row.selected').forEach(r => r.classList.remove('selected'));
  clearInspector();
}

export function renderUsmapInspector(): void {
  const customContainer = dbDom.elMaybe('db-inspector-custom-container');
  const jsonEditor = dbDom.elMaybe('db-json-editor');
  const inspectorTitle = dbDom.elMaybe('db-inspector-title');
  const inspectorActions = dbDom.elMaybe('db-inspector-actions');
  if (!customContainer || !jsonEditor) return;

  if (!dbState.selectedUsmapItem) {
    customContainer.style.display = 'none';
    jsonEditor.style.display = 'block';
    if (inspectorTitle) inspectorTitle.textContent = t('db.inspector_title') || 'Record Inspector';
    clearInspector();
    return;
  }

  const item = dbState.selectedUsmapItem;
  if (inspectorTitle) {
    let catColor = '#38bdf8';
    let catBg = 'rgba(56, 189, 248, 0.15)';
    if (item.category === 'enum') {
      catColor = '#fbbf24';
      catBg = 'rgba(245, 158, 11, 0.15)';
    } else if (item.category === 'name') {
      catColor = '#c084fc';
      catBg = 'rgba(168, 85, 247, 0.15)';
    }

    inspectorTitle.innerHTML = `
      <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; max-width: 220px;" title="${escapeHtml(item.name)}">
        <span style="font-size: 9.5px; font-weight: 700; color: ${catColor}; background: ${catBg}; padding: 1px 5px; border-radius: 3px; text-transform: uppercase; flex-shrink: 0;">${item.category}</span>
        <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; font-weight: 600; font-family: 'Consolas', monospace; color: var(--text-primary);">${escapeHtml(item.name)}</span>
      </div>
    `;
  }

  if (inspectorActions) {
    inspectorActions.innerHTML = `
      <div style="display: flex; gap: 4px; align-items: center;">
        <button id="db-inspector-toggle-visual" class="btn btn-xs ${dbState.usmapInspectorMode === 'visual' ? 'btn-primary' : 'btn-secondary'}">Schema</button>
        <button id="db-inspector-toggle-json" class="btn btn-xs ${dbState.usmapInspectorMode === 'json' ? 'btn-primary' : 'btn-secondary'}">JSON</button>
        <button id="db-inspector-close-btn" class="btn btn-secondary btn-xs" title="Close inspector (Full width table)" style="padding: 2px 7px; margin-left: 4px;">✕</button>
      </div>
    `;

    dbDom.elMaybe('db-inspector-toggle-visual')?.addEventListener('click', () => {
      dbState.usmapInspectorMode = 'visual';
      renderUsmapInspector();
    });
    dbDom.elMaybe('db-inspector-toggle-json')?.addEventListener('click', () => {
      dbState.usmapInspectorMode = 'json';
      renderUsmapInspector();
    });
    dbDom.elMaybe('db-inspector-close-btn')?.addEventListener('click', () => {
      closeUsmapInspector();
    });
  }

  const rawJsonData = dbState.selectedUsmapStructDetails || dbState.selectedUsmapEnumValues || item;

  if (dbState.usmapInspectorMode === 'json') {
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
    const details = dbState.selectedUsmapStructDetails;
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
    const vals = dbState.selectedUsmapEnumValues || [];
    customContainer.innerHTML = `
      <div style="display: flex; flex-direction: column; gap: 12px;">
        <div style="background: var(--bg-primary); padding: 12px 14px; border-radius: var(--card-radius); border: 1px solid var(--border);">
          <div style="font-size: 13px; font-weight: 700; color: #fbbf24; font-family: monospace;">enum class ${escapeHtml(item.name)}</div>
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
      <div style="background: var(--bg-primary); padding: 16px; border-radius: var(--card-radius); border: 1px solid var(--border); display: flex; flex-direction: column; gap: 10px;">
        <div style="font-size: 12px; font-weight: 700; color: #c084fc; font-family: 'Consolas', monospace; word-break: break-all;">FName: "${escapeHtml(item.name)}"</div>
        <div style="font-size: 11px; color: var(--text-muted); line-height: 1.5;">
          Global string table symbol used across Palworld engine reflection, SDK methods, and property serializations.
        </div>
        <button id="db-fname-copy-btn" class="btn btn-secondary btn-xs" style="align-self: flex-start; display: flex; align-items: center; gap: 6px; font-size: 11px; margin-top: 4px;">
          <span>📋</span> <span>Copy Symbol Name</span>
        </button>
      </div>
    `;

    dbDom.elMaybe('db-fname-copy-btn')?.addEventListener('click', () => {
      navigator.clipboard.writeText(item.name);
      showToast(`Copied: ${item.name}`, 'info');
    });
  }
}
