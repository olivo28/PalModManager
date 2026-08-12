import { analyzeZip, checkModExistsCommand, updateModCommand, installMod, checkDependencies, installUe4ss, installPalschema, buildInstallManifest, installModWithManifest } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';

export let _pendingUpdateModId: string | null = null;
export let _pendingBatchPaths: string[] = [];

export function showInstallModal(): void {
  document.getElementById('install-modal')!.classList.add('visible');
}

export function closeInstallModal(): void {
  const modal = document.getElementById('install-modal');
  if (modal) modal.classList.remove('visible');
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

export function setModalStatus(status: string): void {
  const statusEl = document.getElementById('modal-status');
  if (statusEl) statusEl.textContent = status;
}

export function getCleanNameFromFilename(filename: string): string {
  const stem = filename.substring(0, filename.lastIndexOf('.')) || filename;
  const words = stem.split(/\s+/);

  let idIndex = -1;
  for (let i = words.length - 1; i >= 0; i--) {
    const word = words[i].replace(/[()]/g, '');
    if (/^\d+$/.test(word)) {
      const num = parseInt(word, 10);
      if (!(num >= 2020 && num <= 2038)) {
        idIndex = i;
        break;
      }
    }
  }

  let cleanWords = words;
  if (idIndex !== -1) {
    cleanWords = words.slice(0, idIndex);
  } else {
    for (let i = 0; i < words.length; i++) {
      const word = words[i];
      if (/^\d{4}-\d{2}-\d{2}/.test(word) || (word.includes('-') && word.length > 6 && /^\d/.test(word))) {
        cleanWords = words.slice(0, i);
        break;
      }
    }
  }

  const clean: string[] = [];
  for (const word of cleanWords) {
    const lower = word.toLowerCase().replace(/[()]/g, '');
    if (["gamepass", "steam", "gdk", "xbox", "singleplayer", "sp"].includes(lower)) {
      continue;
    }
    clean.push(word);
  }

  const result = clean.join(' ').trim();
  const finalResult = result.replace(/[-\s_]+$/, '').trim();
  return finalResult.length < 2 ? stem.trim() : finalResult;
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
          </div>
        `;
      }
    }).join('');
  }

  const rootNode = buildFileTree(routes);
  compactFileTree(rootNode);

  const container = document.createElement('div');
  container.className = 'modal-overlay visible';
  container.style.zIndex = '2500';
  container.style.position = 'fixed';
  container.style.top = '0';
  container.style.left = '0';
  container.style.right = '0';
  container.style.bottom = '0';
  container.style.background = 'rgba(0,0,0,0.6)';
  container.style.backdropFilter = 'blur(4px)';
  container.style.display = 'flex';
  container.style.alignItems = 'center';
  container.style.justifyContent = 'center';

  container.innerHTML = `
    <div class="modal" style="width: 600px; max-width: 90vw; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); display: flex; flex-direction: column; overflow: hidden;">
      <div class="modal-header" style="padding: 16px 20px; border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;">
        <h3 style="margin: 0; font-size: 16px; font-weight: 600; color: var(--text-primary);">${escapeHtml(modName)} Files</h3>
        <button id="filetree-modal-close-x" style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;">✕</button>
      </div>
      <div class="modal-body" style="padding: 20px; overflow-y: auto; max-height: 60vh; display: flex; flex-direction: column; gap: 8px;">
        ${renderFileTreeHTML(rootNode)}
      </div>
      <div class="modal-footer" style="padding: 12px 20px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end; flex-shrink: 0;">
        <button id="filetree-modal-close" class="btn-primary" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">Close</button>
      </div>
    </div>
  `;

  document.body.appendChild(container);

  const close = () => {
    document.body.removeChild(container);
  };
  container.querySelector('#filetree-modal-close-x')!.addEventListener('click', close);
  container.querySelector('#filetree-modal-close')!.addEventListener('click', close);

  container.querySelectorAll('.tree-folder-header').forEach(hdr => {
    hdr.addEventListener('click', () => {
      const parent = hdr.closest('.tree-folder-node')!;
      const children = parent.querySelector('.tree-folder-children') as HTMLElement;
      const chevron = hdr.querySelector('span')!;
      if (children.style.display === 'none') {
        children.style.display = 'flex';
        chevron.textContent = '📁';
      } else {
        children.style.display = 'none';
        chevron.textContent = '📁';
      }
    });
  });
}

export function renderInstallPreview(analysis: ZipAnalysis, existingMod: { id: string; name: string, version: string } | null = null): void {
  updateState({ currentAnalysis: analysis });
  _pendingUpdateModId = existingMod ? existingMod.id : null;

  const content = document.getElementById('modal-content')!;
  const title = document.getElementById('install-modal-title')!;
  const confirmBtn = document.getElementById('modal-confirm') as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;

  statusEl.textContent = '';
  confirmBtn.textContent = existingMod ? 'Update' : 'Install';

  const filename = analysis.zipPath.split(/[/\\]/).pop() || '';
  const cleanName = getCleanNameFromFilename(filename);

  title.textContent = existingMod ? `Update Mod: ${existingMod.name}` : 'Install New Mod';

  let alertHtml = '';
  if (existingMod) {
    alertHtml = `
      <div style="background:rgba(0,188,255,0.08); border:1px solid rgba(0,188,255,0.2); padding:10px 14px; border-radius:6px; font-size:11px; color:#cbeaff; line-height:1.4;">
        ⚠️ This action will update your existing mod <strong>"${escapeHtml(existingMod.name)}"</strong>. 
        Files will be extracted and merged over the old installation.
      </div>
    `;
  }

  let typeWarning = '';
  if (analysis.detectedType === 'unknown') {
    typeWarning = `
      <div style="background:rgba(255,74,74,0.08); border:1px solid rgba(255,74,74,0.2); padding:10px 14px; border-radius:6px; font-size:11px; color:#ffd4d4; line-height:1.4; margin-top:8px;">
        ⚠️ <strong>Warning:</strong> The installer could not automatically determine the mod type. 
        Please select the correct type manually below to prevent game crashes.
      </div>
    `;
  } else if (analysis.detectedType === 'logicmods') {
    typeWarning = `
      <div style="background:rgba(255,157,0,0.08); border:1px solid rgba(255,157,0,0.2); padding:10px 14px; border-radius:6px; font-size:11px; color:#ffe9d0; line-height:1.4; margin-top:8px;">
        💡 <strong>Note:</strong> This is a Logic Mod (blueprint scripting). Logic Mods are loaded sequentially by the game.
      </div>
    `;
  }

  const selectOptions = [
    { value: 'ue4ss', label: 'UE4SS Mod (Lua)' },
    { value: 'pak', label: 'Asset Mod (Pak)' },
    { value: 'logicmods', label: 'Logic Mod (Pak)' },
    { value: 'palschema', label: 'PalSchema Script' },
    { value: 'hybrid', label: 'Hybrid Mod (Multiple components)' }
  ];

  const selectOptionsHtml = selectOptions.map(o => {
    const isSelected = o.value === analysis.detectedType ? 'selected' : '';
    return `<option value="${o.value}" ${isSelected}>${o.label}</option>`;
  }).join('');

  const showPakOption = analysis.detectedType === 'pak' || analysis.detectedType === 'logicmods' || analysis.detectedType === 'hybrid';
  let pakDestHtml = '';
  if (showPakOption) {
    const defaultLogic = analysis.detectedType === 'logicmods' ? 'checked' : '';
    const defaultMods = analysis.detectedType !== 'logicmods' ? 'checked' : '';

    pakDestHtml = `
      <div style="display:flex; flex-direction:column; gap:6px;">
        <span style="font-weight:600; font-size:11px; color:var(--text-secondary);">Pak Destination</span>
        <div style="display:flex; gap:16px;">
          <label style="display:flex; align-items:center; gap:6px; font-size:12px; color:var(--text-primary); cursor:pointer;">
            <input type="radio" name="pak-dest" value="~mods" ${defaultMods} /> ~mods (Standard asset mods)
          </label>
          <label style="display:flex; align-items:center; gap:6px; font-size:12px; color:var(--text-primary); cursor:pointer;">
            <input type="radio" name="pak-dest" value="logicmods" ${defaultLogic} /> logicmods (Blueprint/script mods)
          </label>
        </div>
      </div>
    `;
  }

  content.innerHTML = `
    <div style="display:flex; flex-direction:column; gap:16px;">
      ${alertHtml}
      ${typeWarning}

      <div style="display:flex; flex-direction:column; gap:6px;">
        <span style="font-weight:600; font-size:11px; color:var(--text-secondary);">Archive File</span>
        <span style="font-family:monospace; font-size:11px; color:var(--text-primary); word-break:break-all;">${escapeHtml(filename)}</span>
      </div>

      <div style="display:flex; flex-direction:column; gap:6px;">
        <span style="font-weight:600; font-size:11px; color:var(--text-secondary);">Mod Name</span>
        <input type="text" id="mod-name-input" value="${escapeHtml(existingMod ? existingMod.name : cleanName)}" style="padding: 8px 12px; border:1px solid var(--border); background:var(--bg-secondary); color:var(--text-primary); border-radius:4px; font-size:13px; outline:none;" />
      </div>

      <div style="display:flex; gap:16px; align-items:flex-start;">
        <div style="display:flex; flex-direction:column; gap:6px; flex:1;">
          <span style="font-weight:600; font-size:11px; color:var(--text-secondary);">Detected Mod Type</span>
          <select id="mod-type-select" style="padding: 8px; border:1px solid var(--border); background:var(--bg-secondary); color:var(--text-primary); border-radius:4px; font-size:12px; outline:none; cursor:pointer;">
            ${selectOptionsHtml}
          </select>
        </div>
        
        <div style="display:flex; flex-direction:column; gap:6px;">
          <span style="font-weight:600; font-size:11px; color:var(--text-secondary);">&nbsp;</span>
          <button id="modal-view-files" class="btn-secondary" style="padding: 8px 14px; font-size:12px; border-radius:4px; cursor:pointer;">View Zip Contents (${analysis.files ? analysis.files.length : 0} files)</button>
        </div>
      </div>

      ${pakDestHtml}
    </div>
  `;

  const typeSelect = document.getElementById('mod-type-select') as HTMLSelectElement;
  typeSelect.addEventListener('change', () => {
    const val = typeSelect.value;
    const currentAnalysis = getState().currentAnalysis;
    if (currentAnalysis) {
      currentAnalysis.detectedType = val;
    }
    renderInstallPreview(getState().currentAnalysis!, existingMod);
  });

  const viewFilesBtn = document.getElementById('modal-view-files')!;
  viewFilesBtn.addEventListener('click', () => {
    showFileTreeModal(analysis.routes || [], cleanName);
  });
}

export function renderBatchInstallPreview(paths: string[]): void {
  _pendingBatchPaths = paths;
  showInstallModal();

  const content = document.getElementById('modal-content')!;
  const title = document.getElementById('install-modal-title')!;
  const confirmBtn = document.getElementById('modal-confirm') as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;

  statusEl.textContent = '';
  confirmBtn.textContent = 'Install All';
  title.textContent = `Batch Install (${paths.length} archives)`;

  const listItemsHtml = paths.map(p => {
    const filename = p.split(/[/\\]/).pop() || '';
    return `<div style="font-family:monospace; font-size:11px; padding:4px 6px; background:var(--bg-tertiary); border:1px solid var(--border); border-radius:4px; word-break:break-all;">📦 ${escapeHtml(filename)}</div>`;
  }).join('');

  content.innerHTML = `
    <div style="display:flex; flex-direction:column; gap:12px;">
      <div style="font-size:12px; color:var(--text-secondary);">
        You are installing <strong>${paths.length} mods</strong> in batch mode. 
        PalModManager will automatically analyze each zip and apply the best installation routing.
      </div>
      <div style="display:flex; flex-direction:column; gap:6px; max-height:200px; overflow-y:auto; padding:6px; border:1px dashed var(--border); border-radius:6px; background:rgba(0,0,0,0.1);">
        ${listItemsHtml}
      </div>
    </div>
  `;
}

export async function handleInstallConfirm(): Promise<void> {
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const cancelBtn = document.getElementById('modal-cancel')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;
  const contentEl = document.getElementById('modal-content')!;

  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  if (_pendingBatchPaths && _pendingBatchPaths.length > 0) {
    let installed = 0;
    let updated = 0;
    let failed = 0;

    const itemsToInstall: any[] = [];
    statusEl.textContent = 'Analyzing archives...';

    for (const path of _pendingBatchPaths) {
      try {
        const analysis = await analyzeZip(path);
        const check = await checkModExistsCommand(path);
        const existingModId = check.exists && check.modInfo ? check.modInfo.id : null;
        const filename = path.split(/[/\\]/).pop() || '';
        const customName = getCleanNameFromFilename(filename);

        itemsToInstall.push({
          path,
          filename,
          customName,
          customType: analysis.detectedType,
          existingModId,
          pakDestination: analysis.detectedType === 'logicmods' ? 'logicmods' : '~mods'
        });
      } catch (e) {
        failed++;
      }
    }

    const resultsHtml: string[] = [];
    contentEl.innerHTML = `
      <div class="install-console-header" style="display:flex;align-items:center;background:#181818;padding:6px 12px;border-top-left-radius:6px;border-top-right-radius:6px;border-bottom:1px solid #282828;">
        <span style="font-size:10px;font-family:monospace;color:#888;font-weight:600;">batch_install.sh</span>
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

    for (let i = 0; i < itemsToInstall.length; i++) {
      const item = itemsToInstall[i];
      statusEl.textContent = `Processing ${item.filename} (${i + 1}/${itemsToInstall.length})...`;

      resultsHtml.push(`<div class="batch-result-item" style="color:#e0af68;font-style:italic;">&gt; Extracting and copying files for ${escapeHtml(item.customName)}...</div>`);
      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;

      try {
        if (item.existingModId) {
          await updateModCommand(item.path, item.existingModId);
          updated++;
          resultsHtml.pop();
          resultsHtml.push(`<div class="batch-result-item success" style="color:#00bcff;font-weight:bold;"><span style="color:#777;">[UP]</span> Updated successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        } else {
          await installMod(item.path, item.customType, item.pakDestination, item.customName);
          installed++;
          resultsHtml.pop();
          resultsHtml.push(`<div class="batch-result-item success" style="color:#4af626;font-weight:bold;"><span style="color:#777;">[OK]</span> Installed successfully: ${escapeHtml(item.customName)} (${escapeHtml(item.customType)})</div>`);
        }
      } catch (e) {
        failed++;
        resultsHtml.pop();
        resultsHtml.push(`<div class="batch-result-item error" style="color:#ff4a4a;font-weight:bold;"><span style="color:#777;">[ERR]</span> Failed: ${escapeHtml(item.filename)} - ${escapeHtml(String(e))}</div>`);
      }

      resultsList.innerHTML = resultsHtml.join('');
      resultsList.scrollTop = resultsList.scrollHeight;
    }

    statusEl.textContent = `Batch complete: ${installed} installed, ${updated} updated, ${failed} failed`;
    cancelBtn.disabled = false;
    cancelBtn.textContent = 'Close';
    confirmBtn.style.display = 'none';

    const { loadMods } = await import('../modsView');
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

          const { loadDependencies } = await import('../modsView');
          await loadDependencies();

          logs.push(`<div style="color:#888;">&gt; Dependencies installed. Starting mod installation...</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;

          retryBtn.style.display = 'none';

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

    if (_pendingUpdateModId) {
      await updateModCommand(state.currentAnalysis.zipPath, _pendingUpdateModId);
    } else {
      await installModWithManifest(manifest, state.currentAnalysis.zipPath);
    }
    logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] Mod installed successfully!</div>`);

    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    statusEl.textContent = _pendingUpdateModId ? 'Updated successfully!' : 'Installed successfully!';
    setTimeout(async () => {
      closeInstallModal();
      const { loadMods } = await import('../modsView');
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

      let existingMod: { id: string; name: string, version: string } | null = null;
      try {
        const checkResult = await checkModExistsCommand(zipPath);
        if (checkResult.exists && checkResult.modInfo) {
          existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name, version: checkResult.modInfo.version };
        }
      } catch { }

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
