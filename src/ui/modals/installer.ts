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
import { t } from '../../utils/i18n';

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

let _onInstallCompleteCallback: ((success: boolean) => void) | null = null;
let _lastInstallSuccess = false;

export function setInstallModalCallback(cb: ((success: boolean) => void) | null): void {
  _onInstallCompleteCallback = cb;
  _lastInstallSuccess = false;
}

export function showInstallModal(): void {
  const modal = document.getElementById('install-modal');
  if (modal) modal.classList.add('visible');
  const content = document.getElementById('modal-content');
  if (content) {
    content.innerHTML = `
      <div style="padding: 40px; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 180px; gap: 12px;">
        <div class="loading-spinner"></div>
        <span style="font-size: 12px; font-weight: 600; color: var(--text-secondary);">${escapeHtml(t('installer.status_analyzing'))}</span>
      </div>
    `;
  }
  const statusEl = document.getElementById('modal-status');
  if (statusEl) {
    statusEl.textContent = '';
  }
}

export function closeInstallModal(): void {
  const modal = document.getElementById('install-modal');
  if (modal) modal.classList.remove('visible');
  updateState({ currentAnalysis: null });
  _pendingUpdateModId = null;
  _pendingBatchPaths = [];
  _batchItems = [];

  const content = document.getElementById('modal-content');
  if (content) {
    content.innerHTML = '';
  }
  const statusEl = document.getElementById('modal-status');
  if (statusEl) {
    statusEl.textContent = '';
  }

  const retryBtn = document.getElementById('modal-install-deps-retry') as HTMLButtonElement | null;
  if (retryBtn) {
    retryBtn.style.display = 'none';
  }

  const confirmBtn = document.getElementById('modal-confirm')! as HTMLButtonElement;
  const cancelBtn = document.getElementById('modal-cancel')! as HTMLButtonElement;
  if (confirmBtn) {
    confirmBtn.style.display = '';
    confirmBtn.textContent = t('installer.btn_install');
    confirmBtn.disabled = false;
  }
  if (cancelBtn) {
    cancelBtn.textContent = t('common.cancel');
    cancelBtn.disabled = false;
  }

  // Restore modal size to default
  const modalEl = document.querySelector('#install-modal .modal') as HTMLElement | null;
  if (modalEl) {
    modalEl.style.width = '750px';
  }

  if (_onInstallCompleteCallback) {
    const cb = _onInstallCompleteCallback;
    _onInstallCompleteCallback = null;
    cb(_lastInstallSuccess);
  }
  _lastInstallSuccess = false;
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

export function showFileTreeModal(routes: any[], modName: string, zipPath?: string): void {
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

  // Helper to build badge for asset types
  function getAssetTypeBadge(type: string): string {
    let color = '#888';
    let bg = 'rgba(255,255,255,0.06)';
    let icon = '📄';

    switch (type) {
      case 'DataTable':
        color = '#00d2d3'; bg = 'rgba(0,210,211,0.15)'; icon = '📊'; break;
      case 'Blueprint':
        color = '#54a0ff'; bg = 'rgba(84,160,255,0.15)'; icon = '⚙️'; break;
      case 'Widget':
        color = '#5f27cd'; bg = 'rgba(95,39,205,0.15)'; icon = '🖥️'; break;
      case 'Material':
        color = '#ff9f43'; bg = 'rgba(255,159,67,0.15)'; icon = '🎨'; break;
      case 'Texture':
        color = '#10ac84'; bg = 'rgba(16,172,132,0.15)'; icon = '🖼️'; break;
      case 'Mesh':
        color = '#ee5253'; bg = 'rgba(238,82,83,0.15)'; icon = '📦'; break;
      case 'Audio':
        color = '#ff6b6b'; bg = 'rgba(255,107,107,0.15)'; icon = '🔊'; break;
      case 'Animation':
        color = '#feca57'; bg = 'rgba(254,202,87,0.15)'; icon = '🎬'; break;
      default:
        color = '#c8d6e5'; bg = 'rgba(200,214,229,0.1)'; icon = '📄'; break;
    }

    return `<span style="display:inline-flex; align-items:center; gap:3px; font-size:9px; font-weight:700; color:${color}; background:${bg}; border:1px solid ${color}33; border-radius:3px; padding:1px 5px; text-transform:uppercase;">${icon} ${type}</span>`;
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
              <span class="tree-folder-icon" style="color: #ffd166; font-size: 13px; display: flex; align-items: center;">📁</span>
              <span class="tree-folder-name" style="font-family: monospace;">${escapeHtml(child.name)}</span>
            </div>
            <div class="tree-folder-children" style="border-left: 1px dashed var(--border); margin-left: 7px; padding-left: 6px; display: flex; flex-direction: column; gap: 2px;">
              ${renderFileTreeHTML(child, depth + 1)}
            </div>
          </div>
        `;
      } else {
        const relativeDest = getRelativeDestPath(child.destPath || '', gamePath);
        const isPak = child.name.toLowerCase().endsWith('.pak');
        const pakId = `pak-expand-${Math.random().toString(36).substring(2, 9)}`;

        return `
          <div class="tree-file-node" data-search-text="${escapeHtml((child.name + ' ' + relativeDest).toLowerCase())}" style="margin-left: ${depth === 0 ? 0 : 12}px; display: flex; flex-direction: column; gap: 4px; padding: 6px 8px; border-radius: 4px; font-size: 11px; transition: background 0.2s; background: rgba(255,255,255,0.01); border: 1px solid var(--border);" onmouseover="this.style.background='rgba(255,255,255,0.03)'" onmouseout="this.style.background='rgba(255,255,255,0.01)'">
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="display: flex; align-items: center; gap: 8px; overflow: hidden; flex-grow: 1;">
                <span style="color: ${isPak ? 'var(--accent)' : 'var(--text-secondary)'}; font-size: 12px; display: flex; align-items: center;">${isPak ? '📦' : '📄'}</span>
                <div style="display: flex; flex-direction: column; overflow: hidden;">
                  <span class="tree-file-name" style="font-family: monospace; color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap; font-weight: 500;">${escapeHtml(child.name)}</span>
                  <span style="font-size: 9px; color: var(--text-muted); text-overflow: ellipsis; overflow: hidden; white-space: nowrap; font-family: monospace;" title="${escapeHtml(child.destPath || '')}">→ ${escapeHtml(relativeDest)}</span>
                </div>
              </div>
              <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                <span style="font-size: 8.5px; padding: 1px 4px; border-radius: 3px; background: var(--bg-secondary); color: var(--accent); border: 1px solid var(--border); text-transform: uppercase;">${escapeHtml(child.routeType || 'FILE')}</span>
                ${isPak ? `<button type="button" class="btn-tiny inspect-pak-btn" data-target="${pakId}" data-pak-name="${escapeHtml(child.name)}" data-pak-dest="${escapeHtml(child.destPath || '')}" style="font-size: 10px; padding: 2px 7px; background: var(--bg-card); border-color: var(--accent); color: var(--accent); cursor: pointer;" data-i18n="installer.btn_inspect_pak">🔍 ${escapeHtml(t('installer.btn_inspect_pak') || 'Inspect .pak')}</button>` : ''}
              </div>
            </div>
            ${isPak ? `
              <div id="${pakId}" class="pak-internal-container" style="display:none; margin-top: 6px; padding: 8px 10px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; flex-direction: column; gap: 6px;">
                <div class="pak-internal-loading" style="font-size: 10.5px; color: var(--text-muted); display: flex; align-items: center; gap: 6px;">
                  <span>⏳</span> <span>${escapeHtml(t('scanner.loading') || 'Loading pak index...')}</span>
                </div>
                <div class="pak-internal-content" style="display:none; flex-direction:column; gap:6px;"></div>
              </div>
            ` : ''}
          </div>
        `;
      }
    }).join('');
  }

  const rootNode = buildFileTree(routes);
  compactFileTree(rootNode);

  const container = document.createElement('div');
  container.id = 'file-tree-modal';
  container.className = 'modal-overlay visible';
  container.style.zIndex = '4500';
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
    <div class="modal" style="width: 720px; max-width: 92vw; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); display: flex; flex-direction: column; overflow: hidden; max-height: 85vh;">
      <div class="modal-header" style="padding: 14px 20px; border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0;">
        <div style="display: flex; align-items: center; gap: 8px;">
          <span style="font-size: 16px;">📂</span>
          <h3 style="margin: 0; font-size: 15px; font-weight: 600; color: var(--text-primary);">${escapeHtml(modName)} - ${escapeHtml(t('installer.btn_show_files'))}</h3>
        </div>
        <button id="filetree-modal-close-x" style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;">✕</button>
      </div>

      <!-- Quick Search Bar -->
      <div style="padding: 10px 20px; border-bottom: 1px solid var(--border); background: var(--bg-secondary); display: flex; align-items: center; gap: 10px;">
        <span style="font-size: 13px; color: var(--text-muted);">🔍</span>
        <input type="text" id="filetree-search-input" placeholder="${escapeHtml(t('scanner.search_placeholder') || 'Search files or assets...')}" style="flex: 1; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 5px 10px; font-size: 12px; outline: none;" />
        <span id="filetree-count-badge" class="badge" style="font-size: 10px; padding: 3px 8px; background: rgba(255,255,255,0.05); color: var(--text-muted); border: 1px solid var(--border);">${escapeHtml(t('installer.file_count_badge', { count: routes.length }) || `${routes.length} files`)}</span>
      </div>

      <div class="modal-body" id="filetree-body-container" style="padding: 16px 20px; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 6px;">
        ${renderFileTreeHTML(rootNode)}
      </div>

      <div class="modal-footer" style="padding: 12px 20px; border-top: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center; flex-shrink: 0; background: var(--bg-secondary);">
        <div style="font-size: 11px; color: var(--text-muted);">
          <span>💡 ${escapeHtml(t('installer.pak_inspect_hint') || 'Click "Inspect .pak" to see internal DataTables, Blueprints & Assets.')}</span>
        </div>
        <button id="filetree-modal-close" class="btn-primary" style="padding: 6px 14px; font-size: 12px; cursor: pointer; border-radius: 4px;">${escapeHtml(t('common.close'))}</button>
      </div>
    </div>
  `;

  document.body.appendChild(container);

  const close = () => {
    if (document.body.contains(container)) {
      document.body.removeChild(container);
    }
  };
  container.querySelector('#filetree-modal-close-x')!.addEventListener('click', close);
  container.querySelector('#filetree-modal-close')!.addEventListener('click', close);

  // Folder collapse toggle
  container.querySelectorAll('.tree-folder-header').forEach(hdr => {
    hdr.addEventListener('click', () => {
      const parent = hdr.closest('.tree-folder-node')!;
      const children = parent.querySelector('.tree-folder-children') as HTMLElement;
      const icon = hdr.querySelector('.tree-folder-icon') as HTMLElement;
      if (children.style.display === 'none') {
        children.style.display = 'flex';
        icon.textContent = '📁';
      } else {
        children.style.display = 'none';
        icon.textContent = '📁';
      }
    });
  });

  // Wire up Inspect .pak buttons
  container.querySelectorAll('.inspect-pak-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const targetId = (btn as HTMLElement).dataset.target!;
      const pakName = (btn as HTMLElement).dataset.pakName!;
      const pakDest = (btn as HTMLElement).dataset.pakDest || '';
      const containerEl = document.getElementById(targetId);
      if (!containerEl) return;

      if (containerEl.style.display === 'flex') {
        containerEl.style.display = 'none';
        btn.textContent = `🔍 ${t('installer.btn_inspect_pak') || 'Inspect .pak'}`;
        return;
      }

      containerEl.style.display = 'flex';
      btn.textContent = `🔼 ${t('common.hide') || 'Hide'}`;

      const loadingEl = containerEl.querySelector('.pak-internal-loading') as HTMLElement;
      const contentEl = containerEl.querySelector('.pak-internal-content') as HTMLElement;
      if (contentEl.children.length > 0) {
        // Already loaded
        loadingEl.style.display = 'none';
        contentEl.style.display = 'flex';
        return;
      }

      loadingEl.style.display = 'flex';
      contentEl.style.display = 'none';

      try {
        const { inspectPakFileTree } = await import('../../api');
        const inspectRes = await inspectPakFileTree(pakName, zipPath || undefined);

        loadingEl.style.display = 'none';
        contentEl.style.display = 'flex';

        // Build summary bar
        const summaryBadges = Object.entries(inspectRes.summaryByType || {})
          .map(([type, count]) => `${getAssetTypeBadge(type)} <span style="font-size:10px; color:var(--text-secondary); margin-right:6px;">×${count}</span>`)
          .join('');

        contentEl.innerHTML = `
          <div style="display:flex; justify-content:space-between; align-items:center; padding-bottom:6px; border-bottom:1px solid var(--border); flex-wrap:wrap; gap:6px;">
            <div style="display:flex; align-items:center; gap:6px; flex-wrap:wrap;">
              <span style="font-size:11px; font-weight:700; color:var(--text-primary);">📦 ${escapeHtml(inspectRes.pakName)}</span>
              <span class="badge" style="font-size:9.5px; padding:1px 6px; background:rgba(0,188,255,0.15); color:#00bcff; border:1px solid rgba(0,188,255,0.3); font-weight:600;">${escapeHtml(t('installer.internal_assets_count', { count: inspectRes.totalFiles }) || `${inspectRes.totalFiles} internal assets`)}</span>
            </div>
            <div style="display:flex; align-items:center; gap:4px; flex-wrap:wrap;">
              ${summaryBadges}
            </div>
          </div>
          <div class="pak-assets-list" style="display:flex; flex-direction:column; gap:2px; max-height:220px; overflow-y:auto; padding-right:4px;">
            ${inspectRes.files.map(f => `
              <div class="pak-asset-row" data-search-text="${escapeHtml((f.name + ' ' + f.path + ' ' + f.assetType).toLowerCase())}" style="display:flex; justify-content:space-between; align-items:center; padding:3px 6px; border-radius:3px; background:rgba(255,255,255,0.02); font-size:10.5px; font-family:monospace; transition:background 0.15s;" onmouseover="this.style.background='rgba(255,255,255,0.05)'" onmouseout="this.style.background='rgba(255,255,255,0.02)'">
                <span style="color:var(--text-primary); text-overflow:ellipsis; overflow:hidden; white-space:nowrap; max-width:68%;" title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span>
                <div style="display:flex; align-items:center; gap:5px;">
                  ${getAssetTypeBadge(f.assetType)}
                  ${f.path.toLowerCase().endsWith('.uasset') || f.path.toLowerCase().endsWith('.uexp') ? `
                    <button class="btn-inspect-uasset-installer" data-pak="${escapeHtml(pakName)}" data-path="${escapeHtml(f.path)}" title="${escapeHtml(t('scanner.uasset_btn_inspect') || 'Deep Inspect Asset')}" style="background:rgba(0,188,255,0.15); border:1px solid rgba(0,188,255,0.3); border-radius:3px; color:var(--accent); cursor:pointer; font-size:9.5px; padding:1px 5px; display:flex; align-items:center; gap:2px;">
                      <span>🔍</span>
                    </button>
                  ` : ''}
                </div>
              </div>
            `).join('')}
          </div>
        `;

        contentEl.querySelectorAll('.btn-inspect-uasset-installer').forEach(b => {
          b.addEventListener('click', (e) => {
            e.stopPropagation();
            const assetPath = (b as HTMLElement).dataset.path;
            const pak = (b as HTMLElement).dataset.pak;
            if (!assetPath) return;
            import('./uassetInspector').then(({ openUAssetInspectorModal }) => {
              openUAssetInspectorModal({
                pakPath: pak || null,
                assetInternalPath: assetPath,
                zipPath: zipPath || null,
              });
            });
          });
        });
      } catch (err: any) {
        loadingEl.style.display = 'none';
        contentEl.style.display = 'flex';
        contentEl.innerHTML = `<span style="font-size:11px; color:var(--danger);">⚠️ ${escapeHtml(String(err))}</span>`;
      }
    });
  });

  // Search filter handler
  const searchInput = container.querySelector('#filetree-search-input') as HTMLInputElement | null;
  if (searchInput) {
    searchInput.addEventListener('input', () => {
      const q = searchInput.value.trim().toLowerCase();
      const fileNodes = container.querySelectorAll('.tree-file-node');
      const assetRows = container.querySelectorAll('.pak-asset-row');

      fileNodes.forEach(node => {
        const text = (node as HTMLElement).dataset.searchText || '';
        const match = !q || text.includes(q);
        (node as HTMLElement).style.display = match ? 'flex' : 'none';
      });

      assetRows.forEach(row => {
        const text = (row as HTMLElement).dataset.searchText || '';
        const match = !q || text.includes(q);
        (row as HTMLElement).style.display = match ? 'flex' : 'none';
      });
    });
  }
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

  // Inherit existing mod metadata if available and missing on analysis
  if (existingMod && !analysis.nexusInfo) {
    const installed = getState().allMods.find(m => m.id === existingMod.id || m.name === existingMod.name);
    if (installed && (installed.nexusPictureUrl || installed.nexusAuthor || installed.nexusModId)) {
      if (!analysis.nexusModId && installed.nexusModId) {
        analysis.nexusModId = installed.nexusModId;
      }
      analysis.nexusInfo = {
        name: installed.name,
        author: installed.nexusAuthor || '',
        summary: installed.nexusSummary || '',
        pictureUrl: installed.nexusPictureUrl || '',
        version: installed.version || '',
        downloads: installed.nexusDownloads || 0,
        endorsements: installed.nexusEndorsements || 0,
      };
    }
  }

  // Background fetch of Nexus metadata for rich single-mod preview card
  if (analysis.nexusModId && !analysis.nexusInfo) {
    fetchNexusInfoAsync(analysis.nexusModId).then(info => {
      if (getState().currentAnalysis === analysis && info) {
        analysis.nexusInfo = info;
        renderInstallPreview(analysis, existingMod);
      }
    }).catch(() => { });
  }

  let cleanName = (analysis as any).preferredName || '';
  if (!cleanName && analysis.nexusInfo?.name) {
    cleanName = analysis.nexusInfo.name;
  }
  if (!cleanName && analysis.modinfo?.name) {
    cleanName = analysis.modinfo.name;
  }
  if (!cleanName) {
    const filename = analysis.zipPath.split(/[/\\]/).pop() || '';
    if (!filename.toLowerCase().startsWith('nexus_')) {
      cleanName = getCleanNameFromFilename(filename);
    }
  }

  let manifest: InstallManifest;
  try {
    manifest = await buildInstallManifest(
      analysis.zipPath,
      getState().currentSettings?.gamePath || '',
      analysis.detectedType === 'logicmods' ? 'logicmods' : '~mods',
      cleanName || null
    );
  } catch (err) {
    content.innerHTML = `<div style="padding:20px;color:#ff4a4a;font-weight:bold;">Error analyzing manifest: ${escapeHtml(String(err))}</div>`;
    return;
  }

  if (!cleanName || cleanName.toLowerCase().startsWith('nexus_') || /^[0-9a-fA-F-]{8,}$/.test(cleanName)) {
    if (manifest.folderName && manifest.folderName !== 'unknown' && !manifest.folderName.toLowerCase().startsWith('nexus_') && !/^[0-9a-fA-F-]{8,}$/.test(manifest.folderName)) {
      cleanName = manifest.folderName;
    } else if (analysis.nexusInfo?.name) {
      cleanName = analysis.nexusInfo.name;
    } else {
      const rawStem = analysis.zipPath.split(/[/\\]/).pop() || '';
      const candidate = getCleanNameFromFilename(rawStem);
      if (candidate && !/^[0-9a-fA-F-]{8,}$/.test(candidate) && !candidate.toLowerCase().startsWith('nexus_')) {
        cleanName = candidate;
      } else if (manifest.folderName && !/^[0-9a-fA-F-]{8,}$/.test(manifest.folderName)) {
        cleanName = manifest.folderName;
      }
    }
  }

  confirmBtn.disabled = false;
  statusEl.textContent = '';

  // Update banner if existing mod found
  let updateHtml = '';
  if (existingMod) {
    _pendingUpdateModId = existingMod.id;
    const existingVerStr = existingMod.version && existingMod.version !== 'unknown' ? `v${existingMod.version}` : '';
    const normPath = analysis.zipPath.replace(/\\/g, '/').toLowerCase();
    const isFromLibrary = normPath.includes('/pmm_library/') || normPath.includes('/library/') || normPath.includes('mods-library');
    const isFromNexusDownload = normPath.includes('nexus_') || normPath.includes('temp') || normPath.includes('palmodmanager_');

    let sourceBadge = '';
    if (isFromLibrary) {
      sourceBadge = `<span class="update-source-badge local" style="background:rgba(46, 204, 113, 0.15);color:#2ecc71;border:1px solid rgba(46, 204, 113, 0.35);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">📦 ${escapeHtml(t('installer.source_local_library'))}</span>`;
    } else if (isFromNexusDownload || analysis.nexusModId) {
      sourceBadge = `<span class="update-source-badge remote" style="background:rgba(0, 188, 255, 0.15);color:#00bcff;border:1px solid rgba(0, 188, 255, 0.35);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">⚡ ${escapeHtml(t('installer.source_nexus_download'))}</span>`;
    } else {
      sourceBadge = `<span class="update-source-badge custom" style="background:rgba(255, 255, 255, 0.08);color:var(--text-muted);border:1px solid var(--border);font-size:9px;font-weight:700;padding:2px 6px;border-radius:4px;display:inline-flex;align-items:center;gap:4px;text-transform:uppercase;letter-spacing:0.4px;">📁 ${escapeHtml(t('installer.source_custom_file'))}</span>`;
    }

    updateHtml = `
      <div class="update-banner" id="update-banner" style="margin-bottom:12px;padding:8px 12px;background:rgba(0,188,255,0.08);border:1px solid rgba(0,188,255,0.25);border-radius:6px;display:flex;align-items:center;justify-content:space-between;gap:10px;">
        <div style="display:flex;flex-direction:column;gap:4px;flex:1;overflow:hidden;">
          <span class="update-banner-text" style="font-size:11px;font-weight:600;color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="${escapeHtml(t('installer.already_exists', { name: existingMod.name, version: existingVerStr }))}">${escapeHtml(t('installer.already_exists', { name: existingMod.name, version: existingVerStr }))}</span>
          <div>${sourceBadge}</div>
        </div>
        <div style="display:flex;gap:4px;background:var(--bg-primary);padding:2px;border-radius:5px;border:1px solid var(--border);flex-shrink:0;">
          <button class="update-mode-btn" id="update-mode-btn" type="button" style="padding:4px 8px;background:#00bcff;color:#fff;border:none;border-radius:3px;font-size:11px;font-weight:600;cursor:pointer;transition:all 0.15s ease;">${escapeHtml(t('installer.mode_update'))}</button>
          <button class="update-mode-btn" id="install-new-mode-btn" type="button" style="padding:4px 8px;background:transparent;color:var(--text-secondary);border:none;border-radius:3px;font-size:11px;font-weight:600;cursor:pointer;transition:all 0.15s ease;">${escapeHtml(t('installer.mode_new'))}</button>
        </div>
      </div>
    `;
    confirmBtn.textContent = t('installer.btn_update');
  } else {
    _pendingUpdateModId = null;
    confirmBtn.textContent = t('installer.btn_install');
  }

  const picUrl = analysis.nexusInfo?.pictureUrl || (analysis.nexusInfo as any)?.picture_url || '';
  let versionVal = analysis.modinfo?.version || '';
  if (!versionVal && analysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(analysis.detectedVersion.trim()) && analysis.detectedVersion.trim() !== '1.0.0' && analysis.detectedVersion.trim() !== 'unknown') {
    versionVal = analysis.detectedVersion;
  }
  if (!versionVal && analysis.nexusInfo?.version && analysis.nexusInfo.version !== '1.0.0' && analysis.nexusInfo.version !== 'unknown') {
    versionVal = analysis.nexusInfo.version;
  }
  if (!versionVal && analysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(analysis.detectedVersion.trim())) {
    versionVal = analysis.detectedVersion;
  }
  if (!versionVal) {
    versionVal = '1.0.0';
  }


  const displayType = manifest.modType === 'hybrid' ? `Hybrid (${[manifest.hasUe4ss ? 'UE4SS' : '', manifest.hasPalschema ? 'PalSchema' : '', manifest.hasPak ? 'Pak' : ''].filter(Boolean).join(' + ')})` : manifest.modType.toUpperCase();

  const isLogicModsDefault = manifest.modType === 'logicmods' || manifest.routes.some((r: any) => r.routeType === 'logicmods');

  let pakDestHtml = `
    <div class="pak-dest-section" id="single-pak-dest-section" style="display: ${manifest.hasPak ? 'block' : 'none'}; margin-top:8px;">
      <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;display:block;margin-bottom:6px;">${escapeHtml(t('installer.pak_dest_title'))}</label>
      <div class="pak-dest-options" style="display:flex;gap:12px;">
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="~mods" ${isLogicModsDefault ? '' : 'checked'} />
          <span>${escapeHtml(t('installer.pak_dest_res'))}</span>
        </label>
        <label class="pak-dest-option" style="display:flex;align-items:center;gap:6px;font-size:12px;cursor:pointer;">
          <input type="radio" name="pak-dest" value="logicmods" ${isLogicModsDefault ? 'checked' : ''} />
          <span>${escapeHtml(t('installer.pak_dest_logic'))}</span>
        </label>
      </div>
    </div>
  `;

  content.innerHTML = `
    <div style="display:flex;gap:18px;align-items:flex-start;padding:2px 0;">
       <!-- Left Column: Card Preview (Nexus Info or Local Modinfo) -->
       ${analysis.nexusInfo ? `
       <div style="width:230px;min-width:230px;max-width:230px;flex-shrink:0;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:120px;overflow:hidden;background:#000;">
             ${picUrl ? `<img src="${escapeHtml(picUrl)}" data-original-src="${escapeHtml(picUrl)}" style="width:100%;height:100%;object-fit:cover;opacity:0.85;" alt="" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this) : (this.onerror=null, this.style.display='none', this.nextElementSibling && (this.nextElementSibling.style.display='flex'));" /><div style="display:none;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">N</div>` : `<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--text-muted);font-weight:bold;font-size:32px;">N</div>`}
             <div style="position:absolute;bottom:6px;right:6px;background:rgba(0,0,0,0.75);padding:2px 7px;border-radius:10px;font-size:9px;color:#00ffcc;font-weight:700;letter-spacing:0.5px;">
                ${analysis.nexusInfo.downloads.toLocaleString()} DLs
             </div>
          </div>
          <div style="padding:10px;display:flex;flex-direction:column;gap:5px;">
             <div style="font-size:12.5px;font-weight:700;color:var(--text-primary);line-height:1.3;word-break:break-word;">${escapeHtml(analysis.nexusInfo.name)}</div>
             <div style="font-size:9.5px;color:var(--text-muted)">${escapeHtml(t('installer.by_author', { author: analysis.nexusInfo.author }))}</div>
             <div style="font-size:10.5px;color:var(--text-secondary);line-height:1.4;margin-top:2px;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.nexusInfo.summary)}</div>
          </div>
       </div>
       ` : (analysis.modinfo ? `
       <div style="width:230px;min-width:230px;max-width:230px;flex-shrink:0;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;overflow:hidden;display:flex;flex-direction:column;box-shadow:0 4px 15px rgba(0,0,0,0.35);">
          <div style="position:relative;width:100%;height:120px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;border-bottom:1px solid var(--border);">
             <div style="font-size:38px;color:var(--accent);">🛠</div>
             <div style="position:absolute;bottom:6px;right:6px;background:rgba(0,0,0,0.75);padding:2px 7px;border-radius:10px;font-size:9px;color:var(--accent);font-weight:700;letter-spacing:0.5px;text-transform:uppercase;">
                ${escapeHtml(t('installer.local_package'))}
             </div>
          </div>
          <div style="padding:10px;display:flex;flex-direction:column;gap:5px;">
             <div style="font-size:12.5px;font-weight:700;color:var(--text-primary);line-height:1.3;word-break:break-word;">${escapeHtml(analysis.modinfo.name || cleanName)}</div>
             <div style="font-size:9.5px;color:var(--text-muted)">${escapeHtml(t('installer.by_author', { author: analysis.modinfo.author || t('common.unknown') }))}</div>
             <div style="font-size:10.5px;color:var(--text-secondary);line-height:1.4;margin-top:2px;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden;">${escapeHtml(analysis.modinfo.description || t('installer.no_description'))}</div>
          </div>
       </div>
       ` : '')}

        <!-- Right Column: Settings Form -->
        <div style="flex:1;min-width:0;display:flex;flex-direction:column;gap:10px;">
           ${updateHtml}

           <div id="config-diff-container" style="display: none; border: 1px solid rgba(0, 188, 255, 0.25); background: rgba(0, 40, 60, 0.15); border-radius: 6px; padding: 6px 10px; margin-top: -2px; margin-bottom: 2px; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 14px;">⚙</span>
                <div style="display: flex; flex-direction: column; text-align: left;">
                   <span style="font-size: 11px; font-weight: bold; color: var(--text-primary);">${escapeHtml(t('installer.config_merge_title'))}</span>
                   <span id="config-diff-summary-text" style="font-size: 9px; color: var(--text-muted);">${escapeHtml(t('installer.config_merge_desc'))}</span>
                </div>
              </div>
              <button id="view-config-diff-btn" class="btn btn-secondary" style="font-size: 10px; padding: 4px 8px; height: auto; line-height: 1; margin: 0;">${escapeHtml(t('installer.btn_show_details'))}</button>
           </div>
           
           <div style="display:flex;gap:12px;">
             <div style="flex:1;display:flex;flex-direction:column;gap:4px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_mod_name'))}</label>
                <input type="text" id="mod-name-input" value="${escapeHtml(existingMod ? existingMod.name : cleanName)}" style="width:100%;padding:7px 10px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;" />
             </div>
             <div style="flex:1;display:flex;flex-direction:column;gap:4px;">
                <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_folder_name'))}</label>
                <input type="text" id="mod-folder-name-input" value="${escapeHtml(manifest.folderName)}" disabled style="width:100%;padding:7px 10px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;" />
             </div>
           </div>

           <div style="display:flex;gap:12px;">
              <div style="flex:1;display:flex;flex-direction:column;gap:6px;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_detected_type'))}</label>
                 <input type="text" value="${escapeHtml(displayType)}" disabled style="width:100%;padding:8px 12px;background:var(--bg-primary);color:var(--text-muted);border:1px solid var(--border);border-radius:4px;font-size:12px;font-weight:600;cursor:not-allowed;" />
              </div>
              <div style="width:120px;display:flex;flex-direction:column;gap:6px;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_version'))}</label>
                 <input type="text" id="mod-version-input" value="${escapeHtml(versionVal)}" style="width:100%;padding:8px 12px;background:var(--bg-secondary);color:var(--text-primary);border:1px solid var(--border);border-radius:4px;font-size:12px;text-align:center;" />
              </div>
           </div>

           <div style="display:flex;flex-direction:column;gap:6px;">
              <div style="display:flex;justify-content:space-between;align-items:center;">
                 <label style="font-size:11px;font-weight:700;color:var(--text-secondary);text-transform:uppercase;letter-spacing:0.5px;">${escapeHtml(t('installer.lbl_files_to_install'))}</label>
                 <button id="view-all-files-btn" class="btn btn-secondary" style="font-size:10px;padding:2px 6px;height:auto;line-height:1;margin:0;">${escapeHtml(t('installer.btn_show_full_list'))}</button>
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
      showFileTreeModal(manifest.routes, cleanName, analysis.zipPath);
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

  // Wire up update/install-new mode toggle buttons
  const updateModeBtn = document.getElementById('update-mode-btn') as HTMLButtonElement | null;
  const installNewModeBtn = document.getElementById('install-new-mode-btn') as HTMLButtonElement | null;
  const folderInput = document.getElementById('mod-folder-name-input') as HTMLInputElement | null;

  function setInstallMode(isUpdate: boolean) {
    if (!existingMod) return;
    if (isUpdate) {
      _pendingUpdateModId = existingMod.id;
      confirmBtn.textContent = t('installer.btn_update');
      if (updateModeBtn) {
        updateModeBtn.style.background = '#00bcff';
        updateModeBtn.style.color = '#fff';
      }
      if (installNewModeBtn) {
        installNewModeBtn.style.background = 'transparent';
        installNewModeBtn.style.color = 'var(--text-secondary)';
      }
      if (folderInput) {
        folderInput.value = manifest.folderName;
        folderInput.disabled = true;
        folderInput.style.cursor = 'not-allowed';
      }
    } else {
      _pendingUpdateModId = null;
      confirmBtn.textContent = t('installer.btn_install');
      if (updateModeBtn) {
        updateModeBtn.style.background = 'transparent';
        updateModeBtn.style.color = 'var(--text-secondary)';
      }
      if (installNewModeBtn) {
        installNewModeBtn.style.background = '#00bcff';
        installNewModeBtn.style.color = '#fff';
      }
      if (folderInput) {
        folderInput.disabled = false;
        folderInput.style.cursor = 'text';
        if (folderInput.value === manifest.folderName) {
          folderInput.value = `${manifest.folderName}_New`;
        }
      }
    }
  }

  if (updateModeBtn) {
    updateModeBtn.addEventListener('click', (e) => {
      e.preventDefault();
      setInstallMode(true);
    });
  }
  if (installNewModeBtn) {
    installNewModeBtn.addEventListener('click', (e) => {
      e.preventDefault();
      setInstallMode(false);
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
  overlay.style.zIndex = '4500';

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
        <button id="config-diff-modal-close-btn" class="btn btn-secondary">${escapeHtml(t('common.close'))}</button>
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
  confirmBtn.textContent = t('installer.btn_install');
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
        <button id="batch-view-files-${idx}" style="padding:2px 6px;background:var(--bg-secondary);color:var(--accent);border:1px solid var(--border);border-radius:4px;font-size:9px;cursor:pointer;white-space:nowrap;font-weight:600;" onmouseover="this.style.background='rgba(255,255,255,0.05)'" onmouseout="this.style.background='var(--bg-secondary)'">${escapeHtml(t('installer.btn_show_files'))}</button>
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
            <option value="hybrid" ${item.type === 'hybrid' ? 'selected' : ''}>${escapeHtml(t('card.type_hybrid'))}</option>
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
            <th style="padding:6px;width:28px;">${escapeHtml(t('installer.batch_col_install'))}</th>
            <th style="padding:6px;width:180px;">${escapeHtml(t('installer.batch_col_archive'))}</th>
            <th style="padding:6px;width:70px;">${escapeHtml(t('installer.batch_col_nexus_id'))}</th>
            <th style="padding:6px;width:60px;">${escapeHtml(t('installer.batch_col_version'))}</th>
            <th style="padding:6px;">${escapeHtml(t('installer.batch_col_target_folder'))}</th>
            <th style="padding:6px;width:90px;">${escapeHtml(t('installer.batch_col_type'))}</th>
            <th style="padding:6px;width:95px;">${escapeHtml(t('installer.batch_col_pak_target'))}</th>
            <th style="padding:6px;width:50px;text-align:right;padding-right:12px;">${escapeHtml(t('installer.batch_col_status'))}</th>
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
        viewBtn.textContent = t('installer.btn_loading_files');
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
          showFileTreeModal(manifest.routes, customName, item.path);
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        } finally {
          viewBtn.textContent = t('installer.btn_show_files');
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

    statusEl.textContent = t('installer.status_batch_complete', { installed, updated, failed });
    cancelBtn.disabled = false;
    cancelBtn.textContent = t('common.close');
    confirmBtn.style.display = 'none';

    const { loadMods, loadLibrary } = await import('../modsView');
    await loadMods();
    await loadLibrary();
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
  confirmBtn.textContent = _pendingUpdateModId ? t('installer.status_updating_btn') : t('installer.status_installing_btn');
  statusEl.textContent = _pendingUpdateModId ? t('installer.status_updating') : t('installer.status_installing');
  confirmBtn.disabled = true;
  cancelBtn.disabled = true;

  const logs: string[] = [];
  const resultsList = contentEl.querySelector('.batch-results-list') as HTMLElement;
  if (resultsList) {
    resultsList.innerHTML = logs.join('');
  }

  const depStatus = await checkDependencies();
  const ue4ssRequired = ['ue4ss', 'palschema', 'hybrid'].includes(customType);
  const palschemaRequired = (customType === 'palschema') || (customType === 'hybrid' && (state.currentAnalysis.hasPalSchemaJson || (state.currentAnalysis.files || []).some((f: string) => f.toLowerCase().includes('palschema'))));

  const missingUe4ss = ue4ssRequired && !depStatus.ue4ss_installed;
  const missingPalSchema = palschemaRequired && !depStatus.palschema_installed;

  if (missingUe4ss || missingPalSchema) {
    const missingNames: string[] = [];
    if (missingUe4ss) missingNames.push('UE4SS');
    if (missingPalSchema) missingNames.push('PalSchema');

    logs.push(`<div style="color:#ff9d00;font-weight:bold;">[WARN] Missing required dependencies: ${missingNames.join(', ')}</div>`);
    logs.push(`<div style="color:#888;">&gt; Please click "Install Deps & Retry" to install them automatically.</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = t('installer.status_missing_deps');

    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? t('installer.btn_update') : t('installer.btn_install');

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
            statusEl.textContent = t('installer.status_downloading_ue4ss');
            await installUe4ss();
            logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] UE4SS installed successfully!</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
          }
          if (missingPalSchema) {
            logs.push(`<div style="color:#e0af68;">&gt; Downloading and installing PalSchema dependency...</div>`);
            resultsList.innerHTML = logs.join('');
            resultsList.scrollTop = resultsList.scrollHeight;
            statusEl.textContent = t('installer.status_downloading_palschema');
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
          logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] ${escapeHtml(t('installer.status_deps_failed'))}: ${escapeHtml(String(err))}</div>`);
          resultsList.innerHTML = logs.join('');
          resultsList.scrollTop = resultsList.scrollHeight;
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
          statusEl.textContent = t('installer.status_deps_failed');
        } finally {
          retryBtn.disabled = false;
        }
      };

      showConfirm(t('installer.confirm_install_missing_deps', { deps: missingNames.join(' & ') }))
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

    if (customName) {
      manifest.displayName = customName;
    }
    const versionInput = document.getElementById('mod-version-input') as HTMLInputElement | null;
    const inputVer = versionInput?.value.trim();
    if (inputVer && !/^[0-9a-fA-F-]{6,}$/.test(inputVer)) {
      manifest.version = inputVer;
    } else if (state.currentAnalysis.modinfo?.version) {
      manifest.version = state.currentAnalysis.modinfo.version;
    } else if (state.currentAnalysis.detectedVersion && !/^[0-9a-fA-F-]{6,}$/.test(state.currentAnalysis.detectedVersion)) {
      manifest.version = state.currentAnalysis.detectedVersion;
    } else if (state.currentAnalysis.nexusInfo?.version) {
      manifest.version = state.currentAnalysis.nexusInfo.version;
    } else {
      manifest.version = '1.0.0';
    }

    if (state.currentAnalysis.nexusModId) {
      manifest.nexusModId = state.currentAnalysis.nexusModId;
    }

    if (_pendingUpdateModId) {
      await updateModCommand(state.currentAnalysis.zipPath, _pendingUpdateModId);
    } else {
      await installModWithManifest(manifest, state.currentAnalysis.zipPath);
    }
    _lastInstallSuccess = true;

    logs.push(`<div style="color:#4af626;font-weight:bold;">[OK] Mod installed successfully!</div>`);

    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;

    statusEl.textContent = _pendingUpdateModId ? 'Updated successfully!' : 'Installed successfully!';
    setTimeout(async () => {
      closeInstallModal();
      const { loadMods, loadLibrary } = await import('../modsView');
      await loadMods();
      await loadLibrary();
    }, 1500);
  } catch (e) {
    _lastInstallSuccess = false;
    logs.push(`<div style="color:#ff4a4a;font-weight:bold;">[ERR] Installation failed: ${escapeHtml(String(e))}</div>`);
    resultsList.innerHTML = logs.join('');
    resultsList.scrollTop = resultsList.scrollHeight;
    statusEl.textContent = 'Installation failed';
    confirmBtn.disabled = false;
    cancelBtn.disabled = false;
    confirmBtn.textContent = _pendingUpdateModId ? t('installer.btn_update') : t('installer.btn_install');
  }
}

export async function openInstallModalForZip(
  zipPath: string,
  preferredName?: string,
  preferredNexusId?: number,
  preferredVersion?: string
): Promise<void> {
  _lastInstallSuccess = false;

  // Dismiss Discovery modal cleanly if currently open
  const discModal = document.getElementById('discovery-mod-modal');
  if (discModal && discModal.classList.contains('visible')) {
    const { closeDiscoveryModal } = await import('../discoveryView');
    closeDiscoveryModal();
  }

  showInstallModal();
  setModalStatus(t('installer.status_analyzing'));

  try {
    const analysis = await analyzeZip(zipPath);
    if (preferredNexusId) {
      analysis.nexusModId = preferredNexusId;
    }
    if (preferredName) {
      (analysis as any).preferredName = preferredName;
    }
    if (preferredVersion) {
      analysis.detectedVersion = preferredVersion;
    }

    let existingMod: { id: string; name: string, version: string } | null = null;
    try {
      const checkResult = await checkModExistsCommand(zipPath);
      if (checkResult.exists && checkResult.modInfo) {
        existingMod = { id: checkResult.modInfo.id, name: checkResult.modInfo.name, version: checkResult.modInfo.version };
      }
    } catch { }

    renderInstallPreview(analysis, existingMod);
  } catch (err: any) {
    console.error('[Installer] analyzeZip error:', err);
    closeInstallModal();
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    throw err;
  }
}


export async function handleInstall(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Mod Archives', extensions: ['zip', 'rar', '7z'] }],
      title: t('installer.dialog_select_archive_title'),
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];

    if (paths.length === 1) {
      await openInstallModalForZip(paths[0]);
    } else {
      renderBatchInstallPreview(paths);
    }
  } catch (e) {
    console.error('Error analyzing:', e);
    closeInstallModal();
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

