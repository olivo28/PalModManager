import type { UAssetDataTableGrid } from '../../../api';
import { tweakDataTableCell } from '../../../api';
import { escapeHtml } from '../../../utils/helpers';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';

export interface DataTableGridOptions {
  grid: UAssetDataTableGrid;
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  onRefresh?: () => void;
}

/**
 * Renders an interactive, searchable DataTable grid
 */
export function renderDataTableGridHtml(grid: UAssetDataTableGrid): string {
  const { rowStructName, columns, rows, totalRows } = grid;

  const rowsJsonObj: Record<string, Record<string, any>> = {};
  for (const row of rows) {
    rowsJsonObj[row.rowName] = row.values;
  }
  const fullJsonStr = JSON.stringify(rowsJsonObj, null, 2);

  const getColTooltip = (col: string): string => {
    switch (col) {
      case 'RelicType': return 'Statue Upgrade Category (EPalRelicType: CapturePower, HungerReduction, MoveSpeed, etc.)';
      case 'Rank': return 'Statue Upgrade Level / Rank';
      case 'RequiredRelicNum': return 'Lifmunk Effigies (relics) required to upgrade';
      case 'EffectRate': return 'Bonus increase rate multiplier (%)';
      case 'ResetRequiredMoney': return 'Gold required to reset upgrade at the statue';
      default: return col;
    }
  };

  return `
    <div class="uasset-dt-container" style="display:flex;flex-direction:column;gap:12px;height:100%;min-height:0;">
      <!-- Toolbar -->
      <div style="display:flex;justify-content:space-between;align-items:center;background:var(--bg-secondary);padding:8px 12px;border-radius:6px;border:1px solid var(--border);flex-wrap:wrap;gap:8px;">
        <div style="display:flex;gap:12px;align-items:center;font-size:12px;">
          <span><strong>${escapeHtml(t('editor.dt_struct_label') || 'Row Struct:')}</strong> <code style="color:#00bcff;">${escapeHtml(rowStructName)}</code></span>
          <span style="color:var(--text-muted);">|</span>
          <span><strong>${escapeHtml(t('editor.dt_rows_label') || 'Rows:')}</strong> <span style="color:var(--text-primary);font-weight:700;">${totalRows.toLocaleString()}</span></span>
          <span style="color:var(--text-muted);">|</span>
          <span><strong>${escapeHtml(t('editor.dt_cols_label') || 'Columns:')}</strong> <span style="color:var(--text-primary);font-weight:700;">${columns.length}</span></span>
        </div>
        
        <div style="display:flex;gap:8px;align-items:center;">
          <!-- View Switcher -->
          <div style="display:flex;background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;overflow:hidden;padding:1px;">
            <button type="button" id="btn-dt-mode-table" class="dt-view-mode-btn active" style="padding:4px 9px;font-size:11px;font-weight:700;border:none;background:rgba(0,188,255,0.2);color:#00bcff;cursor:pointer;border-radius:3px;">
              📊 Table Grid
            </button>
            <button type="button" id="btn-dt-mode-json" class="dt-view-mode-btn" style="padding:4px 9px;font-size:11px;font-weight:700;border:none;background:none;color:var(--text-secondary);cursor:pointer;border-radius:3px;">
              📜 JSON (FModel)
            </button>
          </div>

          <button type="button" id="btn-dt-copy-json" style="padding:4px 8px;font-size:11px;border:1px solid var(--border);background:var(--bg-primary);color:var(--text-secondary);cursor:pointer;border-radius:4px;" title="Copy DataTable as JSON">
            📋 Copy
          </button>

          <input type="text" id="dt-grid-search" placeholder="${escapeHtml(t('editor.dt_search_placeholder') || 'Search rows & values...')}" style="background:var(--bg-primary);border:1px solid var(--border);color:var(--text-primary);padding:4px 8px;border-radius:4px;font-size:11px;width:180px;" />
        </div>
      </div>

      <!-- Table Viewport -->
      <div id="uasset-dt-table-viewport" style="flex:1;overflow:auto;border:1px solid var(--border);border-radius:6px;background:var(--bg-primary);max-height:480px;">
        <table class="uasset-dt-table" style="width:100%;border-collapse:collapse;font-size:11px;font-family:monospace;text-align:left;">
          <thead style="position:sticky;top:0;background:var(--bg-secondary);z-index:2;border-bottom:2px solid var(--border);">
            <tr>
              <th style="padding:8px 12px;border-right:1px solid var(--border);min-width:140px;color:var(--accent);font-weight:700;">RowKey</th>
              ${columns.map(col => `
                <th style="padding:8px 12px;border-right:1px solid var(--border);min-width:120px;white-space:nowrap;color:var(--text-primary);" title="${escapeHtml(getColTooltip(col))}">
                  <span>${escapeHtml(col)}</span>
                  <span style="font-size:9px;color:var(--text-muted);font-weight:normal;margin-left:4px;" title="${escapeHtml(getColTooltip(col))}">ℹ️</span>
                </th>
              `).join('')}
            </tr>
          </thead>
          <tbody id="dt-grid-body">
            ${rows.map(row => {
              const searchContent = `${row.rowName} ${Object.values(row.values).map(v => typeof v === 'object' ? JSON.stringify(v) : String(v)).join(' ')}`.toLowerCase();
              return `
                <tr class="dt-row-item" data-search="${escapeHtml(searchContent)}" style="border-bottom:1px solid rgba(255,255,255,0.05);transition:background 0.15s ease;">
                  <td style="padding:6px 12px;border-right:1px solid var(--border);font-weight:700;color:var(--text-primary);background:rgba(255,255,255,0.02);">
                    ${escapeHtml(row.rowName)}
                  </td>
                  ${columns.map(col => {
                    const val = row.values[col];
                    const isEnum = typeof val === 'string' && val.includes('::');
                    let valStr = val === undefined ? '<span style="color:var(--text-muted);">null</span>' : (typeof val === 'object' ? JSON.stringify(val) : String(val));
                    if (isEnum) {
                      valStr = `<span style="color:#c084fc;font-weight:600;">${escapeHtml(String(val))}</span>`;
                    }
                    const isPrimitive = typeof val === 'number' || typeof val === 'boolean' || typeof val === 'string';
                    return `
                      <td class="dt-cell-item" data-row="${escapeHtml(row.rowName)}" data-col="${escapeHtml(col)}" data-val="${escapeHtml(String(val ?? ''))}" style="padding:6px 12px;border-right:1px solid rgba(255,255,255,0.05);white-space:nowrap;max-width:260px;overflow:hidden;text-overflow:ellipsis;" title="${escapeHtml(String(val ?? ''))}">
                        <div style="display:flex;align-items:center;justify-content:space-between;gap:4px;">
                          <span class="dt-cell-val" style="overflow:hidden;text-overflow:ellipsis;">${valStr}</span>
                          ${isPrimitive ? `
                            <button type="button" class="btn-dt-cell-edit" title="${escapeHtml(t('editor.btn_edit_cell') || 'Edit Cell')}" style="opacity:0.4;border:none;background:rgba(0,188,255,0.15);color:#00bcff;padding:1px 4px;border-radius:3px;cursor:pointer;font-size:10px;transition:opacity 0.15s ease;">
                              ✏️
                            </button>
                          ` : ''}
                        </div>
                      </td>
                    `;
                  }).join('')}
                </tr>
              `;
            }).join('')}
          </tbody>
        </table>
      </div>

      <!-- JSON Viewport (FModel Style) -->
      <div id="uasset-dt-json-viewport" style="display:none;flex:1;overflow:auto;border:1px solid var(--border);border-radius:6px;background:var(--bg-secondary);max-height:480px;padding:12px;">
        <pre id="dt-json-code" style="margin:0;font-family:monospace;font-size:11.5px;color:#e2e8f0;line-height:1.45;white-space:pre-wrap;">${escapeHtml(fullJsonStr)}</pre>
      </div>
    </div>
  `;
}

