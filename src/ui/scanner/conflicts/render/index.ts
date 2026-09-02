import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import {
  lastScanResult,
  subTabHeader,
} from '../../mod';
import {
  buildStatCardsHtml,
  buildInfoBannerHtml,
  buildGamepassNoticeHtml,
  buildSchemaNoticesHtml,
  buildFrameworkMissingHtml,
  buildWarningsHtml,
} from './cards';
import { buildPakConflictsHtml } from './pak_section';
import { buildHookAndTableConflictsHtml, buildInternalConflictsHtml } from './hook_table_section';
import { buildUsmapSectionHtml } from './usmap_section';
import { buildRegistriesSectionHtml } from './registries_section';

export * from './cards';
export * from './pak_section';
export * from './hook_table_section';
export * from './usmap_section';
export * from './registries_section';

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
    const { setupEventListeners } = await import('../../mod');
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

  const pakConflictsHtml = buildPakConflictsHtml(
    allPakConflicts,
    resolvedPakConflicts,
    unresolvedPakConflicts,
    pakCount,
    resolvedPakCount,
    unresolvedPakCount,
    isAllPakResolved
  );

  const contentHtml = buildHookAndTableConflictsHtml(
    res,
    tableCount,
    hookCount,
    hasConflicts,
    pakConflictsHtml
  );

  const usmapSectionHtml = buildUsmapSectionHtml(res);
  const internalConflictsHtml = buildInternalConflictsHtml(res);
  const summariesHtml = buildRegistriesSectionHtml(res);
  const warningsHtml = buildWarningsHtml(res);
  const statCards = buildStatCardsHtml(res, activeConflictsCount, resolvedPakCount, hasConflicts);
  const infoBannerHtml = buildInfoBannerHtml();
  const gamepassNoticeHtml = buildGamepassNoticeHtml(res);
  const frameworkMissingHtml = buildFrameworkMissingHtml();
  const schemaNoticesHtml = buildSchemaNoticesHtml(res);

  container.innerHTML = `
    ${subTabHeader()}
    <div class="scanner-scroll-panel" style="flex: 1 1 auto; min-height: 0; display: block; padding: 20px 24px; overflow-y: auto; overflow-x: hidden; box-sizing: border-box;">
      ${statCards}
      ${frameworkMissingHtml}
      ${infoBannerHtml}
      ${gamepassNoticeHtml}
      ${schemaNoticesHtml}
      ${usmapSectionHtml}
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
      const { openUrl } = await import('../../../../api');
      openUrl('https://www.nexusmods.com/palworld/mods/1626');
    });
  }

  // Patch Builder button listeners
  const openPatchBuilderBtn = document.getElementById('btn-open-patch-builder');
  if (openPatchBuilderBtn && res.pakConflicts) {
    openPatchBuilderBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const { openPatchBuilderModal } = await import('../patchBuilderModal');
      openPatchBuilderModal(res.pakConflicts as any);
    });
  }

  const openExistingPatchesBtn = document.getElementById('btn-open-existing-patches');
  if (openExistingPatchesBtn) {
    openExistingPatchesBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const { openExistingPatchesModal } = await import('../patchBuilderModal');
      openExistingPatchesModal();
    });
  }

  const { setupEventListeners } = await import('../../mod');
  setupEventListeners();
}
