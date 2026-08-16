import { scanModHotkeys, updateModHotkey } from '../../api';
import type { ModHotkey } from '../../api';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';
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

  const query = hotkeyFilter.toLowerCase().trim();
  const filtered = lastHotkeysResult.filter(hk => {
    return hk.modName.toLowerCase().includes(query) ||
           hk.keys.toLowerCase().includes(query) ||
           hk.filePath.toLowerCase().includes(query);
  });

  const listRows = filtered.map((hk, idx) => {
    const isEditing = `${hk.absoluteFilePath}::${hk.lineNumber}` === editingHotkeyKey;
    const actionButtons = isEditing ? `
      <button class="btn-primary btn-sm hk-save-btn" data-idx="${idx}">${escapeHtml(t('common.save'))}</button>
      <button class="btn-secondary btn-sm hk-cancel-btn">${escapeHtml(t('common.cancel'))}</button>
    ` : `
      <button class="hk-edit-btn btn-secondary btn-sm" data-key="${escapeHtml(hk.absoluteFilePath)}::${hk.lineNumber}">${escapeHtml(t('common.edit'))}</button>
      <button class="hk-code-btn btn-secondary btn-sm" data-mod-id="${escapeHtml(hk.modId)}" data-file-path="${escapeHtml(hk.filePath)}" data-line="${hk.lineNumber}">${escapeHtml(t('common.preview'))}</button>
    `;

    const keysDisplay = isEditing ? `
      <input type="text" id="hk-input-${idx}" class="hotkey-edit-input" value="${escapeHtml(hk.keys)}" placeholder="${escapeHtml(t('scanner.hotkey_press_keys'))}" style="padding:6px 12px; background:rgba(0,0,0,0.3); border:1px solid var(--accent); color:var(--text-primary); font-size:11px; font-family:monospace; border-radius:4px; outline:none; width:100%; box-sizing:border-box;" />
    ` : formatKeyboardBadge(hk.keys);

    return `
      <tr>
        <td style="font-weight:700; color:var(--text-primary); font-size:12px;">${escapeHtml(hk.modName)}</td>
        <td style="font-family:monospace; font-size:11px; color:var(--text-muted);">${escapeHtml(hk.filePath)}:L${hk.lineNumber}</td>
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
      <div style="font-size:10px; color:var(--text-muted); font-weight:600; letter-spacing:0.5px;">${escapeHtml(t('common.selected_count', { count: filtered.length }))}</div>
    </div>
    <div class="scanner-scroll-panel" style="flex: 1 1 0; min-height: 0; padding: 20px 24px; overflow-y:auto; box-sizing:border-box;">
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
