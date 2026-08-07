import { invoke } from '@tauri-apps/api/core';
import { showToast } from './toast';
import { scanModHotkeys, updateModHotkey } from '../api';
import type { ModHotkey } from '../api';

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
let lastHotkeysResult: ModHotkey[] | null = null;
let isScanning = false;
let isScanningHotkeys = false;
let activeSubTab: 'conflicts' | 'hotkeys' = 'conflicts';
let editingHotkeyKey: string | null = null; // format: "absoluteFilePath::lineNumber"
let hotkeyFilter = '';

export function renderScannerView(): void {
  const container = document.getElementById('scanner-view');
  if (!container) return;

  if (isScanning || isScanningHotkeys) {
    container.innerHTML = `
      <div style="padding: 24px; box-sizing: border-box; display: flex; align-items: center; justify-content: center; height: 100%; width: 100%;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon spinner" style="width: 48px; height: 48px; border-width: 4px;"></div>
          <div class="scanner-hero-title">${isScanning ? 'Scanning Mods...' : 'Scanning Lua Hotkeys...'}</div>
          <div class="scanner-hero-desc">Reading configurations and parsing files. This may take a few seconds.</div>
        </div>
      </div>
    `;
    return;
  }

  // Active styles for sub-tabs
  const activeStyle = "background:var(--accent);color:#000;border:none;font-size:11px;font-weight:700;padding:6px 14px;border-radius:17px;cursor:pointer;outline:none;";
  const inactiveStyle = "background:transparent;color:var(--text-muted);border:none;font-size:11px;font-weight:600;padding:6px 14px;border-radius:17px;cursor:pointer;outline:none;";

  const customStyles = `
    <style>
      .scanner-sub-tabs {
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 20px;
        padding: 3px;
        display: inline-flex;
        gap: 2px;
      }
      .scanner-sub-tab {
        border: none;
        background: transparent;
        color: var(--text-muted);
        font-size: 11px;
        font-weight: 600;
        padding: 6px 14px;
        border-radius: 17px;
        cursor: pointer;
        transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
        outline: none;
      }
      .scanner-sub-tab.active {
        background: var(--accent);
        color: #000 !important;
        font-weight: 700;
        box-shadow: 0 2px 8px rgba(0, 188, 255, 0.3);
      }
      .scanner-sub-tab:hover:not(.active) {
        color: var(--text-primary);
        background: rgba(255, 255, 255, 0.05);
      }

      .premium-stat-card {
        background: linear-gradient(135deg, rgba(255, 255, 255, 0.03) 0%, rgba(255, 255, 255, 0.01) 100%);
        border: 1px solid rgba(255, 255, 255, 0.06);
        border-radius: 12px;
        padding: 14px 20px;
        min-width: 130px;
        transition: all 0.3s ease;
        position: relative;
        overflow: hidden;
      }
      .premium-stat-card::before {
        content: '';
        position: absolute;
        top: 0; left: 0; right: 0; height: 2px;
        background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.1), transparent);
      }
      .premium-stat-card:hover {
        transform: translateY(-2px);
        border-color: rgba(255, 255, 255, 0.12);
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
      }
      .premium-stat-value {
        font-size: 26px;
        font-weight: 800;
        font-family: var(--font-mono, monospace);
        color: var(--text-primary);
        margin-top: 4px;
      }
      .premium-stat-value.danger {
        color: #ff5f56;
        text-shadow: 0 0 10px rgba(255, 95, 86, 0.2);
      }
      .premium-stat-value.success {
        color: #4af626;
        text-shadow: 0 0 10px rgba(74, 246, 38, 0.2);
      }

      .kbd-chip {
        display: inline-block;
        background: linear-gradient(180deg, #373a40 0%, #212327 100%);
        border: 1px solid #4f525c;
        border-bottom: 3px solid #151619;
        border-radius: 6px;
        padding: 4px 10px;
        font-family: var(--font-mono, monospace);
        font-size: 11px;
        font-weight: 700;
        color: #e2e8f0;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
        text-shadow: 0 1px 0 #000;
        transition: all 0.1s ease;
        letter-spacing: 0.5px;
        margin: 2px;
      }
      .kbd-chip-modifier {
        border-color: #00bcff;
        color: #00bcff;
      }

      .premium-table {
        width: 100%;
        border-collapse: collapse;
        text-align: left;
      }
      .premium-table th {
        background: rgba(0, 0, 0, 0.25);
        border-bottom: 1.5px solid rgba(255, 255, 255, 0.08);
        font-size: 10px;
        font-weight: 700;
        color: var(--text-muted);
        letter-spacing: 0.8px;
        text-transform: uppercase;
        padding: 12px 16px;
      }
      .premium-table tr {
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
        transition: all 0.25s ease;
      }
      .premium-table tbody tr:hover {
        background: rgba(255, 255, 255, 0.015) !important;
      }
      .premium-table td {
        padding: 14px 16px;
        vertical-align: middle;
      }

      .search-wrapper {
        position: relative;
        display: flex;
        align-items: center;
        width: 100%;
        max-width: 320px;
      }
      .search-icon {
        position: absolute;
        left: 12px;
        color: var(--text-muted);
        font-size: 13px;
        pointer-events: none;
      }
      .premium-search-input {
        width: 100%;
        padding: 8px 12px 8px 34px;
        font-size: 12px;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        color: var(--text-primary);
        border-radius: 20px;
        outline: none;
        transition: all 0.3s ease;
      }
      .premium-search-input:focus {
        background: rgba(255, 255, 255, 0.05);
        border-color: var(--accent);
        box-shadow: 0 0 10px rgba(0, 188, 255, 0.15);
      }
    </style>
  `;

  const subTabHeader = `
    ${customStyles}
    <!-- Top Fixed Dashboard Bar -->
    <div style="display:flex;align-items:center;justify-content:space-between;padding: 16px 24px; border-bottom: 1px solid var(--border); background: var(--bg-secondary); flex-shrink: 0;">
      <div style="display:flex;align-items:center;gap:20px;">
        <div style="font-size:16px;font-weight:700;color:var(--text-primary);">${activeSubTab === 'conflicts' ? 'Conflict Scanner' : 'Hotkeys Manager'}</div>
        <div class="scanner-sub-tabs">
          <button class="scanner-sub-tab ${activeSubTab === 'conflicts' ? 'active' : ''}" data-subtab="conflicts">Conflicts</button>
          <button class="scanner-sub-tab ${activeSubTab === 'hotkeys' ? 'active' : ''}" data-subtab="hotkeys">Hotkeys</button>
        </div>
      </div>
      <button id="scanner-re-run-btn" class="scanner-btn-run" style="padding: 6px 14px; font-size:12px;">
        <span>Run Scan</span>
      </button>
    </div>
  `;

  if (activeSubTab === 'conflicts') {
    if (!lastScanResult) {
      container.innerHTML = `
        ${subTabHeader}
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
                      ${hasRows ? `${m.palschemaRows.length} Row${m.palschemaRows.length > 1 ? 's' : ''}` : ''} 
                      ${hasRows && hasHooks ? ' | ' : ''} 
                      ${hasHooks ? `${m.ue4ssHooks.length} Hook${m.ue4ssHooks.length > 1 ? 's' : ''}` : ''}
                    </span>
                  </summary>
                  <div style="cursor: default; padding-top: 10px; display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px;">
                    <div style="min-width: 0;">
                      <div style="font-size: 11px; font-weight: 700; color: var(--warning); text-transform: uppercase; margin-bottom: 6px;">PalSchema Row Edits</div>
                      ${hasRows ? `
                        <ul style="margin: 0; padding-left: 16px; font-size: 12px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 4px; overflow-wrap: anywhere; word-break: break-word; white-space: normal;">
                          ${m.palschemaRows.map(r => `<li>${escapeHtml(r)}</li>`).join('')}
                        </ul>
                      ` : `<div style="font-size: 11px; color: var(--text-muted); font-style: italic;">None</div>`}
                    </div>
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
      ${subTabHeader}
      <div style="flex: 1; overflow-y: auto; padding: 24px; box-sizing: border-box;">
        <div class="scanner-stats-grid" style="margin-bottom: 20px;">
          <div class="premium-stat-card">
            <div class="scanner-stat-label">Total Scanned</div>
            <div class="premium-stat-value">${res.totalScanned}</div>
          </div>
          <div class="premium-stat-card">
            <div class="scanner-stat-label">PalSchema Mods</div>
            <div class="premium-stat-value">${res.palschemaScanned}</div>
          </div>
          <div class="premium-stat-card">
            <div class="scanner-stat-label">UE4SS Mods</div>
            <div class="premium-stat-value">${res.ue4ssScanned}</div>
          </div>
          <div class="premium-stat-card">
            <div class="scanner-stat-label">Conflicts</div>
            <div class="premium-stat-value ${hasConflicts ? 'danger' : 'success'}">${tableCount + hookCount}</div>
          </div>
        </div>

        <div class="scanner-info-banner" style="margin-bottom: 20px; padding: 12px 16px; background: rgba(0, 188, 255, 0.08); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: var(--card-radius); display: flex; flex-direction: column; gap: 6px; font-size: 12px; line-height: 1.45;">
          <div style="font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 6px;">
            <span>ℹ</span>
            <span>Understanding Mod Overlaps & Compatibility</span>
          </div>
          <div style="color: var(--text-secondary);">
            <strong>UE4SS Hook Overlaps:</strong> Multiple mods can hook the exact same engine function simultaneously (callbacks will execute sequentially in order of mod loading).
          </div>
          <div style="color: var(--text-secondary);">
            <strong>PalSchema Row Overlaps:</strong> Field modifications will combine, but duplicate row keys will load-order override.
          </div>
        </div>

        ${contentHtml}
        ${summariesHtml}
        ${warningsHtml}
      </div>
    `;
    setupEventListeners();
  } else {
    // HOTKEYS MANAGER SUB-TAB
    if (!lastHotkeysResult) {
      container.innerHTML = `
        ${subTabHeader}
        <div style="flex:1; display:flex; align-items:center; justify-content:center; padding:24px;">
          <div class="scanner-hero">
            <div class="scanner-hero-icon">⌨</div>
            <div class="scanner-hero-title">Lua Hotkeys Manager</div>
            <div class="scanner-hero-desc">
              Scan all enabled UE4SS and Hybrid mods to list, identify binding collisions, and inline edit their \`RegisterKeyBind\` hotkey triggers directly.
            </div>
            <button id="scanner-start-hotkeys-btn" class="scanner-btn-run">
              <span>Scan Hotkeys</span>
            </button>
          </div>
        </div>
      `;
      setupEventListeners();
      return;
    }

    // Identify collisions/conflicts
    const bindingsMap = new Map<string, number>();
    for (const hk of lastHotkeysResult) {
      const cleanKey = hk.keys.trim().toLowerCase();
      bindingsMap.set(cleanKey, (bindingsMap.get(cleanKey) || 0) + 1);
    }

    const filtered = lastHotkeysResult.filter(hk => {
      const search = hotkeyFilter.trim().toLowerCase();
      if (!search) return true;
      return hk.modName.toLowerCase().includes(search) ||
             hk.filePath.toLowerCase().includes(search) ||
             hk.keys.toLowerCase().includes(search);
    });

    const hotkeysCount = lastHotkeysResult.length;
    const conflictsCount = Array.from(bindingsMap.values()).filter(v => v > 1).length;

    const rowsHtml = filtered.map((hk, idx) => {
      const uniqueKey = `${hk.absoluteFilePath}::${hk.lineNumber}`;
      const isEditing = editingHotkeyKey === uniqueKey;
      const cleanKey = hk.keys.trim().toLowerCase();
      const hasConflict = (bindingsMap.get(cleanKey) || 0) > 1;

      // Check if it looks like a variable/dynamic lookup (e.g. no "Key." and no quotes)
      const isDynamicVariable = !hk.keys.toLowerCase().includes('key.') && !hk.keys.toLowerCase().includes('keys.') && !hk.keys.includes('"') && !hk.keys.includes('\'');

      const keysCell = isEditing
        ? `
          <div style="display:flex; flex-direction:column; gap:4px; width:100%;">
            <input type="text" class="hotkey-edit-input" id="hk-input-${idx}" placeholder="Press keys on your keyboard..." value="${escapeHtml(hk.keys)}" style="width:95%;padding:8px 12px;background:var(--bg-primary);color:var(--text-primary);border:1.5px solid var(--accent);border-radius:6px;font-family:var(--font-mono, monospace);font-size:12px;outline:none;" />
            <div style="font-size:10px;color:var(--text-muted);display:flex;align-items:center;gap:4px;">
              <span>⌨️</span> <span>Click input & press keys to record them automatically.</span>
            </div>
            ${isDynamicVariable ? `
              <div style="font-size:9.5px;color:var(--warning);background:rgba(255,165,0,0.08);border:1px solid rgba(255,165,0,0.2);padding:4px 8px;border-radius:4px;margin-top:2px;line-height:1.3;">
                ⚠️ <strong>Note:</strong> This mod uses a variable/config lookup (<code>${escapeHtml(hk.keys)}</code>). Saving will hardcode the key in the script.
              </div>
            ` : ''}
          </div>
        `
        : `${formatHotkeyToChips(hk.keys)}${hasConflict ? `<span class="mod-card-update-badge" style="background:#ff5f56;color:#fff;border:none;margin-left:10px;font-size:8px;padding:2px 6px;vertical-align:middle;display:inline-block;" title="Another mod uses this exact keybind!">⚠️ CONFLICT</span>` : ''}`;

      const actions = isEditing
        ? `<button class="btn-tiny hk-save-btn" data-idx="${idx}" style="background:var(--success);color:#000;font-weight:700;">Save</button>
           <button class="btn-tiny hk-cancel-btn" style="margin-left:4px;">Cancel</button>`
        : `<button class="btn-tiny hk-edit-btn" data-key="${uniqueKey}">Edit</button>`;

      return `
        <tr style="${hasConflict ? 'background:rgba(255,95,86,0.025)' : ''}">
          <td style="padding:14px 16px;font-weight:700;font-size:12px;color:var(--text-primary);">${escapeHtml(hk.modName)}</td>
          <td style="padding:14px 16px;font-size:11px;font-family:var(--font-mono, monospace);color:var(--text-muted);">${escapeHtml(hk.filePath)}:L${hk.lineNumber}</td>
          <td style="padding:14px 16px;vertical-align:middle;">${keysCell}</td>
          <td style="padding:14px 16px;font-family:var(--font-mono, monospace);font-size:11px;color:var(--text-secondary);max-width:260px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title="${escapeHtml(hk.rawLine)}">${escapeHtml(hk.rawLine)}</td>
          <td style="padding:14px 16px;text-align:right;white-space:nowrap;">${actions}</td>
        </tr>
      `;
    }).join('');

    container.innerHTML = `
      ${subTabHeader}
      <div style="flex: 1; overflow-y: auto; padding: 24px; box-sizing: border-box; display:flex; flex-direction:column; gap:16px;">
        
        <!-- Stats and Filter Bar -->
        <div style="display:flex;align-items:center;justify-content:space-between;gap:16px;flex-wrap:wrap;">
          <div style="display:flex;gap:12px;">
            <div class="premium-stat-card">
              <div class="scanner-stat-label">Total Keybinds</div>
              <div class="premium-stat-value">${hotkeysCount}</div>
            </div>
            <div class="premium-stat-card">
              <div class="scanner-stat-label">Colliding Keys</div>
              <div class="premium-stat-value ${conflictsCount > 0 ? 'danger' : 'success'}">${conflictsCount}</div>
            </div>
          </div>
          <div class="search-wrapper">
            <span class="search-icon">🔍</span>
            <input type="text" id="hk-search-input" class="premium-search-input" placeholder="Search keys or mod names..." value="${escapeHtml(hotkeyFilter)}" />
          </div>
        </div>

        <!-- Hotkeys Table -->
        <div style="border:1px solid var(--border);background:var(--bg-secondary);border-radius:8px;box-shadow: 0 4px 16px rgba(0,0,0,0.35);overflow:hidden;">
          <table class="premium-table">
            <thead>
              <tr>
                <th style="width:15%;">Mod</th>
                <th style="width:25%;">File Path</th>
                <th style="width:30%;">Trigger Keybind</th>
                <th style="width:20%;">Raw Lua Line</th>
                <th style="width:10%;text-align:right;">Actions</th>
              </tr>
            </thead>
            <tbody>
              ${filtered.length > 0 ? rowsHtml : `<tr><td colspan="5" style="padding:24px;text-align:center;color:var(--text-muted);font-style:italic;font-size:12px;">No hotkeys found matching filter criteria.</td></tr>`}
            </tbody>
          </table>
        </div>
      </div>
    `;
    setupEventListeners();
  }
}

function formatHotkeyToChips(keysStr: string): string {
  const keyRegex = /(?:Key|Keys|ModifierKey)\.([a-zA-Z0-9_]+)/g;
  const matches: { name: string; isModifier: boolean }[] = [];
  let match;
  
  while ((match = keyRegex.exec(keysStr)) !== null) {
    const name = match[1];
    const isModifier = match[0].includes('ModifierKey');
    if (!matches.some(m => m.name === name)) {
      matches.push({ name, isModifier });
    }
  }
  
  if (matches.length > 0) {
    matches.sort((a, b) => (a.isModifier === b.isModifier) ? 0 : a.isModifier ? -1 : 1);
    return matches.map(m => {
      return `<kbd class="kbd-chip ${m.isModifier ? 'kbd-chip-modifier' : ''}">${escapeHtml(m.name)}</kbd>`;
    }).join(' <span style="color:var(--text-muted);font-weight:bold;margin:0 2px;">+</span> ');
  }
  
  const cleaned = keysStr.replace(/[{}"']/g, '').trim();
  return `<code style="background:rgba(255, 255, 255, 0.05);padding:4px 8px;border-radius:6px;color:#e0af68;font-size:11px;font-family:var(--font-mono, monospace);border:1px solid rgba(255,255,255,0.05);">${escapeHtml(cleaned)}</code>`;
}

function setupEventListeners(): void {
  document.getElementById('scanner-start-btn')?.addEventListener('click', runScan);
  document.getElementById('scanner-start-hotkeys-btn')?.addEventListener('click', runHotkeysScan);
  
  document.getElementById('scanner-re-run-btn')?.addEventListener('click', () => {
    if (activeSubTab === 'conflicts') {
      runScan();
    } else {
      runHotkeysScan();
    }
  });

  document.querySelectorAll('.scanner-sub-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      const sub = (btn as HTMLElement).dataset.subtab as 'conflicts' | 'hotkeys';
      if (activeSubTab !== sub) {
        activeSubTab = sub;
        editingHotkeyKey = null;
        renderScannerView();
      }
    });
  });

  const search = document.getElementById('hk-search-input') as HTMLInputElement | null;
  if (search) {
    search.addEventListener('input', () => {
      hotkeyFilter = search.value;
      renderScannerView();
      const searchRef = document.getElementById('hk-search-input') as HTMLInputElement | null;
      if (searchRef) {
        searchRef.focus();
        searchRef.setSelectionRange(searchRef.value.length, searchRef.value.length);
      }
    });
  }

  document.querySelectorAll('.hk-edit-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      editingHotkeyKey = (btn as HTMLElement).dataset.key || null;
      renderScannerView();
      
      // Auto-focus the edit input and attach the key recorder
      const input = document.querySelector('.hotkey-edit-input') as HTMLInputElement | null;
      if (input) {
        input.focus();
        input.addEventListener('keydown', (e) => {
          e.preventDefault();
          e.stopPropagation();
          
          const modifiers: string[] = [];
          if (e.ctrlKey) modifiers.push('ModifierKey.CONTROL');
          if (e.shiftKey) modifiers.push('ModifierKey.SHIFT');
          if (e.altKey) modifiers.push('ModifierKey.ALT');
          
          let mainKey = '';
          const code = e.code;
          
          if (code.startsWith('Key')) {
            mainKey = `Key.${code.substring(3).toUpperCase()}`;
          } else if (code.startsWith('Digit')) {
            const digitMap: Record<string, string> = {
              'Digit0': 'ZERO', 'Digit1': 'ONE', 'Digit2': 'TWO', 'Digit3': 'THREE',
              'Digit4': 'FOUR', 'Digit5': 'FIVE', 'Digit6': 'SIX', 'Digit7': 'SEVEN',
              'Digit8': 'EIGHT', 'Digit9': 'NINE'
            };
            mainKey = `Key.${digitMap[code] || code.substring(5)}`;
          } else if (code.startsWith('F') && code.length >= 2) {
            mainKey = `Key.${code}`;
          } else if (code === 'Space') {
            mainKey = 'Key.SPACE';
          } else if (code === 'Escape') {
            mainKey = 'Key.ESCAPE';
          } else if (code === 'Enter') {
            mainKey = 'Key.ENTER';
          } else if (code === 'Tab') {
            mainKey = 'Key.TAB';
          } else if (code.startsWith('Numpad')) {
            mainKey = `Key.NUM_${code.substring(6).toUpperCase()}`;
          } else if (code === 'ArrowUp') {
            mainKey = 'Key.UP_ARROW';
          } else if (code === 'ArrowDown') {
            mainKey = 'Key.DOWN_ARROW';
          } else if (code === 'ArrowLeft') {
            mainKey = 'Key.LEFT_ARROW';
          } else if (code === 'ArrowRight') {
            mainKey = 'Key.RIGHT_ARROW';
          }
          
          if (e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt') {
            if (modifiers.length > 0) {
              input.value = modifiers.map(m => `{${m}}`).join(', ');
            }
            return;
          }
          
          if (mainKey) {
            let result = mainKey;
            if (modifiers.length > 0) {
              result = `${mainKey}, {${modifiers.join(', ')}}`;
            }
            input.value = result;
          }
        });
      }
    });
  });

  document.querySelectorAll('.hk-cancel-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      editingHotkeyKey = null;
      renderScannerView();
    });
  });

  document.querySelectorAll('.hk-save-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const idx = parseInt((btn as HTMLElement).dataset.idx || '0');
      const input = document.getElementById(`hk-input-${idx}`) as HTMLInputElement | null;
      if (!input || !lastHotkeysResult) return;

      const hk = lastHotkeysResult.find(item => `${item.absoluteFilePath}::${item.lineNumber}` === editingHotkeyKey);
      if (!hk) return;

      const newKeys = input.value.trim();
      if (!newKeys) {
        showToast('Keybind cannot be empty', 'error');
        return;
      }

      (btn as HTMLButtonElement).disabled = true;
      try {
        await updateModHotkey(hk.absoluteFilePath, hk.lineNumber, newKeys);
        showToast('Hotkey trigger modified successfully', 'success');
        editingHotkeyKey = null;
        await runHotkeysScan();
      } catch (err: any) {
        showToast(`Failed to update hotkey: ${err}`, 'error');
        (btn as HTMLButtonElement).disabled = false;
      }
    });
  });
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

export async function runHotkeysScan(): Promise<void> {
  if (isScanningHotkeys) return;
  isScanningHotkeys = true;
  renderScannerView();

  try {
    const result = await scanModHotkeys();
    lastHotkeysResult = result;
    showToast(`Hotkey scan complete. Found ${result.length} bindings.`, 'success');
  } catch (err: any) {
    console.error(err);
    showToast(`Hotkey scan failed: ${err}`, 'error');
  } finally {
    isScanningHotkeys = false;
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
