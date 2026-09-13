// UE4SS Runtime Log Controller & Modal
import {
  getUe4ssLogDiagnostics,
  readRawUe4ssLog,
  clearUe4ssLog,
  copyUe4ssLogFileToClipboard,
  revealUe4ssLogInExplorer,
} from '../../api';
import type { Ue4ssLogDiagnostics, Ue4ssLogEntry } from '../../types';
import { escapeHtml } from '../../utils/helpers';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t } from '../../utils/i18n';

let _cachedDiagnostics: Ue4ssLogDiagnostics | null = null;
let _activeFilter: 'all' | 'errors' | 'warnings' | 'mods' | 'lua' = 'all';
let _searchQuery = '';
let _activeModFilter: string | null = null;
let _drawerCategoryFilter = 'all';
let _drawerSearchQuery = '';
let _isInitialized = false;
let _customLogPath: string | null = null;
let _liveMonitoringEnabled = true;
let _livePollTimer: any = null;
let _isPolling = false;

export async function showUe4ssLogModal(): Promise<void> {
  const modal = document.getElementById('ue4ss-log-modal');
  if (!modal) return;

  if (!_isInitialized) {
    initUe4ssLogEvents();
    _isInitialized = true;
  }

  modal.style.display = 'flex';
  await refreshUe4ssLogData();
  startLiveMonitoring();
}

export function hideUe4ssLogModal(): void {
  const modal = document.getElementById('ue4ss-log-modal');
  if (modal) modal.style.display = 'none';
  const drawer = document.getElementById('ue4ss-mods-drawer');
  if (drawer) drawer.style.display = 'none';
  document.getElementById('ue4ss-pill-mods')?.classList.remove('drawer-open');
  stopLiveMonitoring();
}

