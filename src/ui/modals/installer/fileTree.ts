import { getState } from '../../../state';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import type { FileTreeNode } from './types';

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
        const { inspectPakFileTree } = await import('../../../api');
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
            import('../uassetInspector').then(({ openUAssetInspectorModal }) => {
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
