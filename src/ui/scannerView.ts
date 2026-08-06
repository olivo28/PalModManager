import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';

export interface ConflictingMod {
  modId: string;
  modName: string;
  filePath: string;
  lineNumber: number;
  detail: string;
}

export interface TableRowConflict {
  tableName: string;
  rowName: string;
  mods: ConflictingMod[];
}

export interface HookConflict {
  hookTarget: string;
  hookFn: string;
  mods: ConflictingMod[];
}

export interface ModSummary {
  modId: string;
  modName: string;
  modType: string;
  palschemaRows: string[];
  ue4ssHooks: string[];
}

export interface ScanResult {
  totalScanned: number;
  palschemaScanned: number;
  ue4ssScanned: number;
  tableConflicts: TableRowConflict[];
  hookConflicts: HookConflict[];
  warnings: string[];
  modSummaries: ModSummary[];
}

let lastScanResult: ScanResult | null = null;
let isScanning = false;

export function renderScannerView(): void {
  const container = document.getElementById('scanner-view');
  if (!container) return;

  if (isScanning) {
    container.innerHTML = `
      <div style="padding: 24px; box-sizing: border-box; display: flex; align-items: center; justify-content: center; height: 100%; width: 100%;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon spinner" style="width: 48px; height: 48px; border-width: 4px;"></div>
          <div class="scanner-hero-title">Scanning Mods...</div>
          <div class="scanner-hero-desc">Reading JSON configurations and parsing Lua script hooks recursively. This may take a few seconds.</div>
        </div>
      </div>
    `;
    return;
  }

  if (!lastScanResult) {
    container.innerHTML = `
      <div style="padding: 24px; box-sizing: border-box; display: flex; align-items: center; justify-content: center; height: 100%; width: 100%;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon">🛡</div>
          <div class="scanner-hero-title">Conflict & Compatibility Scanner</div>
          <div class="scanner-hero-desc">
            Passive scanner that analyzes enabled mods to detect table collisions (PalSchema row edits) and hook collisions (multiple mods hooking the same engine function in Lua).
          </div>
          <button id="scanner-start-btn" class="scanner-btn-run">
            <span>Run Scanner</span>
          </button>
        </div>
      </div>
    `;

    document.getElementById('scanner-start-btn')?.addEventListener('click', runScan);
    return;
  }

  // Render Stats & Results
  const res = lastScanResult;
  const tableCount = res.tableConflicts.length;
  const hookCount = res.hookConflicts.length;
  const hasConflicts = tableCount > 0 || hookCount > 0;

  let contentHtml = '';

  if (!hasConflicts) {
    contentHtml = `
      <div class="scanner-clean-state">
        <div class="scanner-clean-icon">✅</div>
        <div class="scanner-clean-title">No Conflicts Detected</div>
        <div class="scanner-clean-desc">All enabled mods are fully compatible and modify distinct technologies, rows, and engine functions!</div>
      </div>
    `;
  } else {
    // Render conflicts list
    contentHtml = `
      <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
        
        <!-- LEFT COLUMN: UE4SS LUA HOOK CONFLICTS -->
        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
            <span>UE4SS Lua Hook Conflicts (${hookCount})</span>
            <span style="font-size: 10px; color: var(--text-muted);">Multiple RegisterHook targets</span>
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
                      ${m.detail ? `
                        <div style="font-size: 10px; color: var(--text-muted); background: rgba(0,0,0,0.2); padding: 4px 8px; border-radius: 4px; font-family: monospace; border: 1px solid var(--border); margin-left: 4px; margin-top: 2px;">
                          Line ${m.lineNumber}: ${escapeHtml(m.detail)}
                        </div>
                      ` : ''}
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('') : '<div style="color: var(--text-muted); font-style: italic; font-size: 12px; padding: 8px 0;">No hook conflicts detected.</div>'}
          </div>
        </details>

        <!-- RIGHT COLUMN: PALSCHEMA ROW CONFLICTS -->
        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
            <span>PalSchema Row Conflicts (${tableCount})</span>
            <span style="font-size: 10px; color: var(--text-muted);">Row level overlaps</span>
          </summary>
          <div class="scanner-card-body" style="cursor: default; gap: 14px;">
            ${tableCount > 0 ? res.tableConflicts.map(c => `
              <div class="scanner-conflict-item">
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
            `).join('') : '<div style="color: var(--text-muted); font-style: italic; font-size: 12px; padding: 8px 0;">No row conflicts detected.</div>'}
          </div>
        </details>

      </div>
    `;
  }

  // Render summaries list per mod
  let summariesHtml = '';
  if (res.modSummaries && res.modSummaries.length > 0) {
    summariesHtml = `
      <div class="scanner-card-section" style="margin-bottom: 20px;">
        <div class="scanner-card-header">
          <span>Active Mod Registries (Summary by Mod)</span>
          <span style="font-size:10px;color:var(--text-muted);">Details per mod</span>
        </div>
        <div class="scanner-card-body scanner-mod-summary-grid">
          ${res.modSummaries.map(m => {
            const hasRows = m.palschemaRows && m.palschemaRows.length > 0;
            const hasHooks = m.ue4ssHooks && m.ue4ssHooks.length > 0;
            
            // Skip empty summaries
            if (!hasRows && !hasHooks) {
              return '';
            }

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
                    ${hasRows ? `${m.palschemaRows.length} Row${m.palschemaRows.length > 1 ? 's' : ''}` : ''} 
                    ${hasRows && hasHooks ? ' | ' : ''} 
                    ${hasHooks ? `${m.ue4ssHooks.length} Hook${m.ue4ssHooks.length > 1 ? 's' : ''}` : ''}
                  </span>
                </summary>
                <div style="cursor: default; padding-top: 10px; display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px;">
                  
                  <!-- Rows Column -->
                  <div style="min-width: 0;">
                    <div style="font-size: 11px; font-weight: 700; color: var(--warning); text-transform: uppercase; margin-bottom: 6px;">PalSchema Row Edits</div>
                    ${hasRows ? `
                      <ul style="margin: 0; padding-left: 16px; font-size: 12px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 4px; overflow-wrap: anywhere; word-break: break-word; white-space: normal;">
                        ${m.palschemaRows.map(r => `<li>${escapeHtml(r)}</li>`).join('')}
                      </ul>
                    ` : `<div style="font-size: 11px; color: var(--text-muted); font-style: italic;">None</div>`}
                  </div>

                  <!-- Hooks Column -->
                  <div style="min-width: 0;">
                    <div style="font-size: 11px; font-weight: 700; color: var(--accent); text-transform: uppercase; margin-bottom: 6px;">UE4SS Lua Hooks</div>
                    ${hasHooks ? `
                      <ul style="margin: 0; padding-left: 16px; font-size: 12px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 4px; overflow-wrap: anywhere; word-break: break-word; white-space: normal;">
                        ${m.ue4ssHooks.map(h => `<li>${escapeHtml(h)}</li>`).join('')}
                      </ul>
                    ` : `<div style="font-size: 11px; color: var(--text-muted); font-style: italic;">None</div>`}
                  </div>

                </div>
              </details>
            `;
          }).join('')}
        </div>
      </div>
    `;
  }

  // Render warnings collapsible section if any exist
  let warningsHtml = '';
  if (res.warnings && res.warnings.length > 0) {
    warningsHtml = `
      <details class="scanner-card-section" style="margin-top: 24px; cursor: pointer;">
        <summary class="scanner-card-header" style="outline:none;">
          <span>Scanner Warnings (${res.warnings.length})</span>
          <span style="font-size:10px;color:var(--warning);">Non-fatal errors</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; background: rgba(0,0,0,0.15);">
          ${res.warnings.map(w => `
            <div class="scanner-warning-row">
              <span class="scanner-warning-icon">⚠</span>
              <span>${escapeHtml(w)}</span>
            </div>
          `).join('')}
        </div>
      </details>
    `;
  }

  container.innerHTML = `
    <!-- Top Fixed Dashboard Bar -->
    <div style="display:flex;align-items:center;justify-content:space-between;padding: 16px 24px; border-bottom: 1px solid var(--border); background: var(--bg-secondary); flex-shrink: 0;">
      <div style="font-size:16px;font-weight:700;color:var(--text-primary);">Scanner Dashboard</div>
      <button id="scanner-re-run-btn" class="scanner-btn-run" style="padding: 6px 14px; font-size:12px;">
        <span>Run Scan</span>
      </button>
    </div>

    <!-- Scrollable Content -->
    <div style="flex: 1; overflow-y: auto; padding: 24px; box-sizing: border-box;">
      
      <!-- Stats Grid -->
      <div class="scanner-stats-grid" style="margin-bottom: 20px;">
        <div class="scanner-stat-card">
          <div class="scanner-stat-label">Total Scanned</div>
          <div class="scanner-stat-value">${res.totalScanned}</div>
        </div>
        <div class="scanner-stat-card">
          <div class="scanner-stat-label">PalSchema Mods</div>
          <div class="scanner-stat-value">${res.palschemaScanned}</div>
        </div>
        <div class="scanner-stat-card">
          <div class="scanner-stat-label">UE4SS Mods</div>
          <div class="scanner-stat-value">${res.ue4ssScanned}</div>
        </div>
        <div class="scanner-stat-card">
          <div class="scanner-stat-label">Conflicts</div>
          <div class="scanner-stat-value ${hasConflicts ? 'danger' : 'success'}">${tableCount + hookCount}</div>
        </div>
      </div>

      <!-- Info notice banner -->
      <div class="scanner-info-banner" style="margin-bottom: 20px; padding: 12px 16px; background: rgba(0, 188, 255, 0.08); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: var(--card-radius); display: flex; flex-direction: column; gap: 6px; font-size: 12px; line-height: 1.45;">
        <div style="font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 6px;">
          <span>ℹ</span>
          <span>Understanding Mod Overlaps & Compatibility</span>
        </div>
        <div style="color: var(--text-secondary);">
          <strong>UE4SS Hook Overlaps:</strong> Multiple mods can hook the exact same engine function simultaneously (callbacks will execute sequentially in order of mod loading). This is generally safe and compatible unless a mod explicitly blocks execution, alters return values incompatibly, or cancels event propagation.
        </div>
        <div style="color: var(--text-secondary);">
          <strong>PalSchema Row Overlaps:</strong> If mods modify different keys/fields within the same row (see the JSON values below), they can function together. However, if they modify the exact same field key, only the mod loaded last (lowest in the list/load order) will take effect, overwriting the earlier ones.
        </div>
      </div>

      <!-- Main Results -->
      ${contentHtml}

      <!-- Mod Summaries -->
      ${summariesHtml}

      <!-- Warnings collapsible -->
      ${warningsHtml}

    </div>
  `;

  document.getElementById('scanner-re-run-btn')?.addEventListener('click', runScan);
}

export async function runScan(): Promise<void> {
  if (isScanning) return;
  isScanning = true;
  renderScannerView();

  try {
    const result = await invoke<ScanResult>('scan_conflicts');
    lastScanResult = result;
    showToast(`Scan completed. Scanned ${result.totalScanned} mods.`, 'success');
  } catch (err: any) {
    console.error(err);
    showToast(`Scan failed: ${err}`, 'error');
  } finally {
    isScanning = false;
    renderScannerView();
  }
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
