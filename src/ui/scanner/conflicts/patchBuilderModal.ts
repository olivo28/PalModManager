import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import {
  buildCompatibilityPak,
  listGeneratedPatches,
  deleteGeneratedPatch,
  GeneratedPatchInfo,
  PatchAssetSelection
} from '../../../api';
import { getState } from '../../../state';
import { scannerDom } from '../../../framework';

interface ConflictingModEntry {
  modName: string;
  modFolder: string;
  pakFilename: string;
  pakPath: string;
}

export interface PakConflictItem {
  assetName: string;
  internalPath: string;
  assetType: string;
  mods: ConflictingModEntry[];
  resolvedByPatch?: string | null;
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function getAssetTypeBadgeClass(type: string): string {
  const norm = type.toLowerCase();
  if (norm.includes('datatable') || norm.includes('json') || norm.includes('table')) {
    return 'datatable';
  }
  if (norm.includes('blueprint') || norm.includes('logic')) {
    return 'blueprint';
  }
  return 'lua';
}

/**
 * Opens the Manual Pak Compatibility Patch Builder Modal
 */
export async function openPatchBuilderModal(pakConflicts: PakConflictItem[]): Promise<void> {
  const existingModal = scannerDom.elMaybe('pmm-patch-builder-modal-overlay');
  if (existingModal) existingModal.remove();

  const state = getState();
  const gamePath = state.currentSettings?.gamePath || '';
  const isGamepassDetected = gamePath.toLowerCase().includes('wingdk') || 
                             gamePath.toLowerCase().includes('xbox') || false;

  // Track user selections: map of internalPath -> sourcePakPath (or null for skip)
  const selections: Map<string, string | null> = new Map();

  // Default selection: select the first mod for each conflict
  pakConflicts.forEach(c => {
    if (c.mods && c.mods.length > 0) {
      selections.set(c.internalPath, c.mods[0].pakPath);
    }
  });

  // Extract list of all unique mods involved
  const allModsMap = new Map<string, string>();
  pakConflicts.forEach(c => {
    c.mods.forEach(m => {
      allModsMap.set(m.modName, m.pakPath);
    });
  });
  const uniqueMods = Array.from(allModsMap.entries());

  const overlay = document.createElement('div');
  overlay.id = 'pmm-patch-builder-modal-overlay';
  overlay.className = 'modal-overlay visible';
  overlay.style.zIndex = '3500';

  overlay.innerHTML = `
    <div class="modal" style="width: 860px; max-width: 95vw; max-height: 90vh;">
      
      <!-- Header -->
      <div class="modal-header">
        <div style="display: flex; align-items: center; gap: 10px;">
          <span style="font-size: 20px;">🛠️</span>
          <div>
            <h3 style="margin: 0; font-size: 15px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.patch_builder_title') || 'Manual Pak Compatibility Patch Builder')}</h3>
            <div style="font-size: 11.5px; color: var(--text-secondary); margin-top: 1px;">${escapeHtml(t('scanner.patch_builder_subtitle', { count: pakConflicts.length }) || `Resolve ${pakConflicts.length} asset collision(s) with a dedicated compatibility patch`)}</div>
          </div>
        </div>
        <button id="btn-close-patch-builder" class="modal-close-btn" title="Close (Esc)">✕</button>
      </div>

      <!-- Body -->
      <div class="modal-body" style="padding: 18px 20px; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 16px;">
        
        <!-- Explanation Info Callout -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: var(--card-radius); padding: 12px 14px; font-size: 12px; color: var(--text-secondary); line-height: 1.5; display: flex; gap: 10px; align-items: center;">
          <span style="font-size: 18px; flex-shrink: 0;">💡</span>
          <div>
            <span>${escapeHtml(t('scanner.patch_builder_info') || 'Choose which mod wins for each conflicting asset. PalModManager will extract the selected files and assemble a unified priority patch')}</span>
            <code style="background: var(--bg-primary); border: 1px solid var(--border); color: var(--text-primary); padding: 1px 6px; border-radius: var(--radius); font-family: monospace; font-weight: 700; font-size: 11px; margin: 0 4px;">zzz_PMM_Patch_*_P.pak</code>
            <span>${escapeHtml(t('scanner.patch_builder_info_suffix') || 'that overrides conflicts without modifying the original source mods.')}</span>
          </div>
        </div>

        <!-- Configuration Card -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: 14px 16px; display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 11.5px; font-weight: 700; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px;">${escapeHtml(t('scanner.patch_config_heading') || 'Patch Configuration')}</div>
          
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px; align-items: start;">
            <!-- Patch Name Input -->
            <div>
              <label style="font-size: 11px; color: var(--text-secondary); font-weight: 600; display: block; margin-bottom: 5px;">${escapeHtml(t('scanner.patch_filename_label') || 'Patch File Name')}</label>
              <input id="patch-filename-input" type="text" class="input" value="zzz_PMM_Patch_Compat_P" placeholder="zzz_PMM_Patch_Compat_P" style="width: 100%; font-size: 12px; padding: 7px 10px; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); font-family: monospace; outline: none; box-sizing: border-box;" />
              <span style="font-size: 10px; color: var(--text-muted); margin-top: 4px; display: block;">${escapeHtml(t('scanner.patch_filename_hint') || 'Saved as .pak in Pal/Content/Paks with maximum override priority')}</span>
            </div>

            <!-- Game Pass Toggle Card -->
            <div>
              <label style="font-size: 11px; color: var(--text-secondary); font-weight: 600; display: block; margin-bottom: 5px;">Xbox Game Pass (WinGDK)</label>
              <label style="display: flex; align-items: center; gap: 10px; cursor: pointer; user-select: none; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius); padding: 7px 10px;">
                <input id="patch-gamepass-toggle" type="checkbox" ${isGamepassDetected ? 'checked' : ''} style="cursor: pointer;" />
                <div style="display: flex; flex-direction: column;">
                  <span style="font-size: 11.5px; font-weight: 600; color: var(--text-primary);">${escapeHtml(t('scanner.patch_gamepass_retoc') || 'Convert to IoStore (.utoc / .ucas via retoc)')}</span>
                  <span style="font-size: 9.5px; color: var(--text-muted);">${escapeHtml(t('scanner.patch_gamepass_hint') || 'Required for Game Pass WinGDK compatibility')}</span>
                </div>
              </label>
            </div>
          </div>

          <!-- Quick Mass Selection Buttons -->
          ${uniqueMods.length > 1 ? `
            <div style="display: flex; align-items: center; gap: 8px; margin-top: 2px; padding-top: 10px; border-top: 1px dashed var(--border); flex-wrap: wrap;">
              <span style="font-size: 11px; color: var(--text-secondary); font-weight: 600;">${escapeHtml(t('scanner.patch_quick_select') || 'Quick Winner Selection:')}</span>
              ${uniqueMods.map(([mName, mPakPath]) => `
                <button class="btn btn-secondary btn-sm btn-bulk-select" data-pakpath="${escapeHtml(mPakPath)}" style="font-size: 11px; font-weight: 600; padding: 4px 10px; display: inline-flex; align-items: center; gap: 5px;">
                  <span>🏆</span> <span>${escapeHtml(t('scanner.patch_all_from', { mod: mName }) || `All from ${mName}`)}</span>
                </button>
              `).join('')}
            </div>
          ` : ''}
        </div>

        <!-- Asset Selection Matrix -->
        <div style="display: flex; flex-direction: column; gap: 10px;">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <div style="font-size: 11.5px; font-weight: 700; color: var(--text-primary); text-transform: uppercase; letter-spacing: 0.5px;">
              ${escapeHtml(t('scanner.patch_assets_matrix_heading', { count: pakConflicts.length }) || `Conflicting Assets Selection (${pakConflicts.length})`)}
            </div>
            <span style="font-size: 11px; color: var(--text-muted);">${escapeHtml(t('scanner.patch_select_winner_hint') || 'Select the winning provider for each asset')}</span>
          </div>

          <div id="patch-asset-items-list" style="display: flex; flex-direction: column; gap: 10px;">
            ${pakConflicts.map((conflict, idx) => {
              const badgeClass = getAssetTypeBadgeClass(conflict.assetType);
              const currentSelectedPak = selections.get(conflict.internalPath);

              return `
                <div class="patch-asset-row-card" data-asset-path="${escapeHtml(conflict.internalPath)}" style="background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: 12px 14px; display: flex; flex-direction: column; gap: 8px;">
                  
                  <div style="display: flex; justify-content: space-between; align-items: center; gap: 10px;">
                    <div style="display: flex; align-items: center; gap: 8px; min-width: 0;">
                      <span style="font-size: 13px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(conflict.assetName)}</span>
                      <span class="scanner-conflict-type ${badgeClass}">${escapeHtml(conflict.assetType)}</span>
                    </div>
                  </div>

                  <div style="font-size: 10.5px; color: var(--text-secondary); font-family: monospace; background: var(--bg-primary); padding: 4px 8px; border-radius: var(--radius); border: 1px solid var(--border); display: flex; align-items: center; gap: 6px; word-break: break-all;">
                    <span>📄</span> <span>${escapeHtml(conflict.internalPath)}</span>
                  </div>

                  <!-- Selectable Choice Pills -->
                  <div style="display: flex; gap: 8px; flex-wrap: wrap; margin-top: 2px; padding-top: 6px; border-top: 1px solid var(--border);">
                    ${conflict.mods.map(m => {
                      const isSelected = currentSelectedPak === m.pakPath;

                      return `
                        <label class="patch-mod-option-pill" style="display: flex; align-items: center; gap: 7px; font-size: 11.5px; padding: 6px 12px; background: ${isSelected ? 'var(--accent-dim)' : 'var(--bg-primary)'}; border: 1.5px solid ${isSelected ? 'var(--accent)' : 'var(--border)'}; border-radius: var(--radius); cursor: pointer; color: ${isSelected ? 'var(--text-primary)' : 'var(--text-secondary)'}; user-select: none; transition: all 0.15s ease;">
                          <input type="radio" name="asset-opt-${idx}" value="${escapeHtml(m.pakPath)}" data-asset-path="${escapeHtml(conflict.internalPath)}" ${isSelected ? 'checked' : ''} style="cursor: pointer;" />
                          <span style="font-weight: 600;">${escapeHtml(m.modName)}</span>
                          <span style="font-size: 10.5px; color: var(--text-muted); font-family: monospace;">(${escapeHtml(m.pakFilename)})</span>
                        </label>
                      `;
                    }).join('')}
                    
                    <label class="patch-mod-option-pill skip-pill" style="display: flex; align-items: center; gap: 7px; font-size: 11.5px; padding: 6px 12px; background: ${currentSelectedPak === null ? 'var(--danger-dim)' : 'var(--bg-primary)'}; border: 1.5px solid ${currentSelectedPak === null ? 'var(--danger)' : 'var(--border)'}; border-radius: var(--radius); cursor: pointer; color: ${currentSelectedPak === null ? 'var(--danger)' : 'var(--text-muted)'}; user-select: none; transition: all 0.15s ease;">
                      <input type="radio" name="asset-opt-${idx}" value="__SKIP__" data-asset-path="${escapeHtml(conflict.internalPath)}" ${currentSelectedPak === null ? 'checked' : ''} style="cursor: pointer;" />
                      <span>${escapeHtml(t('scanner.patch_skip_asset') || 'Skip from patch')}</span>
                    </label>
                  </div>
                </div>
              `;
            }).join('')}
          </div>
        </div>

        <!-- Progress status indicator -->
        <div id="patch-build-progress-box" style="display: none; background: var(--bg-card); border: 1px solid var(--accent); border-radius: var(--card-radius); padding: 12px 16px; align-items: center; gap: 12px;">
          <div class="spinner" style="width: 18px; height: 18px; border-width: 2px; flex-shrink: 0;"></div>
          <div style="font-size: 12px; color: var(--text-primary); font-weight: 600;" id="patch-build-progress-text">
            ${escapeHtml(t('scanner.patch_building_progress') || 'Extracting selected assets and packing compatibility .pak...')}
          </div>
        </div>

      </div>

      <!-- Footer -->
      <div class="modal-footer" style="display: flex; justify-content: space-between; align-items: center;">
        <button id="btn-view-managed-patches" class="btn btn-secondary" style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; font-weight: 600;">
          <span>📋</span> <span>${escapeHtml(t('scanner.btn_view_existing_patches') || 'Existing Patches')}</span>
        </button>

        <div style="display: flex; align-items: center; gap: 8px;">
          <button id="btn-cancel-patch-builder" class="btn btn-secondary" style="font-size: 12px; font-weight: 600;">
            ${escapeHtml(t('common.cancel') || 'Cancel')}
          </button>
          <button id="btn-build-patch-confirm" class="btn btn-primary" style="display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 700;">
            <span>🔨</span> <span>${escapeHtml(t('scanner.btn_build_patch_now') || 'Build Compatibility Patch')}</span>
          </button>
        </div>
      </div>

    </div>
  `;

  document.body.appendChild(overlay);

  // Close handlers and ESC key support
  const closeModal = () => {
    window.removeEventListener('keydown', handleKeyDown);
    overlay.remove();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      closeModal();
    }
  };
  window.addEventListener('keydown', handleKeyDown);

