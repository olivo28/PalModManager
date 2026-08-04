import { getSettings, setGamePath, setNexusApiKey, setHideNativeMods, setDebugConsole, analyzeZip, installMod, checkModExistsCommand, updateModCommand, setModVersion as setModVersionApi, fetchNexusInfoAsync, checkDependencies, installUe4ss, installPalschema, setCustomDataPath } from '../api';
import type { ZipAnalysis } from '../api';
import { getState, updateState } from '../state';
import { renderModsView, loadMods } from './modsView';
import { showToast } from './toast';
import { escapeHtml } from '../utils/helpers';

// === SETTINGS MODAL ===

let _tempCustomDataPath: string | null = null;

export function openSettingsModal(): void {
  const modal = document.getElementById('settings-modal')!;
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const keyInput = document.getElementById('settings-api-key')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  const state = getState();

  pathInput.value = state.currentSettings?.gamePath || '';
  keyInput.value = state.currentSettings?.nexusApiKey || '';
  keyInput.type = 'password';
  if (hideNativeCheckbox) {
    hideNativeCheckbox.checked = !!state.currentSettings?.hideNativeMods;
  }
  if (debugConsoleCheckbox) {
    debugConsoleCheckbox.checked = !!state.currentSettings?.debugConsole;
  }

  if (state.currentSettings?.gamePath) {
    pathStatus.textContent = 'Path configured';
    pathStatus.className = 'settings-path-status valid';
  } else {
    pathStatus.textContent = 'No path configured - select your Palworld folder';
    pathStatus.className = 'settings-path-status invalid';
  }

  const dataPathSelect = document.getElementById('settings-data-path-select') as HTMLSelectElement | null;
  const dataPathDisplay = document.getElementById('settings-custom-data-path-display');
  _tempCustomDataPath = state.currentSettings?.customDataPath || null;

  if (dataPathSelect) {
    if (!_tempCustomDataPath) {
      dataPathSelect.value = 'default';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else if (_tempCustomDataPath === '__portable__') {
      dataPathSelect.value = 'portable';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else {
      dataPathSelect.value = 'custom';
      if (dataPathDisplay) {
        dataPathDisplay.style.display = 'block';
        dataPathDisplay.textContent = `Custom Folder: ${_tempCustomDataPath}`;
      }
    }
  }

  modal.classList.add('visible');
}


export function closeSettingsModal(): void {
  document.getElementById('settings-modal')!.classList.remove('visible');
}

export async function handleDataPathChange(): Promise<void> {
  const select = document.getElementById('settings-data-path-select') as HTMLSelectElement | null;
  const display = document.getElementById('settings-custom-data-path-display');
  if (!select) return;

  const value = select.value;
  if (value === 'default') {
    _tempCustomDataPath = null;
    if (display) display.style.display = 'none';
  } else if (value === 'portable') {
    _tempCustomDataPath = '__portable__';
    if (display) display.style.display = 'none';
  } else if (value === 'custom') {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Custom Data Storage Directory',
      });
      if (selected) {
        const path = typeof selected === 'string' ? selected : selected as string;
        _tempCustomDataPath = path;
        if (display) {
          display.style.display = 'block';
          display.textContent = `Custom Folder: ${path}`;
        }
      } else {
        revertDataPathSelect(select, display);
      }
    } catch (e) {
      console.error('Failed to open directory dialog:', e);
      revertDataPathSelect(select, display);
    }
  }
}

function revertDataPathSelect(select: HTMLSelectElement, display: HTMLElement | null): void {
  if (!_tempCustomDataPath) {
    select.value = 'default';
    if (display) display.style.display = 'none';
  } else if (_tempCustomDataPath === '__portable__') {
    select.value = 'portable';
    if (display) display.style.display = 'none';
  } else {
    select.value = 'custom';
    if (display) {
      display.style.display = 'block';
      display.textContent = `Custom Folder: ${_tempCustomDataPath}`;
    }
  }
}

export async function handleSettingsBrowse(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Palworld game folder',
    });
    if (selected) {
      const path = typeof selected === 'string' ? selected : selected as string;
      const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
      pathInput.value = path;
    }
  } catch (e) {
    console.error('Error browsing path:', e);
  }
}

export function handleToggleKeyVisibility(): void {
  const keyInput = document.getElementById('settings-api-key')! as HTMLInputElement;
  const toggleBtn = document.getElementById('settings-toggle-key')!;
  if (keyInput.type === 'password') {
    keyInput.type = 'text';
    toggleBtn.textContent = '\uD83D\uDE48';
  } else {
    keyInput.type = 'password';
    toggleBtn.textContent = '\uD83D\uDC41';
  }
}

