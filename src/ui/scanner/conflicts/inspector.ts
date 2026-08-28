import { escapeHtml } from '../rendering';
import { t } from '../../../utils/i18n';
import {
  lastScanResult,
  registryFilterType,
  registrySearchQuery,
  selectedRegistryModId,
  setSelectedRegistryModId,
  attachMasterListListeners,
  attachInspectUassetListeners,
  type ModSummary,
} from '../mod';

export function buildInspectorContent(activeMod: ModSummary | null): string {
  if (!activeMod) {
    return `
      <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted); font-size: 13px; padding: 40px; gap: 8px;">
        <span style="font-size: 32px;">🔍</span>
        <span>${escapeHtml(t('scanner.no_registries_match'))}</span>
      </div>
    `;
  }

  const hasRows = activeMod.palschemaRows && activeMod.palschemaRows.length > 0;
  const hasHooks = activeMod.ue4ssHooks && activeMod.ue4ssHooks.length > 0;
  const hasPak = activeMod.pakFiles && activeMod.pakFiles.length > 0;

  let typeBadge = '';
  const isMulti = [hasRows, hasHooks, hasPak].filter(Boolean).length > 1;
  if (isMulti) {
    typeBadge = `<span class="scanner-conflict-type" style="font-size: 10px; padding: 3px 8px; background: rgba(147,112,219,0.15); color: rgb(186,104,200); font-weight:700;">Hybrid Mod</span>`;
  } else if (hasPak) {
    typeBadge = `<span class="scanner-conflict-type" style="font-size: 10px; padding: 3px 8px; background: rgba(76,175,80,0.15); color: #81c784; font-weight:700;">Pak Mod</span>`;
  } else if (hasRows) {
    typeBadge = `<span class="scanner-conflict-type" style="font-size: 10px; padding: 3px 8px; background: rgba(255,165,0,0.1); color: var(--warning); font-weight:700;">PalSchema JSON</span>`;
  } else if (hasHooks) {
    typeBadge = `<span class="scanner-conflict-type lua" style="font-size: 10px; padding: 3px 8px; font-weight:700;">UE4SS Lua</span>`;
  }

  const sectionsHtml = [];

  if (hasRows) {
    sectionsHtml.push(`
      <div class="scanner-inspector-card">
        <div style="font-size: 12px; font-weight: 700; color: var(--warning); text-transform: uppercase; margin-bottom: 10px; display: flex; align-items: center; justify-content: space-between;">
          <span>📜 ${escapeHtml(t('scanner.palschema_edits_label'))}</span>
          <span style="font-size: 11px; font-weight: normal; color: var(--text-muted);">${activeMod.palschemaRows.length} ${escapeHtml(t('scanner.row_count', { count: activeMod.palschemaRows.length }))}</span>
        </div>
        <ul class="scanner-detail-list" style="margin: 0; padding-left: 16px; font-size: 11px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 6px; overflow-wrap: anywhere; word-break: break-word; white-space: normal; max-height: 240px; overflow-y: auto;">
          ${activeMod.palschemaRows.map(r => `<li>${escapeHtml(r)}</li>`).join('')}
        </ul>
      </div>
    `);
  }

  if (hasHooks) {
    sectionsHtml.push(`
      <div class="scanner-inspector-card">
        <div style="font-size: 12px; font-weight: 700; color: var(--accent); text-transform: uppercase; margin-bottom: 10px; display: flex; align-items: center; justify-content: space-between;">
          <span>⚡ ${escapeHtml(t('scanner.ue4ss_hooks_label'))}</span>
          <span style="font-size: 11px; font-weight: normal; color: var(--text-muted);">${activeMod.ue4ssHooks.length} ${escapeHtml(t('scanner.hook_count', { count: activeMod.ue4ssHooks.length }))}</span>
        </div>
        <ul class="scanner-detail-list" style="margin: 0; padding-left: 16px; font-size: 11px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 6px; overflow-wrap: anywhere; word-break: break-word; white-space: normal; max-height: 240px; overflow-y: auto;">
          ${activeMod.ue4ssHooks.map(h => `<li>${escapeHtml(h)}</li>`).join('')}
        </ul>
      </div>
    `);
  }

  if (hasPak && activeMod.pakFiles) {
    sectionsHtml.push(`
      <div class="scanner-inspector-card">
        <div style="font-size: 12px; font-weight: 700; color: #81c784; text-transform: uppercase; margin-bottom: 10px; display: flex; align-items: center; justify-content: space-between;">
          <span>📦 ${escapeHtml(t('scanner.pak_assets_label'))}</span>
          <span style="font-size: 11px; font-weight: normal; color: var(--text-muted);">${activeMod.pakFiles.length} ${escapeHtml(t('scanner.pak_asset_count', { count: activeMod.pakFiles.length }))}</span>
        </div>
        <ul class="scanner-detail-list" style="margin: 0; padding: 0; list-style: none; font-size: 11px; color: var(--text-secondary); font-family: monospace; display: flex; flex-direction: column; gap: 6px; overflow-wrap: anywhere; word-break: break-word; white-space: normal; max-height: 280px; overflow-y: auto;">
          ${activeMod.pakFiles.map((p, idx) => {
            const isUasset = p.toLowerCase().endsWith('.uasset');
            return `
              <li style="padding: 6px 10px; background: rgba(0,0,0,0.2); border-radius: 4px; border: 1px solid rgba(255,255,255,0.04); display: flex; flex-direction: column; gap: 4px;">
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                  <span style="overflow: hidden; text-overflow: ellipsis; color: var(--text-primary);">${escapeHtml(p)}</span>
                  ${isUasset ? `
                    <button class="btn-secondary btn-sm inspect-uasset-btn" data-mod-id="${escapeHtml(activeMod.modId)}" data-asset-path="${escapeHtml(p)}" data-drawer-id="drawer-${idx}" style="font-size: 10px; padding: 2px 8px; flex-shrink: 0; display: inline-flex; align-items: center; gap: 4px;">
                      <span>🔍</span> <span>${escapeHtml(t('scanner.btn_inspect_uasset'))}</span>
                    </button>
                  ` : ''}
                </div>
                <div id="drawer-${idx}" class="uasset-inspect-drawer" style="display: none; margin-top: 6px; padding: 8px 10px; background: rgba(0, 188, 255, 0.05); border: 1px solid rgba(0, 188, 255, 0.2); border-radius: 4px; font-size: 10px; color: var(--text-secondary); max-height: 160px; overflow-y: auto;">
                </div>
              </li>
            `;
          }).join('')}
        </ul>
      </div>
    `);
  }

  const isGpNotice = lastScanResult?.isGamepass && lastScanResult.gamepassNotices?.some(n => n.modId === activeMod.modId);

  return `
    <div class="scanner-inspector-header">
      <div style="display: flex; flex-direction: column; gap: 4px; min-width: 0;">
        <div style="display: flex; align-items: center; gap: 10px; min-width: 0;">
          <span style="font-size: 16px; font-weight: 800; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(activeMod.modName)}</span>
          ${typeBadge}
        </div>
        ${isGpNotice ? `
          <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
            <div style="padding: 4px 8px; background: rgba(255, 170, 0, 0.1); border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 4px; color: #ffaa00; font-size: 10.5px; display: inline-flex; align-items: center; gap: 6px; font-weight: 600; width: fit-content;">
              <span>⚠️</span> <span>${escapeHtml(t('scanner.gamepass_missing_badge'))}</span>
            </div>
            <button class="btn-primary btn-sm convert-single-gamepass-btn" data-mod-id="${escapeHtml(activeMod.modId)}" style="font-size: 10.5px; padding: 3px 10px; display: inline-flex; align-items: center; gap: 4px;">
              <span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_gamepass'))}</span>
            </button>
          </div>
        ` : ''}
      </div>
    </div>
    <div style="display: flex; flex-direction: column;">
      ${sectionsHtml.join('')}
    </div>
  `;
}

