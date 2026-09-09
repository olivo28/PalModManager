import type { UAssetInspectionDetails, UAssetLiveProperty } from '../../../api';
import { tweakPakProperty, revertPakToBackup } from '../../../api';
import { escapeHtml, formatBytes } from '../../../utils/helpers';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';

export interface PropertyTweakerOptions {
  details: UAssetInspectionDetails;
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  onRefresh?: () => void;
}

/**
 * Renders the live instantiated properties table, inheritance banner, and tweaker controls
 */
export function renderPropertyTweakerHtml(details: UAssetInspectionDetails): string {
  const {
    instantiatedProperties = [],
    classHierarchy = [],
    hasOriginalBackup = false,
  } = details;

  return `
    <div class="uasset-tweaker-container" style="display:flex;flex-direction:column;gap:16px;">
      <!-- Backup Status Banner -->
      ${hasOriginalBackup ? `
        <div style="display:flex;justify-content:space-between;align-items:center;background:rgba(34,197,94,0.1);border:1px solid rgba(34,197,94,0.3);border-radius:6px;padding:8px 12px;">
          <div style="display:flex;align-items:center;gap:8px;font-size:12px;color:#22c55e;">
            <span>🛡️</span>
            <span><strong>${escapeHtml(t('editor.backup_active_label') || 'Original Backup Preserved:')}</strong> ${escapeHtml(t('editor.backup_active_desc') || 'A clean .original.bak copy is safely stored.')}</span>
          </div>
          <button type="button" id="btn-revert-pak-backup" style="background:rgba(239,68,68,0.15);border:1px solid rgba(239,68,68,0.3);color:#ef4444;font-size:11px;font-weight:600;padding:4px 10px;border-radius:4px;cursor:pointer;display:flex;align-items:center;gap:6px;">
            <span>↺</span> <span>${escapeHtml(t('editor.btn_revert_backup') || 'Revert to Original')}</span>
          </button>
        </div>
      ` : ''}

      <!-- Class Inheritance Chain -->
      ${classHierarchy.length > 0 ? `
        <div style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:10px 14px;">
          <div style="font-size:11px;font-weight:700;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-bottom:6px;">
            ${escapeHtml(t('editor.inheritance_chain_title') || 'C++ Inheritance Tree (UHT & .usmap)')}
          </div>
          <div style="display:flex;flex-wrap:wrap;gap:6px;align-items:center;font-size:11px;font-family:monospace;">
            ${classHierarchy.map((cls, idx) => `
              <span style="background:${idx === 0 ? 'rgba(0,188,255,0.15)' : 'rgba(255,255,255,0.05)'};color:${idx === 0 ? '#00bcff' : 'var(--text-primary)'};padding:2px 8px;border-radius:4px;border:1px solid ${idx === 0 ? 'rgba(0,188,255,0.3)' : 'var(--border)'};font-weight:${idx === 0 ? '700' : '400'};">
                ${escapeHtml(cls)}
              </span>
              ${idx < classHierarchy.length - 1 ? '<span style="color:var(--text-muted);">→</span>' : ''}
            `).join('')}
          </div>
        </div>
      ` : ''}

      <!-- Toolbar -->
      ${(() => {
        const editablePropsCount = instantiatedProperties.filter(p => p.isEditable).length;
        const allDeltasCount = instantiatedProperties.filter(p => p.isDelta).length;
        const hasDeltas = allDeltasCount > 0;

        return `
          <div style="display:flex;flex-direction:column;gap:10px;">
            ${!hasDeltas && instantiatedProperties.length > 0 ? `
              <div style="display:flex;align-items:center;gap:8px;background:rgba(56,189,248,0.08);border:1px solid rgba(56,189,248,0.25);border-radius:6px;padding:8px 12px;font-size:11.5px;color:#38bdf8;">
                <span style="font-size:13px;">ℹ️</span>
                <span>${escapeHtml(t('editor.banner_all_vanilla_matching') || 'All properties in this asset match the official Palworld defaults (0 deltas detected).')}</span>
              </div>
            ` : ''}

            <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;">
              <div style="display:flex;align-items:center;gap:10px;">
                <div style="font-size:12px;color:var(--text-secondary);">
                  <span><strong>${escapeHtml(t('editor.live_properties_count') || 'Properties:')}</strong> ${instantiatedProperties.length}</span>
                </div>
                <div class="prop-filter-group" style="display:flex;background:var(--bg-secondary);border:1px solid var(--border);border-radius:4px;overflow:hidden;padding:2px;gap:2px;">
                  <button type="button" id="prop-filter-editable" class="prop-filter-btn active" style="background:rgba(0,188,255,0.15);border:1px solid rgba(0,188,255,0.3);color:#00bcff;font-size:10.5px;font-weight:700;padding:3px 8px;border-radius:3px;cursor:pointer;">
                    🎯 ${escapeHtml(t('editor.filter_editable_tweaks') || 'Editable Properties')} (${editablePropsCount})
                  </button>
                  <button type="button" id="prop-filter-deltas" class="prop-filter-btn" style="background:none;border:1px solid transparent;color:var(--text-secondary);font-size:10.5px;font-weight:700;padding:3px 8px;border-radius:3px;cursor:pointer;">
                    ⚡ ${escapeHtml(t('editor.filter_only_deltas') || 'Only Deltas')} (${allDeltasCount})
                  </button>
                  <button type="button" id="prop-filter-all" class="prop-filter-btn" style="background:none;border:1px solid transparent;color:var(--text-secondary);font-size:10.5px;font-weight:600;padding:3px 8px;border-radius:3px;cursor:pointer;">
                    📋 ${escapeHtml(t('editor.filter_all_properties') || 'All Properties')} (${instantiatedProperties.length})
                  </button>
                </div>
              </div>
              <div>
                <input type="text" id="prop-tweaker-search" placeholder="${escapeHtml(t('editor.search_properties_placeholder') || 'Filter properties by name or type...')}" style="background:var(--bg-secondary);border:1px solid var(--border);color:var(--text-primary);padding:4px 8px;border-radius:4px;font-size:11px;width:220px;" />
              </div>
            </div>
          </div>
        `;
      })()}

      <!-- Properties Table -->
      <div style="border:1px solid var(--border);border-radius:6px;overflow:hidden;background:var(--bg-primary);">
        <table style="width:100%;border-collapse:collapse;font-size:11px;font-family:monospace;text-align:left;">
          <thead>
            <tr style="background:var(--bg-secondary);border-bottom:1px solid var(--border);">
              <th style="padding:8px 12px;width:210px;color:var(--text-primary);">${escapeHtml(t('editor.col_property_name') || 'Property Name')}</th>
              <th style="padding:8px 12px;width:120px;color:var(--text-primary);">${escapeHtml(t('editor.col_property_type') || 'Type')}</th>
              <th style="padding:8px 12px;width:130px;color:var(--text-primary);">${escapeHtml(t('editor.col_vanilla_default') || 'Vanilla Default')}</th>
              <th style="padding:8px 12px;color:var(--text-primary);">${escapeHtml(t('editor.col_mod_value') || 'Mod Value (Current)')}</th>
              <th style="padding:8px 12px;width:110px;text-align:right;color:var(--text-primary);">${escapeHtml(t('editor.col_property_actions') || 'Action')}</th>
            </tr>
          </thead>
          <tbody id="prop-tweaker-body">
            ${instantiatedProperties.length === 0 ? `
              <tr>
                <td colspan="5" style="padding:24px;text-align:center;color:var(--text-muted);font-style:italic;">
                  ${escapeHtml(t('editor.no_live_properties') || 'No scalar properties found in export payload.')}
                </td>
              </tr>
            ` : instantiatedProperties.map(prop => {
              const searchKey = `${prop.name} ${prop.propertyType} ${prop.rawValueDisplay} ${prop.vanillaDefaultDisplay || ''}`.toLowerCase();
              const isDelta = Boolean(prop.isDelta);
              const isEditable = Boolean(prop.isEditable);
              return `
                <tr class="prop-tweak-row" data-search="${escapeHtml(searchKey)}" data-delta="${isDelta}" data-editable="${isEditable}" style="border-bottom:1px solid rgba(255,255,255,0.04);transition:background 0.15s ease;">
                  <td style="padding:7px 12px;font-weight:600;color:#00bcff;">
                    <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                      <span>${escapeHtml(prop.name)}</span>
                      ${isDelta ? `
                        <span class="badge" style="background:rgba(255,209,102,0.15);color:#ffd166;border:1px solid rgba(255,209,102,0.3);font-size:9px;padding:1px 5px;font-weight:700;">
                          ⚡ ${escapeHtml(t('editor.badge_delta') || 'Delta')}
                        </span>
                      ` : ''}
                    </div>
                  </td>
                  <td style="padding:7px 12px;color:var(--text-secondary);">
                    <div style="display:flex;align-items:center;gap:4px;">
                      <span style="background:rgba(255,255,255,0.06);padding:2px 6px;border-radius:3px;font-size:10px;">
                        ${escapeHtml(prop.propertyType)}
                      </span>
                      ${!isEditable ? `
                        <span style="font-size:9px;color:var(--text-muted);opacity:0.7;">[${escapeHtml(t('editor.badge_internal_node') || 'Node')}]</span>
                      ` : ''}
                    </div>
                  </td>
                  <td style="padding:7px 12px;color:var(--text-muted);">
                    ${prop.vanillaDefaultDisplay ? `
                      <span style="background:rgba(255,255,255,0.03);border:1px solid rgba(255,255,255,0.08);padding:2px 6px;border-radius:3px;font-size:10.5px;color:var(--text-secondary);">
                        ${escapeHtml(prop.vanillaDefaultDisplay)}
                      </span>
                    ` : `<span style="color:var(--text-muted);font-size:10px;">—</span>`}
                  </td>
                  <td style="padding:7px 12px;">
                    <div style="display:flex;align-items:center;gap:8px;">
                      <span class="prop-val-display" style="color:${isDelta && isEditable ? '#ffd166' : isEditable ? 'var(--text-primary)' : 'var(--text-muted)'};font-weight:${isDelta && isEditable ? '700' : '400'};word-break:break-all;">
                        ${escapeHtml(prop.rawValueDisplay)}
                      </span>
                    </div>
                  </td>
                  <td style="padding:7px 12px;text-align:right;">
                    <div style="display:flex;align-items:center;justify-content:flex-end;gap:4px;">
                      ${prop.isEditable ? `
                        <button type="button" class="btn-tweak-prop" data-export="${prop.exportIndex}" data-name="${escapeHtml(prop.name)}" data-type="${escapeHtml(prop.propertyType)}" data-val="${escapeHtml(JSON.stringify(prop.value))}" style="background:rgba(0,188,255,0.12);border:1px solid rgba(0,188,255,0.3);color:#00bcff;padding:3px 8px;border-radius:4px;cursor:pointer;font-size:10px;font-weight:600;">
                          ${escapeHtml(t('editor.btn_edit') || 'Edit')}
                        </button>
                      ` : `
                        <span style="color:var(--text-muted);font-size:10px;">—</span>
                      `}
                      ${prop.isEditable && prop.vanillaDefaultDisplay && prop.rawValueDisplay !== prop.vanillaDefaultDisplay ? `
                        <button type="button" class="btn-reset-prop" data-export="${prop.exportIndex}" data-name="${escapeHtml(prop.name)}" data-type="${escapeHtml(prop.propertyType)}" data-val="${escapeHtml(prop.vanillaDefaultDisplay)}" style="background:rgba(239,68,68,0.12);border:1px solid rgba(239,68,68,0.3);color:#ef4444;padding:3px 6px;border-radius:4px;cursor:pointer;font-size:10px;font-weight:600;" title="${escapeHtml(t('editor.btn_reset_vanilla') || 'Reset to Default')}">
                          ↺
                        </button>
                      ` : ''}
                    </div>
                  </td>
                </tr>
              `;
            }).join('')}
          </tbody>
        </table>
      </div>
    </div>
  `;
}