export async function handleSaveSettings(): Promise<void> {
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const keyInput = document.getElementById('settings-api-key')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const saveBtn = document.getElementById('settings-save')! as HTMLButtonElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  saveBtn.disabled = true;

  try {
    const newPath = pathInput.value.trim();
    const apiKey = keyInput.value.trim();
    const hideNative = hideNativeCheckbox ? hideNativeCheckbox.checked : false;
    const debugConsole = debugConsoleCheckbox ? debugConsoleCheckbox.checked : false;
    const state = getState();

    if (newPath && newPath !== state.currentSettings?.gamePath) {
      try {
        const settings = await setGamePath(newPath);
        updateState({ currentSettings: settings });
        pathStatus.textContent = 'Path configured';
        pathStatus.className = 'settings-path-status valid';
      } catch (e) {
        pathStatus.textContent = String(e);
        pathStatus.className = 'settings-path-status invalid';
        saveBtn.disabled = false;
        return;
      }
    }

    if (apiKey !== (state.currentSettings?.nexusApiKey || '')) {
      const settings = await setNexusApiKey(apiKey || null);
      updateState({ currentSettings: settings });
    }

    if (hideNative !== !!state.currentSettings?.hideNativeMods) {
      const settings = await setHideNativeMods(hideNative);
      updateState({ currentSettings: settings });
    }

    if (debugConsole !== !!state.currentSettings?.debugConsole) {
      const settings = await setDebugConsole(debugConsole);
      updateState({ currentSettings: settings });
    }

    if (_tempCustomDataPath !== (state.currentSettings?.customDataPath || null)) {
      showToast('Migrating data files to new location...', 'info');
      const settings = await setCustomDataPath(_tempCustomDataPath);
      updateState({ currentSettings: settings });
    }

    closeSettingsModal();
    showToast('Settings saved', 'success');

    const { loadGameVersion, loadDependencies } = await import('./modsView');
    loadGameVersion();
    await loadDependencies();
    await loadMods();
  } catch (e) {
    showToast('Failed to save settings: ' + e, 'error');
  } finally {
    saveBtn.disabled = false;
  }
}


// === INSTALL MODAL ===

let _pendingUpdateModId: string | null = null;

