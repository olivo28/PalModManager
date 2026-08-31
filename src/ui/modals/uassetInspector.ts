import { inspectUAssetDeep, type UAssetInspectionDetails } from '../../api';
import { t } from '../../utils/i18n';
import { escapeHtml, formatBytes } from '../../utils/helpers';
import { showToast } from '../toast';

export async function openUAssetInspectorModal(params: {
  modId?: string | null;
  pakPath?: string | null;
  assetInternalPath: string;
  zipPath?: string | null;
}): Promise<void> {
  const existing = document.getElementById('uasset-inspector-modal');
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

  const modalEl = document.getElementById('uasset-inspector-modal');
  const closeBtn = document.getElementById('btn-close-uasset-modal');
  const bodyEl = document.getElementById('uasset-modal-body');

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
            <div style="background: rgba(0, 188, 255, 0.05); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: 8px; padding: 10px 14px; display: flex; align-items: center; gap: 12px;">
              <span style="font-size: 24px; flex-shrink: 0;">🖼️</span>
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <span style="font-size: 11.5px; font-weight: 700; color: #00bcff;">${escapeHtml(t('scanner.uasset_texture_title') || 'DirectDraw Surface / GPU Texture2D Binary')}</span>
                <span style="font-size: 10px; color: var(--text-secondary); line-height: 1.4;">${escapeHtml(t('scanner.uasset_texture_desc', { size: details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : (t('scanner.uasset_embedded_in_uasset') || 'Embedded') }) || `Cooked texture streaming resource. Raw mipmaps and BC7/DXT compressed pixel buffers are embedded in the .uexp payload (${details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : 'Embedded'}).`)}</span>
              </div>
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
                  <span style="font-size: 9px; color: var(--text-muted); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(t('scanner.uasset_package_prefix') || 'Package')}: ${escapeHtml(imp.classPackage)}</span>
                </div>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(imp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Tab 4: Name Map -->
        <div id="tab-names" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <div style="display: flex; gap: 8px; align-items: center;">
            <input type="text" id="uasset-names-filter" placeholder="${escapeHtml(t('scanner.uasset_search_names') || 'Search name table tokens and identifiers...')}" style="flex: 1; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />
            <span id="uasset-names-count" style="font-size: 10.5px; color: var(--text-muted); white-space: nowrap; font-family: monospace;">${escapeHtml(t('scanner.uasset_tokens_count', { count: details.namesSample.length }) || `${details.namesSample.length} tokens`)}</span>
          </div>
          
          <div id="uasset-names-cloud" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 10px; max-height: 240px; min-height: 80px; overflow-y: auto; display: flex; flex-wrap: wrap; gap: 6px; align-content: flex-start;">
            ${details.namesSample.map((name, idx) => {
              const isPath = name.startsWith('/') || name.includes('/');
              const isClass = name.startsWith('BP_') || name.endsWith('_C') || name.startsWith('WBP_') || name.includes('Class') || name.includes('Struct');
              const isText = name.includes(' ') && name.length > 12;
              
              const color = isPath ? '#38bdf8' : isClass ? '#c084fc' : isText ? '#4ade80' : '#ffd166';
              const bg = isPath ? 'rgba(56, 189, 248, 0.08)' : isClass ? 'rgba(192, 132, 252, 0.08)' : isText ? 'rgba(74, 222, 128, 0.08)' : 'rgba(255, 209, 102, 0.08)';
              const border = isPath ? 'rgba(56, 189, 248, 0.25)' : isClass ? 'rgba(192, 132, 252, 0.25)' : isText ? 'rgba(74, 222, 128, 0.25)' : 'rgba(255, 209, 102, 0.25)';

              return `
                <button class="uasset-name-tag" data-name="${escapeHtml(name)}" data-search="${escapeHtml(name.toLowerCase())}" title="${escapeHtml(t('scanner.uasset_click_to_copy') || 'Click to copy')}" style="font-family: monospace; font-size: 11px; background: ${bg}; border: 1px solid ${border}; border-radius: 4px; padding: 3px 8px; color: ${color}; cursor: pointer; display: inline-flex; align-items: center; gap: 5px; transition: transform 0.1s, border-color 0.1s;">
                  <span style="font-size: 9px; opacity: 0.6; font-weight: normal;">#${idx}</span>
                  <span>${escapeHtml(name)}</span>
                </button>
              `;
            }).join('')}
          </div>
        </div>

        <!-- Tab 5: Schema Structure (USMAP Resolved) -->
        <div id="tab-schema" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          ${details.resolvedSchema ? `
            <div style="background: rgba(56, 189, 248, 0.06); border: 1px solid rgba(56, 189, 248, 0.25); border-radius: 8px; padding: 10px 14px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <div style="font-size: 12.5px; font-weight: 700; color: #38bdf8; font-family: monospace;">${escapeHtml(details.resolvedSchema.matchedStructName)}</div>
                <div style="font-size: 10px; color: var(--text-muted);">${details.resolvedSchema.superType ? `Extends: <span style="color: var(--text-primary); font-family: monospace;">${escapeHtml(details.resolvedSchema.superType)}</span> • ` : ''}${escapeHtml(details.resolvedSchema.gameVersion)}</div>
              </div>
              <span class="badge" style="font-size: 10px; padding: 2px 8px; background: rgba(56,189,248,0.15); color: #38bdf8; border: 1px solid rgba(56,189,248,0.3); font-weight: 700;">${details.resolvedSchema.totalProperties} ${escapeHtml(t('scanner.uasset_schema_props_count') || 'Properties Resolved')}</span>
            </div>

            <input type="text" id="uasset-schema-filter" placeholder="${escapeHtml(t('scanner.uasset_search_schema') || 'Filter schema properties by name or type...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11px; outline: none;" />

            <div id="uasset-schema-list" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; padding: 6px; max-height: 240px; min-height: 80px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px;">
              ${details.resolvedSchema.properties.map(prop => `
                <div class="uasset-schema-row" data-search="${escapeHtml((prop.name + ' ' + prop.typeName + ' ' + (prop.structType || '') + ' ' + (prop.enumType || '')).toLowerCase())}" style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                  <div style="display: flex; align-items: center; gap: 8px; overflow: hidden;">
                    <span style="font-size: 9px; color: var(--text-muted); min-width: 24px;">#${prop.index}</span>
                    <strong style="color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(prop.name)}</strong>
                  </div>
                  <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                    ${prop.structType ? `<span style="font-size: 9.5px; color: #c084fc;">📦 ${escapeHtml(prop.structType)}</span>` : ''}
                    ${prop.enumType ? `<span style="font-size: 9.5px; color: #ffd166;">🔢 ${escapeHtml(prop.enumType)}</span>` : ''}
                    ${prop.innerType ? `<span style="font-size: 9.5px; color: #4ade80;">[${escapeHtml(prop.innerType)}]</span>` : ''}
                    <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(prop.typeName)}</span>
                  </div>
                </div>
              `).join('')}
            </div>
          ` : `
            <div style="background: rgba(56, 189, 248, 0.04); border: 1px solid rgba(56, 189, 248, 0.18); border-radius: 8px; padding: 18px 20px; display: flex; flex-direction: column; gap: 8px; text-align: left;">
              <div style="display: flex; align-items: center; gap: 8px; color: #38bdf8; font-weight: 700; font-size: 12.5px;">
                <span>ℹ️</span>
                <span>${escapeHtml(t('scanner.uasset_schema_custom_blueprint_title') || 'Custom Blueprint / UI Widget Asset')}</span>
              </div>
              <div style="font-size: 11px; color: var(--text-secondary); line-height: 1.5;">
                ${escapeHtml(t('scanner.uasset_schema_custom_blueprint_desc') || 'This asset is a custom visual Blueprint or Widget generated by the mod. Unversioned property schemas from USMAP are actively mapped for Unreal Engine DataTables, game structs, characters, items, and native C++ classes.')}
              </div>
              <div style="margin-top: 4px; padding: 8px 12px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 6px; font-size: 10.5px; color: var(--text-muted); display: flex; align-items: center; justify-content: space-between;">
                <span>📚 ${escapeHtml(t('scanner.uasset_active_schema_label') || 'Active Schema Catalog')}: <strong style="color: var(--text-primary);">Palworld v1.0.3</strong></span>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(74, 246, 38, 0.12); color: #4af626; border: 1px solid rgba(74, 246, 38, 0.25);">54,800+ Tokens Active</span>
              </div>
            </div>
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

    tabBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const targetTab = (btn as HTMLElement).dataset.tab;
        tabBtns.forEach(b => {
          b.classList.remove('active');
          (b as HTMLElement).style.background = 'none';
          (b as HTMLElement).style.borderColor = 'transparent';
          (b as HTMLElement).style.color = 'var(--text-secondary)';
          (b as HTMLElement).style.fontWeight = 'normal';
        });
        tabPanes.forEach(p => {
          (p as HTMLElement).style.display = 'none';
        });

        btn.classList.add('active');
        (btn as HTMLElement).style.background = 'var(--bg-card)';
        (btn as HTMLElement).style.border = '1px solid var(--border)';
        (btn as HTMLElement).style.borderBottom = 'none';
        (btn as HTMLElement).style.color = 'var(--accent)';
        (btn as HTMLElement).style.fontWeight = '700';

        const targetPane = document.getElementById(targetTab || '');
        if (targetPane) targetPane.style.display = 'flex';
      });
    });

    // Exports Filter
    const expInput = document.getElementById('uasset-exports-filter') as HTMLInputElement | null;
    if (expInput) {
      expInput.addEventListener('input', () => {
        const q = expInput.value.trim().toLowerCase();
        modalEl.querySelectorAll('.uasset-export-row').forEach(row => {
          const search = (row as HTMLElement).dataset.search || '';
          (row as HTMLElement).style.display = (!q || search.includes(q)) ? 'flex' : 'none';
        });
      });
    }

    // Imports Filter
    const impInput = document.getElementById('uasset-imports-filter') as HTMLInputElement | null;
    if (impInput) {
      impInput.addEventListener('input', () => {
        const q = impInput.value.trim().toLowerCase();
        modalEl.querySelectorAll('.uasset-import-row').forEach(row => {
          const search = (row as HTMLElement).dataset.search || '';
          (row as HTMLElement).style.display = (!q || search.includes(q)) ? 'flex' : 'none';
        });
      });
    }

    // Names Filter & Click-to-copy
    const nameInput = document.getElementById('uasset-names-filter') as HTMLInputElement | null;
    const nameCountEl = document.getElementById('uasset-names-count');
    const nameTags = modalEl.querySelectorAll('.uasset-name-tag');

    if (nameInput) {
      nameInput.addEventListener('input', () => {
        const q = nameInput.value.trim().toLowerCase();
        let visibleCount = 0;
        nameTags.forEach(tag => {
          const search = (tag as HTMLElement).dataset.search || '';
          const match = (!q || search.includes(q));
          (tag as HTMLElement).style.display = match ? 'inline-flex' : 'none';
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

    const closeActionBtn = document.getElementById('btn-close-uasset-action');
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
    document.getElementById('btn-close-uasset-fail')?.addEventListener('click', closeModal);
    showToast(t('scanner.uasset_failed_toast', { error: String(err) }) || `Asset inspection failed: ${String(err)}`, 'error');
  }
}