export function buildMasterItemsHtml(filteredSummaries: ModSummary[], selectedModId: string | null): string {
  if (filteredSummaries.length === 0) {
    return `
      <div style="color: var(--text-muted); font-size: 11px; padding: 24px 12px; text-align: center; font-style: italic;">
        ${escapeHtml(t('scanner.no_registries_match'))}
      </div>
    `;
  }

  return filteredSummaries.map(m => {
    const isSelected = selectedModId === m.modId;
    const hasRows = m.palschemaRows && m.palschemaRows.length > 0;
    const hasHooks = m.ue4ssHooks && m.ue4ssHooks.length > 0;
    const hasPak = m.pakFiles && m.pakFiles.length > 0;

    let badgeHtml = '';
    const isMulti = [hasRows, hasHooks, hasPak].filter(Boolean).length > 1;
    if (isMulti) {
      badgeHtml = `<span class="scanner-conflict-type" style="font-size: 9px; padding: 2px 6px; background: rgba(147,112,219,0.15); color: rgb(186,104,200); font-weight:700;">Hybrid</span>`;
    } else if (hasPak) {
      badgeHtml = `<span class="scanner-conflict-type" style="font-size: 9px; padding: 2px 6px; background: rgba(76,175,80,0.15); color: #81c784; font-weight:700;">Pak</span>`;
    } else if (hasRows) {
      badgeHtml = `<span class="scanner-conflict-type" style="font-size: 9px; padding: 2px 6px; background: rgba(255,165,0,0.1); color: var(--warning); font-weight:700;">PalSchema</span>`;
    } else if (hasHooks) {
      badgeHtml = `<span class="scanner-conflict-type lua" style="font-size: 9px; padding: 2px 6px; font-weight:700;">UE4SS</span>`;
    }

    const summaryPills = [];
    if (hasRows) summaryPills.push(`${m.palschemaRows.length} ${escapeHtml(t('scanner.row_count', { count: m.palschemaRows.length }))}`);
    if (hasHooks) summaryPills.push(`${m.ue4ssHooks.length} ${escapeHtml(t('scanner.hook_count', { count: m.ue4ssHooks.length }))}`);
    if (hasPak && m.pakFiles) summaryPills.push(`${m.pakFiles.length} ${escapeHtml(t('scanner.pak_asset_count', { count: m.pakFiles.length }))}`);

    return `
      <div class="scanner-mod-list-item ${isSelected ? 'active' : ''}" data-mod-id="${escapeHtml(m.modId)}">
        <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
          <span style="font-weight: 700; color: ${isSelected ? 'var(--accent)' : 'var(--text-primary)'}; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(m.modName)}</span>
          ${badgeHtml}
        </div>
        <div style="font-size: 10px; color: var(--text-muted); display: flex; gap: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
          ${summaryPills.join(' • ')}
        </div>
      </div>
    `;
  }).join('');
}

