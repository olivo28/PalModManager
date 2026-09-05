import { inspectUAssetDeep, decodeUAssetTexture, type UAssetInspectionDetails, type TexturePreviewInfo } from '../../api';
import { t } from '../../utils/i18n';
import { escapeHtml, formatBytes } from '../../utils/helpers';
import { showToast } from '../toast';
import { mainDom } from '../../framework';

export async function openUAssetInspectorModal(params: {
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  zipPath?: string | null;
}): Promise<void> {
  const existing = mainDom.elMaybe('uasset-inspector-modal');
  if (existing) existing.remove();

  const loadingModalHtml = `
    <div id="uasset-inspector-modal" class="modal-overlay" style="position: fixed; inset: 0; background: rgba(0,0,0,0.75); backdrop-filter: blur(5px); display: flex; align-items: center; justify-content: center; z-index: 10000; animation: fadeIn 0.15s ease;">
      <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; width: 780px; max-width: 95vw; max-height: 88vh; box-shadow: 0 16px 40px rgba(0,0,0,0.7); display: flex; flex-direction: column; overflow: hidden;">
        
        <div style="background: var(--bg-secondary); padding: 14px 18px; border-bottom: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center;">
          <div style="display: flex; align-items: center; gap: 10px;">
            <span style="font-size: 20px;">📦</span>
            <div>
              <div style="font-size: 14px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.uasset_modal_title') || 'Unreal Engine Asset Inspector')}</div>
              <div style="font-size: 11px; color: var(--text-muted); font-family: monospace;">${escapeHtml(params.assetInternalPath)}</div>
            </div>
          </div>
          <button id="btn-close-uasset-modal" style="background: none; border: none; font-size: 18px; color: var(--text-muted); cursor: pointer; padding: 4px;">✕</button>
        </div>

        <div id="uasset-modal-body" style="padding: 36px 20px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; min-height: 280px;">
          <div class="spinner" style="width: 28px; height: 28px; border: 3px solid rgba(0, 188, 255, 0.2); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
          <span style="font-size: 12.5px; color: var(--text-secondary);">${escapeHtml(t('common.loading') || 'Loading asset details...')}</span>
        </div>

      </div>
    </div>
  `;

  document.body.insertAdjacentHTML('beforeend', loadingModalHtml);

  const modalEl = mainDom.elMaybe('uasset-inspector-modal');
  const closeBtn = mainDom.elMaybe('btn-close-uasset-modal');
  const bodyEl = mainDom.elMaybe('uasset-modal-body');

  const closeModal = () => modalEl?.remove();
  closeBtn?.addEventListener('click', closeModal);
  modalEl?.addEventListener('click', (e) => {
    if (e.target === modalEl) closeModal();
  });

  try {
    const details: UAssetInspectionDetails = await inspectUAssetDeep(params);
    if (!bodyEl || !modalEl) return;

    bodyEl.style.padding = '0';
    bodyEl.style.minHeight = 'auto';
    bodyEl.style.alignItems = 'stretch';
    bodyEl.style.justifyContent = 'flex-start';

    const isTexture = details.assetType.toLowerCase().includes('texture');
    const isMaterial = details.assetType.toLowerCase().includes('material');
    const isDataTable = details.assetType.toLowerCase().includes('datatable') || details.assetType.toLowerCase().includes('table');

    bodyEl.innerHTML = `
      <!-- Tabs Navigation -->
      <div style="background: rgba(0,0,0,0.25); border-bottom: 1px solid var(--border); display: flex; gap: 4px; padding: 8px 16px 0 16px; overflow-x: auto;">
        <button class="uasset-tab-btn active" data-tab="tab-overview" style="background: var(--bg-card); border: 1px solid var(--border); border-bottom: none; color: var(--accent); font-weight: 700; font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          📊 ${escapeHtml(t('scanner.uasset_tab_overview') || 'Overview')}
        </button>
        ${isTexture ? `
        <button class="uasset-tab-btn" data-tab="tab-texture" style="background: none; border: 1px solid transparent; color: #00bcff; font-weight: 700; font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          🖼️ ${escapeHtml(t('scanner.uasset_tab_texture') || 'GPU Texture')}
        </button>
        ` : ''}
        <button class="uasset-tab-btn" data-tab="tab-exports" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          📤 ${escapeHtml(t('scanner.uasset_tab_exports') || 'Exports')} (${details.exports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-imports" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          📥 ${escapeHtml(t('scanner.uasset_tab_imports') || 'Imports')} (${details.imports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-names" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          🔤 ${escapeHtml(t('scanner.uasset_tab_names') || 'Name Map')} (${details.summary.nameCount})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-schema" style="background: none; border: 1px solid transparent; color: #38bdf8; font-size: 11.5px; padding: 6px 14px; border-radius: 6px 6px 0 0; cursor: pointer; transition: all 0.15s ease;">
          ⚡ ${escapeHtml(t('scanner.uasset_tab_schema') || 'Schema (USMAP)')} ${details.resolvedSchema ? `(${details.resolvedSchema.totalProperties})` : ''}
        </button>
      </div>

      <!-- Tab Content Area -->
      <div style="padding: 16px 18px; overflow-y: auto; max-height: 55vh; min-height: 160px; display: flex; flex-direction: column; gap: 12px;">
        
        <!-- Tab 1: Overview -->
        <div id="tab-overview" class="uasset-tab-pane" style="display: flex; flex-direction: column; gap: 12px;">
          <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px;">
            
            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 3px;">
              <span style="font-size: 10px; color: var(--text-muted); text-transform: uppercase; font-weight: 700; letter-spacing: 0.5px;">${escapeHtml(t('scanner.uasset_engine_version') || 'Engine Version')}</span>
              <strong style="color: var(--accent); font-size: 12px;">${escapeHtml(details.engineVersion)}</strong>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 3px;">
              <span style="font-size: 10px; color: var(--text-muted); text-transform: uppercase; font-weight: 700; letter-spacing: 0.5px;">${escapeHtml(t('scanner.uasset_asset_classification') || 'Asset Classification')}</span>
              <span class="badge" style="align-self: flex-start; font-size: 10.5px; padding: 2px 8px; background: rgba(0,188,255,0.15); color: #00bcff; border: 1px solid rgba(0,188,255,0.3); font-weight: 700;">${escapeHtml(details.assetType)}</span>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 3px;">
              <span style="font-size: 10px; color: var(--text-muted); text-transform: uppercase; font-weight: 700; letter-spacing: 0.5px;">${escapeHtml(t('scanner.uasset_header_size') || 'Header Size (.uasset)')}</span>
              <strong style="color: var(--text-primary); font-family: monospace; font-size: 11.5px;">${formatBytes(details.summary.uassetSizeBytes)}</strong>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 3px;">
              <span style="font-size: 10px; color: var(--text-muted); text-transform: uppercase; font-weight: 700; letter-spacing: 0.5px;">${escapeHtml(t('scanner.uasset_payload_size') || 'Payload Size (.uexp)')}</span>
              <strong style="color: var(--text-primary); font-family: monospace; font-size: 11.5px;">${details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : escapeHtml(t('scanner.uasset_embedded_in_uasset') || 'Embedded in .uasset')}</strong>
            </div>

          </div>

          ${isTexture ? `
            <div style="background: rgba(0, 188, 255, 0.05); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: 8px; padding: 12px 16px; display: flex; align-items: center; justify-content: space-between; gap: 12px;">
              <div style="display: flex; align-items: center; gap: 12px;">
                <span style="font-size: 24px; flex-shrink: 0;">🖼️</span>
                <div style="display: flex; flex-direction: column; gap: 2px;">
                  <span style="font-size: 11.5px; font-weight: 700; color: #00bcff;">${escapeHtml(t('scanner.uasset_texture_title') || 'DirectDraw Surface / GPU Texture2D Binary')}</span>
                  <span style="font-size: 10px; color: var(--text-secondary); line-height: 1.4;">${escapeHtml(t('scanner.uasset_texture_desc', { size: details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : (t('scanner.uasset_embedded_in_uasset') || 'Embedded') }) || `Cooked texture streaming resource. Raw mipmaps and BC7/DXT compressed pixel buffers are embedded in the .uexp payload.`)}</span>
                </div>
              </div>
              <button id="btn-quick-goto-texture" class="btn btn-primary btn-sm" style="font-size: 11px; padding: 5px 12px; display: inline-flex; align-items: center; gap: 5px; white-space: nowrap;">
                <span>🖼️</span> <span>${escapeHtml(t('scanner.uasset_tab_texture') || 'View Texture')}</span>
              </button>
            </div>
          ` : (isMaterial ? `
            <div style="background: rgba(192, 132, 252, 0.05); border: 1px solid rgba(192, 132, 252, 0.2); border-radius: 8px; padding: 10px 14px; display: flex; align-items: center; gap: 12px;">
              <span style="font-size: 24px; flex-shrink: 0;">🎨</span>
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <span style="font-size: 11.5px; font-weight: 700; color: #c084fc;">${escapeHtml(t('scanner.uasset_material_title') || 'Material / Shader Graph Instance')}</span>
                <span style="font-size: 10px; color: var(--text-secondary); line-height: 1.4;">${escapeHtml(t('scanner.uasset_material_desc') || 'Contains compiled shader parameter overrides, texture sampler references, and material pipeline definitions.')}</span>
              </div>
            </div>
          ` : (isDataTable ? `
            <div style="background: rgba(74, 246, 38, 0.05); border: 1px solid rgba(74, 246, 38, 0.2); border-radius: 8px; padding: 10px 14px; display: flex; align-items: center; gap: 12px;">
              <span style="font-size: 24px; flex-shrink: 0;">📊</span>
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <span style="font-size: 11.5px; font-weight: 700; color: #4af626;">${escapeHtml(t('scanner.uasset_datatable_title') || 'Unreal Engine DataTable Asset')}</span>
                <span style="font-size: 10px; color: var(--text-secondary); line-height: 1.4;">${escapeHtml(t('scanner.uasset_datatable_desc') || 'Contains structured game data rows, item templates, stats, or configuration mappings.')}</span>
              </div>
            </div>
          ` : ''))}

          <!-- Quick Metrics Bar -->
          <div style="background: rgba(0,0,0,0.2); border: 1px solid var(--border); border-radius: 8px; padding: 10px 14px; display: flex; justify-content: space-around; align-items: center; text-align: center; flex-wrap: wrap; gap: 10px;">
            <div>
              <div style="font-size: 16px; font-weight: 700; color: #4af626;">${details.exports.length}</div>
              <div style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_exports') || 'Exported Objects')}</div>
            </div>
            <div style="width: 1px; height: 24px; background: var(--border);"></div>
            <div>
              <div style="font-size: 16px; font-weight: 700; color: #38bdf8;">${details.imports.length}</div>
              <div style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_imports') || 'Imported Dependencies')}</div>
            </div>
            <div style="width: 1px; height: 24px; background: var(--border);"></div>
            <div>
              <div style="font-size: 16px; font-weight: 700; color: #ffd166;">${details.summary.nameCount}</div>
              <div style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_names') || 'Name Map Tokens')}</div>
            </div>
            <div style="width: 1px; height: 24px; background: var(--border);"></div>
            <div>
              <div style="font-size: 16px; font-weight: 700; color: #38bdf8;">${details.resolvedSchema ? details.resolvedSchema.totalProperties : '—'}</div>
              <div style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_schema') || 'Schema Properties')}</div>
            </div>
          </div>
        </div>

        ${isTexture ? `
        <!-- Tab: GPU Texture Preview (Lazy Loaded) -->
        <div id="tab-texture" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 10px;">
          <div id="tex-lazy-loader" style="padding: 48px 20px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px;">
            <div class="spinner" style="width: 28px; height: 28px; border: 3px solid rgba(0, 188, 255, 0.2); border-top-color: #00bcff; border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
            <span style="font-size: 12px; color: var(--text-secondary);">${escapeHtml(t('scanner.uasset_loading_texture') || 'Decoding GPU texture in memory...')}</span>
          </div>
          <div id="tex-viewport-content" style="display: none; flex-direction: column; gap: 10px;"></div>
        </div>
        ` : ''}

        <!-- Tab 2: Exports -->
        <div id="tab-exports" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <input type="text" id="uasset-exports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_exports') || 'Filter exports by object or class name...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />
          
          <div id="uasset-exports-list" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 6px; max-height: 240px; min-height: 80px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
            ${details.exports.length === 0 ? `
              <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 11px;">${escapeHtml(t('scanner.uasset_no_exports') || 'No exports found in asset header.')}</div>
            ` : details.exports.map(exp => `
              <div class="uasset-export-row" data-search="${escapeHtml((exp.objectName + ' ' + exp.className + ' ' + (exp.outerName || '')).toLowerCase())}" style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                  <strong style="color: #4af626; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(exp.objectName)}</strong>
                  ${exp.outerName ? `<span style="font-size: 9px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_parent_prefix') || 'Parent')}: ${escapeHtml(exp.outerName)}</span>` : ''}
                </div>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(74, 246, 38, 0.12); color: #4af626; border: 1px solid rgba(74, 246, 38, 0.25); white-space: nowrap;">${escapeHtml(exp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Tab 3: Imports -->
        <div id="tab-imports" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <input type="text" id="uasset-imports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_imports') || 'Filter imports by dependency or package...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />
          
          <div id="uasset-imports-list" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 6px; max-height: 240px; min-height: 80px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
            ${details.imports.length === 0 ? `
              <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 11px;">${escapeHtml(t('scanner.uasset_no_imports') || 'No external imports found.')}</div>
            ` : details.imports.map(imp => `
              <div class="uasset-import-row" data-search="${escapeHtml((imp.objectName + ' ' + imp.className + ' ' + imp.classPackage).toLowerCase())}" style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                  <strong style="color: #38bdf8; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(imp.objectName)}</strong>
                  <span style="font-size: 9px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_package_prefix') || 'Package')}: ${escapeHtml(imp.classPackage)}</span>
                </div>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(imp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Tab 4: Name Map -->
        <div id="tab-names" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <input type="text" id="uasset-names-filter" placeholder="${escapeHtml(t('scanner.uasset_search_names') || 'Search name table tokens and identifiers...')}" style="flex: 1; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />
            <span id="uasset-names-count" style="font-size: 10px; color: var(--text-muted); white-space: nowrap;">${escapeHtml(t('scanner.uasset_tokens_count', { count: details.summary.nameCount }) || `${details.summary.nameCount} tokens`)}</span>
          </div>

          <div id="uasset-names-list" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 8px; max-height: 240px; min-height: 80px; overflow-y: auto; display: flex; flex-wrap: wrap; gap: 5px; align-content: flex-start;">
            ${details.namesSample.map(name => `
              <span class="uasset-name-tag" data-name="${escapeHtml(name)}" data-search="${escapeHtml(name.toLowerCase())}" title="${escapeHtml(t('scanner.uasset_click_to_copy') || 'Click to copy')}" style="background: var(--bg-secondary); border: 1px solid var(--border); color: #ffd166; font-family: monospace; font-size: 10px; padding: 2px 7px; border-radius: 4px; cursor: pointer; user-select: all; transition: all 0.1s ease;">
                ${escapeHtml(name)}
              </span>
            `).join('')}
          </div>
        </div>

        <!-- Tab 5: Schema (USMAP) -->
        <div id="tab-schema" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          ${details.resolvedSchema ? `
            <div style="background: rgba(56, 189, 248, 0.06); border: 1px solid rgba(56, 189, 248, 0.2); border-radius: 6px; padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="font-size: 14px;">⚡</span>
                <span><strong>${escapeHtml(details.resolvedSchema.matchedStructName)}</strong> ${details.resolvedSchema.superType ? `<span style="color: var(--text-muted); font-size: 10px;">(extends ${escapeHtml(details.resolvedSchema.superType)})</span>` : ''}</span>
              </div>
              <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.15); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.3);">
                ${details.resolvedSchema.totalProperties} ${escapeHtml(t('scanner.uasset_schema_props_count') || 'Properties Resolved')}
              </span>
            </div>

            <input type="text" id="uasset-schema-filter" placeholder="${escapeHtml(t('scanner.uasset_search_schema') || 'Filter schema properties by name or type...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />

            <div id="uasset-schema-list" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 6px; max-height: 220px; min-height: 80px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
              ${details.resolvedSchema.properties.map(p => `
                <div class="uasset-schema-row" data-search="${escapeHtml((p.name + ' ' + p.typeName + ' ' + (p.structType || '') + ' ' + (p.enumType || '') + ' ' + (p.innerType || '')).toLowerCase())}" style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                  <div style="display: flex; align-items: center; gap: 8px; overflow: hidden;">
                    <span style="font-size: 9.5px; color: var(--text-muted); min-width: 24px;">#${p.index}</span>
                    <strong style="color: #38bdf8; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(p.name)}</strong>
                  </div>
                  <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                    ${p.structType ? `<span style="font-size: 9.5px; color: #ffd166;">${escapeHtml(p.structType)}</span>` : ''}
                    ${p.enumType ? `<span style="font-size: 9.5px; color: #c084fc;">${escapeHtml(p.enumType)}</span>` : ''}
                    ${p.innerType ? `<span style="font-size: 9.5px; color: #4ade80;">&lt;${escapeHtml(p.innerType)}&gt;</span>` : ''}
                    <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25);">${escapeHtml(p.typeName)}</span>
                  </div>
                </div>
              `).join('')}
            </div>
          ` : `
            <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 11px;">${escapeHtml(t('scanner.uasset_no_schema') || 'No USMAP schema mapping available for this asset.')}</div>
          `}
        </div>

      </div>

      <!-- Footer -->
      <div style="background: var(--bg-secondary); padding: 10px 18px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end;">
        <button id="btn-close-uasset-action" class="btn btn-secondary" style="padding: 5px 16px; font-size: 12px; height: auto;">${escapeHtml(t('common.close') || 'Close')}</button>
      </div>
    `;

    // Tab Switching Logic
    const tabBtns = modalEl.querySelectorAll('.uasset-tab-btn');
    const tabPanes = modalEl.querySelectorAll('.uasset-tab-pane');

    const switchTab = (targetTab: string) => {
      tabBtns.forEach(b => {
        b.classList.remove('active');
        (b as HTMLElement).style.background = 'none';
        (b as HTMLElement).style.borderColor = 'transparent';
        (b as HTMLElement).style.color = (b as HTMLElement).dataset.tab === 'tab-texture' ? '#00bcff' : (b as HTMLElement).dataset.tab === 'tab-schema' ? '#38bdf8' : 'var(--text-secondary)';
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
        activeBtn.style.color = targetTab === 'tab-texture' ? '#00bcff' : targetTab === 'tab-schema' ? '#38bdf8' : 'var(--accent)';
        activeBtn.style.fontWeight = '700';
      }

      const targetPane = document.getElementById(targetTab);
      if (targetPane) {
        targetPane.style.display = 'flex';
      }

      // If user navigated to Texture tab, trigger lazy loading
      if (targetTab === 'tab-texture') {
        loadTextureTabContent();
      }
    };

    tabBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const targetTab = (btn as HTMLElement).dataset.tab || 'tab-overview';
        switchTab(targetTab);
      });
    });

    mainDom.elMaybe('btn-quick-goto-texture')?.addEventListener('click', () => {
      switchTab('tab-texture');
    });

    // Lazy Loading Texture Preview State & Renderer
    let cachedTexture: TexturePreviewInfo | null = null;
    let isTextureLoading = false;

    const loadTextureTabContent = async () => {
      if (cachedTexture) return; // Already loaded and cached
      if (isTextureLoading) return;

      isTextureLoading = true;
      const loaderEl = mainDom.elMaybe('tex-lazy-loader');
      const viewportContent = mainDom.elMaybe('tex-viewport-content');

      try {
        const tex = await decodeUAssetTexture(params);
        cachedTexture = tex;

        if (loaderEl) loaderEl.style.display = 'none';
        if (!viewportContent) return;

        viewportContent.style.display = 'flex';
        viewportContent.innerHTML = `
          <!-- Interactive GPU Texture Viewport -->
          <div style="background: var(--bg-primary); border: 1px solid rgba(0, 188, 255, 0.3); border-radius: 10px; overflow: hidden; display: flex; flex-direction: column;">
            
            <!-- Viewport Header Toolbar -->
            <div style="background: rgba(0, 188, 255, 0.08); padding: 8px 14px; border-bottom: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px;">
              
              <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                <span style="font-size: 14px;">🖼️</span>
                <strong style="color: #00bcff; font-size: 12px;">${escapeHtml(t('scanner.uasset_texture_preview_title') || 'GPU Texture Preview')}</strong>
                <span class="badge" style="font-size: 9.5px; padding: 2px 7px; background: rgba(0, 188, 255, 0.15); color: #00bcff; border: 1px solid rgba(0, 188, 255, 0.3); font-weight: 700;">${tex.width} × ${tex.height} px</span>
                <span class="badge" style="font-size: 9.5px; padding: 2px 7px; background: rgba(255, 209, 102, 0.15); color: #ffd166; border: 1px solid rgba(255, 209, 102, 0.3); font-weight: 700;">${escapeHtml(tex.formatName)}</span>
                <span class="badge" style="font-size: 9.5px; padding: 2px 7px; background: rgba(74, 222, 128, 0.15); color: #4ade80; border: 1px solid rgba(74, 222, 128, 0.3);">${tex.sourceFile} (${formatBytes(tex.sizeBytes)})</span>
              </div>

              <!-- Toolbar Controls -->
              <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                
                <!-- Channel Selector -->
                <div style="display: inline-flex; background: rgba(0,0,0,0.3); border: 1px solid var(--border); border-radius: 6px; padding: 2px;">
                  <button class="tex-channel-btn active" data-channel="rgba" style="background: var(--accent); color: #000; border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">RGBA</button>
                  <button class="tex-channel-btn" data-channel="rgb" style="background: none; color: var(--text-secondary); border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">RGB</button>
                  <button class="tex-channel-btn" data-channel="r" style="background: none; color: #f87171; border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">R</button>
                  <button class="tex-channel-btn" data-channel="g" style="background: none; color: #4ade80; border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">G</button>
                  <button class="tex-channel-btn" data-channel="b" style="background: none; color: #60a5fa; border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">B</button>
                  <button class="tex-channel-btn" data-channel="a" style="background: none; color: #c084fc; border: none; border-radius: 4px; padding: 2px 7px; font-size: 10px; font-weight: 700; cursor: pointer;">A</button>
                </div>

                <!-- Zoom Controls -->
                <div style="display: inline-flex; align-items: center; gap: 4px; background: rgba(0,0,0,0.3); border: 1px solid var(--border); border-radius: 6px; padding: 2px 6px;">
                  <button id="btn-tex-zoom-out" style="background: none; border: none; color: var(--text-primary); cursor: pointer; font-size: 12px; padding: 2px 4px;">🔍−</button>
                  <span id="tex-zoom-label" style="font-size: 10px; color: var(--text-muted); min-width: 32px; text-align: center; font-family: monospace;">100%</span>
                  <button id="btn-tex-zoom-in" style="background: none; border: none; color: var(--text-primary); cursor: pointer; font-size: 12px; padding: 2px 4px;">🔍+</button>
                  <button id="btn-tex-zoom-reset" style="background: none; border: none; color: var(--text-secondary); cursor: pointer; font-size: 10px; padding: 2px 4px; border-left: 1px solid var(--border);">1:1</button>
                  <button id="btn-tex-zoom-fit" style="background: none; border: none; color: var(--text-secondary); cursor: pointer; font-size: 10px; padding: 2px 4px;">⤢</button>
                </div>

                <!-- Action Export Buttons -->
                <button id="btn-tex-copy-img" class="btn btn-secondary btn-sm" style="font-size: 10px; padding: 3px 8px; display: inline-flex; align-items: center; gap: 4px;">
                  <span>📋</span> <span>${escapeHtml(t('scanner.uasset_texture_copy') || 'Copy')}</span>
                </button>
                <button id="btn-tex-save-png" class="btn btn-primary btn-sm" style="font-size: 10px; padding: 3px 10px; display: inline-flex; align-items: center; gap: 4px; font-weight: 700;">
                  <span>💾</span> <span>${escapeHtml(t('scanner.uasset_texture_save_png') || 'Save PNG')}</span>
                </button>

              </div>
            </div>

            <!-- Viewport Canvas Stage -->
            <div id="tex-viewport-stage" style="width: 100%; height: 350px; position: relative; overflow: hidden; display: flex; align-items: center; justify-content: center; cursor: grab; background-color: #121417; background-image: linear-gradient(45deg, #1d2127 25%, transparent 25%), linear-gradient(-45deg, #1d2127 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #1d2127 75%), linear-gradient(-45deg, transparent 75%, #1d2127 75%); background-size: 16px 16px; background-position: 0 0, 0 8px, 8px -8px, -8px 0px;">
              <div id="tex-transform-container" style="transform-origin: center center; transition: transform 0.05s ease-out; display: flex; align-items: center; justify-content: center;">
                <img id="tex-preview-img" src="${tex.dataUrl}" alt="${escapeHtml(details.assetName)}" style="max-height: 310px; max-width: 90%; object-fit: contain; box-shadow: 0 8px 24px rgba(0,0,0,0.6); pointer-events: none; border-radius: 4px;" />
              </div>
            </div>

          </div>
        `;

        // Wire Viewport Interactions (Zoom, Pan, Channels, Save, Copy)
        let zoom = 1.0;
        let panX = 0;
        let panY = 0;
        let isDragging = false;
        let startX = 0;
        let startY = 0;

        const stage = mainDom.elMaybe('tex-viewport-stage');
        const container = mainDom.elMaybe('tex-transform-container');
        const img = mainDom.elMaybe('tex-preview-img');
        const zoomLabel = mainDom.elMaybe('tex-zoom-label');

        const updateTransform = () => {
          if (!container) return;
          container.style.transform = `translate(${panX}px, ${panY}px) scale(${zoom})`;
          if (img) {
            img.style.imageRendering = zoom >= 1.5 ? 'pixelated' : 'auto';
          }
          if (zoomLabel) {
            zoomLabel.textContent = `${Math.round(zoom * 100)}%`;
          }
        };

        mainDom.elMaybe('btn-tex-zoom-in')?.addEventListener('click', () => {
          zoom = Math.min(zoom * 1.25, 8.0);
          updateTransform();
        });

        mainDom.elMaybe('btn-tex-zoom-out')?.addEventListener('click', () => {
          zoom = Math.max(zoom / 1.25, 0.15);
          updateTransform();
        });

        mainDom.elMaybe('btn-tex-zoom-reset')?.addEventListener('click', () => {
          zoom = 1.0;
          panX = 0;
          panY = 0;
          updateTransform();
        });

        mainDom.elMaybe('btn-tex-zoom-fit')?.addEventListener('click', () => {
          zoom = 0.85;
          panX = 0;
          panY = 0;
          updateTransform();
        });

        // Wheel Zoom
        stage?.addEventListener('wheel', (e) => {
          e.preventDefault();
          const delta = e.deltaY < 0 ? 1.15 : 0.85;
          zoom = Math.max(0.15, Math.min(8.0, zoom * delta));
          updateTransform();
        }, { passive: false });

        // Drag & Pan
        stage?.addEventListener('mousedown', (e) => {
          isDragging = true;
          startX = e.clientX - panX;
          startY = e.clientY - panY;
          if (stage) stage.style.cursor = 'grabbing';
        });

        window.addEventListener('mousemove', (e) => {
          if (!isDragging) return;
          panX = e.clientX - startX;
          panY = e.clientY - startY;
          updateTransform();
        });

        window.addEventListener('mouseup', () => {
          if (isDragging) {
            isDragging = false;
            if (stage) stage.style.cursor = 'grab';
          }
        });

        // Channel Filters
        const channelBtns = modalEl.querySelectorAll('.tex-channel-btn');
        channelBtns.forEach(btn => {
          btn.addEventListener('click', () => {
            const channel = (btn as HTMLElement).dataset.channel;
            channelBtns.forEach(b => {
              b.classList.remove('active');
              (b as HTMLElement).style.background = 'none';
              const ch = (b as HTMLElement).dataset.channel;
              (b as HTMLElement).style.color = ch === 'r' ? '#f87171' : ch === 'g' ? '#4ade80' : ch === 'b' ? '#60a5fa' : ch === 'a' ? '#c084fc' : 'var(--text-secondary)';
            });

            btn.classList.add('active');
            (btn as HTMLElement).style.background = 'var(--accent)';
            (btn as HTMLElement).style.color = '#000';

            if (!img) return;
            if (channel === 'rgba') {
              img.style.filter = 'none';
              img.style.opacity = '1';
            } else if (channel === 'rgb') {
              img.style.filter = 'none';
              img.style.opacity = '1';
            } else if (channel === 'r') {
              img.style.filter = 'sepia(1) hue-rotate(-50deg) saturate(6)';
            } else if (channel === 'g') {
              img.style.filter = 'sepia(1) hue-rotate(70deg) saturate(6)';
            } else if (channel === 'b') {
              img.style.filter = 'sepia(1) hue-rotate(180deg) saturate(6)';
            } else if (channel === 'a') {
              img.style.filter = 'grayscale(1) contrast(1.5)';
            }
          });
        });

        // Copy Image to Clipboard
        mainDom.elMaybe('btn-tex-copy-img')?.addEventListener('click', async () => {
          try {
            const image = new Image();
            image.src = tex.dataUrl;
            await image.decode();
            const canvas = document.createElement('canvas');
            canvas.width = image.naturalWidth;
            canvas.height = image.naturalHeight;
            const ctx = canvas.getContext('2d');
            ctx?.drawImage(image, 0, 0);
            canvas.toBlob(async (blob) => {
              if (blob) {
                await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
                showToast(t('scanner.uasset_texture_copied') || 'Texture copied to clipboard!', 'success');
              }
            });
          } catch {
            showToast(t('scanner.uasset_texture_copy_failed') || 'Failed to copy image to clipboard', 'error');
          }
        });

        // Save PNG
        mainDom.elMaybe('btn-tex-save-png')?.addEventListener('click', () => {
          try {
            const link = document.createElement('a');
            const cleanName = details.assetName.replace(/\.uasset$/i, '').replace(/\.uexp$/i, '').replace(/\.ubulk$/i, '');
            link.download = `${cleanName}.png`;
            link.href = tex.dataUrl;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            showToast(t('scanner.uasset_texture_saved', { name: `${cleanName}.png` }) || `Saved: ${cleanName}.png`, 'success');
          } catch (err) {
            showToast(String(err), 'error');
          }
        });

      } catch (texErr: any) {
        if (loaderEl) {
          loaderEl.innerHTML = `
            <span style="font-size: 24px;">⚠️</span>
            <span style="font-size: 11px; color: var(--danger);">${escapeHtml(t('scanner.uasset_texture_decode_failed') || 'Failed to decode texture payload')}: ${escapeHtml(String(texErr))}</span>
          `;
        }
      } finally {
        isTextureLoading = false;
      }
    };

    // Exports Filter
    const expInput = mainDom.elMaybe('uasset-exports-filter');
    if (expInput) {
      expInput.addEventListener('input', () => {
        const q = expInput.value.trim().toLowerCase();
        modalEl?.querySelectorAll('.uasset-export-row').forEach(row => {
          const search = (row as HTMLElement).dataset.search || '';
          (row as HTMLElement).style.display = (!q || search.includes(q)) ? 'flex' : 'none';
        });
      });
    }

    // Imports Filter
    const impInput = mainDom.elMaybe('uasset-imports-filter');
    if (impInput) {
      impInput.addEventListener('input', () => {
        const q = impInput.value.trim().toLowerCase();
        modalEl?.querySelectorAll('.uasset-import-row').forEach(row => {
          const search = (row as HTMLElement).dataset.search || '';
          (row as HTMLElement).style.display = (!q || search.includes(q)) ? 'flex' : 'none';
        });
      });
    }

    // Names Filter & Click-to-copy
    const nameInput = mainDom.elMaybe('uasset-names-filter');
    const nameCountEl = mainDom.elMaybe('uasset-names-count');
    const nameTags = modalEl?.querySelectorAll('.uasset-name-tag') || [];

    if (nameInput) {
      nameInput.addEventListener('input', () => {
        const q = nameInput.value.trim().toLowerCase();
        let visibleCount = 0;
        nameTags.forEach(tag => {
          const search = (tag as HTMLElement).dataset.search || '';
          const match = (!q || search.includes(q));
          (tag as HTMLElement).style.display = match ? 'inline-block' : 'none';
          if (match) visibleCount++;
        });
        if (nameCountEl) {
          nameCountEl.textContent = q
            ? (t('scanner.uasset_tokens_count_filtered', { visible: visibleCount, total: nameTags.length }) || `${visibleCount} / ${nameTags.length} tokens`)
            : (t('scanner.uasset_tokens_count', { count: nameTags.length }) || `${nameTags.length} tokens`);
        }
      });
    }

    // Click to copy name token
    nameTags.forEach(tag => {
      tag.addEventListener('click', async () => {
        const name = (tag as HTMLElement).dataset.name || tag.textContent || '';
        try {
          await navigator.clipboard.writeText(name);
          showToast(t('scanner.uasset_copied_token', { token: name }) || `Copied: "${name}"`, 'success');
        } catch {
          // Fallback
        }
      });
    });

    const closeActionBtn = mainDom.elMaybe('btn-close-uasset-action');
    closeActionBtn?.addEventListener('click', closeModal);

  } catch (err: any) {
    if (!bodyEl) return;
    bodyEl.innerHTML = `
      <div style="padding: 24px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px;">
        <span style="font-size: 28px;">⚠️</span>
        <strong style="color: var(--danger); font-size: 13px;">${escapeHtml(t('scanner.uasset_failed_inspect') || 'Failed to inspect asset')}</strong>
        <p style="font-size: 11px; color: var(--text-muted); max-width: 500px; line-height: 1.5; font-family: monospace;">${escapeHtml(String(err))}</p>
        <button id="btn-close-uasset-fail" class="btn-secondary" style="margin-top: 8px; padding: 5px 14px; font-size: 12px;">${escapeHtml(t('common.close') || 'Close')}</button>
      </div>
    `;
    mainDom.elMaybe('btn-close-uasset-fail')?.addEventListener('click', closeModal);
    showToast(t('scanner.uasset_failed_toast', { error: String(err) }) || `Asset inspection failed: ${String(err)}`, 'error');
  }
}
