import type {
  WorldOptionSettings,
  WorldCustomMeta,
} from '../../../../api';
import { subTabHeader } from '../../mod';
import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { renderHeroLandingHtml, attachHeroLandingListeners } from '../hero';
import { attachSavesDoctorListeners } from '../listeners';
import {
  doctorState,
  cachedWorlds,
  selectedWorldDir,
  currentHealthReport,
  isLoadingWorlds,
  isDeepScanning,
  isRepairing,
  isCreatingBackup,
  isPruningBackups,
  isSavingMeta,
  availableProfiles,
} from '../state';
import { getHealthBadge } from './badges';
import { renderWorldsListHtml } from './world_list';
import { renderWorldHeaderAndActionsHtml, renderWorldQuickStatsHtml } from './world_header';
import { renderWorldOptionsHtml, renderWorldMetaFormHtml } from './world_options';
import {
  renderStorageBreakdownHtml,
  renderPlayerRosterHtml,
  renderDeepScanResultsHtml,
} from './deep_scan';

export * from './badges';
export * from './world_list';
export * from './world_header';
export * from './world_options';
export * from './deep_scan';

export async function renderSavesDoctorPanel(container: HTMLElement): Promise<void> {
  const curCached = cachedWorlds !== null ? cachedWorlds : doctorState.cachedWorlds;
  const curLoading = isLoadingWorlds || doctorState.isLoadingWorlds;

  if (curCached === null) {
    if (curLoading) {
      container.innerHTML = `
        <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
          ${subTabHeader()}
          <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;">
            <div class="spinner" style="width: 36px; height: 36px; border: 3px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
            <div style="font-size: 13.5px; font-weight: 600; color: var(--text-primary);">${escapeHtml(t('scanner.saves_doctor_scanning') || 'Scanning Palworld SaveGames...')}</div>
          </div>
        </div>
      `;
      const { setupEventListeners } = await import('../../mod');
      setupEventListeners();
      return;
    }

    // Hero Landing Screen (Zero latency tab switch)
    container.innerHTML = renderHeroLandingHtml(curLoading);
    attachHeroLandingListeners(container, renderSavesDoctorPanel);
    const { setupEventListeners } = await import('../../mod');
    setupEventListeners();
    return;
  }

  const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
  const selectedWorld = curCached?.find(w => w.worldDir === curWorldDir) || curCached?.[0] || null;
  const curHealth = currentHealthReport || doctorState.currentHealthReport;
  const worldOptions: WorldOptionSettings | null = curHealth?.worldOptions || selectedWorld?.worldOptions || null;
  const customMeta: WorldCustomMeta | null = curHealth?.customMeta || selectedWorld?.customMeta || null;

  const curProfiles = availableProfiles.length > 0 ? availableProfiles : doctorState.availableProfiles;
  const curCreating = isCreatingBackup || doctorState.isCreatingBackup;
  const curPruning = isPruningBackups || doctorState.isPruningBackups;
  const curDeep = isDeepScanning || doctorState.isDeepScanning;
  const curSaving = isSavingMeta || doctorState.isSavingMeta;
  const curRepairing = isRepairing || doctorState.isRepairing;
  const curRestoring = doctorState.isRestoringBackup;

  const worldsListHtml = renderWorldsListHtml(curCached, curLoading, selectedWorld);
  const worldHeaderHtml = selectedWorld ? renderWorldHeaderAndActionsHtml(selectedWorld, customMeta, curCreating, curPruning, curDeep) : '';
  const worldQuickStatsHtml = selectedWorld ? renderWorldQuickStatsHtml(selectedWorld) : '';
  const worldMetaFormHtml = selectedWorld ? renderWorldMetaFormHtml(customMeta, curProfiles, curSaving) : '';
  const worldOptionsHtml = selectedWorld ? renderWorldOptionsHtml(worldOptions) : '';
  const storageBreakdownHtml = selectedWorld ? renderStorageBreakdownHtml(curHealth) : '';
  const playerRosterHtml = selectedWorld ? renderPlayerRosterHtml(curHealth) : '';
  const deepScanResultsHtml = selectedWorld ? renderDeepScanResultsHtml(curHealth, selectedWorld, curRepairing, curRestoring, curDeep) : '';

  container.innerHTML = `
    <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
      ${subTabHeader()}
      
      <div class="scanner-scroll-panel" style="padding: calc(14px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); box-sizing: border-box; display: flex; flex-direction: column; gap: calc(12px * var(--ui-scale, 1)); height: 100%; flex: 1; min-height: 0; overflow: hidden;">
        
        <!-- Header & Directory Selector Bar -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: calc(10px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: calc(10px * var(--ui-scale, 1)); flex-shrink: 0;">
          <div style="display: flex; flex-direction: column; gap: 3px;">
            <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
              <span style="font-size: calc(17px * var(--ui-scale, 1));">💾</span>
              <span style="font-size: var(--text-md, 14.5px); font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.saves_doctor_title') || 'Save Health Doctor & World Hub')}</span>
            </div>
            <span style="font-size: var(--text-sm, 11px); color: var(--text-muted);">${escapeHtml(t('scanner.saves_doctor_desc') || 'Inspect world options, players roster, storage health, safety snapshots, and 1-click rescue.')}</span>
          </div>

          <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
            <button id="doctor-custom-folder-btn" class="btn-secondary" style="padding: calc(6px * var(--ui-scale, 1)) calc(12px * var(--ui-scale, 1)); font-size: var(--text-sm, 11.5px); display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
              <span>📁</span> <span>${escapeHtml(t('scanner.btn_choose_saves_folder') || 'Choose Saves Folder')}</span>
            </button>
            <button id="doctor-refresh-btn" class="btn-primary" style="padding: calc(6px * var(--ui-scale, 1)) calc(14px * var(--ui-scale, 1)); font-size: var(--text-sm, 11.5px); display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
              <span>↻</span> <span>${escapeHtml(t('common.refresh') || 'Refresh')}</span>
            </button>
          </div>
        </div>

        <!-- Main 2-Column Master/Detail Layout (Fills Full Height) -->
        <div class="scanner-master-detail" style="grid-template-columns: clamp(260px, calc(320px * var(--ui-scale, 1)), 400px) 1fr; flex: 1; height: 100%; min-height: 0; display: grid; gap: calc(12px * var(--ui-scale, 1)); overflow: hidden;">
          
          <!-- Left: Worlds List -->
          ${worldsListHtml}

          <!-- Right: Deep Inspector & Save Hub Panel -->
          <div class="scanner-inspector-panel" id="doctor-inspector-root" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: calc(16px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); box-sizing: border-box; overflow-y: auto; gap: calc(14px * var(--ui-scale, 1));">
            ${selectedWorld ? `
              ${worldHeaderHtml}
              ${worldQuickStatsHtml}
              ${worldMetaFormHtml}
              ${worldOptionsHtml}
              ${storageBreakdownHtml}
              ${playerRosterHtml}
              <div id="doctor-deep-scan-results" style="display: flex; flex-direction: column; gap: calc(14px * var(--ui-scale, 1)); flex: 1;">
                ${deepScanResultsHtml}
              </div>
            ` : `
              <div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); font-size: var(--text-base, 13px);">
                Select a world on the left to inspect its health.
              </div>
            `}
          </div>

        </div>
      </div>
    </div>
  `;

  // Attach event handlers
  attachSavesDoctorListeners(container, renderSavesDoctorPanel);
  const { setupEventListeners } = await import('../../mod');
  setupEventListeners();
}