  scannerDom.elMaybe('btn-close-patch-builder')?.addEventListener('click', closeModal);
  scannerDom.elMaybe('btn-cancel-patch-builder')?.addEventListener('click', closeModal);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) closeModal();
  });

  // Radio button click handlers
  overlay.querySelectorAll<HTMLInputElement>('input[type="radio"]').forEach(radio => {
    radio.addEventListener('change', () => {
      const assetPath = radio.dataset.assetPath;
      if (!assetPath) return;
      const val = radio.value;
      selections.set(assetPath, val === '__SKIP__' ? null : val);

      // Refresh visual highlight on sibling labels
      const parentRow = radio.closest('.patch-asset-row-card');
      if (parentRow) {
        parentRow.querySelectorAll('.patch-mod-option-pill').forEach(lbl => {
          const r = lbl.querySelector('input');
          if (r?.checked) {
            const isSkip = r.value === '__SKIP__';
            (lbl as HTMLElement).style.background = isSkip ? 'var(--danger-dim)' : 'var(--accent-dim)';
            (lbl as HTMLElement).style.borderColor = isSkip ? 'var(--danger)' : 'var(--accent)';
            (lbl as HTMLElement).style.color = isSkip ? 'var(--danger)' : 'var(--text-primary)';
          } else {
            (lbl as HTMLElement).style.background = 'var(--bg-primary)';
            (lbl as HTMLElement).style.borderColor = 'var(--border)';
            (lbl as HTMLElement).style.color = 'var(--text-secondary)';
          }
        });
      }
    });
  });

  // Bulk selection buttons
  overlay.querySelectorAll<HTMLButtonElement>('.btn-bulk-select').forEach(btn => {
    btn.addEventListener('click', () => {
      const targetPak = btn.dataset.pakpath;
      if (!targetPak) return;

      pakConflicts.forEach(c => {
        const hasMod = c.mods.some(m => m.pakPath === targetPak);
        if (hasMod) {
          selections.set(c.internalPath, targetPak);
        }
      });

      // Update all radio checks in DOM
      overlay.querySelectorAll<HTMLInputElement>('input[type="radio"]').forEach(r => {
        const assetPath = r.dataset.assetPath;
        if (assetPath && selections.get(assetPath) === r.value) {
          r.checked = true;
          r.dispatchEvent(new Event('change'));
        }
      });
    });
  });

  // Existing Patches viewer button
  scannerDom.elMaybe('btn-view-managed-patches')?.addEventListener('click', () => {
    openExistingPatchesModal();
  });

  // Build confirmation handler
  scannerDom.elMaybe('btn-build-patch-confirm')?.addEventListener('click', async () => {
    const filenameInput = scannerDom.elMaybe('patch-filename-input');
    const gamepassToggle = scannerDom.elMaybe('patch-gamepass-toggle');
    const progressBox = scannerDom.elMaybe('patch-build-progress-box');
    const buildBtn = scannerDom.elMaybe('btn-build-patch-confirm');

    const patchName = filenameInput?.value?.trim() || 'zzz_PMM_Patch_Compat_P';
    const isGamepass = gamepassToggle?.checked || false;

    // Filter active selections (skip nulls)
    const selectedEntries: PatchAssetSelection[] = [];
    selections.forEach((pakPath, assetPath) => {
      if (pakPath) {
        selectedEntries.push({
          assetPath,
          sourcePakPath: pakPath,
        });
      }
    });

    if (selectedEntries.length === 0) {
      showToast(t('scanner.err_no_assets_selected') || 'Please select at least one asset to include in the patch.', 'warning');
      return;
    }

    try {
      if (progressBox) progressBox.style.display = 'flex';
      if (buildBtn) buildBtn.disabled = true;

      const res = await buildCompatibilityPak({
        patchName,
        selections: selectedEntries,
        isGamepass,
      });

      showToast(
        t('scanner.patch_created_success', {
          name: res.patchName,
          count: res.totalAssetsPacked,
          size: formatBytes(res.fileSizeBytes)
        }) || `Compatibility patch '${res.patchName}' built successfully (${res.totalAssetsPacked} assets, ${formatBytes(res.fileSizeBytes)})!`,
        'success'
      );

      closeModal();

      // Trigger quiet scan refresh & mods state refresh
      const { runScan } = await import('./runner');
      runScan();
      const { loadMods } = await import('../../modsView');
      loadMods();
    } catch (err: any) {
      console.error('buildCompatibilityPak failed:', err);
      showToast(t('scanner.patch_build_failed', { error: String(err) }) || `Failed to build patch: ${err}`, 'error');
    } finally {
      if (progressBox) progressBox.style.display = 'none';
      if (buildBtn) buildBtn.disabled = false;
    }
  });
}

