import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../toast';
import { lastScanResult, setLastScanResult, setIsScanning, activeSubTab, isScanning, renderScannerView, subTabHeader } from './mod';
import { escapeHtml } from './rendering';
import { t } from '../../utils/i18n';

import { ConflictingMod, TableRowConflict, HookConflict, ModSummary, ScanResult } from './mod';

export async function runScan(): Promise<void> {
  if (isScanning) return;
  setIsScanning(true);
  renderScannerView();

  try {
    const result = await invoke<ScanResult>('scan_conflicts');
    setLastScanResult(result);
    showToast(t('scanner.toast_scan_success', { count: result.totalScanned }), 'success');
  } catch (err: any) {
    console.error(err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  } finally {
    setIsScanning(false);
    renderScannerView();
  }
}

export async function renderConflictsPanel(container: HTMLElement): Promise<void> {
  if (!lastScanResult) {
    container.innerHTML = `
      ${subTabHeader()}
      <div style="flex:1; display:flex; align-items:center; justify-content:center; padding:24px;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon">🛡</div>
          <div class="scanner-hero-title">${escapeHtml(t('scanner.hero_initial_title'))}</div>
          <div class="scanner-hero-desc">
            ${escapeHtml(t('scanner.hero_initial_desc'))}
          </div>
          <button id="scanner-start-btn" class="scanner-btn-run">
            <span>${escapeHtml(t('scanner.btn_run_scanner'))}</span>
          </button>
        </div>
      </div>
    `;
    const { setupEventListeners } = await import('./mod');
    setupEventListeners();
    return;
  }

  const res = lastScanResult;
  const tableCount = res.tableConflicts.length;
  const hookCount = res.hookConflicts.length;
  const hasConflicts = tableCount > 0 || hookCount > 0;

  let contentHtml = '';
  if (!hasConflicts) {
    contentHtml = `
      <div class="scanner-clean-state" style="margin-bottom: 20px;">
        <div class="scanner-clean-icon">✅</div>
        <div class="scanner-clean-title">${escapeHtml(t('scanner.no_conflicts_title'))}</div>
        <div class="scanner-clean-desc">${escapeHtml(t('scanner.no_conflicts_desc'))}</div>
      </div>
    `;
  } else {
    contentHtml = `
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
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
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
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
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

  // 1. Internal conflicts HTML
  const internalTableCount = res.internalTableConflicts ? res.internalTableConflicts.length : 0;
  const internalHookCount = res.internalHookConflicts ? res.internalHookConflicts.length : 0;
  const hasInternalConflicts = internalTableCount > 0 || internalHookCount > 0;

  let internalConflictsHtml = '';
  if (hasInternalConflicts) {
    internalConflictsHtml = `
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

  // 2. Active Mod Registries Summaries HTML
  let summariesHtml = '';
  if (res.modSummaries && res.modSummaries.length > 0) {
    summariesHtml = `
      <div class="scanner-card-section" style="margin-bottom: 20px;">
        <div class="scanner-card-header">
          <span>${escapeHtml(t('scanner.registries_title'))}</span>
          <span style="font-size:10px;color:var(--text-muted);">${escapeHtml(t('scanner.registries_desc'))}</span>
        </div>
        <div class="scanner-card-body scanner-mod-summary-grid">
          ${res.modSummaries.map(m => {
            const hasRows = m.palschemaRows && m.palschemaRows.length > 0;
            const hasHooks = m.ue4ssHooks && m.ue4ssHooks.length > 0;
            if (!hasRows && !hasHooks) return '';

            let badgeHtml = '';
            const dynamicType = (hasRows && hasHooks) ? 'Hybrid' : (hasRows ? 'PalSchema' : 'Ue4ss');
            if (dynamicType === 'Ue4ss') {
              badgeHtml = `<span class="scanner-conflict-type lua" style="font-size: 9px; padding: 2px 6px; margin-left: 8px; font-weight:700;">UE4SS</span>`;
            } else if (dynamicType === 'PalSchema') {
              badgeHtml = `<span class="scanner-conflict-type" style="font-size: 9px; padding: 2px 6px; background: rgba(255,165,0,0.1); color: var(--warning); margin-left: 8px; font-weight:700;">PalSchema</span>`;
            } else if (dynamicType === 'Hybrid') {
              badgeHtml = `<span class="scanner-conflict-type" style="font-size: 9px; padding: 2px 6px; background: rgba(147,112,219,0.15); color: rgb(186,104,200); margin-left: 8px; font-weight:700;">Hybrid</span>`;
            }

            return `
              <details style="border: 1px solid var(--border); border-radius: 4px; padding: 10px; cursor: pointer; background: rgba(255,255,255,0.01); min-width: 0;">
                <summary style="font-weight: 700; color: var(--text-primary); outline: none; display: flex; align-items: center; justify-content: space-between;">
                  <div style="display: flex; align-items: center; gap: 4px; min-width: 0;">
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(m.modName)}</span>
                    ${badgeHtml}
                  </div>
                  <span style="font-weight: normal; font-size: 11px; color: var(--text-muted); flex-shrink: 0; margin-left: 8px;">
                    ${hasRows ? `${m.palschemaRows.length} ${escapeHtml(t('scanner.row_count', { count: m.palschemaRows.length }))}` : ''} 
                    ${hasRows && hasHooks ? ' | ' : ''} 
                    ${hasHooks ? `${m.ue4ssHooks.length} ${escapeHtml(t('scanner.hook_count', { count: m.ue4ssHooks.length }))}` : ''}
                  </span>
                </summary>
                <div style="cursor: default; padding-top: 10px; display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px;">
                  <div style="min-width: 0;">
                    <div style="font-size: 11px; font-weight: 700; color: var(--warning); text-transform: uppercase; margin-bottom: 6px;">${escapeHtml(t('scanner.palschema_edits_label'))}</div>
                    ${hasRows ? `
                      <ul class="scanner-detail-list" style="margin: 0; padding-left: 16px; font-size: 11px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 4px; overflow-wrap: anywhere; word-break: break-word; white-space: normal; max-height: 220px; overflow-y: auto;">
                        ${m.palschemaRows.map(r => `<li>${escapeHtml(r)}</li>`).join('')}
                      </ul>
                    ` : `<div style="font-size: 11px; color: var(--text-muted); font-style: italic;">${escapeHtml(t('common.none'))}</div>`}
                  </div>
                  <div style="min-width: 0;">
                    <div style="font-size: 11px; font-weight: 700; color: var(--accent); text-transform: uppercase; margin-bottom: 6px;">${escapeHtml(t('scanner.ue4ss_hooks_label'))}</div>
                    ${hasHooks ? `
                      <ul class="scanner-detail-list" style="margin: 0; padding-left: 16px; font-size: 11px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 4px; overflow-wrap: anywhere; word-break: break-word; white-space: normal; max-height: 220px; overflow-y: auto;">
                        ${m.ue4ssHooks.map(h => `<li>${escapeHtml(h)}</li>`).join('')}
                      </ul>
                    ` : `<div style="font-size: 11px; color: var(--text-muted); font-style: italic;">${escapeHtml(t('common.none'))}</div>`}
                  </div>
                </div>
              </details>
            `;
          }).join('')}
        </div>
      </div>
    `;
  }

  // 3. Warnings HTML
  let warningsHtml = '';
  if (res.warnings && res.warnings.length > 0) {
    warningsHtml = `
      <details class="scanner-card-section" style="margin-top: 24px; cursor: pointer;">
        <summary class="scanner-card-header" style="outline:none;">
          <span>${escapeHtml(t('scanner.warnings_title', { count: res.warnings.length }))}</span>
          <span style="font-size:10px;color:var(--warning);">${escapeHtml(t('scanner.warnings_desc'))}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; background: rgba(0,0,0,0.15);">
          ${res.warnings.map(w => `
            <div class="scanner-warning-row" style="display: flex; gap: 8px; align-items: center; font-size: 11px; padding: 4px 0;">
              <span class="scanner-warning-icon" style="color: var(--warning);">⚠</span>
              <span>${escapeHtml(w)}</span>
            </div>
          `).join('')}
        </div>
      </details>
    `;
  }

  const statCards = `
    <!-- Stats Row -->
    <div style="display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;flex-shrink:0;">
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_mods_scanned'))}</div>
        <div class="premium-stat-value">${res.totalScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_palschema_json'))}</div>
        <div class="premium-stat-value">${res.palschemaScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_ue4ss_lua'))}</div>
        <div class="premium-stat-value">${res.ue4ssScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_conflicts'))}</div>
        <div class="premium-stat-value ${hasConflicts ? 'danger' : 'success'}">${tableCount + hookCount}</div>
      </div>
    </div>
  `;

  const infoBannerHtml = `
    <!-- Info notice banner -->
    <div class="scanner-info-banner" style="margin-bottom: 20px; padding: 14px 18px; background: rgba(0, 188, 255, 0.06); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: var(--card-radius); display: flex; flex-direction: column; gap: 8px; font-size: 12px; line-height: 1.5;">
      <div style="font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 8px; font-size: 13px;">
        <span style="font-size: 14px;">ℹ</span>
        <span>${escapeHtml(t('scanner.info_title'))}</span>
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_ue4ss_title'))}:</strong> ${escapeHtml(t('scanner.info_ue4ss_desc'))}
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_palschema_title'))}:</strong> ${escapeHtml(t('scanner.info_palschema_desc'))}
      </div>
    </div>
  `;

  container.innerHTML = `
    ${subTabHeader()}
    <div class="scanner-scroll-panel" style="flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; padding: 20px 24px; overflow-y: auto; box-sizing: border-box;">
      ${statCards}
      ${infoBannerHtml}
      ${contentHtml}
      ${internalConflictsHtml}
      ${summariesHtml}
      ${warningsHtml}
    </div>
  `;

  const { setupEventListeners } = await import('./mod');
  setupEventListeners();
}
