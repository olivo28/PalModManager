import { escapeHtml } from '../rendering';
import { t } from '../../../utils/i18n';
import { getState } from '../../../state';
import type { ModInfo } from '../../../types';
import {
  lastScanResult,
  subTabHeader,
  registryFilterType,
  registrySearchQuery,
  selectedRegistryModId,
  setSelectedRegistryModId,
} from '../mod';
import { buildMasterItemsHtml, buildInspectorContent } from './inspector';

export async function renderConflictsPanel(container: HTMLElement): Promise<void> {
  if (!lastScanResult) {
    container.innerHTML = `
      ${subTabHeader()}
      <div style="flex:1; display:flex; align-items:center; justify-content:center; padding:24px;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon">🛡</div>
          <div class="scanner-hero-title">${escapeHtml(t('scanner.hero_initial_title'))}</div>
          <div class="scanner-hero-desc">
            ${escapeHtml(t('scanner.hero_initial_desc'))}
          </div>
          <button id="scanner-start-btn" class="scanner-btn-run">
            <span>${escapeHtml(t('scanner.btn_run_scanner'))}</span>
          </button>
        </div>
      </div>
    `;
    const { setupEventListeners } = await import('../mod');
    setupEventListeners();
    return;
  }

  const res = lastScanResult;
  const tableCount = res.tableConflicts.length;
  const hookCount = res.hookConflicts.length;
  const allPakConflicts = res.pakConflicts || [];
  const unresolvedPakConflicts = allPakConflicts.filter(c => !c.resolvedByPatch);
  const resolvedPakConflicts = allPakConflicts.filter(c => !!c.resolvedByPatch);
  const pakCount = allPakConflicts.length;
  const unresolvedPakCount = unresolvedPakConflicts.length;
  const resolvedPakCount = resolvedPakConflicts.length;
  const isAllPakResolved = pakCount > 0 && unresolvedPakCount === 0 && resolvedPakCount > 0;
  const activeConflictsCount = tableCount + hookCount + unresolvedPakCount;
  const hasConflicts = activeConflictsCount > 0;

  let pakConflictsHtml = '';
  if (pakCount > 0) {
    if (isAllPakResolved) {
      // All collisions are resolved by an active compatibility patch!
      const activePatchName = resolvedPakConflicts[0]?.resolvedByPatch || 'zzz_PMM_Patch_Compat_P.pak';
      pakConflictsHtml = `
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
                        ${c.mods.map(m => `
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
    } else {
      // Unresolved collisions exist
      pakConflictsHtml = `
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
                    ${c.mods.map(m => `
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
      `;
    }
  }

  let contentHtml = '';
  if (!hasConflicts) {
    contentHtml = `
      ${pakConflictsHtml}
      ${!pakConflictsHtml ? `
        <div class="scanner-clean-state" style="margin-bottom: 20px;">
          <div class="scanner-clean-icon">✅</div>
          <div class="scanner-clean-title">${escapeHtml(t('scanner.no_conflicts_title'))}</div>
          <div class="scanner-clean-desc">${escapeHtml(t('scanner.no_conflicts_desc'))}</div>
        </div>
      ` : ''}
    `;
  } else {
    contentHtml = `
      ${pakConflictsHtml}
      <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
            <span>${escapeHtml(t('scanner.hook_conflicts_title', { count: hookCount }))}</span>
            <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.hook_conflicts_desc'))}</span>
          </summary>
          <div class="scanner-card-body" style="cursor: default; gap: 14px;">
            ${hookCount > 0 ? res.hookConflicts.map(c => `
              <div class="scanner-conflict-item">
                <div class="scanner-conflict-header">
                  <span class="scanner-conflict-title">${escapeHtml(c.hookTarget)}</span>
                  <span class="scanner-conflict-type lua">${escapeHtml(c.hookFn)}</span>
                </div>
                <div class="scanner-conflict-mods">
                  ${c.mods.map(m => `
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('') : `<div style="color:var(--text-muted); font-size:11px;">${escapeHtml(t('scanner.hook_conflicts_none'))}</div>`}
          </div>
        </details>

        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer;" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between;">
            <span>${escapeHtml(t('scanner.table_conflicts_title', { count: tableCount }))}</span>
            <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.table_conflicts_desc'))}</span>
          </summary>
          <div class="scanner-card-body" style="cursor: default; gap: 14px;">
            ${tableCount > 0 ? res.tableConflicts.map(c => `
              <div class="scanner-conflict-item">
                <div class="scanner-conflict-header">
                  <span class="scanner-conflict-title">${escapeHtml(c.tableName)}</span>
                  <span class="scanner-conflict-type json">${escapeHtml(c.rowName)}</span>
                </div>
                <div class="scanner-conflict-mods">
                  ${c.mods.map(m => `
                    <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                        <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                      </div>
                      <div style="font-size:10px; color:var(--text-muted); font-family:monospace; padding-left: 8px;">↳ ${escapeHtml(m.detail)}</div>
                    </div>
                  `).join('')}
                </div>
              </div>
            `).join('') : `<div style="color:var(--text-muted); font-size:11px;">${escapeHtml(t('scanner.table_conflicts_none'))}</div>`}
          </div>
        </details>
      </div>
    `;
  }

  // 1. Internal conflicts HTML
  const internalTableCount = res.internalTableConflicts ? res.internalTableConflicts.length : 0;
  const internalHookCount = res.internalHookConflicts ? res.internalHookConflicts.length : 0;
  const hasInternalConflicts = internalTableCount > 0 || internalHookCount > 0;

  let internalConflictsHtml = '';
  if (hasInternalConflicts) {
    internalConflictsHtml = `
      <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer; border-color: rgba(255, 165, 0, 0.2);" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(255, 165, 0, 0.03); border-bottom: 1px solid rgba(255, 165, 0, 0.08);">
            <span style="color: var(--warning); display: flex; align-items: center; gap: 6px; font-weight: 700;">
              ${escapeHtml(t('scanner.internal_conflicts_title', { count: internalTableCount + internalHookCount }))}
            </span>
            <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.internal_conflicts_desc'))}</span>
          </summary>
          <div class="scanner-card-body" style="cursor: default; gap: 14px; padding-top: 14px;">
            ${internalHookCount > 0 ? `
              <div style="font-weight: 700; font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">${escapeHtml(t('scanner.internal_ue4ss_duplicates'))}</div>
              ${res.internalHookConflicts.map(c => `
                <div class="scanner-conflict-item" style="border-left: 2.5px solid var(--warning);">
                  <div class="scanner-conflict-header">
                    <span class="scanner-conflict-title">${escapeHtml(c.hookTarget)}</span>
                    <span class="scanner-conflict-type lua">${escapeHtml(c.hookFn)}</span>
                  </div>
                  <div class="scanner-conflict-mods">
                    ${c.mods.map(m => `
                      <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                        <div style="display: flex; align-items: center; gap: 8px;">
                          <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                          <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)}:L${m.lineNumber})</span>
                        </div>
                        ${m.detail ? `
                          <div style="font-size: 10px; color: var(--text-muted); background: rgba(0,0,0,0.2); padding: 4px 8px; border-radius: 4px; font-family: monospace; border: 1px solid var(--border); margin-left: 4px; margin-top: 2px;">
                            ${escapeHtml(t('scanner.line_number_prefix', { line: m.lineNumber, detail: m.detail }))}
                          </div>
                        ` : ''}
                      </div>
                    `).join('')}
                  </div>
                </div>
              `).join('')}
            ` : ''}
            
            ${internalTableCount > 0 ? `
              <div style="font-weight: 700; font-size: 11px; color: var(--text-secondary); margin-top: 10px; margin-bottom: 4px;">${escapeHtml(t('scanner.internal_palschema_duplicates'))}</div>
              ${res.internalTableConflicts.map(c => `
                <div class="scanner-conflict-item" style="border-left: 2.5px solid var(--warning);">
                  <div class="scanner-conflict-header">
                    <span class="scanner-conflict-title">${escapeHtml(c.tableName)}::${escapeHtml(c.rowName)}</span>
                    <span class="scanner-conflict-type">PalSchema</span>
                  </div>
                  <div class="scanner-conflict-mods">
                    ${c.mods.map(m => `
                      <div class="scanner-conflict-mod-row" style="flex-direction: column; align-items: flex-start; gap: 2px; margin-bottom: 6px;">
                        <div style="display: flex; align-items: center; gap: 8px;">
                          <span class="scanner-conflict-mod-name">${escapeHtml(m.modName)}</span>
                          <span class="scanner-conflict-mod-file">(${escapeHtml(m.filePath)})</span>
                        </div>
                        ${m.detail ? `
                          <div style="font-size: 10px; color: var(--text-muted); background: rgba(0,0,0,0.2); padding: 4px 8px; border-radius: 4px; font-family: monospace; border: 1px solid var(--border); margin-left: 4px; margin-top: 2px;">
                            ${escapeHtml(m.detail)}
                          </div>
                        ` : ''}
                      </div>
                    `).join('')}
                  </div>
                </div>
              `).join('')}
            ` : ''}
          </div>
        </details>
      </div>
    `;
  }

  // 2. Active Mod Registries Summaries HTML (Master-Detail Inspector)
  let summariesHtml = '';
  if (res.modSummaries && res.modSummaries.length > 0) {
    const totalCount = res.modSummaries.length;
    const pakCount = res.modSummaries.filter(m => (m.pakFiles?.length ?? 0) > 0 && !m.palschemaRows?.length && !m.ue4ssHooks?.length).length;
    const ue4ssCount = res.modSummaries.filter(m => (m.ue4ssHooks?.length ?? 0) > 0 && !m.palschemaRows?.length && !(m.pakFiles?.length ?? 0)).length;
    const palschemaCount = res.modSummaries.filter(m => (m.palschemaRows?.length ?? 0) > 0 && !(m.ue4ssHooks?.length ?? 0) && !(m.pakFiles?.length ?? 0)).length;
    const hybridCount = res.modSummaries.filter(m => {
      const activeTypes = [(m.palschemaRows?.length ?? 0) > 0, (m.ue4ssHooks?.length ?? 0) > 0, (m.pakFiles?.length ?? 0) > 0].filter(Boolean).length;
      return activeTypes > 1;
    }).length;

    const query = registrySearchQuery.toLowerCase().trim();
    const filteredSummaries = res.modSummaries.filter(m => {
      const hasRows = (m.palschemaRows?.length ?? 0) > 0;
      const hasHooks = (m.ue4ssHooks?.length ?? 0) > 0;
      const hasPak = (m.pakFiles?.length ?? 0) > 0;
      const isHybrid = [hasRows, hasHooks, hasPak].filter(Boolean).length > 1;

      // Filter by type
      if (registryFilterType === 'pak' && (!hasPak || isHybrid)) return false;
      if (registryFilterType === 'ue4ss' && (!hasHooks || isHybrid)) return false;
      if (registryFilterType === 'palschema' && (!hasRows || isHybrid)) return false;
      if (registryFilterType === 'hybrid' && !isHybrid) return false;

      // Filter by search query
      if (query) {
        const matchesName = m.modName.toLowerCase().includes(query);
        const matchesRows = m.palschemaRows?.some(r => r.toLowerCase().includes(query));
        const matchesHooks = m.ue4ssHooks?.some(h => h.toLowerCase().includes(query));
        const matchesPaks = m.pakFiles?.some(p => p.toLowerCase().includes(query));
        if (!matchesName && !matchesRows && !matchesHooks && !matchesPaks) return false;
      }

      return true;
    });

    // Ensure selected mod is valid
    let activeMod = filteredSummaries.find(m => m.modId === selectedRegistryModId);
    if (!activeMod && filteredSummaries.length > 0) {
      activeMod = filteredSummaries[0];
      setSelectedRegistryModId(activeMod.modId);
    }

    // Left List Items
    const listItemsHtml = buildMasterItemsHtml(filteredSummaries, selectedRegistryModId);

    // Right Inspector Content
    const inspectorHtml = buildInspectorContent(activeMod || null);

    summariesHtml = `
      <div class="scanner-card-section" style="margin-bottom: 20px;">
        <div class="scanner-card-header" style="display: flex; align-items: center; justify-content: space-between; gap: 16px;">
          <div style="display: flex; flex-direction: column; gap: 2px; min-width: 0;">
            <span style="font-weight: 700; color: var(--text-primary); font-size: 13px;">${escapeHtml(t('scanner.registries_title'))}</span>
            <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.registries_desc'))}</span>
          </div>

          <div class="scanner-filter-chips" style="display: inline-flex; gap: 2px; background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 20px; padding: 2px; flex-shrink: 0;">
            <button class="scanner-filter-chip registry-filter-btn ${registryFilterType === 'all' ? 'active' : ''}" data-type="all">${escapeHtml(t('scanner.filter_all'))} (${totalCount})</button>
            <button class="scanner-filter-chip registry-filter-btn ${registryFilterType === 'pak' ? 'active' : ''}" data-type="pak">📦 ${escapeHtml(t('scanner.filter_pak'))} (${pakCount})</button>
            <button class="scanner-filter-chip registry-filter-btn ${registryFilterType === 'ue4ss' ? 'active' : ''}" data-type="ue4ss">⚡ ${escapeHtml(t('scanner.filter_ue4ss'))} (${ue4ssCount})</button>
            <button class="scanner-filter-chip registry-filter-btn ${registryFilterType === 'palschema' ? 'active' : ''}" data-type="palschema">📜 ${escapeHtml(t('scanner.filter_palschema'))} (${palschemaCount})</button>
            <button class="scanner-filter-chip registry-filter-btn ${registryFilterType === 'hybrid' ? 'active' : ''}" data-type="hybrid">🔮 ${escapeHtml(t('scanner.filter_hybrid'))} (${hybridCount})</button>
          </div>
        </div>

        <div class="scanner-master-detail">
          <div class="scanner-master-list">
            <div class="scanner-master-search-header" style="padding: 10px; border-bottom: 1px solid var(--border); background: rgba(0, 0, 0, 0.2);">
              <div class="search-wrapper" style="width: 100%; max-width: 100%;">
                <span class="search-icon" style="left: 10px; font-size: 11px;">🔍</span>
                <input type="text" id="registry-search-input" class="premium-search-input" value="${escapeHtml(registrySearchQuery)}" placeholder="${escapeHtml(t('scanner.search_registry_placeholder'))}" style="width: 100%; box-sizing: border-box; padding: 6px 10px 6px 28px; font-size: 11px; border-radius: 6px;" />
              </div>
            </div>
            <div class="scanner-master-items" style="flex: 1; overflow-y: auto;">
              ${filteredSummaries.length > 0 ? listItemsHtml : `
                <div style="color: var(--text-muted); font-size: 11px; padding: 24px 12px; text-align: center; font-style: italic;">
                  ${escapeHtml(t('scanner.no_registries_match'))}
                </div>
              `}
            </div>
          </div>

          <div id="scanner-inspector-root" class="scanner-inspector-panel">
            ${inspectorHtml}
          </div>
        </div>
      </div>
    `;
  }

  // 3. Warnings HTML
  let warningsHtml = '';
  if (res.warnings && res.warnings.length > 0) {
    warningsHtml = `
      <details class="scanner-card-section" style="margin-top: 24px; cursor: pointer;">
        <summary class="scanner-card-header" style="outline:none;">
          <span>${escapeHtml(t('scanner.warnings_title', { count: res.warnings.length }))}</span>
          <span style="font-size:10px;color:var(--warning);">${escapeHtml(t('scanner.warnings_desc'))}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; background: rgba(0,0,0,0.15);">
          ${res.warnings.map(w => `
            <div class="scanner-warning-row" style="display: flex; gap: 8px; align-items: center; font-size: 11px; padding: 4px 0;">
              <span class="scanner-warning-icon" style="color: var(--warning);">⚠</span>
              <span>${escapeHtml(w)}</span>
            </div>
          `).join('')}
        </div>
      </details>
    `;
  }

  const statCards = `
    <!-- Stats Row -->
    <div style="display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;flex-shrink:0;">
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_mods_scanned'))}</div>
        <div class="premium-stat-value">${res.totalScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_palschema_json'))}</div>
        <div class="premium-stat-value">${res.palschemaScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_ue4ss_lua'))}</div>
        <div class="premium-stat-value">${res.ue4ssScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_pak_files'))}</div>
        <div class="premium-stat-value">${res.pakScanned ?? 0}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_conflicts'))}</div>
        <div class="premium-stat-value ${hasConflicts ? 'danger' : 'success'}">${hasConflicts ? activeConflictsCount : (resolvedPakCount > 0 ? `0 (✓${resolvedPakCount})` : '0')}</div>
      </div>
    </div>
  `;

  const infoBannerHtml = `
    <!-- Info notice banner -->
    <div class="scanner-info-banner" style="margin-bottom: 20px; padding: 14px 18px; background: var(--bg-card); border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: var(--card-radius); display: flex; flex-direction: column; gap: 8px; font-size: 12px; line-height: 1.5;">
      <div style="font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 8px; font-size: 13px;">
        <span style="font-size: 14px;">ℹ</span>
        <span>${escapeHtml(t('scanner.info_title'))}</span>
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_ue4ss_title'))}:</strong> ${escapeHtml(t('scanner.info_ue4ss_desc'))}
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_palschema_title'))}:</strong> ${escapeHtml(t('scanner.info_palschema_desc'))}
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_pak_title'))}:</strong> ${escapeHtml(t('scanner.info_pak_desc'))}
      </div>
    </div>
  `;

  let gamepassNoticeHtml = '';
  if (res.isGamepass && res.gamepassNotices && res.gamepassNotices.length > 0) {
    const gpCount = res.gamepassNotices.length;
    gamepassNoticeHtml = `
      <div class="scanner-card-section" style="border-color: var(--border); background: var(--bg-card); margin-bottom: 20px;">
        <div class="scanner-card-header" style="background: var(--bg-primary); border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
          <span style="color: var(--accent); display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 13px;">
            <span>🎮</span> ${escapeHtml(t('scanner.gamepass_notice_title'))} (${gpCount})
          </span>
          <div style="display: flex; align-items: center; gap: 10px;">
            <span style="font-size: 10px; color: var(--text-muted); font-weight: 600;">PC Game Pass (WinGDK)</span>
            <button id="btn-convert-all-gamepass" class="btn btn-primary btn-sm" style="font-size: 10.5px; padding: 4px 12px; display: inline-flex; align-items: center; gap: 4px;">
              <span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_all_gamepass', { count: gpCount }))}</span>
            </button>
          </div>
        </div>
        <div class="scanner-card-body" style="gap: 10px; padding: 14px 16px;">
          <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
            ${escapeHtml(t('scanner.gamepass_notice_desc'))}
          </div>
          <div style="display: flex; flex-direction: column; gap: 6px; margin-top: 4px;">
            ${res.gamepassNotices.map(n => `
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius); font-size: 11px; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px; overflow: hidden; min-width: 200px;">
                  <span style="font-weight: 700; color: var(--text-primary);">${escapeHtml(n.modName)}</span>
                  <span style="color: var(--text-muted); font-family: monospace; font-size: 10px;">(${escapeHtml(n.pakFilename)})</span>
                </div>
                <div style="display: flex; align-items: center; gap: 8px; flex-shrink: 0;">
                  <span style="font-size: 9px; padding: 2px 6px; background: rgba(255, 170, 0, 0.15); color: #ffaa00; border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 4px; font-weight: 700; text-transform: uppercase;">
                    ${escapeHtml(t('scanner.gamepass_missing_badge'))} (${n.missingContainers.join(', ')})
                  </span>
                  <button class="btn btn-secondary btn-sm convert-single-gamepass-btn" data-mod-id="${escapeHtml(n.modId)}" style="font-size: 10px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;">
                    <span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_gamepass'))}</span>
                  </button>
                </div>
              </div>
            `).join('')}
          </div>
        </div>
      </div>
    `;
  }

  // 4. USMAP Engine Schema Compatibility Notices HTML
  const schemaNoticeCount = res.schemaNotices ? res.schemaNotices.length : 0;
  let schemaNoticesHtml = '';
  if (schemaNoticeCount > 0) {
    schemaNoticesHtml = `
      <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
        <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer; border-color: rgba(56, 189, 248, 0.3);" open>
          <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(56, 189, 248, 0.05); border-bottom: 1px solid rgba(56, 189, 248, 0.15);">
            <span style="color: #38bdf8; display: flex; align-items: center; gap: 6px; font-weight: 700;">
              <span>⚡</span> <span>${escapeHtml(t('scanner.schema_notices_title', { count: schemaNoticeCount }) || `Engine Schema Compatibility (${schemaNoticeCount})`)}</span>
            </span>
            <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.schema_notices_desc') || 'Verified against Palworld v1.0.3 USMAP Schema')}</span>
          </summary>
          <div class="scanner-card-body" style="cursor: default; gap: 10px; padding-top: 14px;">
            ${res.schemaNotices!.map(n => `
              <div class="scanner-conflict-item" style="border-left: 2.5px solid #38bdf8;">
                <div class="scanner-conflict-header">
                  <span class="scanner-conflict-title">${escapeHtml(n.modName)}</span>
                  <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(n.structName)}</span>
                </div>
                <div class="scanner-conflict-path" style="font-size: 10.5px; color: var(--text-secondary); margin-top: 3px;">
                  <span>📄</span> <span>${escapeHtml(n.assetPath)}</span>
                </div>
                <div style="font-size: 10.5px; color: var(--text-muted); margin-top: 4px; line-height: 1.4;">
                  ${escapeHtml(n.message)}
                </div>
              </div>
            `).join('')}
          </div>
        </details>
      </div>
    `;
  }

  // Missing Frameworks Check (e.g. Altermatic runtime replacer framework missing)
  const state = getState();
  const allMods: ModInfo[] = state.allMods || [];
  const isAltermaticInstalled = state.dependencies?.altermatic_installed || 
    allMods.some((m: ModInfo) => m.enabled && (m.nexusModId === 1626 || m.name.toLowerCase().includes('altermatic - runtime') || (m.name.toLowerCase().startsWith('altermatic') && m.type === 'altermatic')));

  const altermaticMods = allMods.filter((m: ModInfo) => m.enabled && m.type === 'altermatic' && m.nexusModId !== 1626 && !m.name.toLowerCase().includes('altermatic - runtime') && !m.name.toLowerCase().startsWith('altermatic'));
  const isAltermaticMissing = altermaticMods.length > 0 && !isAltermaticInstalled;

  let frameworkMissingHtml = '';
  if (isAltermaticMissing) {
    frameworkMissingHtml = `
      <div style="width: 100%; margin-bottom: 20px; background: rgba(255, 118, 117, 0.12); border: 1px solid rgba(255, 118, 117, 0.35); border-radius: var(--card-radius); padding: 14px 18px; display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap;">
        <div style="display: flex; align-items: center; gap: 12px;">
          <span style="font-size: 22px;">⚠️</span>
          <div style="display: flex; flex-direction: column; gap: 2px;">
            <span style="font-size: 13px; font-weight: 700; color: var(--type-altermatic);">${escapeHtml(t('installer.altermatic_missing_title') || 'Altermatic Framework Required')}</span>
            <span style="font-size: 11px; color: var(--text-secondary);">${escapeHtml(t('scanner.altermatic_missing_body', { count: altermaticMods.length, names: altermaticMods.map(m => m.name).slice(0, 3).join(', ') }) || `You have ${altermaticMods.length} active Altermatic replacer mod(s) (${altermaticMods.map(m => m.name).slice(0, 3).join(', ')}), but the base Altermatic framework is not installed.`)}</span>
          </div>
        </div>
        <button id="btn-scanner-download-altermatic" class="btn btn-secondary" style="font-size: 11px; font-weight: 700; padding: 6px 14px; border-color: var(--type-altermatic); color: var(--type-altermatic); white-space: nowrap;">
          <span>📦</span> <span>${escapeHtml(t('installer.btn_get_altermatic') || 'Get Altermatic (#1626)')}</span>
        </button>
      </div>
    `;
  }

  container.innerHTML = `
    ${subTabHeader()}
    <div class="scanner-scroll-panel" style="flex: 1 1 auto; min-height: 0; display: block; padding: 20px 24px; overflow-y: auto; overflow-x: hidden; box-sizing: border-box;">
      ${statCards}
      ${frameworkMissingHtml}
      ${infoBannerHtml}
      ${gamepassNoticeHtml}
      ${schemaNoticesHtml}
      ${contentHtml}
      ${internalConflictsHtml}
      ${summariesHtml}
      ${warningsHtml}
    </div>
  `;

  const downloadAltermaticBtn = document.getElementById('btn-scanner-download-altermatic');
  if (downloadAltermaticBtn) {
    downloadAltermaticBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const { openUrl } = await import('../../../api');
      openUrl('https://www.nexusmods.com/palworld/mods/1626');
    });
  }

  // Patch Builder button listeners
  const openPatchBuilderBtn = document.getElementById('btn-open-patch-builder');
  if (openPatchBuilderBtn && res.pakConflicts) {
    openPatchBuilderBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const { openPatchBuilderModal } = await import('./patchBuilderModal');
      openPatchBuilderModal(res.pakConflicts as any);
    });
  }

  const openExistingPatchesBtn = document.getElementById('btn-open-existing-patches');
  if (openExistingPatchesBtn) {
    openExistingPatchesBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const { openExistingPatchesModal } = await import('./patchBuilderModal');
      openExistingPatchesModal();
    });
  }

  const { setupEventListeners } = await import('../mod');
  setupEventListeners();
}
