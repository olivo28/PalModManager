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
    bodyEl.style.alignItems = 'stretch';
    bodyEl.style.justifyContent = 'flex-start';

    bodyEl.innerHTML = `
      <!-- Tabs Navigation -->
      <div style="background: rgba(0,0,0,0.2); border-bottom: 1px solid var(--border); display: flex; gap: 4px; padding: 6px 14px; overflow-x: auto;">
        <button class="uasset-tab-btn active" data-tab="tab-overview" style="background: var(--bg-card); border: 1px solid var(--border); border-bottom: none; color: var(--accent); font-weight: 700; font-size: 11.5px; padding: 6px 12px; border-radius: 6px 6px 0 0; cursor: pointer;">
          📊 ${escapeHtml(t('scanner.uasset_tab_overview') || 'Overview')}
        </button>
        <button class="uasset-tab-btn" data-tab="tab-exports" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 12px; border-radius: 6px 6px 0 0; cursor: pointer;">
          📤 ${escapeHtml(t('scanner.uasset_tab_exports') || 'Exports')} (${details.exports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-imports" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 12px; border-radius: 6px 6px 0 0; cursor: pointer;">
          📥 ${escapeHtml(t('scanner.uasset_tab_imports') || 'Imports')} (${details.imports.length})
        </button>
        <button class="uasset-tab-btn" data-tab="tab-names" style="background: none; border: 1px solid transparent; color: var(--text-secondary); font-size: 11.5px; padding: 6px 12px; border-radius: 6px 6px 0 0; cursor: pointer;">
          🔤 ${escapeHtml(t('scanner.uasset_tab_names') || 'Name Map')} (${details.summary.nameCount})
        </button>
      </div>

      <!-- Tab Content Area -->
      <div style="padding: 16px 18px; overflow-y: auto; max-height: calc(88vh - 125px); display: flex; flex-direction: column; gap: 14px;">
        
        <!-- Tab 1: Overview -->
        <div id="tab-overview" class="uasset-tab-pane" style="display: flex; flex-direction: column; gap: 12px;">
          <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(210px, 1fr)); gap: 10px;">
            
            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <span style="font-size: 10.5px; color: var(--text-muted); text-transform: uppercase; font-weight: 700;">Engine Version</span>
              <strong style="color: var(--accent); font-size: 12.5px;">${escapeHtml(details.engineVersion)}</strong>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <span style="font-size: 10.5px; color: var(--text-muted); text-transform: uppercase; font-weight: 700;">Asset Classification</span>
              <span class="badge" style="align-self: flex-start; font-size: 11px; padding: 2px 8px; background: rgba(0,188,255,0.15); color: #00bcff; border: 1px solid rgba(0,188,255,0.3);">${escapeHtml(details.assetType)}</span>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <span style="font-size: 10.5px; color: var(--text-muted); text-transform: uppercase; font-weight: 700;">Header Size (.uasset)</span>
              <strong style="color: var(--text-primary); font-family: monospace; font-size: 12px;">${formatBytes(details.summary.uassetSizeBytes)}</strong>
            </div>

            <div style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <span style="font-size: 10.5px; color: var(--text-muted); text-transform: uppercase; font-weight: 700;">Payload Size (.uexp)</span>
              <strong style="color: var(--text-primary); font-family: monospace; font-size: 12px;">${details.summary.uexpSizeBytes ? formatBytes(details.summary.uexpSizeBytes) : 'Embedded in .uasset'}</strong>
            </div>

          </div>

          <!-- Quick Metrics Bar -->
          <div style="background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 8px; padding: 12px 14px; display: flex; justify-content: space-around; align-items: center; text-align: center; flex-wrap: wrap; gap: 10px;">
            <div>
              <div style="font-size: 18px; font-weight: 700; color: #4af626;">${details.exports.length}</div>
              <div style="font-size: 10.5px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_exports') || 'Exported Objects')}</div>
            </div>
            <div style="width: 1px; height: 28px; background: var(--border);"></div>
            <div>
              <div style="font-size: 18px; font-weight: 700; color: #38bdf8;">${details.imports.length}</div>
              <div style="font-size: 10.5px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_imports') || 'Imported Dependencies')}</div>
            </div>
            <div style="width: 1px; height: 28px; background: var(--border);"></div>
            <div>
              <div style="font-size: 18px; font-weight: 700; color: #ffd166;">${details.summary.nameCount}</div>
              <div style="font-size: 10.5px; color: var(--text-muted);">${escapeHtml(t('scanner.uasset_tab_names') || 'Name Map Tokens')}</div>
            </div>
          </div>
        </div>

        <!-- Tab 2: Exports -->
        <div id="tab-exports" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <input type="text" id="uasset-exports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_exports') || 'Filter exports by object or class name...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11.5px; outline: none;" />
          
          <div id="uasset-exports-list" style="display: flex; flex-direction: column; gap: 4px; max-height: 380px; overflow-y: auto;">
            ${details.exports.length === 0 ? `
              <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 11.5px;">No exports found in asset header.</div>
            ` : details.exports.map(exp => `
              <div class="uasset-export-row" data-search="${escapeHtml((exp.objectName + ' ' + exp.className + ' ' + (exp.outerName || '')).toLowerCase())}" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; padding: 7px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                  <strong style="color: #4af626; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(exp.objectName)}</strong>
                  ${exp.outerName ? `<span style="font-size: 9.5px; color: var(--text-muted);">Parent: ${escapeHtml(exp.outerName)}</span>` : ''}
                </div>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(74, 246, 38, 0.12); color: #4af626; border: 1px solid rgba(74, 246, 38, 0.25); white-space: nowrap;">${escapeHtml(exp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Tab 3: Imports -->
        <div id="tab-imports" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <input type="text" id="uasset-imports-filter" placeholder="${escapeHtml(t('scanner.uasset_search_imports') || 'Filter imports by dependency or package...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11.5px; outline: none;" />
          
          <div id="uasset-imports-list" style="display: flex; flex-direction: column; gap: 4px; max-height: 380px; overflow-y: auto;">
            ${details.imports.length === 0 ? `
              <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 11.5px;">No external imports found.</div>
            ` : details.imports.map(imp => `
              <div class="uasset-import-row" data-search="${escapeHtml((imp.objectName + ' ' + imp.className + ' ' + imp.classPackage).toLowerCase())}" style="background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; padding: 7px 10px; display: flex; justify-content: space-between; align-items: center; gap: 10px; font-family: monospace; font-size: 11px;">
                <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                  <strong style="color: #38bdf8; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(imp.objectName)}</strong>
                  <span style="font-size: 9.5px; color: var(--text-muted); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">Package: ${escapeHtml(imp.classPackage)}</span>
                </div>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(imp.className)}</span>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Tab 4: Name Map -->
        <div id="tab-names" class="uasset-tab-pane" style="display: none; flex-direction: column; gap: 8px;">
          <input type="text" id="uasset-names-filter" placeholder="${escapeHtml(t('scanner.uasset_search_names') || 'Search name table tokens and identifiers...')}" style="width: 100%; background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); border-radius: 4px; padding: 6px 10px; font-size: 11.5px; outline: none;" />
          
          <div id="uasset-names-cloud" style="display: flex; flex-wrap: wrap; gap: 6px; max-height: 380px; overflow-y: auto; padding: 4px;">
            ${details.namesSample.map(name => `
              <span class="uasset-name-tag" data-search="${escapeHtml(name.toLowerCase())}" style="font-family: monospace; font-size: 10.5px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: 4px; padding: 3px 7px; color: #ffd166;">${escapeHtml(name)}</span>
            `).join('')}
          </div>
        </div>

      </div>

      <!-- Footer -->
      <div style="background: var(--bg-secondary); padding: 10px 18px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end;">
        <button id="btn-close-uasset-action" class="btn-secondary" style="padding: 5px 14px; font-size: 12px;">${escapeHtml(t('common.close') || 'Close')}</button>
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

    // Names Filter
    const nameInput = document.getElementById('uasset-names-filter') as HTMLInputElement | null;
    if (nameInput) {
      nameInput.addEventListener('input', () => {
        const q = nameInput.value.trim().toLowerCase();
        modalEl.querySelectorAll('.uasset-name-tag').forEach(tag => {
          const search = (tag as HTMLElement).dataset.search || '';
          (tag as HTMLElement).style.display = (!q || search.includes(q)) ? 'inline-block' : 'none';
        });
      });
    }

    const closeActionBtn = document.getElementById('btn-close-uasset-action');
    closeActionBtn?.addEventListener('click', closeModal);

  } catch (err: any) {
    if (!bodyEl) return;
    bodyEl.innerHTML = `
      <div style="padding: 24px; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 8px;">
        <span style="font-size: 28px;">⚠️</span>
        <strong style="color: var(--danger); font-size: 13px;">Failed to inspect asset</strong>
        <p style="font-size: 11px; color: var(--text-muted); max-width: 500px; line-height: 1.5; font-family: monospace;">${escapeHtml(String(err))}</p>
        <button id="btn-close-uasset-fail" class="btn-secondary" style="margin-top: 8px; padding: 5px 14px; font-size: 12px;">${escapeHtml(t('common.close') || 'Close')}</button>
      </div>
    `;
    document.getElementById('btn-close-uasset-fail')?.addEventListener('click', closeModal);
    showToast(`Asset inspection failed: ${String(err)}`, 'error');
  }
}
