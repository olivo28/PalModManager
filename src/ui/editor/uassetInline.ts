import { inspectUAssetDeep, decodeUAssetTexture, type UAssetInspectionDetails, type TexturePreviewInfo } from '../../api';
import { t } from '../../utils/i18n';
import { escapeHtml, formatBytes } from '../../utils/helpers';
import { showToast } from '../toast';
import { renderDataTableGridHtml, setupDataTableGridEvents } from './uasset/datatableGrid';
import { renderPropertyTweakerHtml, setupPropertyTweakerEvents } from './uasset/propertyTweaker';

export interface InlineUAssetOptions {
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  onBack?: () => void;
}

export async function renderInlineUAssetInspector(
  container: HTMLElement,
  options: InlineUAssetOptions
): Promise<void> {
  const { modId, pakPath, assetInternalPath, onBack } = options;

  container.className = 'editor-inline-uasset';
  container.innerHTML = `
    <div class="editor-inline-uasset-header">
      <div style="display:flex;align-items:center;gap:12px;min-width:0;">
        ${onBack ? `
          <button id="btn-editor-inline-back" class="editor-inline-back-btn">
            <span>←</span> <span>${escapeHtml(t('editor.btn_back_to_pak'))}</span>
          </button>
        ` : ''}
        <div style="display:flex;flex-direction:column;min-width:0;">
          <div style="font-size:13px;font-weight:700;color:var(--text-primary);font-family:monospace;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title="${escapeHtml(assetInternalPath)}">
            ${escapeHtml(assetInternalPath)}
          </div>
          <div style="font-size:11px;color:var(--text-muted);display:flex;gap:8px;align-items:center;">
            <span>${escapeHtml(t('scanner.uasset_modal_title'))}</span>
          </div>
        </div>
      </div>
      <div>
        <button id="btn-editor-inline-popout" class="editor-inline-open-modal-btn" title="${escapeHtml(t('editor.btn_open_modal'))}">
          <span>⛶</span> <span>${escapeHtml(t('editor.btn_open_modal'))}</span>
        </button>
      </div>
    </div>
    <div id="editor-inline-uasset-content" style="flex:1;display:flex;flex-direction:column;align-items:center;justify-content:center;min-height:0;padding:32px;">
      <div class="spinner" style="width:28px;height:28px;border:3px solid rgba(0,188,255,0.2);border-top-color:var(--accent);border-radius:50%;animation:spin 0.8s linear infinite;"></div>
      <span style="margin-top:12px;font-size:12px;color:var(--text-secondary);">${escapeHtml(t('common.loading'))}</span>
    </div>
  `;

  const backBtn = container.querySelector('#btn-editor-inline-back');
  if (backBtn && onBack) {
    backBtn.addEventListener('click', () => {
      onBack();
    });
  }

  const popoutBtn = container.querySelector('#btn-editor-inline-popout');
  if (popoutBtn) {
    popoutBtn.addEventListener('click', () => {
      import('../modals/uassetInspector').then(({ openUAssetInspectorModal }) => {
        openUAssetInspectorModal({
          modId: modId || null,
          pakPath: pakPath || null,
          assetInternalPath,
        });
      });
    });
  }

  const contentEl = container.querySelector('#editor-inline-uasset-content') as HTMLElement;
  if (!contentEl) return;

  try {
    const details: UAssetInspectionDetails = await inspectUAssetDeep({
      modId: modId || null,
      pakPath: pakPath || null,
      assetInternalPath,
    });

    const isTexture = details.assetType.toLowerCase().includes('texture');
    const isMaterial = details.assetType.toLowerCase().includes('material');
    const isDataTable = details.assetType.toLowerCase().includes('datatable') || details.assetType.toLowerCase().includes('table');
    const hasDatatableGrid = Boolean(details.datatableGrid && details.datatableGrid.totalRows > 0);
    const hasLiveProperties = Boolean(details.instantiatedProperties && details.instantiatedProperties.length > 0);
    const hasSchema = Boolean(details.resolvedSchema && details.resolvedSchema.totalProperties > 0);
    // On DataTables, the primary interactive editor is the dedicated DataTable grid.
    // Show tweaker only on non-DataTables or if there are actual editable scalar properties.
    const hasEditableScalarProps = Boolean(details.instantiatedProperties && details.instantiatedProperties.some(p => p.isEditable));
    const showTweaker = hasDatatableGrid ? hasEditableScalarProps : (hasLiveProperties || hasSchema);

    contentEl.style.padding = '0';
    contentEl.style.alignItems = 'stretch';
    contentEl.style.justifyContent = 'flex-start';

    contentEl.innerHTML = `
      <div class="editor-inline-uasset-tabs">
        <button class="uasset-tab-btn active" data-tab="tab-inline-overview" style="background:var(--bg-card);border:1px solid var(--border);border-bottom:none;color:var(--accent);font-weight:700;font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          📊 ${escapeHtml(t('scanner.uasset_tab_overview'))}
        </button>
        ${hasDatatableGrid ? `
        <button class="uasset-tab-btn" data-tab="tab-inline-datatable" style="background:none;border:1px solid transparent;color:#4af626;font-weight:700;font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          📊 ${escapeHtml(t('editor.tab_datatable_grid'))} (${details.datatableGrid!.totalRows})
        </button>
        ` : ''}
        ${showTweaker ? `
        <button class="uasset-tab-btn" data-tab="tab-inline-tweaker" style="background:none;border:1px solid transparent;color:#ffd166;font-weight:700;font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          ✏️ ${escapeHtml(t('editor.tab_live_properties'))} (${details.instantiatedProperties?.length || 0})
        </button>
        ` : ''}
        ${isTexture ? `
        <button class="uasset-tab-btn" data-tab="tab-inline-texture" style="background:none;border:1px solid transparent;color:#00bcff;font-weight:700;font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          🖼️ ${escapeHtml(t('scanner.uasset_tab_texture'))}
        </button>
        ` : ''}
        <button class="uasset-tab-btn" data-tab="tab-inline-exports" style="background:none;border:1px solid transparent;color:var(--text-secondary);font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          📤 ${escapeHtml(t('scanner.uasset_tab_exports'))} (${details.exports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-inline-imports" style="background:none;border:1px solid transparent;color:var(--text-secondary);font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          📥 ${escapeHtml(t('scanner.uasset_tab_imports'))} (${details.imports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-inline-names" style="background:none;border:1px solid transparent;color:var(--text-secondary);font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          🏷️ ${escapeHtml(t('scanner.uasset_tab_names'))} (${details.summary.nameCount})
        </button>
        ${details.resolvedSchema ? `
        <button class="uasset-tab-btn" data-tab="tab-inline-schema" style="background:none;border:1px solid transparent;color:#38bdf8;font-weight:700;font-size:11.5px;padding:6px 14px;border-radius:6px 6px 0 0;cursor:pointer;">
          🧬 ${escapeHtml(t('scanner.uasset_tab_schema'))} (${details.resolvedSchema.totalProperties})
        </button>
        ` : ''}
      </div>

      <div class="editor-inline-uasset-body">
        <!-- Overview Pane -->
        <div id="tab-inline-overview" class="uasset-tab-pane" style="display:flex;flex-direction:column;gap:12px;">
          <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:10px;">
            <div style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:10px 12px;display:flex;flex-direction:column;gap:3px;">
              <span style="font-size:10px;color:var(--text-muted);text-transform:uppercase;font-weight:700;">${escapeHtml(t('scanner.uasset_engine_version'))}</span>
              <strong style="color:var(--accent);font-size:12px;">${escapeHtml(details.engineVersion)}</strong>
            </div>
            <div style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:10px 12px;display:flex;flex-direction:column;gap:3px;">
              <span style="font-size:10px;color:var(--text-muted);text-transform:uppercase;font-weight:700;">${escapeHtml(t('scanner.uasset_asset_classification'))}</span>
              <span class="badge" style="align-self:flex-start;font-size:10.5px;padding:2px 8px;background:rgba(0,188,255,0.15);color:#00bcff;border:1px solid rgba(0,188,255,0.3);font-weight:700;">${escapeHtml(details.assetType)}</span>
            </div>
            <div style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:10px 12px;display:flex;flex-direction:column;gap:3px;">
              <span style="font-size:10px;color:var(--text-muted);text-transform:uppercase;font-weight:700;">${escapeHtml(t('scanner.uasset_header_size'))}</span>
              <strong style="color:var(--text-primary);font-family:monospace;font-size:11.5px;">${formatBytes(details.summary.uassetSizeBytes)}</strong>
            </div>
            <div style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:10px 12px;display:flex;flex-direction:column;gap:3px;">
              <span style="font-size:10px;color:var(--text-muted);text-transform:uppercase;font-weight:700;">${escapeHtml(t('scanner.uasset_payload_size'))}</span>
              <strong style="color:var(--text-primary);font-family:monospace;font-size:11.5px;">${details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : escapeHtml(t('scanner.uasset_embedded_in_uasset'))}</strong>
            </div>
          </div>

          ${isTexture ? `
            <div style="background:rgba(0,188,255,0.05);border:1px solid rgba(0,188,255,0.2);border-radius:8px;padding:12px 16px;display:flex;align-items:center;justify-content:space-between;gap:12px;">
              <div style="display:flex;align-items:center;gap:12px;">
                <span style="font-size:24px;">🖼️</span>
                <div>
                  <div style="font-size:11.5px;font-weight:700;color:#00bcff;">${escapeHtml(t('scanner.uasset_texture_title'))}</div>
                  <div style="font-size:10px;color:var(--text-secondary);">${escapeHtml(t('scanner.uasset_texture_desc', { size: details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : (t('scanner.uasset_embedded_in_uasset')) }))}</div>
                </div>
              </div>
              <button id="btn-inline-goto-texture" class="editor-pak-btn-inspect" style="padding:5px 12px;font-size:11px;">
                <span>🖼️</span> <span>${escapeHtml(t('scanner.uasset_tab_texture'))}</span>
              </button>
            </div>
          ` : (isMaterial ? `
            <div style="background:rgba(192,132,252,0.05);border:1px solid rgba(192,132,252,0.2);border-radius:8px;padding:10px 14px;display:flex;align-items:center;gap:12px;">
              <span style="font-size:24px;">🎨</span>
              <div>
                <div style="font-size:11.5px;font-weight:700;color:#c084fc;">${escapeHtml(t('scanner.uasset_material_title'))}</div>
                <div style="font-size:10px;color:var(--text-secondary);">${escapeHtml(t('scanner.uasset_material_desc'))}</div>
              </div>
            </div>
          ` : (isDataTable ? `
            <div style="background:rgba(74,246,38,0.05);border:1px solid rgba(74,246,38,0.2);border-radius:8px;padding:10px 14px;display:flex;align-items:center;gap:12px;">
              <span style="font-size:24px;">📊</span>
              <div>
                <div style="font-size:11.5px;font-weight:700;color:#4af626;">${escapeHtml(t('scanner.uasset_datatable_title'))}</div>
                <div style="font-size:10px;color:var(--text-secondary);">${escapeHtml(t('scanner.uasset_datatable_desc'))}</div>
              </div>
            </div>
          ` : ''))}

          <div style="background:rgba(0,0,0,0.2);border:1px solid var(--border);border-radius:8px;padding:12px;display:flex;justify-content:space-around;align-items:center;text-align:center;">
            <div>
              <div style="font-size:16px;font-weight:700;color:#4af626;">${details.exports.length}</div>
              <div style="font-size:10px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_exports'))}</div>
            </div>
            <div style="width:1px;height:24px;background:var(--border);"></div>
            <div>
              <div style="font-size:16px;font-weight:700;color:#38bdf8;">${details.imports.length}</div>
              <div style="font-size:10px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_imports'))}</div>
            </div>
            <div style="width:1px;height:24px;background:var(--border);"></div>
            <div>
              <div style="font-size:16px;font-weight:700;color:#ffd166;">${details.summary.nameCount}</div>
              <div style="font-size:10px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_names'))}</div>
            </div>
            ${details.resolvedSchema ? `
            <div style="width:1px;height:24px;background:var(--border);"></div>
            <div>
              <div style="font-size:16px;font-weight:700;color:#38bdf8;">${details.resolvedSchema.totalProperties}</div>
              <div style="font-size:10px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_schema'))}</div>
            </div>
            ` : ''}
          </div>
        </div>

        ${isTexture ? `
        <!-- Texture Pane -->
        <div id="tab-inline-texture" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:10px;">
          <div id="inline-tex-lazy-loader" style="padding:48px 20px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:12px;">
            <div class="spinner" style="width:28px;height:28px;border:3px solid rgba(0,188,255,0.2);border-top-color:#00bcff;border-radius:50%;animation:spin 0.8s linear infinite;"></div>
            <span style="font-size:12px;color:var(--text-secondary);">${escapeHtml(t('scanner.uasset_loading_texture'))}</span>
          </div>
          <div id="inline-tex-viewport-content" style="display:none;flex-direction:column;gap:10px;"></div>
        </div>
        ` : ''}

        <!-- Exports Pane -->
        <div id="tab-inline-exports" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:8px;">
          <input type="text" id="inline-exports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_exports'))}" style="width:100%;background:var(--bg-secondary);border:1px solid var(--border);color:var(--text-primary);border-radius:4px;padding:6px 10px;font-size:11px;outline:none;" />
          <div id="inline-exports-list" style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:6px;max-height:420px;overflow-y:auto;display:flex;flex-direction:column;gap:4px;">
            ${details.exports.map(exp => `
              <div class="uasset-export-row" data-search="${escapeHtml((exp.objectName + ' ' + exp.className + ' ' + (exp.outerName || '')).toLowerCase())}" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;padding:6px 10px;display:flex;justify-content:space-between;align-items:center;gap:10px;font-family:monospace;font-size:11px;">
                <div style="display:flex;flex-direction:column;gap:2px;overflow:hidden;">
                  <strong style="color:#4af626;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;">${escapeHtml(exp.objectName)}</strong>
                  ${exp.outerName ? `<span style="font-size:9px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_parent_prefix'))}: ${escapeHtml(exp.outerName)}</span>` : ''}
                </div>
                <span class="badge" style="font-size:9.5px;padding:2px 6px;background:rgba(74,246,38,0.12);color:#4af626;border:1px solid rgba(74,246,38,0.25);">${escapeHtml(exp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Imports Pane -->
        <div id="tab-inline-imports" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:8px;">
          <input type="text" id="inline-imports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_imports'))}" style="width:100%;background:var(--bg-secondary);border:1px solid var(--border);color:var(--text-primary);border-radius:4px;padding:6px 10px;font-size:11px;outline:none;" />
          <div id="inline-imports-list" style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:6px;max-height:420px;overflow-y:auto;display:flex;flex-direction:column;gap:4px;">
            ${details.imports.map(imp => `
              <div class="uasset-import-row" data-search="${escapeHtml((imp.objectName + ' ' + imp.className + ' ' + imp.classPackage).toLowerCase())}" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;padding:6px 10px;display:flex;justify-content:space-between;align-items:center;gap:10px;font-family:monospace;font-size:11px;">
                <div style="display:flex;flex-direction:column;gap:2px;overflow:hidden;">
                  <strong style="color:#38bdf8;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;">${escapeHtml(imp.objectName)}</strong>
                  <span style="font-size:9px;color:var(--text-muted);">${escapeHtml(t('scanner.uasset_package_prefix'))}: ${escapeHtml(imp.classPackage)}</span>
                </div>
                <span class="badge" style="font-size:9.5px;padding:2px 6px;background:rgba(56,189,248,0.12);color:#38bdf8;border:1px solid rgba(56,189,248,0.25);">${escapeHtml(imp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Names Pane -->
        <div id="tab-inline-names" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:8px;">
          <div style="display:flex;align-items:center;gap:10px;">
            <input type="text" id="inline-names-filter" placeholder="${escapeHtml(t('scanner.uasset_search_names'))}" style="flex:1;background:var(--bg-secondary);border:1px solid var(--border);color:var(--text-primary);border-radius:4px;padding:6px 10px;font-size:11px;outline:none;" />
            <span id="inline-names-count" style="font-size:10px;color:var(--text-muted);font-family:monospace;white-space:nowrap;">${(details.namesSample || []).length} tokens</span>
          </div>
          <div id="inline-names-list" style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:8px;max-height:420px;overflow-y:auto;display:flex;flex-wrap:wrap;gap:4px;">
            ${(details.namesSample || []).map((nm: string) => `
              <span class="inline-name-tag" data-name="${escapeHtml(nm)}" data-search="${escapeHtml(nm.toLowerCase())}" style="font-family:monospace;font-size:10px;padding:2px 6px;border-radius:3px;background:var(--bg-primary);border:1px solid var(--border);color:var(--text-primary);cursor:pointer;" title="Click to copy">${escapeHtml(nm)}</span>
            `).join('')}
          </div>
        </div>

        ${hasDatatableGrid ? `
        <!-- DataTable Grid Pane -->
        <div id="tab-inline-datatable" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:10px;">
          ${renderDataTableGridHtml(details.datatableGrid!)}
        </div>
        ` : ''}

        ${showTweaker ? `
        <!-- Properties & Tweaker Pane -->
        <div id="tab-inline-tweaker" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:12px;">
          ${renderPropertyTweakerHtml(details)}
        </div>
        ` : ''}

        ${details.resolvedSchema ? `
        <!-- Schema Pane -->
        <div id="tab-inline-schema" class="uasset-tab-pane" style="display:none;flex-direction:column;gap:8px;">
          <input type="text" id="inline-schema-filter" placeholder="${escapeHtml(t('scanner.uasset_search_schema'))}" style="width:100%;background:var(--bg-secondary);border:1px solid var(--border);color:var(--text-primary);border-radius:4px;padding:6px 10px;font-size:11px;outline:none;" />
          <div id="inline-schema-list" style="background:var(--bg-secondary);border:1px solid var(--border);border-radius:6px;padding:6px;max-height:420px;overflow-y:auto;display:flex;flex-direction:column;gap:4px;">
            ${details.resolvedSchema.properties.map(p => `
              <div class="uasset-schema-row" data-search="${escapeHtml((p.name + ' ' + p.typeName + ' ' + (p.structType || '') + ' ' + (p.enumType || '') + ' ' + (p.innerType || '')).toLowerCase())}" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:4px;padding:6px 10px;display:flex;justify-content:space-between;align-items:center;gap:10px;font-family:monospace;font-size:11px;">
                <div style="display:flex;align-items:center;gap:8px;overflow:hidden;">
                  <span style="font-size:9.5px;color:var(--text-muted);min-width:24px;">#${p.index}</span>
                  <strong style="color:#38bdf8;text-overflow:ellipsis;overflow:hidden;white-space:nowrap;">${escapeHtml(p.name)}</strong>
                </div>
                <div style="display:flex;align-items:center;gap:6px;flex-shrink:0;">
                  ${p.structType ? `<span style="font-size:9.5px;color:#ffd166;">${escapeHtml(p.structType)}</span>` : ''}
                  ${p.enumType ? `<span style="font-size:9.5px;color:#c084fc;">${escapeHtml(p.enumType)}</span>` : ''}
                  ${p.innerType ? `<span style="font-size:9.5px;color:#4ade80;">&lt;${escapeHtml(p.innerType)}&gt;</span>` : ''}
                  <span class="badge" style="font-size:9.5px;padding:2px 6px;background:rgba(56,189,248,0.12);color:#38bdf8;border:1px solid rgba(56,189,248,0.25);">${escapeHtml(p.typeName)}</span>
                </div>
              </div>
            `).join('')}
          </div>
        </div>
        ` : ''}
      </div>
    `;

    // Tab switcher
    const tabBtns = contentEl.querySelectorAll('.uasset-tab-btn');
    const tabPanes = contentEl.querySelectorAll('.uasset-tab-pane');

    const getTabColor = (tabName: string | undefined): string => {
      if (tabName === 'tab-inline-texture') return '#00bcff';
      if (tabName === 'tab-inline-schema') return '#38bdf8';
      if (tabName === 'tab-inline-datatable') return '#4af626';
      if (tabName === 'tab-inline-tweaker') return '#ffd166';
      return 'var(--accent)';
    };

    const switchTab = (targetTab: string) => {
      tabBtns.forEach(b => {
        b.classList.remove('active');
        (b as HTMLElement).style.background = 'none';
        (b as HTMLElement).style.borderColor = 'transparent';
        const tab = (b as HTMLElement).dataset.tab;
        (b as HTMLElement).style.color = tab === 'tab-inline-texture' ? '#00bcff' : tab === 'tab-inline-schema' ? '#38bdf8' : tab === 'tab-inline-datatable' ? '#4af626' : tab === 'tab-inline-tweaker' ? '#ffd166' : 'var(--text-secondary)';
        (b as HTMLElement).style.fontWeight = 'normal';
      });
      tabPanes.forEach(p => {
        (p as HTMLElement).style.display = 'none';
      });

      const activeBtn = Array.from(tabBtns).find(b => (b as HTMLElement).dataset.tab === targetTab) as HTMLElement | undefined;
      if (activeBtn) {
        activeBtn.classList.add('active');
        activeBtn.style.background = 'var(--bg-card)';
        activeBtn.style.border = '1px solid var(--border)';
        activeBtn.style.borderBottom = 'none';
        activeBtn.style.color = getTabColor(targetTab);
        activeBtn.style.fontWeight = '700';
      }

      const targetPane = contentEl.querySelector(`#${targetTab}`) as HTMLElement | null;
      if (targetPane) targetPane.style.display = 'flex';

      if (targetTab === 'tab-inline-texture') loadTextureContent();
    };

    tabBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const tab = (btn as HTMLElement).dataset.tab;
        if (tab) switchTab(tab);
      });
    });

    const gotoTexBtn = contentEl.querySelector('#btn-inline-goto-texture');
    if (gotoTexBtn) {
      gotoTexBtn.addEventListener('click', () => switchTab('tab-inline-texture'));
    }

    // Lazy load texture
    let textureLoaded = false;
    const loadTextureContent = async () => {
      if (textureLoaded) return;
      textureLoaded = true;

      const loaderEl = contentEl.querySelector('#inline-tex-lazy-loader') as HTMLElement | null;
      const viewportEl = contentEl.querySelector('#inline-tex-viewport-content') as HTMLElement | null;

      try {
        const texInfo: TexturePreviewInfo = await decodeUAssetTexture({
          modId: modId || null,
          pakPath: pakPath || null,
          assetInternalPath,
        });

        if (loaderEl) loaderEl.style.display = 'none';
        if (viewportEl) {
          viewportEl.style.display = 'flex';
          viewportEl.innerHTML = `
            <div style="display:flex;justify-content:space-between;align-items:center;background:var(--bg-secondary);padding:8px 12px;border-radius:6px;border:1px solid var(--border);">
              <div style="display:flex;gap:16px;font-family:monospace;font-size:11px;">
                <span><strong>Format:</strong> <span style="color:#00bcff;">${escapeHtml(texInfo.formatName || '')}</span></span>
                <span><strong>Dimensions:</strong> <span style="color:var(--text-primary);">${texInfo.width} × ${texInfo.height}</span></span>
                <span><strong>Mipmaps:</strong> <span style="color:var(--text-primary);">${texInfo.mipCount}</span></span>
              </div>
            </div>
            <div style="display:flex;justify-content:center;align-items:center;padding:24px;background:repeating-conic-gradient(#1f2430 0% 25%, #151922 0% 50%) 50% / 20px 20px;border-radius:8px;border:1px solid var(--border);min-height:260px;max-height:420px;overflow:auto;">
              <img src="${escapeHtml(texInfo.dataUrl)}" alt="Texture Preview" style="max-width:100%;max-height:360px;object-fit:contain;image-rendering:pixelated;box-shadow:0 4px 20px rgba(0,0,0,0.5);border-radius:4px;" />
            </div>
          `;
        }
      } catch (err) {
        if (loaderEl) loaderEl.style.display = 'none';
        if (viewportEl) {
          viewportEl.style.display = 'flex';
          viewportEl.innerHTML = `
            <div style="padding:24px;text-align:center;color:var(--danger);font-size:11px;">
              ⚠️ ${escapeHtml(String(err))}
            </div>
          `;
        }
      }
    };

    // Filter bindings
    const bindFilter = (inputId: string, rowSelector: string) => {
      const input = contentEl.querySelector(`#${inputId}`) as HTMLInputElement | null;
      if (!input) return;
      input.addEventListener('input', () => {
        const q = input.value.trim().toLowerCase();
        contentEl.querySelectorAll(rowSelector).forEach(el => {
          const search = (el as HTMLElement).dataset.search || '';
          (el as HTMLElement).style.display = (!q || search.includes(q)) ? '' : 'none';
        });
      });
    };

    bindFilter('inline-exports-filter', '.uasset-export-row');
    bindFilter('inline-imports-filter', '.uasset-import-row');
    bindFilter('inline-schema-filter', '.uasset-schema-row');

    const nameInput = contentEl.querySelector('#inline-names-filter') as HTMLInputElement | null;
    const nameTags = contentEl.querySelectorAll('.inline-name-tag');
    const nameCountEl = contentEl.querySelector('#inline-names-count');

    if (nameInput) {
      nameInput.addEventListener('input', () => {
        const q = nameInput.value.trim().toLowerCase();
        let count = 0;
        nameTags.forEach(tag => {
          const search = (tag as HTMLElement).dataset.search || '';
          const match = (!q || search.includes(q));
          (tag as HTMLElement).style.display = match ? 'inline-block' : 'none';
          if (match) count++;
        });
        if (nameCountEl) nameCountEl.textContent = `${count} / ${nameTags.length} tokens`;
      });
    }

    nameTags.forEach(tag => {
      tag.addEventListener('click', async () => {
        const name = (tag as HTMLElement).dataset.name || '';
        try {
          await navigator.clipboard.writeText(name);
          showToast(t('scanner.uasset_copied_token', { token: name }) || `Copied: "${name}"`, 'success');
        } catch {}
      });
    });

    const onRefresh = () => {
      renderInlineUAssetInspector(container, options);
    };

    if (details.datatableGrid && details.datatableGrid.totalRows > 0) {
      setupDataTableGridEvents(contentEl, {
        grid: details.datatableGrid,
        modId,
        pakPath,
        assetInternalPath,
        onRefresh,
      });
    }

    if (showTweaker) {
      setupPropertyTweakerEvents(contentEl, {
        details,
        modId,
        pakPath,
        assetInternalPath,
        onRefresh,
      });
    }

  } catch (err) {
    contentEl.innerHTML = `
      <div style="padding:32px;text-align:center;color:var(--danger);font-size:12px;">
        ⚠️ ${escapeHtml(String(err))}
      </div>
    `;
  }
}