/**
 * Binds events for property tweaker (search filtering, value editing, and backup revert)
 */
export function setupPropertyTweakerEvents(
  container: HTMLElement,
  options: PropertyTweakerOptions
): void {
  const { pakPath, assetInternalPath, modId, onRefresh } = options;

  // 1. Live Search & 3-Way Delta Filter
  const searchInput = container.querySelector('#prop-tweaker-search') as HTMLInputElement | null;
  const filterEditableBtn = container.querySelector('#prop-filter-editable') as HTMLButtonElement | null;
  const filterDeltasBtn = container.querySelector('#prop-filter-deltas') as HTMLButtonElement | null;
  const filterAllBtn = container.querySelector('#prop-filter-all') as HTMLButtonElement | null;

  type FilterMode = 'editable' | 'deltas' | 'all';
  let currentFilterMode: FilterMode = filterEditableBtn ? 'editable' : (filterDeltasBtn ? 'deltas' : 'all');

  const applyFilters = () => {
    const q = searchInput?.value.trim().toLowerCase() || '';
    container.querySelectorAll('.prop-tweak-row').forEach(row => {
      const el = row as HTMLElement;
      const s = el.dataset.search || '';
      const isDelta = el.dataset.delta === 'true';
      const isEditable = el.dataset.editable === 'true';
      const matchesSearch = !q || s.includes(q);

      let matchesFilter = true;
      if (currentFilterMode === 'editable') {
        matchesFilter = isEditable;
      } else if (currentFilterMode === 'deltas') {
        matchesFilter = isDelta;
      }

      el.style.display = (matchesSearch && matchesFilter) ? '' : 'none';
    });
  };

  const updateFilterBtnStyles = () => {
    if (filterEditableBtn) {
      const active = currentFilterMode === 'editable';
      filterEditableBtn.classList.toggle('active', active);
      filterEditableBtn.style.background = active ? 'rgba(0,188,255,0.15)' : 'none';
      filterEditableBtn.style.borderColor = active ? 'rgba(0,188,255,0.3)' : 'transparent';
      filterEditableBtn.style.color = active ? '#00bcff' : 'var(--text-secondary)';
    }
    if (filterDeltasBtn) {
      const active = currentFilterMode === 'deltas';
      filterDeltasBtn.classList.toggle('active', active);
      filterDeltasBtn.style.background = active ? 'rgba(255,209,102,0.15)' : 'none';
      filterDeltasBtn.style.borderColor = active ? 'rgba(255,209,102,0.3)' : 'transparent';
      filterDeltasBtn.style.color = active ? '#ffd166' : 'var(--text-secondary)';
    }
    if (filterAllBtn) {
      const active = currentFilterMode === 'all';
      filterAllBtn.classList.toggle('active', active);
      filterAllBtn.style.background = active ? 'rgba(255,255,255,0.1)' : 'none';
      filterAllBtn.style.borderColor = active ? 'rgba(255,255,255,0.2)' : 'transparent';
      filterAllBtn.style.color = active ? 'var(--text-primary)' : 'var(--text-secondary)';
    }
  };

  if (searchInput) {
    searchInput.addEventListener('input', applyFilters);
  }

  if (filterEditableBtn) {
    filterEditableBtn.addEventListener('click', () => {
      currentFilterMode = 'editable';
      updateFilterBtnStyles();
      applyFilters();
    });
  }

  if (filterDeltasBtn) {
    filterDeltasBtn.addEventListener('click', () => {
      currentFilterMode = 'deltas';
      updateFilterBtnStyles();
      applyFilters();
    });
  }

  if (filterAllBtn) {
    filterAllBtn.addEventListener('click', () => {
      currentFilterMode = 'all';
      updateFilterBtnStyles();
      applyFilters();
    });
  }

  // Initial filter apply & style setup
  updateFilterBtnStyles();
  applyFilters();

  // 2. Revert Backup Button
  const revertBtn = container.querySelector('#btn-revert-pak-backup') as HTMLButtonElement | null;
  if (revertBtn && pakPath) {
    revertBtn.addEventListener('click', async () => {
      const confirmed = await showConfirm(
        t('editor.revert_confirm_title') || 'Revert .pak to Original Backup',
        t('editor.revert_confirm_desc') || 'Are you sure you want to restore the original backup over this .pak file? All custom property modifications will be undone.'
      );
      if (!confirmed) return;

      try {
        revertBtn.disabled = true;
        const res = await revertPakToBackup({ modId, pakPath });
        if (res.success) {
          showToast(res.message, 'success');
          if (onRefresh) onRefresh();
        } else {
          showToast(res.message, 'error');
        }
      } catch (err) {
        showToast(String(err), 'error');
      } finally {
        revertBtn.disabled = false;
      }
    });
  }

  // Helper for applying property edits
  const executePropertyEdit = async (
    btn: HTMLElement,
    exportIndex: number,
    propertyName: string,
    propertyType: string,
    newVal: any,
    loadingText: string,
    defaultText: string
  ) => {
    if (!pakPath) {
      showToast(t('editor.err_no_pak_path') || 'Cannot edit: pak path not resolved.', 'error');
      return;
    }

    try {
      (btn as HTMLButtonElement).textContent = loadingText;
      const res = await tweakPakProperty({
        modId,
        pakPath,
        assetInternalPath,
        exportIndex,
        propertyName,
        newValue: newVal,
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
      (btn as HTMLButtonElement).textContent = defaultText;
    }
  };

  // 3. Edit Property Buttons
  container.querySelectorAll('.btn-tweak-prop').forEach(btn => {
    btn.addEventListener('click', async () => {
      const exportIndex = parseInt((btn as HTMLElement).dataset.export || '0', 10);
      const propertyName = (btn as HTMLElement).dataset.name || '';
      const propertyType = (btn as HTMLElement).dataset.type || '';
      const rawValJson = (btn as HTMLElement).dataset.val || '';

      let currentVal: any = '';
      try {
        currentVal = JSON.parse(rawValJson);
      } catch {
        currentVal = rawValJson;
      }

      const newValStr = prompt(`Edit property [${propertyName} (${propertyType})]:`, String(currentVal));
      if (newValStr === null || newValStr === String(currentVal)) return;

      let parsedVal: any = newValStr;
      if (propertyType.includes('Int') || propertyType.includes('Byte')) {
        const n = parseInt(newValStr, 10);
        if (isNaN(n)) {
          showToast('Invalid integer value', 'error');
          return;
        }
        parsedVal = n;
      } else if (propertyType.includes('Float') || propertyType.includes('Double')) {
        const f = parseFloat(newValStr);
        if (isNaN(f)) {
          showToast('Invalid float value', 'error');
          return;
        }
        parsedVal = f;
      } else if (propertyType.includes('Bool')) {
        parsedVal = newValStr.toLowerCase() === 'true' || newValStr === '1';
      }

      await executePropertyEdit(
        btn as HTMLElement,
        exportIndex,
        propertyName,
        propertyType,
        parsedVal,
        '⏳',
        t('editor.btn_edit') || 'Edit'
      );
    });
  });

  // 4. Reset to Vanilla Default Buttons
  container.querySelectorAll('.btn-reset-prop').forEach(btn => {
    btn.addEventListener('click', async () => {
      const exportIndex = parseInt((btn as HTMLElement).dataset.export || '0', 10);
      const propertyName = (btn as HTMLElement).dataset.name || '';
      const propertyType = (btn as HTMLElement).dataset.type || '';
      const vanillaValStr = (btn as HTMLElement).dataset.val || '';

      const confirmed = await showConfirm(
        t('editor.reset_prop_confirm_title') || 'Reset Property to Vanilla',
        `${t('editor.reset_prop_confirm_desc') || 'Reset property'} [${propertyName}] ${t('editor.to_vanilla_value') || 'to vanilla value'}: ${vanillaValStr}?`
      );
      if (!confirmed) return;

      let parsedVal: any = vanillaValStr;
      if (propertyType.includes('Int') || propertyType.includes('Byte')) {
        parsedVal = parseInt(vanillaValStr, 10);
      } else if (propertyType.includes('Float') || propertyType.includes('Double')) {
        parsedVal = parseFloat(vanillaValStr);
      } else if (propertyType.includes('Bool')) {
        parsedVal = vanillaValStr.toLowerCase() === 'true' || vanillaValStr === '1';
      }

      await executePropertyEdit(
        btn as HTMLElement,
        exportIndex,
        propertyName,
        propertyType,
        parsedVal,
        '⏳',
        '↺'
      );
    });
  });
}