export function updateMasterDetailInPlace(): void {
  if (!lastScanResult?.modSummaries) return;

  const query = registrySearchQuery.toLowerCase().trim();
  const filteredSummaries = lastScanResult.modSummaries.filter(m => {
    const hasRows = (m.palschemaRows?.length ?? 0) > 0;
    const hasHooks = (m.ue4ssHooks?.length ?? 0) > 0;
    const hasPak = (m.pakFiles?.length ?? 0) > 0;
    const isHybrid = [hasRows, hasHooks, hasPak].filter(Boolean).length > 1;

    if (registryFilterType === 'pak' && (!hasPak || isHybrid)) return false;
    if (registryFilterType === 'ue4ss' && (!hasHooks || isHybrid)) return false;
    if (registryFilterType === 'palschema' && (!hasRows || isHybrid)) return false;
    if (registryFilterType === 'hybrid' && !isHybrid) return false;

    if (query) {
      const matchesName = m.modName.toLowerCase().includes(query);
      const matchesRows = m.palschemaRows?.some(r => r.toLowerCase().includes(query));
      const matchesHooks = m.ue4ssHooks?.some(h => h.toLowerCase().includes(query));
      const matchesPaks = m.pakFiles?.some(p => p.toLowerCase().includes(query));
      if (!matchesName && !matchesRows && !matchesHooks && !matchesPaks) return false;
    }

    return true;
  });

  let activeMod = filteredSummaries.find(m => m.modId === selectedRegistryModId);
  if (!activeMod && filteredSummaries.length > 0) {
    activeMod = filteredSummaries[0];
    setSelectedRegistryModId(activeMod.modId);
  } else if (filteredSummaries.length === 0) {
    setSelectedRegistryModId(null);
  }

  // Update filter buttons active class
  document.querySelectorAll('.registry-filter-btn').forEach(btn => {
    const type = (btn as HTMLElement).dataset.type;
    if (type === registryFilterType) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });

  // Update left items
  const masterItemsEl = document.querySelector('.scanner-master-items');
  if (masterItemsEl) {
    masterItemsEl.innerHTML = buildMasterItemsHtml(filteredSummaries, selectedRegistryModId);
  }

  // Update right inspector panel
  const inspectorRoot = document.getElementById('scanner-inspector-root');
  if (inspectorRoot) {
    inspectorRoot.innerHTML = buildInspectorContent(activeMod || null);
  }

  // Rebind event listeners on master items and inspect buttons
  attachMasterListListeners();
  attachInspectUassetListeners();
}
