import { scanModHotkeys, updateModHotkey } from '../../api';
import type { ModHotkey } from '../../api';
import { showToast } from '../toast';
import { lastHotkeysResult, setLastHotkeysResult, setIsScanningHotkeys, editingHotkeyKey, setEditingHotkeyKey, hotkeyFilter, isScanningHotkeys, renderScannerView, subTabHeader } from './mod';
import { escapeHtml, formatKeyboardBadge } from './rendering';

export async function runHotkeysScan(): Promise<void> {
  if (isScanningHotkeys) return;
  setIsScanningHotkeys(true);
  renderScannerView();

  const startTime = Date.now();

  try {
    const result = await scanModHotkeys();
    
    // Ensure the loading animation stays visible for at least 1 second to prevent flickering
    const elapsed = Date.now() - startTime;
    if (elapsed < 1000) {
      await new Promise(resolve => setTimeout(resolve, 1000 - elapsed));
    }

    setLastHotkeysResult(result);
    showToast(`Hotkey scan complete. Found ${result.length} bindings.`, 'success');
  } catch (err: any) {
    console.error(err);
    showToast(`Hotkey scan failed: ${err}`, 'error');
  } finally {
    setIsScanningHotkeys(false);
    renderScannerView();
  }
}

export async function renderHotkeysPanel(container: HTMLElement): Promise<void> {
  if (!lastHotkeysResult) {
    container.innerHTML = `
      ${subTabHeader()}
      <div style="flex:1; display:flex; align-items:center; justify-content:center; padding:24px;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon">⌨️</div>
          <div class="scanner-hero-title">Lua Hotkeys Manager</div>
          <div class="scanner-hero-desc">
            Scans all enabled UE4SS mods to look for asynchronous key bindings (\`Key.SOMETHING\`) configured inside Lua scripts. You can bind new shortcut combinations dynamically.
          </div>
          <button id="scanner-start-hotkeys-btn" class="scanner-btn-run">
            <span>Scan Hotkeys</span>
          </button>
        </div>
      </div>
    `;
    const { setupEventListeners } = await import('./mod');
    setupEventListeners();
    return;
  }

  if (lastHotkeysResult.length === 0) {
    container.innerHTML = `
      ${subTabHeader()}
      <div style="flex:1; display:flex; align-items:center; justify-content:center; padding:24px;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon">⌨️</div>
          <div class="scanner-hero-title">No hotkeys registered</div>
          <div class="scanner-hero-desc">
            No hotkeys were found in any of your enabled UE4SS mods.
          </div>
          <button id="scanner-start-hotkeys-btn" class="scanner-btn-run">
            <span>Rescan Hotkeys</span>
          </button>
        </div>
      </div>
    `;
    const { setupEventListeners } = await import('./mod');
    setupEventListeners();
    return;
  }

  const query = hotkeyFilter.toLowerCase().trim();
  const filtered = lastHotkeysResult.filter(hk => {
    return hk.modName.toLowerCase().includes(query) ||
           hk.currentKeys.toLowerCase().includes(query) ||
           hk.scriptName.toLowerCase().includes(query);
  });

  const listRows = filtered.map((hk, idx) => {
    const isEditing = `${hk.absoluteFilePath}::${hk.lineNumber}` === editingHotkeyKey;
    const actionButtons = isEditing ? `
      <button class="scanner-action-btn success hk-save-btn" data-idx="${idx}" style="padding:4px 8px; font-size:10px;">Save</button>
      <button class="scanner-action-btn hk-cancel-btn" style="padding:4px 8px; font-size:10px;">Cancel</button>
    ` : `
      <button class="hk-edit-btn scanner-action-btn" data-key="${escapeHtml(hk.absoluteFilePath)}::${hk.lineNumber}" style="padding:4px 8px; font-size:10px;">Change</button>
      <button class="hk-code-btn scanner-action-btn" data-mod-id="${escapeHtml(hk.modId)}" data-file-path="${escapeHtml(hk.scriptName)}" data-line="${hk.lineNumber}" style="padding:4px 8px; font-size:10px;">View Code</button>
    `;

    const keysDisplay = isEditing ? `
      <input type="text" id="hk-input-${idx}" class="hotkey-edit-input" value="${escapeHtml(hk.currentKeys)}" placeholder="Press keys..." style="padding:6px 12px; background:rgba(0,0,0,0.3); border:1px solid var(--accent); color:var(--text-primary); font-size:11px; font-family:monospace; border-radius:4px; outline:none; width:100%; box-sizing:border-box;" />
    ` : formatKeyboardBadge(hk.currentKeys);

    return `
      <tr>
        <td style="font-weight:700; color:var(--text-primary); font-size:12px;">${escapeHtml(hk.modName)}</td>
        <td style="font-family:monospace; font-size:11px; color:var(--text-muted);">${escapeHtml(hk.scriptName)}:L${hk.lineNumber}</td>
        <td>${keysDisplay}</td>
        <td style="width: 140px; text-align: right;">
          <div style="display:flex; gap:4px; justify-content:flex-end;">
            ${actionButtons}
          </div>
        </td>
      </tr>
    `;
  }).join('');

  const tableBody = listRows ? `
    <table class="premium-table">
      <thead>
        <tr>
          <th>Mod Name</th>
          <th>Location</th>
          <th>Keybinds Mapped</th>
          <th style="text-align:right;">Actions</th>
        </tr>
      </thead>
      <tbody>
        ${listRows}
      </tbody>
    </table>
  ` : `<div style="text-align:center; padding: 48px; color:var(--text-muted); font-size:12px;">No hotkeys found matching filter "${escapeHtml(hotkeyFilter)}".</div>`;

  container.innerHTML = `
    ${subTabHeader()}
    <!-- Top Filter Bar -->
    <div style="display:flex; align-items:center; justify-content:space-between; padding: 12px 24px; border-bottom: 1px solid var(--border); background: rgba(0,0,0,0.15); flex-shrink: 0;">
      <div class="search-wrapper">
        <span class="search-icon">🔍</span>
        <input type="text" id="hk-search-input" class="premium-search-input" placeholder="Search hotkeys..." value="${escapeHtml(hotkeyFilter)}" />
      </div>
      <div style="font-size:10px; color:var(--text-muted); font-weight:600; letter-spacing:0.5px;">Found ${filtered.length} active keybinds</div>
    </div>
    <div style="flex:1; padding: 20px 24px; overflow-y:auto; box-sizing:border-box;">
      <div class="scanner-card-section" style="cursor: default; padding: 0;">
        ${tableBody}
      </div>
    </div>
  `;

  const { setupEventListeners } = await import('./mod');
  setupEventListeners();
}
export { lastHotkeysResult };
export type { ModHotkey };