export async function refreshUe4ssLogData(): Promise<void> {
  const container = document.getElementById('ue4ss-log-container');
  if (container && !_cachedDiagnostics) {
    container.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);gap:8px;">
        <span class="spinner" style="width:16px;height:16px;border:2px solid var(--accent);border-top-color:transparent;border-radius:50%;animation:spin 0.8s linear infinite;"></span>
        <span>${escapeHtml(t('ue4ss_log.loading') || 'Reading UE4SS log...')}</span>
      </div>
    `;
  }

  try {
    const diag = await getUe4ssLogDiagnostics(10000, _customLogPath || undefined);
    _cachedDiagnostics = diag;
    renderLogHeaderAndStats(diag);
    renderLogEntries();
  } catch (err: any) {
    if (container) {
      container.innerHTML = `
        <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;color:#ff5f56;gap:8px;padding:24px;text-align:center;">
          <span style="font-size:28px;">⚠️</span>
          <span style="font-weight:700;">${escapeHtml(t('ue4ss_log.error_reading') || 'Failed to read UE4SS log')}</span>
          <span style="font-size:11px;color:var(--text-muted);">${escapeHtml(String(err))}</span>
        </div>
      `;
    }
  }
}

function startLiveMonitoring(): void {
  stopLiveMonitoring();
  if (!_liveMonitoringEnabled) return;

  _livePollTimer = setInterval(async () => {
    const modal = document.getElementById('ue4ss-log-modal');
    if (!modal || modal.style.display === 'none' || !_liveMonitoringEnabled || _isPolling) {
      return;
    }

    _isPolling = true;
    try {
      const diag = await getUe4ssLogDiagnostics(10000, _customLogPath || undefined);

      // Only re-render if log size or modification timestamp changed
      const hasChanged = !_cachedDiagnostics ||
        _cachedDiagnostics.file_size_bytes !== diag.file_size_bytes ||
        _cachedDiagnostics.total_lines !== diag.total_lines ||
        _cachedDiagnostics.last_modified !== diag.last_modified;

      if (hasChanged) {
        const container = document.getElementById('ue4ss-log-container');
        const wasNearBottom = container
          ? (container.scrollHeight - container.scrollTop - container.clientHeight < 80)
          : false;

        _cachedDiagnostics = diag;
        renderLogHeaderAndStats(diag);
        renderLogEntries();

        if (wasNearBottom && container) {
          container.scrollTop = container.scrollHeight;
        }
      }
    } catch {
      // Silently ignore transient locks while game writes to log
    } finally {
      _isPolling = false;
    }
  }, 1500);
}

function stopLiveMonitoring(): void {
  if (_livePollTimer) {
    clearInterval(_livePollTimer);
    _livePollTimer = null;
  }
  _isPolling = false;
}

function renderLogHeaderAndStats(diag: Ue4ssLogDiagnostics): void {
  const metaEl = document.getElementById('ue4ss-log-meta');
  if (metaEl) {
    const sizeKb = (diag.file_size_bytes / 1024).toFixed(1);
    const dateStr = diag.last_modified ? ` • ${diag.last_modified}` : '';
    metaEl.textContent = `${diag.file_path} (${sizeKb} KB${dateStr})`;
    metaEl.title = diag.file_path;
  }

  const resetBtn = document.getElementById('ue4ss-log-btn-reset-default');
  if (resetBtn) {
    resetBtn.style.display = _customLogPath ? 'inline-flex' : 'none';
  }

  const linesEl = document.getElementById('ue4ss-stat-lines');
  if (linesEl) linesEl.textContent = String(diag.total_lines);

  const modsEl = document.getElementById('ue4ss-stat-mods');
  if (modsEl) {
    const loadedCount = diag.loaded_mods.filter(m => m.status === 'loaded').length;
    modsEl.textContent = String(loadedCount);
    const summary = diag.loaded_mods.map(m => `${m.name} (${m.status})`).join('\n');
    modsEl.title = summary || 'No mods detected';
  }

  const warnsEl = document.getElementById('ue4ss-stat-warnings');
  if (warnsEl) warnsEl.textContent = String(diag.warning_count);

  const errsEl = document.getElementById('ue4ss-stat-errors');
  if (errsEl) errsEl.textContent = String(diag.error_count);

  const drawerCount = document.getElementById('ue4ss-drawer-count');
  if (drawerCount) {
    drawerCount.textContent = String(diag.loaded_mods.length);
  }

  const drawer = document.getElementById('ue4ss-mods-drawer');
  if (drawer && drawer.style.display !== 'none') {
    renderModsDrawer();
  }
}

function updateActiveModBanner(): void {
  const banner = document.getElementById('ue4ss-active-mod-banner');
  const nameEl = document.getElementById('ue4ss-active-mod-name');
  if (!banner || !nameEl) return;

  if (_activeModFilter) {
    nameEl.textContent = _activeModFilter;
    banner.style.display = 'flex';
  } else {
    banner.style.display = 'none';
  }
}

function renderLogEntries(): void {
  const container = document.getElementById('ue4ss-log-container');
  const countInfo = document.getElementById('ue4ss-log-count-info');
  if (!container || !_cachedDiagnostics) return;

  if (!_cachedDiagnostics.exists || _cachedDiagnostics.entries.length === 0) {
    container.innerHTML = `
      <div style="display:flex;flex-direction:column;align-items:center;justify-content:center;height:100%;color:var(--text-muted);gap:8px;padding:24px;text-align:center;">
        <span style="font-size:32px;opacity:0.4;">📜</span>
        <span style="font-weight:600;">${escapeHtml(t('ue4ss_log.empty_title') || 'No log entries found')}</span>
        <span style="font-size:11px;max-width:400px;opacity:0.8;">${escapeHtml(t('ue4ss_log.empty_desc') || 'The UE4SS log file is either empty or the game has not been run with UE4SS installed yet.')}</span>
      </div>
    `;
    if (countInfo) countInfo.textContent = 'Showing 0 entries';
    return;
  }

  const q = _searchQuery.toLowerCase().trim();
  const activeMod = _activeModFilter ? _activeModFilter.toLowerCase() : null;

  const filtered = _cachedDiagnostics.entries.filter(entry => {
    // Active Mod filter (from drawer card click)
    if (activeMod) {
      const matchEntryMod = entry.mod_name ? entry.mod_name.toLowerCase() === activeMod : false;
      const matchMsgMod = entry.message.toLowerCase().includes(activeMod);
      if (!matchEntryMod && !matchMsgMod) return false;
    }

    // Severity filter
    if (_activeFilter === 'errors' && entry.level !== 'error' && entry.level !== 'crash') return false;
    if (_activeFilter === 'warnings' && entry.level !== 'warning') return false;
    if (_activeFilter === 'mods' && entry.level !== 'mod') return false;
    if (_activeFilter === 'lua') {
      const isLua = (entry.tag && entry.tag.toLowerCase().includes('lua')) || !!entry.script_file || entry.message.toLowerCase().includes('lua');
      if (!isLua) return false;
    }

    // Search query filter
    if (q) {
      const matchMsg = entry.message.toLowerCase().includes(q);
      const matchTag = entry.tag ? entry.tag.toLowerCase().includes(q) : false;
      const matchMod = entry.mod_name ? entry.mod_name.toLowerCase().includes(q) : false;
      const matchFile = entry.script_file ? entry.script_file.toLowerCase().includes(q) : false;
      if (!matchMsg && !matchTag && !matchMod && !matchFile) return false;
    }

    return true;
  });

  if (countInfo) {
    if (_cachedDiagnostics.total_lines > _cachedDiagnostics.entries.length) {
      countInfo.textContent = `Showing ${filtered.length} of ${_cachedDiagnostics.entries.length} entries (${_cachedDiagnostics.total_lines} total lines, all errors & warnings preserved)`;
    } else {
      countInfo.textContent = `Showing ${filtered.length} of ${_cachedDiagnostics.entries.length} entries`;
    }
  }

  if (filtered.length === 0) {
    container.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-size:12px;">
        ${escapeHtml(t('ue4ss_log.no_matches') || 'No entries matching current filter and search')}
      </div>
    `;
    return;
  }

  container.innerHTML = filtered.map(entry => {
    let levelBadge = '';
    let rowBg = 'transparent';
    let textColor = '#d1d5db';

    switch (entry.level) {
      case 'crash':
        levelBadge = `<span style="background:rgba(255,95,86,0.25);color:#ff5f56;border:1px solid rgba(255,95,86,0.5);font-size:9px;font-weight:700;padding:1px 5px;border-radius:3px;">CRASH</span>`;
        rowBg = 'rgba(255,95,86,0.06)';
        textColor = '#ff7b72';
        break;
      case 'error':
        levelBadge = `<span style="background:rgba(255,95,86,0.15);color:#ff5f56;border:1px solid rgba(255,95,86,0.3);font-size:9px;font-weight:700;padding:1px 4px;border-radius:3px;">ERROR</span>`;
        rowBg = 'rgba(255,95,86,0.03)';
        textColor = '#ffa198';
        break;
      case 'warning':
        levelBadge = `<span style="background:rgba(255,170,0,0.15);color:#ffaa00;border:1px solid rgba(255,170,0,0.3);font-size:9px;font-weight:700;padding:1px 4px;border-radius:3px;">WARN</span>`;
        textColor = '#f0c674';
        break;
      case 'mod':
        levelBadge = `<span style="background:rgba(46,204,113,0.15);color:#2ecc71;border:1px solid rgba(46,204,113,0.3);font-size:9px;font-weight:700;padding:1px 4px;border-radius:3px;">MOD</span>`;
        textColor = '#7ee787';
        break;
      default:
        levelBadge = `<span style="background:rgba(255,255,255,0.05);color:var(--text-muted);font-size:9px;padding:1px 4px;border-radius:3px;">INFO</span>`;
        textColor = '#c9d1d9';
    }

    const timestampHtml = entry.timestamp
      ? `<span style="color:var(--text-muted);opacity:0.6;font-size:10.5px;margin-right:8px;flex-shrink:0;">${escapeHtml(entry.timestamp)}</span>`
      : '';

    const tagHtml = entry.tag
      ? `<span style="color:#79c0ff;margin-right:6px;font-weight:600;flex-shrink:0;">[${escapeHtml(entry.tag)}]</span>`
      : '';

    let jumpLinkHtml = '';
    if (entry.script_file && entry.script_line && entry.mod_name) {
      jumpLinkHtml = `
        <button class="ue4ss-jump-btn" data-mod="${escapeHtml(entry.mod_name)}" data-file="${escapeHtml(entry.script_file)}" data-line="${entry.script_line}" style="margin-left:8px;display:inline-flex;align-items:center;gap:3px;padding:1px 6px;font-size:10px;font-weight:600;background:rgba(0,188,255,0.12);color:#00bcff;border:1px solid rgba(0,188,255,0.3);border-radius:4px;cursor:pointer;white-space:nowrap;" title="Open in PalModManager Script Editor">
          <span>📝</span> <span>${escapeHtml(entry.script_file)}:${entry.script_line}</span>
        </button>
      `;
    }

    return `
      <div class="ue4ss-log-row" style="display:flex;align-items:flex-start;gap:6px;padding:3px 6px;border-radius:3px;background:${rowBg};">
        <span style="color:var(--text-muted);opacity:0.4;min-width:38px;text-align:right;user-select:none;font-size:10px;padding-top:2px;">${entry.line_number}</span>
        <div style="flex-shrink:0;padding-top:1px;">${levelBadge}</div>
        ${timestampHtml}
        ${tagHtml}
        <span style="color:${textColor};word-break:break-all;flex:1;">${escapeHtml(entry.message)}${jumpLinkHtml}</span>
      </div>
    `;
  }).join('');

  // Attach jump button listeners
  container.querySelectorAll('.ue4ss-jump-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const el = btn as HTMLElement;
      const modName = el.dataset.mod;
      const scriptFile = el.dataset.file;
      const line = parseInt(el.dataset.line || '1', 10);
      if (!modName || !scriptFile) return;

      try {
        const { openFileAtLine } = await import('../editor/mod');
        await openFileAtLine(modName, scriptFile, line);
        hideUe4ssLogModal();
        showToast(t('ue4ss_log.jumped_to_editor', { file: scriptFile, line }) || `Opened ${scriptFile} at line ${line}`, 'info');
      } catch (jumpErr) {
        showToast(String(jumpErr), 'error');
      }
    });
  });
}