/**
 * Modal to view and manage existing generated compatibility patches
 */
export async function openExistingPatchesModal(): Promise<void> {
  const existing = scannerDom.elMaybe('pmm-existing-patches-modal');
  if (existing) existing.remove();

  let patches: GeneratedPatchInfo[] = [];
  try {
    patches = await listGeneratedPatches();
  } catch (err) {
    console.error('Failed to list generated patches:', err);
  }

  const overlay = document.createElement('div');
  overlay.id = 'pmm-existing-patches-modal';
  overlay.className = 'modal-overlay visible';
  overlay.style.zIndex = '3600';

  const renderContent = () => `
    <div class="modal" style="width: 700px; max-width: 95vw; max-height: 80vh;">
      
      <div class="modal-header">
        <div style="display: flex; align-items: center; gap: 10px;">
          <span style="font-size: 18px;">📋</span>
          <div>
            <h3 style="margin: 0; font-size: 15px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.existing_patches_title') || 'Managed Compatibility Patches')}</h3>
            <div style="font-size: 11.5px; color: var(--text-secondary); margin-top: 1px;">${escapeHtml(t('scanner.existing_patches_subtitle') || 'Active custom patches in Pal/Content/Paks')}</div>
          </div>
        </div>
        <button id="btn-close-existing-patches" class="modal-close-btn" title="Close (Esc)">✕</button>
      </div>

      <div class="modal-body" style="padding: 16px 20px; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 10px;">
        ${patches.length === 0 ? `
          <div style="text-align: center; padding: 40px 20px; color: var(--text-muted); font-size: 12px; display: flex; flex-direction: column; align-items: center; gap: 10px; background: var(--bg-card); border: 1px dashed var(--border); border-radius: var(--card-radius);">
            <div style="font-size: 28px;">📭</div>
            <div style="font-weight: 600; color: var(--text-primary); font-size: 13px;">${escapeHtml(t('scanner.no_existing_patches') || 'No generated compatibility patches found in Paks folder.')}</div>
            <div style="font-size: 11px; color: var(--text-muted); max-width: 360px;">${escapeHtml(t('scanner.no_existing_patches_hint') || 'Use the Compatibility Patch Builder to generate unified overrides for conflicting assets.')}</div>
          </div>
        ` : patches.map(p => `
          <div class="existing-patch-card" style="display: flex; justify-content: space-between; align-items: center; padding: 12px 14px; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); gap: 12px;">
            <div style="display: flex; align-items: center; gap: 10px; min-width: 0;">
              <span style="font-size: 20px; flex-shrink: 0;">📦</span>
              <div style="min-width: 0; display: flex; flex-direction: column; gap: 2px;">
                ${p.displayName && p.displayName !== p.fileName ? `
                  <span style="font-size: 13px; font-weight: 700; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    ${escapeHtml(p.displayName)}
                  </span>
                  <span style="font-size: 10.5px; color: var(--text-muted); font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    ${escapeHtml(p.fileName)}
                  </span>
                ` : `
                  <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary); font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    ${escapeHtml(p.fileName)}
                  </span>
                `}
                <div style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-secondary); margin-top: 2px;">
                  <span>🕒 ${escapeHtml(p.createdAt)}</span>
                  <span>•</span>
                  <span style="color: var(--text-primary); font-weight: 600;">📁 ${formatBytes(p.fileSizeBytes)}</span>
                  ${p.resolvedAssetsCount && p.resolvedAssetsCount > 0 ? `
                    <span>•</span>
                    <span style="color: var(--success); font-weight: 600;">✓ ${p.resolvedAssetsCount} asset(s)</span>
                  ` : ''}
                  ${p.isGamepass ? '<span style="color: var(--accent); font-weight: 700; background: var(--accent-dim); padding: 1px 6px; border-radius: var(--radius); font-size: 10px;">IoStore WinGDK</span>' : ''}
                </div>
              </div>
            </div>

            <button class="btn btn-danger btn-sm btn-delete-patch" data-patch-path="${escapeHtml(p.filePath)}" data-patch-name="${escapeHtml(p.displayName || p.fileName)}" style="padding: 5px 12px; font-size: 11px; font-weight: 600; display: flex; align-items: center; gap: 5px;">
              <span>🗑️</span> <span>${escapeHtml(t('common.delete') || 'Delete')}</span>
            </button>
          </div>
        `).join('')}
      </div>

      <div class="modal-footer" style="display: flex; justify-content: flex-end;">
        <button id="btn-close-existing-patches-footer" class="btn btn-secondary" style="font-size: 12px; font-weight: 600;">
          ${escapeHtml(t('common.close') || 'Close')}
        </button>
      </div>

    </div>
  `;

  overlay.innerHTML = renderContent();
  document.body.appendChild(overlay);

  const close = () => {
    window.removeEventListener('keydown', handleKeyDown);
    overlay.remove();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.stopPropagation();
      close();
    }
  };
  window.addEventListener('keydown', handleKeyDown);

  scannerDom.elMaybe('btn-close-existing-patches')?.addEventListener('click', close);
  scannerDom.elMaybe('btn-close-existing-patches-footer')?.addEventListener('click', close);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });

  // Delete button handlers
  overlay.querySelectorAll<HTMLButtonElement>('.btn-delete-patch').forEach(btn => {
    btn.addEventListener('click', async () => {
      const pPath = btn.dataset.patchPath;
      const pName = btn.dataset.patchName;
      if (!pPath || !pName) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_delete_patch_title') || 'Delete Compatibility Patch',
        t('scanner.confirm_delete_patch_msg', { name: pName }) || `Are you sure you want to permanently delete patch '${pName}'?`,
        t('common.delete') || 'Delete',
        t('common.cancel') || 'Cancel'
      );

      if (confirmed) {
        try {
          await deleteGeneratedPatch(pPath);
          showToast(t('scanner.patch_deleted_success', { name: pName }) || `Patch '${pName}' deleted.`, 'success');
          const { runScan } = await import('./runner');
          runScan();
          const { loadMods } = await import('../../modsView');
          loadMods();
          patches = await listGeneratedPatches();
          overlay.innerHTML = renderContent();
          close();
          openExistingPatchesModal();
        } catch (err) {
          showToast(String(err), 'error');
        }
      }
    });
  });
}
