import { inspectModPakContents, type PakInspectionResult } from '../../api';
import { t } from '../../utils/i18n';
import { escapeHtml, formatBytes } from '../../utils/helpers';
import { setStatusBarMode } from './monaco/statusBar';

export interface PakExplorerOptions {
  modId: string;
  pakFileName: string;
  pakPath?: string | null;
  fileSize?: number;
  onInspectAsset: (assetInternalPath: string) => void;
}

export async function renderPakExplorer(
  container: HTMLElement,
  options: PakExplorerOptions
): Promise<void> {
  const { modId, pakFileName, pakPath, fileSize, onInspectAsset } = options;
  const cleanPakName = pakFileName.replace(/^\[Pak\]\s*/i, '');
  const fileSizeFormatted = fileSize ? formatBytes(fileSize) : '';

  container.className = 'editor-pak-explorer';
  container.innerHTML = `
    <div class="editor-pak-header">
      <div class="editor-pak-title-group">
        <span class="editor-pak-icon">📦</span>
        <div style="min-width:0;">
          <div class="editor-pak-title" title="${escapeHtml(cleanPakName)}">${escapeHtml(cleanPakName)}</div>
          <div class="editor-pak-meta">
            <span id="editor-pak-badge-count" class="editor-pak-badge">...</span>
            ${fileSizeFormatted ? `<span class="editor-pak-badge size">${escapeHtml(fileSizeFormatted)}</span>` : ''}
          </div>
        </div>
      </div>
      <div class="editor-pak-search-wrap">
        <span class="editor-pak-search-icon">🔍</span>
        <input type="text" id="editor-pak-filter-input" class="editor-pak-search-input" placeholder="${escapeHtml(t('editor.search_pak_assets'))}" autocomplete="off" spellcheck="false" />
      </div>
    </div>
    <div id="editor-pak-body-content" style="flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:0;padding:32px;">
      <div class="spinner" style="width:28px;height:28px;border:3px solid rgba(0,188,255,0.2);border-top-color:var(--accent);border-radius:50%;animation:spin 0.8s linear infinite;"></div>
      <span style="margin-top:12px;font-size:12px;color:var(--text-secondary);">${escapeHtml(t('common.loading'))}</span>
    </div>
  `;

  const bodyEl = container.querySelector('#editor-pak-body-content') as HTMLElement | null;
  const badgeCountEl = container.querySelector('#editor-pak-badge-count');
  const searchInput = container.querySelector('#editor-pak-filter-input') as HTMLInputElement | null;

  try {
    const pakResults: PakInspectionResult[] = await inspectModPakContents(modId);
    const cleanTarget = cleanPakName.toLowerCase().replace(/\\/g, '/').replace(/^.*[/\\]/, '');

    // Match either exact pak or best match
    const matched = pakResults.filter(p => {
      const pName = p.pakName.toLowerCase().replace(/\\/g, '/').replace(/^.*[/\\]/, '');
      return pName === cleanTarget || pName.includes(cleanTarget) || cleanTarget.includes(pName);
    });

    const activePak = matched.length > 0 ? matched[0] : pakResults[0];

    if (!activePak || !activePak.files || activePak.files.length === 0) {
      if (bodyEl) {
        bodyEl.innerHTML = `
          <div style="font-size:36px;margin-bottom:8px;opacity:0.6;">📦</div>
          <div style="font-size:13px;font-weight:600;color:var(--text-primary);margin-bottom:4px;">${escapeHtml(t('editor.no_content_available'))}</div>
          <div style="font-size:11px;color:var(--text-muted);">${escapeHtml(cleanPakName)}</div>
        `;
      }
      if (badgeCountEl) badgeCountEl.textContent = '0 assets';
      setStatusBarMode('binary', { assetCount: 0, size: fileSizeFormatted, lang: 'Pak' });
      return;
    }

    const totalFiles = activePak.files.length;
    if (badgeCountEl) {
      badgeCountEl.textContent = t('editor.pak_assets_count', { count: totalFiles }) || `${totalFiles} assets`;
    }

    setStatusBarMode('binary', {
      assetCount: totalFiles,
      size: fileSizeFormatted,
      lang: 'Pak',
    });

    // Compute category counts
    const counts: Record<string, number> = {
      all: totalFiles,
      blueprints: 0,
      textures: 0,
      materials: 0,
      tables: 0,
      other: 0,
    };

    activePak.files.forEach(f => {
      const lowerPath = f.path.toLowerCase();
      const assetTypeLower = (f.assetType || '').toLowerCase();
      if (assetTypeLower.includes('blueprint') || lowerPath.includes('/blueprint/')) {
        counts.blueprints++;
      } else if (assetTypeLower.includes('texture') || lowerPath.endsWith('.png') || lowerPath.endsWith('.dds')) {
        counts.textures++;
      } else if (assetTypeLower.includes('material')) {
        counts.materials++;
      } else if (assetTypeLower.includes('table') || assetTypeLower.includes('data')) {
        counts.tables++;
      } else {
        counts.other++;
      }
    });

    const chipDefinitions: Array<{ key: string; label: string; icon: string }> = [
      { key: 'all', label: t('editor.filter_all') || 'All', icon: '📦' },
      { key: 'blueprints', label: t('editor.filter_blueprints') || 'Blueprints', icon: '🔷' },
      { key: 'textures', label: t('editor.filter_textures') || 'Textures', icon: '🖼️' },
      { key: 'materials', label: t('editor.filter_materials') || 'Materials', icon: '🎨' },
      { key: 'tables', label: t('editor.filter_tables') || 'DataTables', icon: '📊' },
      { key: 'other', label: t('editor.filter_other') || 'Other', icon: '⚡' },
    ];

    const activeChips = chipDefinitions.filter(c => c.key === 'all' || (counts[c.key] && counts[c.key] > 0));

    if (bodyEl) {
      bodyEl.style.padding = '0';
      bodyEl.style.alignItems = 'stretch';
      bodyEl.style.justifyContent = 'flex-start';

      bodyEl.innerHTML = `
        <div class="editor-pak-container">
          <div class="editor-pak-card">
            <div class="editor-pak-card-toolbar">
              <div class="editor-pak-filter-chips" id="editor-pak-chips">
                ${activeChips.map(c => `
                  <button class="editor-pak-chip ${c.key === 'all' ? 'active' : ''}" data-cat="${c.key}">
                    <span>${c.icon}</span>
                    <span>${escapeHtml(c.label)}</span>
                    <span class="count">${counts[c.key] ?? 0}</span>
                  </button>
                `).join('')}
              </div>
            </div>

            <div class="editor-pak-table-wrap">
              <table class="editor-pak-table">
                <thead>
                  <tr>
                    <th style="width:62%;"><span style="margin-right:6px;">📁</span>${escapeHtml(t('editor.col_asset_path'))}</th>
                    <th style="width:18%;"><span style="margin-right:6px;">🏷️</span>${escapeHtml(t('editor.col_asset_type'))}</th>
                    <th style="width:20%;text-align:right;"><span style="margin-right:6px;">⚡</span>${escapeHtml(t('editor.col_asset_actions'))}</th>
                  </tr>
                </thead>
                <tbody id="editor-pak-tbody">
                  ${activePak.files.map(f => {
                    const lowerPath = f.path.toLowerCase();
                    const isInspectable = lowerPath.endsWith('.uasset') || lowerPath.endsWith('.uexp');
                    const isPayload = lowerPath.endsWith('.uexp') || lowerPath.endsWith('.ubulk') || lowerPath.endsWith('.uptnl');
                    const lastSlash = Math.max(f.path.lastIndexOf('/'), f.path.lastIndexOf('\\'));
                    const dirPart = lastSlash >= 0 ? f.path.substring(0, lastSlash + 1) : '';
                    const filePart = lastSlash >= 0 ? f.path.substring(lastSlash + 1) : f.path;

                    let catKey = 'other';
                    let pillClass = 'asset';
                    let pillIcon = '⚡';
                    const assetTypeLower = (f.assetType || '').toLowerCase();

                    if (assetTypeLower.includes('texture') || lowerPath.endsWith('.png') || lowerPath.endsWith('.dds')) {
                      pillClass = 'texture';
                      pillIcon = '🖼️';
                      catKey = 'textures';
                    } else if (assetTypeLower.includes('blueprint') || lowerPath.includes('/blueprint/')) {
                      pillClass = 'blueprint';
                      pillIcon = '🔷';
                      catKey = 'blueprints';
                    } else if (assetTypeLower.includes('material')) {
                      pillClass = 'material';
                      pillIcon = '🎨';
                      catKey = 'materials';
                    } else if (assetTypeLower.includes('table') || assetTypeLower.includes('data')) {
                      pillClass = 'datatable';
                      pillIcon = '📊';
                      catKey = 'tables';
                    }

                    return `
                      <tr class="editor-pak-row" data-search="${escapeHtml((f.name + ' ' + f.path + ' ' + f.assetType).toLowerCase())}" data-type="${catKey}">
                        <td class="editor-pak-cell path">
                          <div class="editor-pak-path-cell" title="${escapeHtml(f.path)}">
                            <span class="editor-pak-folder-icon">📁</span>
                            <span class="dir">${escapeHtml(dirPart)}</span>
                            <span class="file">${escapeHtml(filePart)}</span>
                            ${isPayload ? `<span class="editor-pak-companion-badge">${escapeHtml(t('editor.companion_payload') || 'Payload')}</span>` : ''}
                          </div>
                        </td>
                        <td class="editor-pak-cell type">
                          <span class="editor-pak-type-pill ${pillClass}">
                            <span>${pillIcon}</span>
                            <span>${escapeHtml(f.assetType)}</span>
                          </span>
                        </td>
                        <td class="editor-pak-cell actions" style="text-align:right;">
                          ${isInspectable ? `
                            <div class="editor-pak-btn-group">
                              <button class="editor-pak-btn-inspect" data-path="${escapeHtml(f.path)}" title="${escapeHtml(t('editor.btn_inspect_asset') || 'Inspect Asset')}">
                                <span>🔍</span> <span>${escapeHtml(t('editor.btn_inspect_asset') || 'Inspect')}</span>
                              </button>
                              <button class="editor-pak-btn-popout" data-path="${escapeHtml(f.path)}" title="${escapeHtml(t('editor.btn_open_modal') || 'Open in Window')}">
                                <span>⛶</span>
                              </button>
                            </div>
                          ` : ''}
                        </td>
                      </tr>
                    `;
                  }).join('')}
                </tbody>
              </table>
            </div>

            <div class="editor-pak-card-footer">
              <span id="editor-pak-footer-count" class="editor-pak-footer-text">
                ${escapeHtml(t('editor.footer_showing_assets', { count: totalFiles, total: totalFiles }) || `Showing ${totalFiles} of ${totalFiles} assets`)}
              </span>
              <span class="editor-pak-footer-meta">
                ${fileSizeFormatted ? `<span>${escapeHtml(fileSizeFormatted)}</span> • ` : ''}
                <span>Unreal Engine Pak</span>
              </span>
            </div>
          </div>
        </div>
      `;

      let activeCategory = 'all';
      const rows = bodyEl.querySelectorAll<HTMLTableRowElement>('.editor-pak-row');
      const footerCountEl = bodyEl.querySelector('#editor-pak-footer-count');

      const applyFilters = () => {
        const q = searchInput ? searchInput.value.trim().toLowerCase() : '';
        let visibleCount = 0;

        rows.forEach(row => {
          const searchData = row.dataset.search || '';
          const rowType = row.dataset.type || '';
          const matchesCategory = activeCategory === 'all' || rowType === activeCategory;
          const matchesQuery = !q || searchData.includes(q);
          const visible = matchesCategory && matchesQuery;

          row.style.display = visible ? '' : 'none';
          if (visible) visibleCount++;
        });

        if (badgeCountEl) {
          badgeCountEl.textContent = (q || activeCategory !== 'all')
            ? `${visibleCount} / ${totalFiles}`
            : (t('editor.pak_assets_count', { count: totalFiles }) || `${totalFiles} assets`);
        }

        if (footerCountEl) {
          footerCountEl.textContent = t('editor.footer_showing_assets', { count: visibleCount, total: totalFiles }) || `Showing ${visibleCount} of ${totalFiles} assets`;
        }
      };

      // Category chip click listeners
      bodyEl.querySelectorAll<HTMLButtonElement>('.editor-pak-chip').forEach(chip => {
        chip.addEventListener('click', () => {
          bodyEl.querySelectorAll('.editor-pak-chip').forEach(c => c.classList.remove('active'));
          chip.classList.add('active');
          activeCategory = chip.dataset.cat || 'all';
          applyFilters();
        });
      });

      // Live search input listener
      if (searchInput) {
        searchInput.addEventListener('input', applyFilters);
      }

      // Inline inspection button click
      bodyEl.querySelectorAll<HTMLButtonElement>('.editor-pak-btn-inspect').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          const assetPath = btn.dataset.path;
          if (assetPath) {
            onInspectAsset(assetPath);
          }
        });
      });

      // Popout to detached modal button click
      bodyEl.querySelectorAll<HTMLButtonElement>('.editor-pak-btn-popout').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          const assetPath = btn.dataset.path;
          if (assetPath) {
            import('../modals/uassetInspector').then(({ openUAssetInspectorModal }) => {
              openUAssetInspectorModal({
                modId: modId || null,
                pakPath: pakPath || null,
                assetInternalPath: assetPath,
              });
            });
          }
        });
      });
    }
  } catch (err) {
    if (bodyEl) {
      bodyEl.innerHTML = `
        <div style="padding:32px;text-align:center;color:var(--danger);font-size:12px;">
          ⚠️ ${escapeHtml(String(err))}
        </div>
      `;
    }
    setStatusBarMode('binary', { size: fileSizeFormatted, lang: 'Pak' });
  }
}
