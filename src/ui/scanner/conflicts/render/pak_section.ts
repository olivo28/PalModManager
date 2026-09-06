import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import type { PakConflict, PakModSource } from '../../mod';

export function buildPakConflictsHtml(
  allPakConflicts: PakConflict[],
  resolvedPakConflicts: PakConflict[],
  unresolvedPakConflicts: PakConflict[],
  pakCount: number,
  resolvedPakCount: number,
  unresolvedPakCount: number,
  isAllPakResolved: boolean
): string {
  if (pakCount === 0) return '';

  if (isAllPakResolved) {
    const activePatchName = resolvedPakConflicts[0]?.resolvedByPatch || 'zzz_PMM_Patch_Compat_P.pak';
    return `
      <div class="scanner-card-section" style="width: 100%; margin-bottom: 20px; border-color: var(--success);">
        <div class="scanner-card-header" style="display: flex; align-items: center; justify-content: space-between; background: var(--success-dim); border-bottom: 1px solid var(--border); padding: 10px 16px;">
          <span style="color: var(--success); font-weight: 700; display: flex; align-items: center; gap: 8px;">
            <span>✅</span>
            <span>${escapeHtml(t('scanner.pak_conflicts_resolved_title', { count: resolvedPakCount }) || `Pak Asset Conflicts: ${resolvedPakCount} Resolved`)}</span>
          </span>
          <span style="font-size: 10.5px; color: var(--success); font-weight: 600;">
            ${escapeHtml(activePatchName)}
          </span>
        </div>
        <div class="scanner-card-body" style="gap: 12px; padding: 14px;">
          
          <div style="display: flex; justify-content: space-between; align-items: center; background: var(--bg-primary); padding: 12px 16px; border-radius: var(--card-radius); border: 1px solid var(--success); gap: 12px; flex-wrap: wrap;">
            <div style="font-size: 12px; color: var(--text-primary); display: flex; align-items: center; gap: 8px;">
              <span style="font-size: 18px;">🎉</span>
              <span>${escapeHtml(t('scanner.pak_patch_resolved_banner', { count: resolvedPakCount, patch: activePatchName }) || `All ${resolvedPakCount} conflicting asset collisions are successfully resolved by compatibility patch '${activePatchName}'.`)}</span>
            </div>
            <div style="display: flex; align-items: center; gap: 10px;">
              <button id="btn-open-existing-patches" class="btn btn-secondary btn-sm" style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; font-weight: 600; padding: 6px 14px;">
                <span>📋</span> <span>${escapeHtml(t('scanner.btn_view_existing_patches') || 'Existing Patches')}</span>
              </button>
              <button id="btn-open-patch-builder" class="btn btn-primary btn-sm" style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; font-weight: 700; padding: 6px 16px;">
                <span>🛠️</span> <span>${escapeHtml(t('scanner.btn_rebuild_compat_patch') || 'Rebuild Patch')}</span>
              </button>
            </div>
          </div>

          <details style="margin-top: 4px; border: 1px solid var(--border); border-radius: var(--radius); padding: 6px 12px; background: rgba(0,0,0,0.15);">
            <summary style="font-size: 11px; font-weight: 600; color: var(--text-muted); cursor: pointer; user-select: none;">
              🔍 ${escapeHtml(t('scanner.btn_show_resolved_assets', { count: resolvedPakCount }) || `Show Detailed Resolved Assets (${resolvedPakCount})`)}
            </summary>
            <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 10px;">
              ${allPakConflicts.map(c => {
                let badgeClass = 'lua';
                if (c.assetType === 'DataTable') {
                  badgeClass = 'datatable';
                } else if (c.assetType === 'Blueprint') {
                  badgeClass = 'blueprint';
                }

                return `
                  <div class="scanner-conflict-item" style="border-left: 3px solid var(--success);">
                    <div class="scanner-conflict-header">
                      <span class="scanner-conflict-title">${escapeHtml(c.assetName)}</span>
                      <span class="scanner-conflict-type ${badgeClass}">${escapeHtml(c.assetType)}</span>
                      <span style="font-size: 10.5px; font-weight: 700; color: var(--success); background: var(--success-dim); padding: 2px 7px; border-radius: var(--radius); display: inline-flex; align-items: center; gap: 4px;">
                        <span>✓</span> <span>${escapeHtml(t('scanner.resolved_by_tag', { patch: c.resolvedByPatch || activePatchName }) || `Resolved by ${c.resolvedByPatch || activePatchName}`)}</span>
                      </span>
                    </div>
                    <div class="scanner-conflict-path">
                      <span>📄</span> <span>${escapeHtml(c.internalPath)}</span>
                    </div>
                    <div class="scanner-conflict-mods">
                      ${c.mods.map((m: PakModSource) => `
                        <div class="scanner-conflict-mod-row">
                          <span style="font-size: 13px;">📦</span>
                          <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                          <span class="scanner-conflict-mod-file">(${escapeHtml(m.pakFilename)})</span>
                        </div>
                      `).join('')}
                    </div>
                  </div>
                `;
              }).join('')}
            </div>
          </details>
        </div>
      </div>
    `;
  }

  return `
    <details class="scanner-card-section" style="width: 100%; cursor: pointer; margin-bottom: 20px; border-color: rgba(255, 75, 75, 0.25);" open>
      <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(255, 75, 75, 0.04); border-bottom: 1px solid var(--border);">
        <span style="color: var(--danger); font-weight: 700; display: flex; align-items: center; gap: 8px;">
          <span>📦</span>
          <span>${escapeHtml(t('scanner.pak_conflicts_title', { count: unresolvedPakCount }))}</span>
        </span>
        <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.pak_conflicts_desc'))}</span>
      </summary>
      <div class="scanner-card-body" style="cursor: default; gap: 12px; padding-top: 14px;">
        
        <div style="display: flex; justify-content: space-between; align-items: center; background: var(--bg-primary); padding: 12px 16px; border-radius: var(--card-radius); border: 1px solid var(--border); gap: 12px; flex-wrap: wrap;">
          <div style="font-size: 12px; color: var(--text-secondary); display: flex; align-items: center; gap: 8px;">
            <span style="font-size: 16px;">💡</span>
            <span>${escapeHtml(t('scanner.pak_patch_banner_hint') || 'Resolve overlapping .pak assets by building a custom compatibility patch.')}</span>
          </div>
          <div style="display: flex; align-items: center; gap: 10px;">
            <button id="btn-open-existing-patches" class="btn btn-secondary btn-sm" style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; font-weight: 600; padding: 6px 14px;">
              <span>📋</span> <span>${escapeHtml(t('scanner.btn_view_existing_patches') || 'Existing Patches')}</span>
            </button>
            <button id="btn-open-patch-builder" class="btn btn-primary btn-sm" style="display: flex; align-items: center; gap: 6px; font-size: 11.5px; font-weight: 700; padding: 6px 16px;">
              <span>🛠️</span> <span>${escapeHtml(t('scanner.btn_create_compat_patch') || 'Build Compatibility Patch')}</span>
            </button>
          </div>
        </div>

        ${allPakConflicts.map(c => {
          let badgeClass = 'lua';
          if (c.assetType === 'DataTable') {
            badgeClass = 'datatable';
          } else if (c.assetType === 'Blueprint') {
            badgeClass = 'blueprint';
          }

          const isResolved = !!c.resolvedByPatch;

          return `
            <div class="scanner-conflict-item" style="border-left: 3px solid ${isResolved ? 'var(--success)' : 'var(--danger)'};">
              <div class="scanner-conflict-header">
                <span class="scanner-conflict-title">${escapeHtml(c.assetName)}</span>
                <span class="scanner-conflict-type ${badgeClass}">${escapeHtml(c.assetType)}</span>
                ${isResolved ? `
                  <span style="font-size: 10.5px; font-weight: 700; color: var(--success); background: var(--success-dim); padding: 2px 7px; border-radius: var(--radius); display: inline-flex; align-items: center; gap: 4px;">
                    <span>✓</span> <span>${escapeHtml(t('scanner.resolved_by_tag', { patch: c.resolvedByPatch! }) || `Resolved by ${c.resolvedByPatch}`)}</span>
                  </span>
                ` : ''}
              </div>
              <div class="scanner-conflict-path">
                <span>📄</span> <span>${escapeHtml(c.internalPath)}</span>
              </div>
                <div class="scanner-conflict-mods">
                  ${c.mods.map((m: PakModSource) => `
                    <div class="scanner-conflict-mod-row" style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                      <div style="display: flex; align-items: center; gap: 6px; overflow: hidden;">
                        <span style="font-size: 13px;">📦</span>
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.pakFilename)})</span>
                      </div>
                      <button class="btn btn-danger-subtle btn-xs scan-disable-mod-btn" data-mod-id="${escapeHtml(m.modId)}" style="font-size: 10px; padding: 2px 8px; flex-shrink: 0;">
                        🚫 ${escapeHtml(t('scanner.btn_disable_mod') || 'Disable')}
                      </button>
                    </div>
                  `).join('')}
                </div>
            </div>
          `;
        }).join('')}
      </div>
    </details>
  `;
}