function getCleanNameFromFilename(filename: string): string {
  const stem = filename.substring(0, filename.lastIndexOf('.')) || filename;
  const words = stem.split(/\s+/);
  const clean: string[] = [];
  for (const word of words) {
    if (/^\d+$/.test(word)) {
      break;
    }
    if (/^\d/.test(word) && word.includes('-') && word.length > 6) {
      break;
    }
    clean.push(word);
  }
  const result = clean.join(' ').replace(/\s*\(\s*$/, '').trim();
  return result.length < 2 ? stem.trim() : result;
}

export function showInstallModal(): void {
  document.getElementById('install-modal')!.classList.add('visible');
}

export function closeInstallModal(): void {
  document.getElementById('install-modal')!.classList.remove('visible');
  updateState({ currentAnalysis: null });
  _pendingUpdateModId = null;
  _pendingBatchPaths = [];

  const retryBtn = document.getElementById('modal-install-deps-retry') as HTMLButtonElement | null;
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const cancelBtn = document.getElementById('modal-cancel')! as HTMLButtonElement;
  if (confirmBtn) {
    confirmBtn.style.display = '';
    confirmBtn.textContent = 'Install';
    confirmBtn.disabled = false;
  }
  if (cancelBtn) {
    cancelBtn.textContent = 'Cancel';
    cancelBtn.disabled = false;
  }
}

export function setModalStatus(text: string): void {
  document.getElementById('modal-status')!.textContent = text;
}

export function renderInstallPreview(analysis: ZipAnalysis, existingMod?: { id: string; name: string; version?: string } | null): void {
  updateState({ currentAnalysis: analysis });
  const content = document.getElementById('modal-content')!;
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;

  const retryBtn = document.getElementById('modal-install-deps-retry') as HTMLButtonElement | null;
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  statusEl.textContent = '';

  // Background fetch of Nexus metadata for rich single-mod preview card
  if (analysis.nexusModId && !analysis.nexusInfo) {
    fetchNexusInfoAsync(analysis.nexusModId).then(info => {
      if (getState().currentAnalysis === analysis && info) {
        analysis.nexusInfo = info;
        renderInstallPreview(analysis, existingMod);
      }
    }).catch(() => {});
  }

  // Update banner if existing mod found
  let updateHtml = '';
  if (existingMod) {
    _pendingUpdateModId = existingMod.id;
    updateHtml = `
      <div class="update-banner" id="update-banner" style="margin-bottom:12px;padding:8px 12px;background:rgba(0,188,255,0.1);border:1px solid rgba(0,188,255,0.25);border-radius:6px;display:flex;align-items:center;justify-content:space-between;gap:12px;">
        <span class="update-banner-text" style="font-size:11px;font-weight:600;color:var(--text-primary);">"${escapeHtml(existingMod.name)}" already exists. (Installed: v${existingMod.version || 'unknown'}). Update it?</span>
        <div style="display:flex;gap:8px;">
          <button class="update-banner-btn" id="update-banner-btn" style="padding:4px 10px;background:#00bcff;color:#fff;border:none;border-radius:4px;font-size:11px;font-weight:600;cursor:pointer;">Update</button>
          <button class="update-banner-btn" id="install-new-btn" style="padding:4px 10px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:11px;font-weight:600;cursor:pointer;">Install as New</button>
        </div>
      </div>

    `;
    confirmBtn.textContent = 'Update';
  } else {
    _pendingUpdateModId = null;
    confirmBtn.textContent = 'Install';
  }

  const cleanName = getCleanNameFromFilename(analysis.zipPath.split(/[/\\]/).pop() || '');

  let pakDestHtml = `
    <div class="pak-dest-section" id="single-pak-dest-section" style="display: ${analysis.detectedType === 'pak' || analysis.detectedType === 'logicmods' || (analysis.detectedType === 'hybrid' && analysis.hasPak) ? 'block' : 'none'}; margin-top:8px;">
      <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;display:block;margin-bottom:6px;">Pak destination</label>
      <div class="pak-dest-options" style="display:flex;gap:12px;">
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="~mods" ${analysis.detectedType === 'pak' || analysis.detectedType === 'hybrid' ? 'checked' : ''} />
          <span>~mods/ (Resource Paks)</span>
        </label>
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="logicmods" ${analysis.detectedType === 'logicmods' ? 'checked' : ''} />
          <span>LogicMods/ (Blueprint Logic)</span>
        </label>
      </div>
    </div>
  `;

  const picUrl = analysis.nexusInfo?.pictureUrl || (analysis.nexusInfo as any)?.picture_url || '';
  const versionVal = analysis.detectedVersion || analysis.nexusInfo?.version || '1.0';

  content.innerHTML = `
    <div style="display:flex;gap:24px;align-items:stretch;padding:4px 0;">
       <!-- Left Column: Card Preview (Nexus Info) -->
       ${analysis.nexusInfo ? `
       <div style="width:260px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:140px;overflow:hidden;background:#000;">
             ${picUrl ? `<img src="${escapeHtml(picUrl)}" style="width:100%;height:100%;object-fit:cover;opacity:0.85;" alt="" />` : `<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">N</div>`}
             <div style="position:absolute;bottom:8px;right:8px;background:rgba(0,0,0,0.75);padding:2px 8px;border-radius:12px;font-size:9px;color:#00ffcc;font-weight:700;letter-spacing:0.5px;">
                ${analysis.nexusInfo.downloads.toLocaleString()} DLs
             </div>
          </div>
          <div style="padding:14px;display:flex;flex-direction:column;gap:8px;flex:1;">
             <div style="font-size:13px;font-weight:700;color:var(--text-primary);line-height:1.35;word-break:break-word;">${escapeHtml(analysis.nexusInfo.name)}</div>
             <div style="font-size:10px;color:var(--text-muted)">by ${escapeHtml(analysis.nexusInfo.author)}</div>
             <div style="font-size:11px;color:var(--text-secondary);line-height:1.45;margin-top:4px;flex:1;display:-webkit-box;-webkit-line-clamp:4;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.nexusInfo.summary)}</div>
          </div>
       </div>
       ` : ''}

       <!-- Right Column: Settings Form -->
       <div style="flex:1;display:flex;flex-direction:column;gap:14px;justify-content:center;">
          ${updateHtml}
          
          <div style="display:flex;flex-direction:column;gap:6px;">
             <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Mod Name</label>
             <input type="text" id="mod-name-input" value="${escapeHtml(cleanName)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:13px;font-weight:600;" />
          </div>

          <div style="display:flex;gap:12px;">
             <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Install Type</label>
                <select id="mod-type-select" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;cursor:pointer;">
                  <option value="ue4ss" ${analysis.detectedType === 'ue4ss' ? 'selected' : ''}>UE4SS</option>
                  <option value="palschema" ${analysis.detectedType === 'palschema' ? 'selected' : ''}>PalSchema</option>
                  <option value="pak" ${analysis.detectedType === 'pak' ? 'selected' : ''}>Pak (~mods)</option>
                  <option value="logicmods" ${analysis.detectedType === 'logicmods' ? 'selected' : ''}>LogicMods</option>
                  <option value="hybrid" ${analysis.detectedType === 'hybrid' ? 'selected' : ''}>Hybrid</option>
                </select>
             </div>
             <div style="width:120px;display:flex;flex-direction:column;gap:6px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Version</label>
                <input type="text" id="mod-version-input" value="${escapeHtml(versionVal)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;text-align:center;" />
             </div>
          </div>

          <div style="display:flex;align-items:center;gap:8px;font-size:11px;color:var(--text-muted);background:var(--bg-secondary);padding:8px 12px;border-radius:4px;border:1px solid var(--border);">
             <span style="font-size:14px;">📂</span>
             <span>Archive contains <strong>${analysis.fileCount}</strong> files</span>
          </div>

          ${analysis.nexusModId && !analysis.nexusInfo ? `
             <div style="font-size:11px;color:var(--text-muted);background:var(--bg-secondary);border:1px solid var(--border);padding:6px 12px;border-radius:4px;display:flex;align-items:center;gap:6px;">
                <span style="font-weight:700;color:var(--accent);">N</span> NexusMods ID: <strong>#${analysis.nexusModId}</strong>
             </div>
          ` : ''}

          ${pakDestHtml}
       </div>
    </div>
  `;


  // Dynamically show/hide pak destination options based on selected type
  const typeSelect = document.getElementById('mod-type-select') as HTMLSelectElement;
  const pakDestSection = document.getElementById('single-pak-dest-section');
  if (typeSelect && pakDestSection) {
    typeSelect.addEventListener('change', () => {
      const type = typeSelect.value;
      const analysis = getState().currentAnalysis;
      const hasPak = analysis ? analysis.hasPak : false;
      if (type === 'pak' || type === 'logicmods' || (type === 'hybrid' && hasPak)) {
        pakDestSection.style.display = 'block';
        if (type !== 'hybrid') {
          const radio = pakDestSection.querySelector(`input[value="${type}"]`) as HTMLInputElement;
          if (radio) radio.checked = true;
        }
      } else {
        pakDestSection.style.display = 'none';
      }
    });
  }

  // Wire up update/install-new buttons
  const updateBannerBtn = document.getElementById('update-banner-btn');
  const installNewBtn = document.getElementById('install-new-btn');
  if (updateBannerBtn) {
    updateBannerBtn.addEventListener('click', () => {
      _pendingUpdateModId = existingMod!.id;
      confirmBtn.textContent = 'Update';
      const banner = document.getElementById('update-banner');
      if (banner) banner.style.display = 'none';
    });
  }
  if (installNewBtn) {
    installNewBtn.addEventListener('click', () => {
      _pendingUpdateModId = null;
      confirmBtn.textContent = 'Install';
      const banner = document.getElementById('update-banner');
      if (banner) banner.style.display = 'none';
    });
  }

  confirmBtn.disabled = false;
}


let _pendingBatchPaths: string[] = [];


interface BatchItem {
  path: string;
  filename: string;
  name: string;
  type: string;
  existingModId: string | null;
  existingVersion?: string | null;
  nexusModId?: number | null;
  version?: string | null;
  error?: string;
}
let _batchItems: BatchItem[] = [];

export async function renderBatchInstallPreview(paths: string[]): Promise<void> {
  _pendingBatchPaths = paths;
  updateState({ currentAnalysis: null });
  _pendingUpdateModId = null;
  _batchItems = [];

  const content = document.getElementById('modal-content')!;
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;

  confirmBtn.disabled = true;
  confirmBtn.textContent = 'Install';
  statusEl.textContent = '';

  // Render loading state first
  content.innerHTML = `
    <div style="display:flex;flex-direction:column;align-items:center;padding:24px;color:var(--text-secondary)">
      <div style="font-size:14px;font-weight:600;margin-bottom:8px">Analyzing ${paths.length} archives...</div>
      <div style="font-size:11px;color:var(--text-muted)">Scanning contents, checking versions and detecting types</div>
    </div>
  `;
  showInstallModal();

  // Run analysis sequentially to avoid hammering the Nexus API in parallel
  const results: BatchItem[] = [];
  for (const path of paths) {
    const filename = path.split(/[/\\]/).pop() || '';
    try {
      const analysis = await analyzeZip(path);
      const check = await checkModExistsCommand(path);

      const cleanName = getCleanNameFromFilename(filename);

      results.push({
        path,
        filename,
        name: cleanName,
        type: analysis.detectedType,
        existingModId: check.exists && check.modInfo ? check.modInfo.id : null,
        existingVersion: check.exists && check.modInfo ? check.modInfo.version : null,
        nexusModId: analysis.nexusModId,
        version: analysis.detectedVersion || null,
      });
    } catch (e) {
      results.push({
        path,
        filename,
        name: filename,
        type: 'unknown',
        existingModId: null,
        error: String(e),
      });
    }
  }
  _batchItems = results;

  // Render interactive list
  const rows = _batchItems.map((item, idx) => {
    if (item.error) {
      return `
        <tr style="border-bottom:1px solid var(--border-light)">
          <td style="padding:6px 4px;width:28px;"><input type="checkbox" id="batch-install-${idx}" disabled /></td>
          <td style="padding:6px;font-size:10px;max-width:160px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:var(--danger);" title="${escapeHtml(item.filename)}">${escapeHtml(item.filename)}</td>
          <td colspan="5" style="padding:6px;color:var(--danger);font-size:10px;font-style:italic;">Failed to analyze: ${escapeHtml(item.error)}</td>
        </tr>
      `;
    }

    let stateBadge = `<span style="background:var(--accent-dim);color:var(--accent);border:1px solid var(--accent);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">NEW</span>`;
    if (item.existingModId) {
      if (item.existingVersion && item.version && item.existingVersion.trim().toLowerCase() === item.version.trim().toLowerCase()) {
        stateBadge = `<span style="background:rgba(255,255,255,0.06);color:var(--text-muted);border:1px solid var(--border);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">INSTALLED</span>`;
      } else {
        stateBadge = `<span style="background:var(--success-dim);color:var(--success);border:1px solid var(--success);font-size:8px;padding:1px 4px;font-weight:700;border-radius:2px;">UPDATE</span>`;
      }
    }

    const idText = item.nexusModId ? `#${item.nexusModId}` : '<span style="color:var(--text-muted)">—</span>';
    const verText = item.version ? `v${item.version}` : '<span style="color:var(--text-muted)">—</span>';

    return `
      <tr style="border-bottom:1px solid var(--border-light)">
        <td style="padding:6px 4px;width:28px;"><input type="checkbox" id="batch-install-${idx}" checked style="cursor:pointer;" /></td>
        <td style="padding:6px;font-size:10px;width:160px;max-width:160px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;color:var(--text-secondary);" title="${escapeHtml(item.filename)}">${escapeHtml(item.filename)}</td>
        <td style="padding:6px;font-size:11px;width:70px;white-space:nowrap;color:var(--text-muted);font-weight:600;">${idText}</td>
        <td style="padding:6px;font-size:11px;width:60px;white-space:nowrap;color:var(--text-primary);font-weight:600;">${verText}</td>
        <td style="padding:6px;"><input type="text" id="batch-name-${idx}" value="${escapeHtml(item.name)}" style="width:100%;padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:11px;" /></td>
        <td style="padding:6px;width:90px;">
          <select id="batch-type-${idx}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
            <option value="ue4ss" ${item.type === 'ue4ss' ? 'selected' : ''}>UE4SS</option>
            <option value="palschema" ${item.type === 'palschema' ? 'selected' : ''}>PalSchema</option>
            <option value="pak" ${item.type === 'pak' ? 'selected' : ''}>Pak (~mods)</option>
            <option value="logicmods" ${item.type === 'logicmods' ? 'selected' : ''}>LogicMods</option>
            <option value="hybrid" ${item.type === 'hybrid' ? 'selected' : ''}>Hybrid</option>
          </select>
        </td>
        <td style="padding:6px;width:50px;text-align:right;">${stateBadge}</td>
      </tr>
    `;
  }).join('');

  content.innerHTML = `
    <div id="batch-table-wrapper" style="overflow-y:auto;border:1px solid var(--border);background:var(--bg-secondary);border-radius:4px;margin-bottom:8px;">
      <table style="width:100%;border-collapse:collapse;text-align:left;">
        <thead style="position:sticky;top:0;z-index:2;">
          <tr style="background:var(--bg-tertiary);border-bottom:1px solid var(--border);font-size:10px;font-weight:700;color:var(--text-muted);text-transform:uppercase;">
            <th style="padding:6px;width:28px;">Inst.</th>
            <th style="padding:6px;width:160px;">Archive</th>
            <th style="padding:6px;width:70px;">Nexus ID</th>
            <th style="padding:6px;width:60px;">Version</th>
            <th style="padding:6px;">Target Mod Folder</th>
            <th style="padding:6px;width:90px;">Type</th>
            <th style="padding:6px;width:50px;text-align:right;padding-right:12px;">Status</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    </div>
  `;

  // Fixed width — no dynamic growing
  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '850px';
  }
  const wrapper = document.getElementById('batch-table-wrapper');
  if (wrapper) {
    wrapper.style.maxHeight = 'calc(80vh - 150px)';
  }

  confirmBtn.disabled = false;
}

