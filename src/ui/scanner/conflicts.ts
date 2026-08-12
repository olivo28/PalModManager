import { invoke } from '@tauri-apps/api/core';
import { showToast } from '../toast';
import { lastScanResult, setLastScanResult, setIsScanning, activeSubTab, isScanning, renderScannerView, subTabHeader } from './mod';
import { escapeHtml } from './rendering';

import { ConflictingMod, TableRowConflict, HookConflict, ModSummary, ScanResult } from './mod';

export async function runScan(): Promise<void> {
  if (isScanning) return;
  setIsScanning(true);
  renderScannerView();

  try {
    const result = await invoke<ScanResult>('scan_conflicts');
    setLastScanResult(result);
    showToast(`Scan completed. Scanned ${result.totalScanned} mods.`, 'success');
  } catch (err: any) {
    console.error(err);
    showToast(`Scan failed: ${err}`, 'error');
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
      <div class="scanner-clean-state">
        <div class="scanner-clean-icon">✅</div>
        <div class="scanner-clean-title">No Conflicts Detected</div>
        <div class="scanner-clean-desc">All enabled mods are fully compatible and modify distinct technologies, rows, and engine functions!</div>
      </div>
    `;
  } else {
    contentHtml = `
      <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
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
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('') : '<div style="color:var(--text-muted); font-size:11px;">No hook conflicts.</div>'}
          </div>
        </details>

        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
            <span>PalSchema Row Collision Conflicts (${tableCount})</span>
            <span style="font-size: 10px; color: var(--text-muted);">Multiple mods modifying same rows</span>
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
            `).join('') : '<div style="color:var(--text-muted); font-size:11px;">No row collisions.</div>'}
          </div>
        </details>
      </div>
    `;
  }

  const statCards = `
    <!-- Stats Row -->
    <div style="display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;flex-shrink:0;">
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">Mods Scanned</div>
        <div class="premium-stat-value">${res.totalScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">PalSchema JSON</div>
        <div class="premium-stat-value">${res.palschemaScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">UE4SS Lua</div>
        <div class="premium-stat-value">${res.ue4ssScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">Conflicts</div>
        <div class="premium-stat-value ${hasConflicts ? 'danger' : 'success'}">${tableCount + hookCount}</div>
      </div>
    </div>
  `;

  container.innerHTML = `
    ${subTabHeader()}
    <div style="flex:1; display:flex; flex-direction:column; padding: 20px 24px; overflow-y:auto; box-sizing:border-box;">
      ${statCards}
      ${contentHtml}
    </div>
  `;

  const { setupEventListeners } = await import('./mod');
  setupEventListeners();
}
