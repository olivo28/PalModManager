import {
  analyzeZip,
  checkModExistsCommand,
  updateModCommand,
  installMod,
  checkDependencies,
  installUe4ss,
  installPalschema,
  buildInstallManifest,
  installModWithManifest,
  fetchNexusInfoAsync,
  previewConfigDiff,
  setModIgnoredKeys
} from '../../api';
import type { ZipAnalysis, InstallManifest } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';

export let _pendingUpdateModId: string | null = null;
export let _pendingBatchPaths: string[] = [];

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
  isLogicModsDefault?: boolean;
}

export let _batchItems: BatchItem[] = [];

export function showInstallModal(): void {
  document.getElementById('install-modal')!.classList.add('visible');
}

export function closeInstallModal(): void {
  const modal = document.getElementById('install-modal');
  if (modal) modal.classList.remove('visible');
  updateState({ currentAnalysis: null });
  _pendingUpdateModId = null;
  _pendingBatchPaths = [];
  _batchItems = [];

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

  // Restore modal size to default
  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '750px';
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

  // eslint-disable-next-line no-inner-declarations
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

export async function renderInstallPreview(analysis: ZipAnalysis, existingMod: { id: string; name: string, version: string } | null = null): Promise<void> {
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
    }).catch(() => { });
  }

  let cleanName = getCleanNameFromFilename(analysis.zipPath.split(/[/\\]/).pop() || '');
  if (!analysis.nexusInfo && analysis.modinfo?.name) {
    cleanName = analysis.modinfo.name;
  }

  let manifest: InstallManifest;
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
        <span class="update-banner-text" style="font-size:11px;font-weight:600;color:var(--text-primary);">${escapeHtml(existingMod.name)} already exists. (Installed: ${(existingMod.version && existingMod.version !== 'unknown') ? 'v' + existingMod.version : 'unknown version'}). Update it?</span>
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

           <div id="config-diff-container" style="display: none; border: 1px solid rgba(0, 188, 255, 0.25); background: rgba(0, 40, 60, 0.15); border-radius: 6px; padding: 8px 12px; margin-top: -4px; margin-bottom: 4px; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 14px;">⚙</span>
                <div style="display: flex; flex-direction: column; text-align: left;">
                   <span style="font-size: 11px; font-weight: bold; color: var(--text-primary);">Config Settings Merge</span>
                   <span id="config-diff-summary-text" style="font-size: 9px; color: var(--text-muted);">Differences detected in config files.</span>
                </div>
              </div>
              <button id="view-config-diff-btn" class="btn btn-secondary" style="font-size: 10px; padding: 4px 8px; height: auto; line-height: 1; margin: 0;">Show Details</button>
           </div>
           
           <div style="display:flex;gap:12px;">
             <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">Mod Display Name</label>
                <input type="text" id="mod-name-input" value="${escapeHtml(existingMod ? existingMod.name : cleanName)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:13px;font-weight:600;" />
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
  `;

  // Wire up Show Full List button
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

  if (existingMod) {
    previewConfigDiff(analysis.zipPath, existingMod.id).then(diffs => {
      const diffContainer = document.getElementById('config-diff-container');
      const summaryText = document.getElementById('config-diff-summary-text');
      const viewBtn = document.getElementById('view-config-diff-btn');
      if (diffContainer && diffs && diffs.length > 0) {
        diffContainer.style.display = 'flex';
        if (summaryText) {
          const filesCount = diffs.length;
          summaryText.textContent = `Differences detected in ${filesCount} config file${filesCount === 1 ? '' : 's'}.`;
        }
        if (viewBtn) {
          viewBtn.onclick = (e) => {
            e.preventDefault();
            showConfigDiffModal(diffs, existingMod.id);
          };
        }
      }
    }).catch(err => {
      console.error("Failed to preview config diffs:", err);
    });
  }

  confirmBtn.disabled = false;
}

export function showConfigDiffModal(diffs: any[], modId: string): void {
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay visible';
  overlay.id = 'config-diff-modal';

  const state = getState();
  const currentMod = state.allMods.find(m => m.id === modId);
  const currentIgnoredKeys = currentMod?.ignoredKeys || [];

  let html = `
    <div class="modal" style="max-width:850px; width:100%; max-height:85vh; display:flex; flex-direction:column; background:var(--bg-secondary); border:1px solid var(--border); border-radius:8px; box-shadow:0 12px 36px rgba(0,0,0,0.5);">
      <div class="modal-header" style="padding:16px 20px; border-bottom:1px solid var(--border); display:flex; align-items:center; justify-content:space-between;">
        <h3 style="margin:0; font-size:16px; font-weight:700; color:var(--text-primary);">⚙ Config Settings Merge Preview</h3>
        <button class="modal-close-btn" id="config-diff-modal-close-x" style="background:none; border:none; color:var(--text-muted); cursor:pointer; font-size:16px;">✕</button>
      </div>
      <div class="modal-body" style="flex:1; overflow-y:auto; padding:20px; display:flex; flex-direction:column; gap:16px; background:var(--bg-primary);">
  `;

  const collapseByDefault = diffs.length > 1;

  for (let i = 0; i < diffs.length; i++) {
    const diff = diffs[i];
    html += `
      <div class="config-diff-card" style="background:var(--bg-secondary); border:1px solid var(--border); border-radius:6px; padding:12px; display:flex; flex-direction:column; gap:4px;">
        <div class="config-diff-file-header" data-index="${i}" style="cursor:pointer; font-weight:700; font-family:monospace; font-size:12px; color:var(--text-primary); display:flex; align-items:center; justify-content:space-between; padding:2px 0; user-select:none; word-break:break-all;">
          <span>📄 ${escapeHtml(diff.file_name)}</span>
          <span class="toggle-icon" style="font-size:10px; color:var(--text-muted); padding-left:8px;">${collapseByDefault ? '▲' : '▼'}</span>
        </div>
        <div class="config-diff-file-content" id="config-diff-file-content-${i}" style="display: ${collapseByDefault ? 'none' : 'flex'}; flex-direction:column; gap:12px; margin-top:8px; border-top:1px solid rgba(255,255,255,0.03); padding-top:8px;">
    `;

    if (diff.keys_user_changed && diff.keys_user_changed.length > 0) {
      html += `
        <div>
          <div style="color:#ff9000; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🟢 Your changes to preserve</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,144,0,0.15); border:1px solid rgba(255,144,0,0.25);">${diff.keys_user_changed.length}</span>
          </div>
          <div style="overflow-x:auto; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:4px;">
            <table style="width:100%; border-collapse:collapse; font-size:10px; text-align:left; font-family:monospace;">
              <thead>
                <tr style="border-bottom:1px solid var(--border); color:var(--text-muted);">
                  <th style="padding:6px 8px; font-weight:bold; width: 60px;">Preserve</th>
                  <th style="padding:6px 8px; font-weight:bold;">Setting / Key</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">Your Value</th>
                  <th style="padding:6px 8px; font-weight:bold; width:150px; text-align:right;">Default Value</th>
                </tr>
              </thead>
              <tbody>
                ${diff.keys_user_changed.map((c: any) => {
        const isPreserved = !currentIgnoredKeys.includes(c.key);
        return `
                    <tr style="border-bottom:1px solid rgba(255,255,255,0.02); hover:background:rgba(255,255,255,0.01);">
                      <td style="padding:6px 8px; text-align:center;">
                        <input type="checkbox" class="preserve-key-switch" data-key="${escapeHtml(c.key)}" ${isPreserved ? 'checked' : ''} style="cursor:pointer;" />
                      </td>
                      <td style="padding:6px 8px; color:var(--text-primary); word-break:break-all;" title="${escapeHtml(c.key)}">${escapeHtml(c.key)}</td>
                      <td style="padding:6px 8px; color:#ff9000; font-weight:bold; text-align:right; word-break:break-all;">${escapeHtml(c.old_value)}</td>
                      <td style="padding:6px 8px; opacity:0.6; text-decoration:line-through; text-align:right; word-break:break-all;">${escapeHtml(c.new_value)}</td>
                    </tr>
                  `;
      }).join('')}
              </tbody>
            </table>
          </div>
        </div>
      `;
    }

    if (diff.keys_added_by_author && diff.keys_added_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#00bcff; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔵 New settings added by author</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(0,188,255,0.15); border:1px solid rgba(0,188,255,0.25);">${diff.keys_added_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_added_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(0,188,255,0.08); border:1px solid rgba(0,188,255,0.15); border-radius:4px; color:#00bcff; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    if (diff.keys_removed_by_author && diff.keys_removed_by_author.length > 0) {
      html += `
        <div>
          <div style="color:#ff5000; font-size:11px; font-weight:700; margin-bottom:6px; display:flex; align-items:center; gap:6px;">
            <span>🔴 Settings removed by author</span>
            <span style="font-size:9px; padding:1px 5px; border-radius:10px; background:rgba(255,80,0,0.15); border:1px solid rgba(255,80,0,0.25);">${diff.keys_removed_by_author.length}</span>
          </div>
          <div style="display:flex; flex-wrap:wrap; gap:6px; background:var(--bg-primary); border:1px solid var(--border); border-radius:4px; padding:10px;">
            ${diff.keys_removed_by_author.map((k: string) => `
              <span style="font-family:monospace; font-size:9px; padding:2px 6px; background:rgba(255,80,0,0.08); border:1px solid rgba(255,80,0,0.15); border-radius:4px; color:#ff5000; word-break:break-all;" title="${escapeHtml(k)}">${escapeHtml(k)}</span>
            `).join('')}
          </div>
        </div>
      `;
    }

    html += `
        </div>
      </div>
    `;
  }

  html += `
      </div>
      <div class="modal-footer" style="padding:14px 20px; border-top:1px solid var(--border); display:flex; justify-content:flex-end; background:var(--bg-secondary); border-bottom-left-radius:8px; border-bottom-right-radius:8px;">
        <button id="config-diff-modal-close-btn" class="btn btn-secondary">Close</button>
      </div>
    </div>
  `;

  overlay.innerHTML = html;
  document.body.appendChild(overlay);

  let localIgnoredKeys = [...currentIgnoredKeys];
  overlay.querySelectorAll('.preserve-key-switch').forEach(checkbox => {
    checkbox.addEventListener('change', (e) => {
      const target = e.target as HTMLInputElement;
      const key = target.dataset.key!;
      if (target.checked) {
        localIgnoredKeys = localIgnoredKeys.filter(k => k !== key);
      } else {
        if (!localIgnoredKeys.includes(key)) {
          localIgnoredKeys.push(key);
        }
      }

      setModIgnoredKeys(modId, localIgnoredKeys).then(updatedMod => {
        const modInState = state.allMods.find(m => m.id === modId);
        if (modInState) {
          modInState.ignoredKeys = localIgnoredKeys;
        }
      }).catch(err => {
        console.error("Failed to update ignored keys:", err);
      });
    });
  });

  overlay.querySelectorAll('.config-diff-file-header').forEach(header => {
    header.addEventListener('click', () => {
      const idx = (header as HTMLElement).dataset.index;
      const content = overlay.querySelector(`#config-diff-file-content-${idx}`) as HTMLElement;
      const icon = header.querySelector('.toggle-icon') as HTMLElement;
      if (content && icon) {
        if (content.style.display === 'none') {
          content.style.display = 'flex';
          icon.textContent = '▼';
        } else {
          content.style.display = 'none';
          icon.textContent = '▲';
        }
      }
    });
  });

  const close = () => {
    overlay.classList.remove('visible');
    setTimeout(() => overlay.remove(), 200);
  };

  overlay.querySelector('#config-diff-modal-close-x')!.addEventListener('click', close);
  overlay.querySelector('#config-diff-modal-close-btn')!.addEventListener('click', close);
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

  content.innerHTML = `
    <div style="display:flex;flex-direction:column;align-items:center;padding:24px;color:var(--text-secondary)">
      <div style="font-size:14px;font-weight:600;margin-bottom:8px">Analyzing ${paths.length} archives...</div>
      <div style="font-size:11px;color:var(--text-muted)">Scanning contents, checking versions and detecting types</div>
    </div>
  `;
  showInstallModal();

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
      } catch { }

      let nameVal = getCleanNameFromFilename(filename);
      if (!analysis.nexusInfo && analysis.modinfo?.name) {
        nameVal = analysis.modinfo.name;
      }

      const isLogicModsDefault = analysis.detectedType === 'logicmods' || (analysis.files && analysis.files.some((f: string) => f.toLowerCase().includes('logicmods')));
      const manifest = await buildInstallManifest(
        path,
        getState().currentSettings?.gamePath || '',
        isLogicModsDefault ? 'logicmods' : '~mods',
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
        isLogicModsDefault,
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
          <option value="~mods" ${!item.isLogicModsDefault ? 'selected' : ''}>~mods</option>
          <option value="logicmods" ${item.isLogicModsDefault ? 'selected' : ''}>logicmods</option>
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
                <option value="logicmods" ${val === 'logicmods' ? 'selected' : ''}>logicmods</option>
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

export async function handleInstallConfirm(): Promise<void> {
  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const cancelBtn = document.getElementById('modal-cancel')! as HTMLButtonElement;
  const statusEl = document.getElementById('modal-status')!;
  const contentEl = document.getElementById('modal-content')!;

  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  if (_pendingBatchPaths.length > 0) {
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
        const isUpdate = !!item.existingModId;

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
