import { getSettings, setGamePath, setHideNativeMods, setDebugConsole, analyzeZip, installMod, checkModExistsCommand, updateModCommand, setModVersion as setModVersionApi, fetchNexusInfoAsync, checkDependencies, installUe4ss, installPalschema, setCustomDataPath, setToolbarScale, buildInstallManifest, installModWithManifest } from '../api';
import type { ZipAnalysis } from '../api';
import { getState, updateState } from '../state';
import { renderModsView, loadMods } from './modsView';
import { showToast } from './toast';
import { showConfirm } from './confirm';
import { escapeHtml } from '../utils/helpers';

// === SETTINGS MODAL ===

let _tempCustomDataPath: string | null = null;

export function openSettingsModal(): void {
  const modal = document.getElementById('settings-modal')!;
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const forceLoadOrderCheckbox = document.getElementById('settings-force-load-order')! as HTMLInputElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  const state = getState();

  pathInput.value = state.currentSettings?.gamePath || '';
  if (hideNativeCheckbox) {
    hideNativeCheckbox.checked = !!state.currentSettings?.hideNativeMods;
  }
  if (debugConsoleCheckbox) {
    debugConsoleCheckbox.checked = !!state.currentSettings?.debugConsole;
  }
  if (forceLoadOrderCheckbox) {
    forceLoadOrderCheckbox.checked = !!state.currentSettings?.forceLoadOrder;
    
    // Clear old event listener to prevent multiple bindings if modal opens multiple times
    const newCheckbox = forceLoadOrderCheckbox.cloneNode(true) as HTMLInputElement;
    forceLoadOrderCheckbox.parentNode!.replaceChild(newCheckbox, forceLoadOrderCheckbox);
    
    newCheckbox.addEventListener('change', async () => {
      if (newCheckbox.checked) {
        const confirmed = await showConfirm(
          'Enable Load Order Settings',
          'Enabling Load Order moves PalSchema mods to a dynamic /Storage folder and creates NTFS junctions. This restructuring can cause issues with certain custom mod setups. It is highly recommended to back up your current profile/folder before proceeding.<br><br>Do you want to continue?',
          'Yes, Enable',
          'Cancel'
        );
        if (!confirmed) {
          newCheckbox.checked = false;
        }
      }
    });
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

  // Toolbar Scaling slider
  const scaleInput = document.getElementById('settings-toolbar-scale') as HTMLInputElement | null;
  const scaleValue = document.getElementById('settings-toolbar-scale-value');
  const initialScale = state.currentSettings?.toolbarScale || 1.0;
  if (scaleInput) {
    scaleInput.value = initialScale.toString();
    if (scaleValue) {
      scaleValue.textContent = `${Math.round(initialScale * 100)}%`;
    }
    
    scaleInput.addEventListener('input', () => {
      const scale = parseFloat(scaleInput.value);
      if (scaleValue) {
        scaleValue.textContent = `${Math.round(scale * 100)}%`;
      }
      document.documentElement.style.setProperty('--toolbar-scale', scale.toString());
    });
  }

  modal.classList.add('visible');
}


export function closeSettingsModal(): void {
  document.getElementById('settings-modal')!.classList.remove('visible');
  const savedScale = getState().currentSettings?.toolbarScale || 1.0;
  document.documentElement.style.setProperty('--toolbar-scale', savedScale.toString());
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

export async function handleSaveSettings(): Promise<void> {
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const forceLoadOrderCheckbox = document.getElementById('settings-force-load-order')! as HTMLInputElement;
  const saveBtn = document.getElementById('settings-save')! as HTMLButtonElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  saveBtn.disabled = true;

  try {
    const newPath = pathInput.value.trim();
    const hideNative = hideNativeCheckbox ? hideNativeCheckbox.checked : false;
    const debugConsole = debugConsoleCheckbox ? debugConsoleCheckbox.checked : false;
    const forceLoadOrder = forceLoadOrderCheckbox ? forceLoadOrderCheckbox.checked : false;
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

    if (hideNative !== !!state.currentSettings?.hideNativeMods) {
      const settings = await setHideNativeMods(hideNative);
      updateState({ currentSettings: settings });
    }

    if (debugConsole !== !!state.currentSettings?.debugConsole) {
      const settings = await setDebugConsole(debugConsole);
      updateState({ currentSettings: settings });
    }

    if (forceLoadOrder !== !!state.currentSettings?.forceLoadOrder) {
      const { setForceLoadOrder } = await import('../api');
      const settings = await setForceLoadOrder(forceLoadOrder);
      updateState({ currentSettings: settings });
      const { updateLoadTabVisibility } = await import('./loadView');
      updateLoadTabVisibility();
    }

    if (_tempCustomDataPath !== (state.currentSettings?.customDataPath || null)) {
      showToast('Migrating data files to new location...', 'info');
      const settings = await setCustomDataPath(_tempCustomDataPath);
      updateState({ currentSettings: settings });
    }

    const scaleInput = document.getElementById('settings-toolbar-scale') as HTMLInputElement | null;
    if (scaleInput) {
      const scale = parseFloat(scaleInput.value);
      if (scale !== (state.currentSettings?.toolbarScale || 1.0)) {
        const settings = await setToolbarScale(scale);
        updateState({ currentSettings: settings });
        document.documentElement.style.setProperty('--toolbar-scale', scale.toString());
      }
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
interface FileTreeNode {
  name: string;
  routeType?: string;
  destPath?: string;
  children: Map<string, FileTreeNode>;
}

export function showFileTreeModal(routes: any[], modName: string): void {
  const state = getState();
  const gamePath = state.currentSettings?.gamePath || '';

  function buildFileTree(routes: any[]): FileTreeNode {
    const root: FileTreeNode = { name: 'Root', children: new Map() };
    for (const r of routes) {
      const parts = r.zipPath.split('/').filter((p: string) => p.length > 0);
      let current = root;
      for (let i = 0; i < parts.length; i++) {
        const part = parts[i];
        const isLeaf = i === parts.length - 1;
        if (!current.children.has(part)) {
          current.children.set(part, { name: part, children: new Map() });
        }
        current = current.children.get(part)!;
        if (isLeaf) {
          current.routeType = r.routeType;
          current.destPath = r.destPath;
        }
      }
    }
    return root;
  }

  function compactFileTree(node: FileTreeNode): void {
    for (const child of node.children.values()) {
      compactFileTree(child);
    }

    if (node.name !== 'Root' && node.children.size === 1) {
      const childKey = Array.from(node.children.keys())[0];
      const childNode = node.children.get(childKey)!;
      if (childNode.children.size > 0) {
        node.name = `${node.name}/${childNode.name}`;
        node.children = childNode.children;
        compactFileTree(node);
      }
    }
  }

  function getRelativeDestPath(destPath: string, gamePath: string): string {
    if (!destPath || !gamePath) return destPath;
    const normalizedDest = destPath.replace(/\\/g, '/').toLowerCase();
    const normalizedGame = gamePath.replace(/\\/g, '/').toLowerCase();
    
    if (normalizedDest.startsWith(normalizedGame)) {
      let rel = destPath.substring(gamePath.length);
      if (rel.startsWith('/') || rel.startsWith('\\')) {
        rel = rel.substring(1);
      }
      return rel;
    }
    return destPath;
  }

  function renderFileTreeHTML(node: FileTreeNode, depth: number = 0): string {
    const sortedChildren = Array.from(node.children.values()).sort((a, b) => {
      const aIsFolder = a.children.size > 0;
      const bIsFolder = b.children.size > 0;
      if (aIsFolder !== bIsFolder) {
        return aIsFolder ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    });

    return sortedChildren.map(child => {
      const isFolder = child.children.size > 0;
      if (isFolder) {
        return `
          <div class="tree-folder-node" style="margin-left: ${depth === 0 ? 0 : 12}px; display: flex; flex-direction: column; gap: 4px;">
            <div class="tree-folder-header" style="display: flex; align-items: center; gap: 8px; padding: 4px 8px; border-radius: 4px; color: var(--text-primary); font-weight: 600; font-size: 12px; background: rgba(255,255,255,0.02); user-select: none; transition: background 0.2s; cursor: pointer;" onmouseover="this.style.background='rgba(255,255,255,0.05)'" onmouseout="this.style.background='rgba(255,255,255,0.02)'">
              <span style="color: #ffd166; font-size: 13px; display: flex; align-items: center;">📁</span>
              <span style="font-family: monospace;">${escapeHtml(child.name)}</span>
            </div>
            <div class="tree-folder-children" style="border-left: 1px dashed var(--border); margin-left: 7px; padding-left: 6px; display: flex; flex-direction: column; gap: 2px;">
              ${renderFileTreeHTML(child, depth + 1)}
            </div>
          </div>
        `;
      } else {
        const relativeDest = getRelativeDestPath(child.destPath || '', gamePath);
        return `
          <div class="tree-file-node" style="margin-left: ${depth === 0 ? 0 : 12}px; display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; border-radius: 4px; font-size: 11px; gap: 12px; transition: background 0.2s;" onmouseover="this.style.background='rgba(255,255,255,0.03)'" onmouseout="this.style.background='transparent'">
            <div style="display: flex; align-items: center; gap: 8px; overflow: hidden; flex-grow: 1;">
              <span style="color: var(--text-secondary); font-size: 12px; display: flex; align-items: center;">📄</span>
              <div style="display: flex; flex-direction: column; overflow: hidden;">
                <span style="font-family: monospace; color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap; font-weight: 500;">${escapeHtml(child.name)}</span>
                <span style="font-size: 9px; color: var(--text-muted); text-overflow: ellipsis; overflow: hidden; white-space: nowrap; font-family: monospace;" title="${escapeHtml(child.destPath || '')}">→ ${escapeHtml(relativeDest)}</span>
              </div>
            </div>
            <span style="font-size: 8px; padding: 2px 4px; border-radius: 3px; background: var(--bg-secondary); color: var(--accent); border: 1px solid var(--border); text-transform: uppercase; font-weight: 700; height: fit-content; white-space: nowrap;">${escapeHtml(child.routeType || '')}</span>
          </div>
        `;
      }
    }).join('');
  }

  const overlay = document.createElement('div');
  overlay.id = 'full-files-modal-overlay';
  overlay.style.position = 'fixed';
  overlay.style.top = '0';
  overlay.style.left = '0';
  overlay.style.width = '100vw';
  overlay.style.height = '100vh';
  overlay.style.background = 'rgba(0,0,0,0.85)';
  overlay.style.display = 'flex';
  overlay.style.alignItems = 'center';
  overlay.style.justifyContent = 'center';
  overlay.style.zIndex = '9999';

  const content = document.createElement('div');
  content.style.background = 'var(--bg-secondary)';
  content.style.border = '1px solid var(--border)';
  content.style.borderRadius = 'var(--card-radius)';
  content.style.width = '90%';
  content.style.maxWidth = '750px';
  content.style.maxHeight = '80vh';
  content.style.display = 'flex';
  content.style.flexDirection = 'column';
  content.style.overflow = 'hidden';
  content.style.boxShadow = '0 12px 30px rgba(0,0,0,0.6)';

  const header = document.createElement('div');
  header.style.padding = '16px';
  header.style.borderBottom = '1px solid var(--border)';
  header.style.display = 'flex';
  header.style.justifyContent = 'space-between';
  header.style.alignItems = 'center';
  header.innerHTML = `
    <h3 style="margin:0;font-size:15px;font-weight:600;color:var(--text-primary);">Files to Install for ${escapeHtml(modName)} (${routes.length})</h3>
    <button id="close-full-files-btn" style="background:none;border:none;color:var(--text-secondary);cursor:pointer;font-size:22px;line-height:1;padding:4px;">&times;</button>
  `;

  const listContainer = document.createElement('div');
  listContainer.style.padding = '16px';
  listContainer.style.overflowY = 'auto';
  listContainer.style.flexGrow = '1';
  listContainer.style.display = 'flex';
  listContainer.style.flexDirection = 'column';
  listContainer.style.gap = '8px';

  const fileTree = buildFileTree(routes);
  compactFileTree(fileTree);
  listContainer.innerHTML = renderFileTreeHTML(fileTree);

  content.appendChild(header);
  content.appendChild(listContainer);
  overlay.appendChild(content);
  document.body.appendChild(overlay);

  const closeBtn = document.getElementById('close-full-files-btn');
  closeBtn?.addEventListener('click', () => overlay.remove());
  overlay.addEventListener('click', (ev) => {
    if (ev.target === overlay) overlay.remove();
  });
}

export function setModalStatus(text: string): void {
  document.getElementById('modal-status')!.textContent = text;
}

export async function renderInstallPreview(analysis: ZipAnalysis, existingMod?: { id: string; name: string; version?: string } | null): Promise<void> {
  updateState({ currentAnalysis: analysis });
  const content = document.getElementById('modal-content')!;
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;

  const retryBtn = document.getElementById('modal-install-deps-retry') as HTMLButtonElement | null;
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  statusEl.textContent = 'Preparing install...';
  confirmBtn.disabled = true;

  // Background fetch of Nexus metadata for rich single-mod preview card
  if (analysis.nexusModId && !analysis.nexusInfo) {
    fetchNexusInfoAsync(analysis.nexusModId).then(info => {
      if (getState().currentAnalysis === analysis && info) {
        analysis.nexusInfo = info;
        renderInstallPreview(analysis, existingMod);
      }
    }).catch(() => {});
  }

  let cleanName = getCleanNameFromFilename(analysis.zipPath.split(/[/\\]/).pop() || '');
  if (!analysis.nexusInfo && analysis.modinfo?.name) {
    cleanName = analysis.modinfo.name;
  }

  let manifest;
  try {
    manifest = await buildInstallManifest(
      analysis.zipPath,
      getState().currentSettings?.gamePath || '',
      analysis.detectedType === 'logicmods' ? 'logicmods' : '~mods',
      cleanName
    );
  } catch (err) {
    content.innerHTML = `<div style="padding:20px;color:#ff4a4a;font-weight:bold;">Error analyzing manifest: ${escapeHtml(String(err))}</div>`;
    return;
  }

  confirmBtn.disabled = false;
  statusEl.textContent = '';

  // Update banner if existing mod found
  let updateHtml = '';
  if (existingMod) {
    _pendingUpdateModId = existingMod.id;
    updateHtml = `
      <div class="update-banner" id="update-banner" style="margin-bottom:12px;padding:8px 12px;background:rgba(0,188,255,0.1);border:1px solid rgba(0,188,255,0.25);border-radius:6px;display:flex;align-items:center;justify-content:space-between;gap:12px;">
        <span class="update-banner-text" style="font-size:11px;font-weight:600;color:var(--text-primary);">${escapeHtml(existingMod.name)} already exists. (Installed: v${existingMod.version || 'unknown'}). Update it?</span>
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

  const picUrl = analysis.nexusInfo?.pictureUrl || (analysis.nexusInfo as any)?.picture_url || '';
  let versionVal = analysis.detectedVersion || analysis.nexusInfo?.version || '1.0';
  if (!analysis.nexusInfo && analysis.modinfo?.version) {
    versionVal = analysis.modinfo.version;
  }

  const displayType = manifest.modType === 'hybrid' ? `Hybrid (${[manifest.hasUe4ss ? 'UE4SS' : '', manifest.hasPalschema ? 'PalSchema' : '', manifest.hasPak ? 'Pak' : ''].filter(Boolean).join(' + ')})` : manifest.modType.toUpperCase();

  const isLogicModsDefault = manifest.modType === 'logicmods' || manifest.routes.some((r: any) => r.routeType === 'logicmods');

  let pakDestHtml = `
    <div class="pak-dest-section" id="single-pak-dest-section" style="display: ${manifest.hasPak ? 'block' : 'none'}; margin-top:8px;">
      <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;display:block;margin-bottom:6px;">Pak destination</label>
      <div class="pak-dest-options" style="display:flex;gap:12px;">
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="~mods" ${isLogicModsDefault ? '' : 'checked'} />
          <span>~mods/ (Resource Paks)</span>
        </label>
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="logicmods" ${isLogicModsDefault ? 'checked' : ''} />
          <span>LogicMods/ (Blueprint Logic)</span>
        </label>
      </div>
    </div>
  `;

  content.innerHTML = `
    <div style="display:flex;gap:24px;align-items:stretch;padding:4px 0;">
       <!-- Left Column: Card Preview (Nexus Info or Local Modinfo) -->
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
       ` : (analysis.modinfo ? `
       <div style="width:260px;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:140px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;border-bottom:1px solid var(--border);">
             <div style="font-size:42px;color:var(--accent);">🛠</div>
             <div style="position:absolute;bottom:8px;right:8px;background:rgba(0,0,0,0.75);padding:2px 8px;border-radius:12px;font-size:9px;color:var(--accent);font-weight:700;letter-spacing:0.5px;text-transform:uppercase;">
                Local Package
             </div>
          </div>
          <div style="padding:14px;display:flex;flex-direction:column;gap:8px;flex:1;">
             <div style="font-size:13px;font-weight:700;color:var(--text-primary);line-height:1.35;word-break:break-word;">${escapeHtml(analysis.modinfo.name || cleanName)}</div>
             <div style="font-size:10px;color:var(--text-muted)">by ${escapeHtml(analysis.modinfo.author || 'Unknown')}</div>
             <div style="font-size:11px;color:var(--text-secondary);line-height:1.45;margin-top:4px;flex:1;display:-webkit-box;-webkit-line-clamp:4;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.modinfo.description || 'No description provided.')}</div>
          </div>
       </div>
       ` : '')}

       <!-- Right Column: Settings Form -->
       <div style="flex:1;display:flex;flex-direction:column;gap:14px;justify-content:center;">
          ${updateHtml}
          
          <div style="display:flex;gap:12px;">
            <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
               <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Mod Display Name</label>
               <input type="text" id="mod-name-input" value="${escapeHtml(cleanName)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:13px;font-weight:600;" />
            </div>
            <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
               <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Folder Name (Disk)</label>
               <input type="text" id="mod-folder-name-input" value="${escapeHtml(manifest.folderName)}" disabled style="width:100%;padding:8px 12px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:13px;font-weight:600;cursor:not-allowed;" />
            </div>
          </div>

          <div style="display:flex;gap:12px;">
             <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Detected Type</label>
                <input type="text" value="${escapeHtml(displayType)}" disabled style="width:100%;padding:8px 12px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;" />
             </div>
             <div style="width:120px;display:flex;flex-direction:column;gap:6px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Version</label>
                <input type="text" id="mod-version-input" value="${escapeHtml(versionVal)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;text-align:center;" />
             </div>
          </div>

          <div style="display:flex;flex-direction:column;gap:6px;">
             <div style="display:flex;justify-content:space-between;align-items:center;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Files to install</label>
                <button id="view-all-files-btn" class="btn btn-secondary" style="font-size:10px;padding:2px 6px;height:auto;line-height:1;margin:0;">Show Full List</button>
             </div>
             <div class="manifest-files-list" style="max-height:85px;overflow-y:auto;background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;padding:6px;font-family:monospace;font-size:10px;display:flex;flex-direction:column;gap:4px;">
               ${manifest.routes.map((r: any) => `
                 <div style="display:flex;justify-content:space-between;align-items:center;padding:2px 4px;border-radius:2px;background:rgba(255,255,255,0.02);">
                   <span style="color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px;" title="${escapeHtml(r.zipPath)}">${escapeHtml(r.zipPath)}</span>
                   <span style="font-size:8px;padding:1px 3px;border-radius:3px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);text-transform:uppercase;">${r.routeType}</span>
                 </div>
               `).join('')}
             </div>
          </div>

          ${pakDestHtml}
       </div>
    </div>
  `;  // Wire up Show Full List button
  const viewAllBtn = document.getElementById('view-all-files-btn');
  if (viewAllBtn) {
    viewAllBtn.addEventListener('click', (e) => {
      e.preventDefault();
      showFileTreeModal(manifest.routes, cleanName);
    });
  }

  const pakDestRadios = document.querySelectorAll('input[name="pak-dest"]');
  pakDestRadios.forEach(radio => {
    radio.addEventListener('change', async (e) => {
      const selectedDest = (e.target as HTMLInputElement).value;
      try {
        const newManifest = await buildInstallManifest(
          analysis.zipPath,
          getState().currentSettings?.gamePath || '',
          selectedDest,
          cleanName
        );
        const filesListContainer = document.querySelector('.manifest-files-list');
        if (filesListContainer) {
          filesListContainer.innerHTML = newManifest.routes.map((r: any) => `
            <div style="display:flex;justify-content:space-between;align-items:center;padding:2px 4px;border-radius:2px;background:rgba(255,255,255,0.02);">
              <span style="color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:180px;" title="${escapeHtml(r.zipPath)}">${escapeHtml(r.zipPath)}</span>
              <span style="font-size:8px;padding:1px 3px;border-radius:3px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);text-transform:uppercase;">${r.routeType}</span>
            </div>
          `).join('');
        }
        manifest = newManifest;
      } catch (err) {
        console.error("Failed to rebuild manifest on pak dest change:", err);
      }
    });
  });

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
  existingModInfo?: any;
  existingVersion?: string | null;
  nexusModId?: number | null;
  version?: string | null;
  error?: string;
  hasPak?: boolean;
}
let _batchItems: BatchItem[] = [];

function getModFolderName(m: any): string {
  const path = m.game_path || m.disabled_path || '';
  return path.split(/[/\\]/).pop() || '';
}

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
      let existingModId: string | null = null;
      let existingModInfo: any = null;
      try {
        const checkResult = await checkModExistsCommand(path);
        if (checkResult.exists && checkResult.modInfo) {
          existingModId = checkResult.modInfo.id;
          existingModInfo = checkResult.modInfo;
        }
      } catch {}

      let nameVal = getCleanNameFromFilename(filename);
      if (!analysis.nexusInfo && analysis.modinfo?.name) {
        nameVal = analysis.modinfo.name;
      }

      const manifest = await buildInstallManifest(
        path,
        getState().currentSettings?.gamePath || '',
        analysis.detectedType === 'logicmods' ? 'logicmods' : '~mods',
        nameVal
      );

      results.push({
        path,
        filename,
        name: nameVal,
        type: manifest.modType,
        existingModId,
        existingModInfo,
        existingVersion: existingModInfo?.version,
        nexusModId: analysis.nexusModId,
        version: analysis.detectedVersion || analysis.nexusInfo?.version || '1.0',
        hasPak: manifest.hasPak,
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

    const isPakOrLogicOrHybrid = item.type === 'pak' || item.type === 'logicmods' || (item.type === 'hybrid' && item.hasPak);
    let pakDestSelectHtml = `<span style="color:var(--text-muted);font-size:10px;">—</span>`;
    if (isPakOrLogicOrHybrid) {
      pakDestSelectHtml = `
        <select id="batch-pak-dest-${idx}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
          <option value="~mods" ${item.type === 'pak' || item.type === 'hybrid' ? 'selected' : ''}>~mods</option>
          <option value="LogicMods" ${item.type === 'logicmods' ? 'selected' : ''}>LogicMods</option>
        </select>
      `;
    }

    return `
      <tr style="border-bottom:1px solid var(--border-light)">
        <td style="padding:6px 4px;width:28px;"><input type="checkbox" id="batch-install-${idx}" checked style="cursor:pointer;" /></td>
        <td style="padding:6px;font-size:10px;width:180px;max-width:180px;color:var(--text-secondary);">
          <div style="display:flex;align-items:center;justify-content:space-between;gap:8px;overflow:hidden;">
            <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex-grow:1;" title="${escapeHtml(item.filename)}">${escapeHtml(item.filename)}</span>
            <button id="batch-view-files-${idx}" style="padding:2px 6px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);border-radius:4px;font-size:9px;cursor:pointer;white-space:nowrap;font-weight:600;" onmouseover="this.style.background='rgba(255,255,255,0.05)'" onmouseout="this.style.background='var(--bg-secondary)'">Show Files</button>
          </div>
        </td>
        <td style="padding:6px;font-size:11px;width:70px;white-space:nowrap;color:var(--text-muted);font-weight:600;">${idText}</td>
        <td style="padding:6px;font-size:11px;width:60px;white-space:nowrap;color:var(--text-primary);font-weight:600;">${verText}</td>
        <td style="padding:6px;"><input type="text" id="batch-name-${idx}" value="${escapeHtml(item.name)}" style="width:100%;padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:11px;" /></td>
        <td style="padding:6px;width:90px;">
          <select id="batch-type-${idx}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
            <option value="ue4ss" ${item.type === 'ue4ss' ? 'selected' : ''}>UE4SS</option>
            <option value="palschema" ${item.type === 'palschema' ? 'selected' : ''}>PalSchema</option>
            <option value="pak" ${item.type === 'pak' || item.type === 'logicmods' ? 'selected' : ''}>Pak</option>
            <option value="hybrid" ${item.type === 'hybrid' ? 'selected' : ''}>Hybrid</option>
          </select>
        </td>
        <td id="batch-pak-dest-container-${idx}" style="padding:6px;width:95px;">${pakDestSelectHtml}</td>
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
            <th style="padding:6px;width:180px;">Archive</th>
            <th style="padding:6px;width:70px;">Nexus ID</th>
            <th style="padding:6px;width:60px;">Version</th>
            <th style="padding:6px;">Target Mod Folder</th>
            <th style="padding:6px;width:90px;">Type</th>
            <th style="padding:6px;width:95px;">Pak Target</th>
            <th style="padding:6px;width:50px;text-align:right;padding-right:12px;">Status</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    </div>
  `;

  // Add listeners to type selectors and view files buttons
  for (let i = 0; i < _batchItems.length; i++) {
    const item = _batchItems[i];
    const typeSelect = document.getElementById(`batch-type-${i}`) as HTMLSelectElement | null;
    if (typeSelect) {
      typeSelect.addEventListener('change', () => {
        const val = typeSelect.value;
        const destContainer = document.getElementById(`batch-pak-dest-container-${i}`);
        if (destContainer) {
          if (val === 'pak' || val === 'logicmods' || (val === 'hybrid' && item.hasPak)) {
            destContainer.innerHTML = `
              <select id="batch-pak-dest-${i}" style="padding:2px 4px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);font-size:10px;width:100%;">
                <option value="~mods" ${val === 'pak' || val === 'hybrid' ? 'selected' : ''}>~mods</option>
                <option value="LogicMods" ${val === 'logicmods' ? 'selected' : ''}>LogicMods</option>
              </select>
            `;
          } else {
            destContainer.innerHTML = `<span style="color:var(--text-muted);font-size:10px;">—</span>`;
          }
        }
      });
    }

    const viewBtn = document.getElementById(`batch-view-files-${i}`) as HTMLButtonElement | null;
    if (viewBtn) {
      viewBtn.addEventListener('click', async (e) => {
        e.preventDefault();
        viewBtn.textContent = 'Loading...';
        viewBtn.disabled = true;
        try {
          const customName = (document.getElementById(`batch-name-${i}`) as HTMLInputElement)?.value || item.name;
          const pakDest = (document.getElementById(`batch-pak-dest-${i}`) as HTMLSelectElement)?.value || '~mods';
          const manifest = await buildInstallManifest(
            item.path,
            getState().currentSettings?.gamePath || '',
            pakDest,
            customName
          );
          showFileTreeModal(manifest.routes, customName);
        } catch (err) {
          showToast(`Error loading file list: ${err}`, 'error');
        } finally {
          viewBtn.textContent = 'Show Files';
          viewBtn.disabled = false;
        }
      });
    }
  }

  // Fixed width — no dynamic growing
  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '900px';
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
      pakDestination: string | null;
      existingModId: string | null;
    }> = [];

    for (let i = 0; i < _batchItems.length; i++) {
      const item = _batchItems[i];
      const installCheckbox = document.getElementById(`batch-install-${i}`) as HTMLInputElement | null;
      
      if (installCheckbox && installCheckbox.checked && !item.error) {
        const nameInput = document.getElementById(`batch-name-${i}`) as HTMLInputElement | null;
        const typeSelect = document.getElementById(`batch-type-${i}`) as HTMLSelectElement | null;
        const pakDestSelect = document.getElementById(`batch-pak-dest-${i}`) as HTMLSelectElement | null;
        
        const inputName = nameInput && nameInput.value.trim() ? nameInput.value.trim() : item.name;
        const existingFolder = item.existingModInfo ? getModFolderName(item.existingModInfo) : '';
        const isUpdate = item.existingModId && (inputName.toLowerCase() === existingFolder.toLowerCase());

        itemsToInstall.push({
          path: item.path,
          filename: item.filename,
          customName: inputName,
          customType: typeSelect ? typeSelect.value : item.type,
          pakDestination: pakDestSelect ? pakDestSelect.value : null,
          existingModId: isUpdate ? item.existingModId : null,
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
          await installMod(item.path, item.customType, item.pakDestination, item.customName);
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

  let pakDestination: string | null = null;
  if (customType === 'pak' || customType === 'logicmods' || customType === 'hybrid') {
    const checked = document.querySelector('input[name="pak-dest"]:checked') as HTMLInputElement;
    pakDestination = checked ? checked.value : (customType === 'logicmods' ? 'logicmods' : '~mods');
  }

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
            executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state, pakDestination);
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

      // Prompt the user automatically
      showConfirm(`This mod requires missing dependencies: ${missingNames.join(' and ')}. Would you like to download and install them automatically now?`)
        .then(confirmed => {
          if (confirmed) {
            retryBtn.click();
          }
        });
    }
    return;
  }

  await executeModInstallation(logs, resultsList, statusEl, confirmBtn, cancelBtn, customType, customName, state, pakDestination);
}

async function executeModInstallation(
  logs: string[],
  resultsList: HTMLElement,
  statusEl: HTMLElement,
  confirmBtn: HTMLButtonElement,
  cancelBtn: HTMLButtonElement,
  customType: string,
  customName: string | null,
  state: any,
  pakDestination: string | null
) {

  logs.push(`<div style="color:#e0af68;">&gt; Extracting ZIP contents to temporary directory...</div>`);
  resultsList.innerHTML = logs.join('');

  try {
    logs.push(`<div style="color:#e0af68;">&gt; Copying files to destination folder...</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    const manifest = await buildInstallManifest(
      state.currentAnalysis.zipPath,
      state.currentSettings?.gamePath || '',
      pakDestination,
      customName
    );

    const versionInput = document.getElementById('mod-version-input') as HTMLInputElement | null;
    if (versionInput && versionInput.value.trim()) {
      manifest.version = versionInput.value.trim();
    }

    await installModWithManifest(manifest, state.currentAnalysis.zipPath);
    logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] Mod installed successfully!</div>`);



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
