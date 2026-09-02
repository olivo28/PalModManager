import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import {
  registryFilterType,
  registrySearchQuery,
  selectedRegistryModId,
  setSelectedRegistryModId,
  type ScanResult,
} from '../../mod';
import { buildMasterItemsHtml, buildInspectorContent } from '../inspector';

export function buildRegistriesSectionHtml(res: ScanResult): string {
  if (!res.modSummaries || res.modSummaries.length === 0) return '';

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

  return `
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
