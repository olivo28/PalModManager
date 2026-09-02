import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import type { ScanResult } from '../../mod';

export function buildHookAndTableConflictsHtml(
  res: ScanResult,
  tableCount: number,
  hookCount: number,
  hasConflicts: boolean,
  pakConflictsHtml: string
): string {
  if (!hasConflicts) {
    return `
      ${pakConflictsHtml}
      ${!pakConflictsHtml ? `
        <div class="scanner-clean-state" style="margin-bottom: 20px;">
          <div class="scanner-clean-icon">✅</div>
          <div class="scanner-clean-title">${escapeHtml(t('scanner.no_conflicts_title'))}</div>
          <div class="scanner-clean-desc">${escapeHtml(t('scanner.no_conflicts_desc'))}</div>
        </div>
      ` : ''}
    `;
  }

  return `
    ${pakConflictsHtml}
    <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
      <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
        <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
          <span>${escapeHtml(t('scanner.hook_conflicts_title', { count: hookCount }))}</span>
          <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.hook_conflicts_desc'))}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; gap: 14px;">
          ${hookCount > 0 ? res.hookConflicts.map(c => `
            <div class="scanner-conflict-item">
              <div class="scanner-conflict-header">
                <span class="scanner-conflict-title">${escapeHtml(c.hookTarget)}</span>
                <span class="scanner-conflict-type lua">${escapeHtml(c.hookFn)}</span>
              </div>
              <div class="scanner-conflict-mods">
                ${c.mods.map(m => `
                  <div class="scanner-conflict-mod-row" style="display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 6px;">
                    <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 2px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
                    </div>
                    <div style="display: flex; align-items: center; gap: 6px;">
                      <button class="btn btn-secondary btn-xs scan-code-jump-btn" data-mod-id="${escapeHtml(m.modId)}" data-file-path="${escapeHtml(m.filePath)}" data-line="${m.lineNumber}" style="font-size: 10px; padding: 2px 8px;">
                        📝 ${escapeHtml(t('common.edit') || 'Edit')}
                      </button>
                      <button class="btn btn-danger-subtle btn-xs scan-disable-mod-btn" data-mod-id="${escapeHtml(m.modId)}" style="font-size: 10px; padding: 2px 8px;">
                        🚫 ${escapeHtml(t('scanner.btn_disable_mod') || 'Disable')}
                      </button>
                    </div>
                  </div>
                `).join('')}
              </div>
            </div>
          `).join('') : `<div style="color:var(--text-muted); font-size:11px;">${escapeHtml(t('scanner.hook_conflicts_none'))}</div>`}
        </div>
      </details>

      <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
        <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
          <span>${escapeHtml(t('scanner.table_conflicts_title', { count: tableCount }))}</span>
          <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.table_conflicts_desc'))}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; gap: 14px;">
          ${tableCount > 0 ? res.tableConflicts.map(c => `
            <div class="scanner-conflict-item">
              <div class="scanner-conflict-header">
                <span class="scanner-conflict-title">${escapeHtml(c.tableName)}</span>
                <span class="scanner-conflict-type json">${escapeHtml(c.rowName)}</span>
              </div>
              <div class="scanner-conflict-mods">
                ${c.mods.map(m => `
                  <div class="scanner-conflict-mod-row" style="display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 6px;">
                    <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 2px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
                    </div>
                    <div style="display: flex; align-items: center; gap: 6px;">
                      <button class="btn btn-secondary btn-xs scan-code-jump-btn" data-mod-id="${escapeHtml(m.modId)}" data-file-path="${escapeHtml(m.filePath)}" data-line="${m.lineNumber}" style="font-size: 10px; padding: 2px 8px;">
                        📝 ${escapeHtml(t('common.edit') || 'Edit')}
                      </button>
                      <button class="btn btn-danger-subtle btn-xs scan-disable-mod-btn" data-mod-id="${escapeHtml(m.modId)}" style="font-size: 10px; padding: 2px 8px;">
                        🚫 ${escapeHtml(t('scanner.btn_disable_mod') || 'Disable')}
                      </button>
                    </div>
                  </div>
                `).join('')}
              </div>
            </div>
          `).join('') : `<div style="color:var(--text-muted); font-size:11px;">${escapeHtml(t('scanner.table_conflicts_none'))}</div>`}
        </div>
      </details>
    </div>
  `;
}

export function buildInternalConflictsHtml(res: ScanResult): string {
  const internalTableCount = res.internalTableConflicts ? res.internalTableConflicts.length : 0;
  const internalHookCount = res.internalHookConflicts ? res.internalHookConflicts.length : 0;
  const hasInternalConflicts = internalTableCount > 0 || internalHookCount > 0;

  if (!hasInternalConflicts) return '';

  return `
    <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
      <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer; border-color: rgba(255, 165, 0, 0.2);" open>
        <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(255, 165, 0, 0.03); border-bottom: 1px solid rgba(255, 165, 0, 0.08);">
          <span style="color: var(--warning); display: flex; align-items: center; gap: 6px; font-weight: 700;">
            ${escapeHtml(t('scanner.internal_conflicts_title', { count: internalTableCount + internalHookCount }))}
          </span>
          <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.internal_conflicts_desc'))}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; gap: 14px; padding-top: 14px;">
          ${internalHookCount > 0 ? `
            <div style="font-weight: 700; font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">${escapeHtml(t('scanner.internal_ue4ss_duplicates'))}</div>
            ${res.internalHookConflicts.map(c => `
              <div class="scanner-conflict-item" style="border-left: 2.5px solid var(--warning);">
                <div class="scanner-conflict-header">
                  <span class="scanner-conflict-title">${escapeHtml(c.hookTarget)}</span>
                  <span class="scanner-conflict-type lua">${escapeHtml(c.hookFn)}</span>
                </div>
                <div class="scanner-conflict-mods">
                  ${c.mods.map(m => `
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      ${m.detail ? `
                        <div style="font-size: 10px; color: var(--text-muted); background: rgba(0,0,0,0.2); padding: 4px 8px; border-radius: 4px; font-family: monospace; border: 1px solid var(--border); margin-left: 4px; margin-top: 2px;">
                          ${escapeHtml(t('scanner.line_number_prefix', { line: m.lineNumber, detail: m.detail }))}
                        </div>
                      ` : ''}
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('')}
          ` : ''}
          
          ${internalTableCount > 0 ? `
            <div style="font-weight: 700; font-size: 11px; color: var(--text-secondary); margin-top: 10px; margin-bottom: 4px;">${escapeHtml(t('scanner.internal_palschema_duplicates'))}</div>
            ${res.internalTableConflicts.map(c => `
              <div class="scanner-conflict-item" style="border-left: 2.5px solid var(--warning);">
                <div class="scanner-conflict-header">
                  <span class="scanner-conflict-title">${escapeHtml(c.tableName)}::${escapeHtml(c.rowName)}</span>
                  <span class="scanner-conflict-type">PalSchema</span>
                </div>
                <div class="scanner-conflict-mods">
                  ${c.mods.map(m => `
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)})</span>
                      </div>
                      ${m.detail ? `
                        <div style="font-size: 10px; color: var(--text-muted); background: rgba(0,0,0,0.2); padding: 4px 8px; border-radius: 4px; font-family: monospace; border: 1px solid var(--border); margin-left: 4px; margin-top: 2px;">
                          ${escapeHtml(m.detail)}
                        </div>
                      ` : ''}
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('')}
          ` : ''}
        </div>
      </details>
    </div>
  `;
}
