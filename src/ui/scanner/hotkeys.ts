import { scanModHotkeys, updateModHotkey } from '../../api';
import type { ModHotkey } from '../../api';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
import { lastHotkeysResult, setLastHotkeysResult, setIsScanningHotkeys, editingHotkeyKey, setEditingHotkeyKey, hotkeyFilter, isScanningHotkeys, renderScannerView, subTabHeader } from './mod';
import { escapeHtml, formatKeyboardBadge } from './rendering';

function normalizeKeyString(keys: string): string {
  return (keys || '')
    .toLowerCase()
    .replace(/[{}]/g, '')
    .replace(/\s+/g, '')
    .replace(/key\./g, '')
    .replace(/modifierkey\./g, '');
}

const functionKeyCandidates = ['Key.F1', 'Key.F2', 'Key.F3', 'Key.F4', 'Key.F5', 'Key.F6', 'Key.F7', 'Key.F8', 'Key.F9', 'Key.F10', 'Key.F11', 'Key.F12', 'Key.INSERT', 'Key.DELETE', 'Key.HOME', 'Key.END'];

function getUnusedKey(usedKeys: string[]): string {
  const normalizedUsed = new Set(usedKeys.map(k => normalizeKeyString(k)));
  for (const candidate of functionKeyCandidates) {
    if (!normalizedUsed.has(normalizeKeyString(candidate))) {
      return candidate;
    }
  }
  return 'Key.F10';
}

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
    showToast(t('scanner.toast_hotkeys_success', { count: result.length }), 'success');
  } catch (err: any) {
    console.error(err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
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
          <div class="scanner-hero-title">${escapeHtml(t('scanner.hero_hotkeys_initial_title'))}</div>
          <div class="scanner-hero-desc">
            ${escapeHtml(t('scanner.hero_hotkeys_initial_desc'))}
          </div>
          <button id="scanner-start-hotkeys-btn" class="scanner-btn-run">
            <span>${escapeHtml(t('scanner.btn_scan_hotkeys'))}</span>
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
          <div class="scanner-hero-title">${escapeHtml(t('scanner.hero_no_hotkeys_title'))}</div>
          <div class="scanner-hero-desc">
            ${escapeHtml(t('scanner.hero_no_hotkeys_desc'))}
          </div>
          <button id="scanner-start-hotkeys-btn" class="scanner-btn-run">
            <span>${escapeHtml(t('scanner.btn_rescan_hotkeys'))}</span>
          </button>
        </div>
      </div>
    `;
    const { setupEventListeners } = await import('./mod');
    setupEventListeners();
    return;
  }

  // Count key occurrences to detect conflicts
  const keyFrequency: Record<string, number> = {};
  const allNormalized = lastHotkeysResult.map(hk => normalizeKeyString(hk.keys));
  for (const n of allNormalized) {
    if (n) keyFrequency[n] = (keyFrequency[n] || 0) + 1;
  }

  const conflictingNormalized = new Set(
    Object.keys(keyFrequency).filter(k => keyFrequency[k] > 1)
  );

  const totalConflicts = lastHotkeysResult.filter(hk => conflictingNormalized.has(normalizeKeyString(hk.keys))).length;
  const suggestedKey = getUnusedKey(lastHotkeysResult.map(hk => hk.keys));

  const query = hotkeyFilter.toLowerCase().trim();
  const filtered = lastHotkeysResult.filter(hk => {
    return hk.modName.toLowerCase().includes(query) ||
           hk.keys.toLowerCase().includes(query) ||
           hk.filePath.toLowerCase().includes(query) ||
           (hk.variableName && hk.variableName.toLowerCase().includes(query));
  });

  const listRows = filtered.map((hk, idx) => {
    const isEditing = `${hk.absoluteFilePath}::${hk.lineNumber}` === editingHotkeyKey;
    const isConflicting = conflictingNormalized.has(normalizeKeyString(hk.keys));

    const actionButtons = isEditing ? `
      <button class="btn-primary btn-sm hk-save-btn" data-idx="${idx}">${escapeHtml(t('common.save'))}</button>
      <button class="btn-secondary btn-sm hk-cancel-btn">${escapeHtml(t('common.cancel'))}</button>
    ` : `
      <button class="hk-edit-btn btn-secondary btn-sm" data-key="${escapeHtml(hk.absoluteFilePath)}::${hk.lineNumber}">${escapeHtml(t('common.edit'))}</button>
      <button class="hk-code-btn btn-secondary btn-sm" data-mod-id="${escapeHtml(hk.modId)}" data-file-path="${escapeHtml(hk.definitionFilePath || hk.filePath)}" data-line="${hk.definitionLineNumber || hk.lineNumber}">${escapeHtml(t('common.preview'))}</button>
    `;

    const keysDisplay = isEditing ? `
      <div style="display:flex; flex-direction:column; gap:4px; width:100%;">
        <input type="text" id="hk-input-${idx}" class="hotkey-edit-input" value="${escapeHtml(hk.keys)}" placeholder="${escapeHtml(t('scanner.hotkey_press_keys'))}" style="padding:6px 12px; background:rgba(0,0,0,0.3); border:1px solid var(--accent); color:var(--text-primary); font-size:11px; font-family:monospace; border-radius:4px; outline:none; width:100%; box-sizing:border-box;" />
        <div style="font-size:9.5px; color:var(--text-muted);">${escapeHtml(t('scanner.hotkey_press_hint') || 'Press any key on keyboard to capture, or type manually.')}</div>
      </div>
    ` : `
      <div style="display:flex; align-items:center; gap:8px; flex-wrap:wrap;">
        ${formatKeyboardBadge(hk.keys)}
        ${isConflicting ? `
          <span style="font-size:10px; font-weight:700; color:var(--danger); background:rgba(255,75,75,0.15); border:1px solid rgba(255,75,75,0.3); border-radius:4px; padding:2px 6px;">
            ⚠️ ${escapeHtml(t('scanner.hotkey_conflict_tag') || 'Conflict')}
          </span>
          <button class="hk-quick-rebind-btn" data-idx="${idx}" data-suggest="${escapeHtml(suggestedKey)}" style="font-size:9.5px; font-weight:600; background:none; border:1px dashed var(--accent); color:var(--accent); border-radius:4px; padding:2px 6px; cursor:pointer;" title="${escapeHtml(t('scanner.btn_quick_rebind_tooltip') || 'Quickly reassign to free function key')}">
            ⚡ ${escapeHtml(t('scanner.btn_quick_rebind', { key: suggestedKey }) || `Rebind to ${suggestedKey}`)}
          </button>
        ` : ''}
      </div>
    `;

    const locationDisplay = hk.isVariable ? `
      <div>
        <div style="font-family:monospace; font-size:11px; color:var(--text-primary);">${escapeHtml(hk.definitionFilePath || hk.filePath)}:L${hk.definitionLineNumber || hk.lineNumber}</div>
        <div style="font-size:9.5px; color:var(--accent); display:flex; align-items:center; gap:4px; margin-top:2px;">
          <span>⚙</span>
          <span><code>${escapeHtml(hk.variableName || 'Variable')}</code></span>
          <span style="opacity:0.6; font-size:9px;">(${escapeHtml(hk.filePath)}:L${hk.lineNumber})</span>
        </div>
      </div>
    ` : `
      <div style="font-family:monospace; font-size:11px; color:var(--text-muted);">${escapeHtml(hk.filePath)}:L${hk.lineNumber}</div>
    `;

    return `
      <tr style="${isConflicting ? 'background: rgba(255, 75, 75, 0.04);' : ''}">
        <td style="font-weight:700; color:var(--text-primary); font-size:12px;">
          ${escapeHtml(hk.modName)}
        </td>
        <td>${locationDisplay}</td>
        <td>${keysDisplay}</td>
        <td style="width: 140px; text-align: right;">
          <div style="display:flex; gap:4px; justify-content:flex-end;">
            ${actionButtons}
          </div>
        </td>
      </tr>
    `;
  }).join('');

  const conflictBanner = totalConflicts > 0 ? `
    <div style="margin-bottom:16px; padding:10px 16px; background:rgba(255,75,75,0.08); border:1px solid rgba(255,75,75,0.3); border-radius:var(--card-radius); display:flex; align-items:center; justify-content:space-between; gap:12px;">
      <div style="display:flex; align-items:center; gap:8px; font-size:12px; color:var(--text-primary);">
        <span style="font-size:16px;">⚠️</span>
        <span><b>${totalConflicts}</b> ${escapeHtml(t('scanner.hotkey_conflicts_detected_banner') || 'hotkey conflicts detected between active mods.')}</span>
      </div>
      <div style="font-size:11px; color:var(--text-muted);">
        ${escapeHtml(t('scanner.hotkey_conflicts_hint') || 'Use Edit or Quick Rebind to resolve overlapping keys.')}
      </div>
    </div>
  ` : '';

  const tableBody = listRows ? `
    <table class="premium-table">
      <thead>
        <tr>
          <th>${escapeHtml(t('scanner.col_mod'))}</th>
          <th>${escapeHtml(t('scanner.col_location'))}</th>
          <th>${escapeHtml(t('scanner.col_binding'))}</th>
          <th style="text-align:right;">${escapeHtml(t('scanner.col_action'))}</th>
        </tr>
      </thead>
      <tbody>
        ${listRows}
      </tbody>
    </table>
  ` : `<div style="text-align:center; padding: 48px; color:var(--text-muted); font-size:12px;">${escapeHtml(t('scanner.hero_no_hotkeys_title'))}</div>`;

  container.innerHTML = `
    ${subTabHeader()}
    <!-- Top Filter Bar -->
    <div style="display:flex; align-items:center; justify-content:space-between; padding: 12px 24px; border-bottom: 1px solid var(--border); background: rgba(0,0,0,0.15); flex-shrink: 0;">
      <div class="search-wrapper">
        <span class="search-icon">🔍</span>
        <input type="text" id="hk-search-input" class="premium-search-input" placeholder="${escapeHtml(t('scanner.hotkeys_search_placeholder'))}" value="${escapeHtml(hotkeyFilter)}" />
      </div>
      <div style="display:flex; align-items:center; gap:12px;">
        <button id="scanner-rescan-hotkeys-btn" class="btn btn-secondary btn-sm" style="display:flex; align-items:center; gap:6px; font-size:11px; padding:4px 10px;">
          <span>🔄</span> <span>${escapeHtml(t('scanner.btn_rescan_hotkeys'))}</span>
        </button>
        <div style="font-size:10px; color:var(--text-muted); font-weight:600; letter-spacing:0.5px;">${escapeHtml(t('common.selected_count', { count: filtered.length }))}</div>
      </div>
    </div>
    <div class="scanner-scroll-panel" style="flex: 1 1 0; min-height: 0; padding: 20px 24px; overflow-y:auto; box-sizing:border-box;">
      ${conflictBanner}
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