function renderModsDrawer(): void {
  const listEl = document.getElementById('ue4ss-drawer-list');
  const countEl = document.getElementById('ue4ss-drawer-count');
  if (!listEl || !_cachedDiagnostics) return;

  const allMods = _cachedDiagnostics.loaded_mods;
  if (countEl) countEl.textContent = String(allMods.length);

  const q = _drawerSearchQuery.toLowerCase().trim();
  const filteredMods = allMods.filter(mod => {
    if (_drawerCategoryFilter === 'palschema' && mod.mod_type !== 'palschema') return false;
    if (_drawerCategoryFilter === 'lua' && mod.mod_type !== 'lua') return false;
    if (_drawerCategoryFilter === 'cpp' && mod.mod_type !== 'cpp') return false;
    if (_drawerCategoryFilter === 'disabled' && mod.status !== 'disabled') return false;
    if (q && !mod.name.toLowerCase().includes(q)) return false;
    return true;
  });

  if (filteredMods.length === 0) {
    listEl.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;height:120px;color:var(--text-muted);font-size:11.5px;">
        ${escapeHtml(t('ue4ss_log.no_mods_found') || 'No mods found')}
      </div>
    `;
    return;
  }

  listEl.innerHTML = filteredMods.map(mod => {
    const isActive = _activeModFilter && _activeModFilter.toLowerCase() === mod.name.toLowerCase();

    const mType = mod.mod_type || 'native';
    const typeLabel = t(`ue4ss_log.mod_type_${mType}`) || (mType === 'cpp' ? 'C++' : mType);
    const typeBadge = `<span class="ue4ss-badge-type ue4ss-badge-${escapeHtml(mType)}">${escapeHtml(typeLabel)}</span>`;

    const statusLabel = mod.status === 'loaded' ? (t('common.active') || 'Loaded') : mod.status === 'disabled' ? (t('common.disabled') || 'Disabled') : mod.status;
    const statusBadge = `<span class="ue4ss-badge-status ue4ss-badge-status-${escapeHtml(mod.status)}">${escapeHtml(statusLabel)}</span>`;

    return `
      <div class="ue4ss-mod-card ${isActive ? 'active' : ''}" data-mod-name="${escapeHtml(mod.name)}">
        <div class="ue4ss-mod-card-top">
          <span class="ue4ss-mod-card-name" title="${escapeHtml(mod.name)}">${escapeHtml(mod.name)}</span>
          ${statusBadge}
        </div>
        <div class="ue4ss-mod-card-bottom">
          ${typeBadge}
        </div>
      </div>
    `;
  }).join('');

  listEl.querySelectorAll('.ue4ss-mod-card').forEach(card => {
    card.addEventListener('click', () => {
      const modName = (card as HTMLElement).dataset.modName;
      if (!modName) return;

      if (_activeModFilter && _activeModFilter.toLowerCase() === modName.toLowerCase()) {
        _activeModFilter = null;
      } else {
        _activeModFilter = modName;
      }

      updateActiveModBanner();
      renderModsDrawer();
      renderLogEntries();
    });
  });
}

async function loadCustomLogFile(filePath: string): Promise<void> {
  _customLogPath = filePath;
  _cachedDiagnostics = null;
  _activeModFilter = null;
  updateActiveModBanner();
  await refreshUe4ssLogData();
  const fileName = filePath.split(/[/\\]/).pop() || filePath;
  showToast(t('ue4ss_log.external_log_loaded', { name: fileName }) || `Loaded external log: ${fileName}`, 'success');
}

function initUe4ssLogEvents(): void {
  const modal = document.getElementById('ue4ss-log-modal');
  const closeX = document.getElementById('ue4ss-log-modal-close-x');
  const closeBtn = document.getElementById('ue4ss-log-btn-close');
  const refreshBtn = document.getElementById('ue4ss-log-btn-refresh');
  const copyFileBtn = document.getElementById('ue4ss-log-btn-copy-file');
  const copyTextBtn = document.getElementById('ue4ss-log-btn-copy-text');
  const revealBtn = document.getElementById('ue4ss-log-btn-reveal');
  const clearBtn = document.getElementById('ue4ss-log-btn-clear');
  const openExtBtn = document.getElementById('ue4ss-log-btn-open-external');
  const fileInput = document.getElementById('ue4ss-log-file-input') as HTMLInputElement | null;
  const resetDefaultBtn = document.getElementById('ue4ss-log-btn-reset-default');
  const liveBadge = document.getElementById('ue4ss-live-badge');
  const liveLabel = document.getElementById('ue4ss-live-label');
  const dragOverlay = document.getElementById('ue4ss-drag-overlay');
  const searchInput = document.getElementById('ue4ss-log-search') as HTMLInputElement | null;

  closeX?.addEventListener('click', hideUe4ssLogModal);
  closeBtn?.addEventListener('click', hideUe4ssLogModal);

  // Close on backdrop click
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) hideUe4ssLogModal();
  });

  // Close on Escape key press (close drawer first if open)
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal && modal.style.display !== 'none') {
      const drawer = document.getElementById('ue4ss-mods-drawer');
      if (drawer && drawer.style.display !== 'none') {
        drawer.style.display = 'none';
        document.getElementById('ue4ss-pill-mods')?.classList.remove('drawer-open');
        e.stopPropagation();
        return;
      }
      hideUe4ssLogModal();
    }
  });

  // LIVE Badge toggle
  liveBadge?.addEventListener('click', () => {
    _liveMonitoringEnabled = !_liveMonitoringEnabled;
    if (_liveMonitoringEnabled) {
      liveBadge.classList.remove('paused');
      liveBadge.classList.add('active');
      if (liveLabel) liveLabel.textContent = t('ue4ss_log.live_active') || 'LIVE';
      startLiveMonitoring();
      showToast(t('ue4ss_log.live_resumed') || 'Live log monitoring active', 'info');
    } else {
      liveBadge.classList.remove('active');
      liveBadge.classList.add('paused');
      if (liveLabel) liveLabel.textContent = t('ue4ss_log.live_paused') || 'PAUSED';
      stopLiveMonitoring();
      showToast(t('ue4ss_log.live_paused_toast') || 'Live log monitoring paused', 'info');
    }
  });

  refreshBtn?.addEventListener('click', async () => {
    await refreshUe4ssLogData();
    showToast(t('common.refreshed'), 'success');
  });

  // Open External Log button
  openExtBtn?.addEventListener('click', async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: false,
        multiple: false,
        title: t('ue4ss_log.select_log_file') || 'Select UE4SS Log File',
        filters: [
          { name: 'Log Files (*.log, *.txt)', extensions: ['log', 'txt'] },
          { name: 'All Files (*.*)', extensions: ['*'] }
        ]
      });
      if (selected && typeof selected === 'string') {
        await loadCustomLogFile(selected);
      }
    } catch {
      fileInput?.click();
    }
  });

  fileInput?.addEventListener('change', async () => {
    const file = fileInput.files?.[0];
    if (file) {
      const path = (file as any).path;
      if (path) {
        await loadCustomLogFile(path);
      }
    }
  });

  // Reset to Game Log button
  resetDefaultBtn?.addEventListener('click', async () => {
    _customLogPath = null;
    _cachedDiagnostics = null;
    _activeModFilter = null;
    updateActiveModBanner();
    await refreshUe4ssLogData();
    showToast(t('ue4ss_log.reset_to_game_log') || 'Switched back to game UE4SS log', 'success');
  });

  // Drag and drop onto modal
  modal?.addEventListener('dragover', (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (dragOverlay) dragOverlay.style.display = 'flex';
  });

  modal?.addEventListener('dragleave', (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.target === modal || e.target === dragOverlay) {
      if (dragOverlay) dragOverlay.style.display = 'none';
    }
  });

  modal?.addEventListener('drop', async (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (dragOverlay) dragOverlay.style.display = 'none';

    const files = e.dataTransfer?.files;
    if (files && files.length > 0) {
      const file = files[0];
      const path = (file as any).path;
      if (path) {
        await loadCustomLogFile(path);
      }
    }
  });

  // Copy physical log file to clipboard (CF_HDROP / FileDrop for Discord, Explorer, etc.)
  copyFileBtn?.addEventListener('click', async () => {
    try {
      await copyUe4ssLogFileToClipboard(_customLogPath || undefined);
      showToast(t('ue4ss_log.file_copied') || 'Log file copied to clipboard! Ready to paste into Discord.', 'success');
    } catch (e) {
      showToast(String(e), 'error');
    }
  });

  // Copy raw text to clipboard
  copyTextBtn?.addEventListener('click', async () => {
    try {
      const rawText = await readRawUe4ssLog(5000, _customLogPath || undefined);
      await navigator.clipboard.writeText(rawText);
      showToast(t('ue4ss_log.text_copied') || 'Log text copied to clipboard', 'success');
    } catch (e) {
      showToast(String(e), 'error');
    }
  });

  // Reveal log in File Explorer
  revealBtn?.addEventListener('click', async () => {
    try {
      await revealUe4ssLogInExplorer(_customLogPath || undefined);
    } catch (e) {
      showToast(String(e), 'error');
    }
  });

  const metaWrap = document.getElementById('ue4ss-log-meta-wrap');
  metaWrap?.addEventListener('dblclick', async () => {
    try {
      await revealUe4ssLogInExplorer(_customLogPath || undefined);
    } catch (e) {
      showToast(String(e), 'error');
    }
  });

  clearBtn?.addEventListener('click', async () => {
    const confirmed = await showConfirm(t('ue4ss_log.confirm_clear') || 'Are you sure you want to truncate and clear the UE4SS log file?');
    if (!confirmed) return;

    try {
      await clearUe4ssLog(_customLogPath || undefined);
      await refreshUe4ssLogData();
      showToast(t('ue4ss_log.cleared') || 'UE4SS log cleared', 'info');
    } catch (e) {
      showToast(String(e), 'error');
    }
  });

  // Filter chips
  document.querySelectorAll('.ue4ss-filter-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      document.querySelectorAll('.ue4ss-filter-chip').forEach(c => c.classList.remove('active'));
      chip.classList.add('active');
      _activeFilter = ((chip as HTMLElement).dataset.filter as any) || 'all';
      renderLogEntries();
    });
  });

  // Search input with debounce
  let searchDebounce: any = null;
  searchInput?.addEventListener('input', () => {
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => {
      _searchQuery = searchInput.value;
      renderLogEntries();
    }, 150);
  });

  // Loaded Mods Drawer controls
  const drawer = document.getElementById('ue4ss-mods-drawer');
  const drawerCloseBtn = document.getElementById('ue4ss-mods-drawer-close');
  const pillMods = document.getElementById('ue4ss-pill-mods');
  const pillLines = document.getElementById('ue4ss-pill-lines');
  const pillWarns = document.getElementById('ue4ss-pill-warnings');
  const pillErrs = document.getElementById('ue4ss-pill-errors');
  const clearModFilterBtn = document.getElementById('ue4ss-clear-mod-filter-btn');
  const drawerSearchInput = document.getElementById('ue4ss-drawer-search') as HTMLInputElement | null;

  const toggleDrawer = () => {
    if (!drawer) return;
    const isVisible = drawer.style.display !== 'none';
    if (isVisible) {
      drawer.style.display = 'none';
      pillMods?.classList.remove('drawer-open');
    } else {
      drawer.style.display = 'flex';
      pillMods?.classList.add('drawer-open');
      renderModsDrawer();
      drawerSearchInput?.focus();
    }
  };

  pillMods?.addEventListener('click', toggleDrawer);
  drawerCloseBtn?.addEventListener('click', () => {
    if (drawer) drawer.style.display = 'none';
    pillMods?.classList.remove('drawer-open');
  });

  // Interactive Stat Pill shortcuts
  const selectSeverityFilter = (filterName: 'all' | 'warnings' | 'errors') => {
    document.querySelectorAll('.ue4ss-filter-chip').forEach(chip => {
      const el = chip as HTMLElement;
      if (el.dataset.filter === filterName) {
        el.click();
      }
    });
  };

  pillLines?.addEventListener('click', () => selectSeverityFilter('all'));
  pillWarns?.addEventListener('click', () => selectSeverityFilter('warnings'));
  pillErrs?.addEventListener('click', () => selectSeverityFilter('errors'));

  // Clear mod filter
  clearModFilterBtn?.addEventListener('click', () => {
    _activeModFilter = null;
    updateActiveModBanner();
    renderModsDrawer();
    renderLogEntries();
  });

  // Drawer category chips
  document.querySelectorAll('.ue4ss-drawer-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      document.querySelectorAll('.ue4ss-drawer-chip').forEach(c => c.classList.remove('active'));
      chip.classList.add('active');
      _drawerCategoryFilter = (chip as HTMLElement).dataset.modFilter || 'all';
      renderModsDrawer();
    });
  });

  // Drawer search input with debounce
  let drawerSearchDebounce: any = null;
  drawerSearchInput?.addEventListener('input', () => {
    clearTimeout(drawerSearchDebounce);
    drawerSearchDebounce = setTimeout(() => {
      _drawerSearchQuery = drawerSearchInput.value;
      renderModsDrawer();
    }, 120);
  });
}