/**
 * Binds interactive events for DataTable grid (search filter, view modes, and cell editing)
 */
export function setupDataTableGridEvents(
  container: HTMLElement,
  options: DataTableGridOptions
): void {
  const { pakPath, assetInternalPath, modId, onRefresh } = options;

  // 1. View Mode Switching (Table vs JSON)
  const btnTable = container.querySelector('#btn-dt-mode-table') as HTMLElement | null;
  const btnJson = container.querySelector('#btn-dt-mode-json') as HTMLElement | null;
  const tableViewport = container.querySelector('#uasset-dt-table-viewport') as HTMLElement | null;
  const jsonViewport = container.querySelector('#uasset-dt-json-viewport') as HTMLElement | null;

  if (btnTable && btnJson && tableViewport && jsonViewport) {
    btnTable.addEventListener('click', () => {
      btnTable.classList.add('active');
      btnTable.style.background = 'rgba(0,188,255,0.2)';
      btnTable.style.color = '#00bcff';
      btnJson.classList.remove('active');
      btnJson.style.background = 'none';
      btnJson.style.color = 'var(--text-secondary)';
      tableViewport.style.display = 'block';
      jsonViewport.style.display = 'none';
    });

    btnJson.addEventListener('click', () => {
      btnJson.classList.add('active');
      btnJson.style.background = 'rgba(0,188,255,0.2)';
      btnJson.style.color = '#00bcff';
      btnTable.classList.remove('active');
      btnTable.style.background = 'none';
      btnTable.style.color = 'var(--text-secondary)';
      tableViewport.style.display = 'none';
      jsonViewport.style.display = 'block';
    });
  }

  // 2. Copy JSON Button
  const btnCopy = container.querySelector('#btn-dt-copy-json') as HTMLElement | null;
  const jsonCodeEl = container.querySelector('#dt-json-code') as HTMLElement | null;
  if (btnCopy && jsonCodeEl) {
    btnCopy.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(jsonCodeEl.textContent || '');
        btnCopy.textContent = '✅ Copied!';
        setTimeout(() => { btnCopy.textContent = '📋 Copy'; }, 2000);
      } catch (_) {
        showToast('Failed to copy to clipboard', 'error');
      }
    });
  }

  // 3. Live Search
  const searchInput = container.querySelector('#dt-grid-search') as HTMLInputElement | null;
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const q = searchInput.value.trim().toLowerCase();
      container.querySelectorAll('.dt-row-item').forEach(el => {
        const s = (el as HTMLElement).dataset.search || '';
        (el as HTMLElement).style.display = (!q || s.includes(q)) ? '' : 'none';
      });
    });
  }

  // 4. Cell Hover & Edit
  container.querySelectorAll('.dt-cell-item').forEach(cellEl => {
    const btn = cellEl.querySelector('.btn-dt-cell-edit') as HTMLElement | null;
    if (btn) {
      cellEl.addEventListener('mouseenter', () => { btn.style.opacity = '1'; });
      cellEl.addEventListener('mouseleave', () => { btn.style.opacity = '0.4'; });

      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (!pakPath) {
          showToast(t('editor.err_no_pak_path') || 'Cannot edit: pak path not resolved.', 'error');
          return;
        }

        const row = (cellEl as HTMLElement).dataset.row || '';
        const col = (cellEl as HTMLElement).dataset.col || '';
        const currentVal = (cellEl as HTMLElement).dataset.val || '';

        const newValStr = prompt(`Edit [Row ${row} -> Column ${col}]:`, currentVal);
        if (newValStr === null || newValStr === currentVal) return;

        let parsedVal: any = newValStr;
        if (!isNaN(Number(newValStr)) && newValStr.trim() !== '') {
          parsedVal = Number(newValStr);
        } else if (newValStr.toLowerCase() === 'true') {
          parsedVal = true;
        } else if (newValStr.toLowerCase() === 'false') {
          parsedVal = false;
        }

        try {
          btn.textContent = '⏳';
          const res = await tweakDataTableCell({
            modId,
            pakPath,
            assetInternalPath,
            rowName: row,
            columnName: col,
            newValue: parsedVal,
          });

          if (res.success) {
            showToast(res.message, 'success');
            if (onRefresh) onRefresh();
          } else {
            showToast(res.message, 'error');
          }
        } catch (err) {
          showToast(String(err), 'error');
        } finally {
          btn.textContent = '✏️';
        }
      });
    }
  });
}