export async function handleConfirmInstall(): Promise<void> {
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const cancelBtn = document.getElementById('modal-cancel')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;
  const contentEl = document.getElementById('modal-content')!;

  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  if (_pendingBatchPaths.length > 0) {
    // 1. Read values from DOM first before clearing the HTML
    const itemsToInstall: Array<{
      path: string;
      filename: string;
      customName: string;
      customType: string;
      existingModId: string | null;
    }> = [];

    for (let i = 0; i < _batchItems.length; i++) {
      const item = _batchItems[i];
      const installCheckbox = document.getElementById(`batch-install-${i}`) as HTMLInputElement | null;
      
      if (installCheckbox && installCheckbox.checked && !item.error) {
        const nameInput = document.getElementById(`batch-name-${i}`) as HTMLInputElement | null;
        const typeSelect = document.getElementById(`batch-type-${i}`) as HTMLSelectElement | null;
        
        itemsToInstall.push({
          path: item.path,
          filename: item.filename,
          customName: nameInput && nameInput.value.trim() ? nameInput.value.trim() : item.name,
          customType: typeSelect ? typeSelect.value : item.type,
          existingModId: item.existingModId,
        });
      }
    }

    // 2. Clear HTML and prepare results container
    let installed = 0;
    let updated = 0;
    let failed = 0;

    const resultsHtml: string[] = [];
    contentEl.innerHTML = `
      <div class="install-console-header" style="display:flex;align-items:center;background:#181818;padding:6px 12px;border-top-left-radius:6px;border-top-right-radius:6px;border-bottom:1px solid #282828;">
        <span style="font-size:10px;font-family:monospace;color:#888;font-weight:600;">install_log.sh</span>
        <div style="flex:1"></div>
        <div style="display:flex;gap:5px;">
          <span style="width:8px;height:8px;border-radius:50%;background:#ff5f56;display:inline-block;"></span>
          <span style="width:8px;height:8px;border-radius:50%;background:#ffbd2e;display:inline-block;"></span>
          <span style="width:8px;height:8px;border-radius:50%;background:#27c93f;display:inline-block;"></span>
        </div>
      </div>
      <div class="batch-results-list" style="display:flex;flex-direction:column;gap:6px;max-height:280px;min-height:220px;overflow-y:auto;background:#0d0d0d;padding:14px;font-family:monospace;font-size:11px;line-height:1.5;border-bottom-left-radius:6px;border-bottom-right-radius:6px;box-shadow:inset 0 0 10px rgba(0,0,0,0.8);color:#d0d0d0;border:1px solid #282828;border-top:none;"></div>
    `;
    const resultsList = contentEl.querySelector('.batch-results-list')!;

    // 3. Process collected items
    for (let i = 0; i < itemsToInstall.length; i++) {
      const item = itemsToInstall[i];
      statusEl.textContent = `Processing ${item.filename} (${i + 1}/${itemsToInstall.length})...`;

      resultsHtml.push(`<div class="batch-result-item" style="color:#e0af68;font-style:italic;">&gt; Extracting and copying files for ${escapeHtml(item.customName)}...</div>`);
      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;

      try {
        if (item.existingModId) {
          // Re-install/Update
          await updateModCommand(item.path, item.existingModId);
          updated++;
          resultsHtml.pop(); // Remove "Extracting..." line
          resultsHtml.push(`<div class="batch-result-item success" style="color:#00bcff;font-weight:bold;"><span style="color:#777;">[UP]</span> Updated successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        } else {
          // Install new
          let pakDestination: string | null = null;
          if (item.customType === 'pak' || item.customType === 'logicmods' || item.customType === 'hybrid') {
            pakDestination = item.customType === 'logicmods' ? 'logicmods' : '~mods';
          }
          await installMod(item.path, item.customType, pakDestination, item.customName);
          installed++;
          resultsHtml.pop(); // Remove "Extracting..." line
          resultsHtml.push(`<div class="batch-result-item success" style="color:#4af626;font-weight:bold;"><span style="color:#777;">[OK]</span> Installed successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        }
      } catch (e) {
        failed++;
        resultsHtml.pop(); // Remove "Extracting..." line
        resultsHtml.push(`<div class="batch-result-item error" style="color:#ff4a4a;font-weight:bold;"><span style="color:#777;">[ERR]</span> Failed: ${escapeHtml(item.filename)} - ${escapeHtml(String(e))}</div>`);
      }

      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;
    }


    statusEl.textContent = `Batch complete: ${installed} installed, ${updated} updated, ${failed} failed`;
    cancelBtn.disabled = false;
    cancelBtn.textContent = 'Close';
    confirmBtn.style.display = 'none';

    loadMods();
    return;
  }


  const state = getState();
  if (!state.currentAnalysis) return;

  const typeSelect = document.getElementById('mod-type-select') as HTMLSelectElement | null;
  const customType = typeSelect ? typeSelect.value : state.currentAnalysis.detectedType;

  const nameInput = document.getElementById('mod-name-input') as HTMLInputElement | null;
  const customName = nameInput && nameInput.value.trim() ? nameInput.value.trim() : null;

  confirmBtn.textContent = _pendingUpdateModId ? 'Updating...' : 'Installing...';
  statusEl.textContent = _pendingUpdateModId ? 'Updating mod...' : 'Extracting and installing mod...';

  contentEl.innerHTML = `
    <div class="install-console-header" style="display:flex;align-items:center;background:#181818;padding:6px 12px;border-top-left-radius:6px;border-top-right-radius:6px;border-bottom:1px solid #282828;">
      <span style="font-size:10px;font-family:monospace;color:#888;font-weight:600;">install_log.sh</span>
      <div style="flex:1"></div>
      <div style="display:flex;gap:5px;">
        <span style="width:8px;height:8px;border-radius:50%;background:#ff5f56;display:inline-block;"></span>
        <span style="width:8px;height:8px;border-radius:50%;background:#ffbd2e;display:inline-block;"></span>
        <span style="width:8px;height:8px;border-radius:50%;background:#27c93f;display:inline-block;"></span>
      </div>
    </div>
    <div class="batch-results-list" style="display:flex;flex-direction:column;gap:6px;max-height:280px;min-height:220px;overflow-y:auto;background:#0d0d0d;padding:14px;font-family:monospace;font-size:11px;line-height:1.5;border-bottom-left-radius:6px;border-bottom-right-radius:6px;box-shadow:inset 0 0 10px rgba(0,0,0,0.8);color:#d0d0d0;border:1px solid #282828;border-top:none;"></div>
  `;
  const resultsList = contentEl.querySelector('.batch-results-list') as HTMLElement;
  const logs: string[] = [];

  // Check dependencies first
  const depStatus = await checkDependencies();
  const ue4ssRequired = ['ue4ss', 'palschema', 'hybrid'].includes(customType);
  const palschemaRequired = (customType === 'palschema') || (customType === 'hybrid' && (state.currentAnalysis.hasPalSchemaJson || (state.currentAnalysis.files || []).some((f: string) => f.toLowerCase().includes('palschema'))));

  const missingUe4ss = ue4ssRequired && !depStatus.ue4ss_installed;
  const missingPalSchema = palschemaRequired && !depStatus.palschema_installed;

  if (missingUe4ss || missingPalSchema) {
    const missingNames = [];
    if (missingUe4ss) missingNames.push('UE4SS');
    if (missingPalSchema) missingNames.push('PalSchema');

    logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] Installation failed: Missing dependencies (${missingNames.join(', ')}).</div>`);
    logs.push(`<div style="color:#888;">&gt; Please click "Install Deps & Retry" to install them automatically.</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = 'Missing dependencies';

    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? 'Update' : 'Install';

    const retryBtn = document.getElementById('modal-install-deps-retry') as HTMLButtonElement | null;
    if (retryBtn) {
      retryBtn.style.display = '';
      retryBtn.onclick = async () => {
        retryBtn.disabled = true;
        try {
          // Download and install in priority order (UE4SS first, then PalSchema)
          if (missingUe4ss) {
            logs.push(`<div style="color:#e0af68;">&gt; Downloading and installing UE4SS dependency...</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
            statusEl.textContent = 'Downloading and installing UE4SS...';
            await installUe4ss();
            logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] UE4SS installed successfully!</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
          }
          if (missingPalSchema) {
            logs.push(`<div style="color:#e0af68;">&gt; Downloading and installing PalSchema dependency...</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
            statusEl.textContent = 'Downloading and installing PalSchema...';
            await installPalschema();
            logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] PalSchema installed successfully!</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
          }
          
          // Load dependencies to update GUI state / badges!
          const { loadDependencies } = await import('./modsView');
          await loadDependencies();

          logs.push(`<div style="color:#888;">&gt; Dependencies installed. Starting mod installation...</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;

          retryBtn.style.display = 'none';
          
          // Small timeout so the user sees the final dependency status log before mod installation proceeds
          setTimeout(() => {
            executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state);
          }, 1000);
        } catch (err) {
          logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] Failed to install dependencies: ${escapeHtml(String(err))}</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;
          showToast('Failed to install dependencies: ' + err, 'error');
          statusEl.textContent = 'Failed to install dependencies';
        } finally {
          retryBtn.disabled = false;
        }
      };
    }
    return;
  }

  await executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state);
}

async function executeModInstallation(
  logs: string[],
  resultsList: HTMLElement,
  statusEl: HTMLElement,
  confirmBtn: HTMLButtonElement,
  cancelBtn: HTMLButtonElement,
  customType: string,
  customName: string | null,
  state: any
) {

  logs.push(`<div style="color:#e0af68;">&gt; Extracting ZIP contents to temporary directory...</div>`);
  resultsList.innerHTML = logs.join('');

  try {
    let pakDestination: string | null = null;
    if (customType === 'pak' || customType === 'logicmods' || customType === 'hybrid') {
      const checked = document.querySelector('input[name="pak-dest"]:checked') as HTMLInputElement;
      pakDestination = checked ? checked.value : (customType === 'logicmods' ? 'logicmods' : '~mods');
    }

    logs.push(`<div style="color:#e0af68;">&gt; Copying files to destination folder...</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    if (_pendingUpdateModId) {
      await updateModCommand(state.currentAnalysis.zipPath, _pendingUpdateModId);
      logs.push(`<div style="color:#00bcff;font-weight:bold;">[UP] Mod updated successfully!</div>`);
    } else {
      await installMod(state.currentAnalysis.zipPath, customType, pakDestination, customName);
      logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] Mod installed successfully!</div>`);
    }

    const versionInput = document.getElementById('mod-version-input') as HTMLInputElement | null;
    if (versionInput && versionInput.value.trim()) {
      try {
        const afterInstall = getState().allMods;
        const last = afterInstall[afterInstall.length - 1];
        if (last && !_pendingUpdateModId) {
          await setModVersionApi(last.id, versionInput.value.trim());
          logs.push(`<div style="color:#888;">&gt; Setting version parameter to: v${versionInput.value.trim()}</div>`);
        }
      } catch {}
    }

    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    statusEl.textContent = _pendingUpdateModId ? 'Updated successfully!' : 'Installed successfully!';
    setTimeout(() => {
      closeInstallModal();
      loadMods();
    }, 1500);
  } catch (e) {
    logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] Installation failed: ${escapeHtml(String(e))}</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = 'Installation failed';
    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? 'Update' : 'Install';
  }
}





export async function handleInstall(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Mod Archives', extensions: ['zip', 'rar', '7z'] }],
      title: 'Select mod archive (.zip, .rar, .7z)',
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];

    if (paths.length === 1) {
      const zipPath = paths[0];
      showInstallModal();
      setModalStatus('Analyzing archive file...');

      const analysis = await analyzeZip(zipPath);

      let existingMod: { id: string; name: string } | null = null;
      try {
        const checkResult = await checkModExistsCommand(zipPath);
        if (checkResult.exists && checkResult.modInfo) {
          existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name };
        }
      } catch {}

      renderInstallPreview(analysis, existingMod);
    } else {
      renderBatchInstallPreview(paths);
    }
  } catch (e) {
    console.error('Error analyzing:', e);
    closeInstallModal();
    showToast('Failed to analyze archives: ' + e, 'error');
  }
}
