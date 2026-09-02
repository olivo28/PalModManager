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
    container.innerHTML = renderHeroLandingHtml();
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
  const deepScanResultsHtml = selectedWorld ? renderDeepScanResultsHtml(curHealth, selectedWorld, curRepairing, curRestoring) : '';

  container.innerHTML = `
    <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
      ${subTabHeader()}
      
      <div class="scanner-scroll-panel" style="padding: 16px 20px; box-sizing: border-box; display: flex; flex-direction: column; gap: 14px; height: 100%; flex: 1; min-height: 0; overflow: hidden;">
        
        <!-- Header & Directory Selector Bar -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: 12px 18px; display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; flex-shrink: 0;">
          <div style="display: flex; flex-direction: column; gap: 3px;">
            <div style="display: flex; align-items: center; gap: 8px;">
              <span style="font-size: 17px;">💾</span>
              <span style="font-size: 14.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.saves_doctor_title') || 'Save Health Doctor & World Hub')}</span>
            </div>
            <span style="font-size: 11px; color: var(--text-muted);">${escapeHtml(t('scanner.saves_doctor_desc') || 'Inspect world options, players roster, storage health, safety snapshots, and 1-click rescue.')}</span>
          </div>

          <div style="display: flex; align-items: center; gap: 8px;">
            <button id="doctor-custom-folder-btn" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;">
              <span>📁</span> <span>${escapeHtml(t('scanner.btn_choose_saves_folder') || 'Choose Saves Folder')}</span>
            </button>
            <button id="doctor-refresh-btn" class="btn-primary" style="padding: 6px 14px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;">
              <span>↻</span> <span>${escapeHtml(t('common.refresh') || 'Refresh')}</span>
            </button>
          </div>
        </div>

        <!-- Main 2-Column Master/Detail Layout (Fills Full Height) -->
        <div class="scanner-master-detail" style="grid-template-columns: 340px 1fr; flex: 1; height: 100%; min-height: 0; display: grid; gap: 14px; overflow: hidden;">
          
          <!-- Left: Worlds List -->
          ${worldsListHtml}

          <!-- Right: Deep Inspector & Save Hub Panel -->
          <div class="scanner-inspector-panel" id="doctor-inspector-root" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: 18px 20px; box-sizing: border-box; overflow-y: auto; gap: 14px;">
            ${selectedWorld ? `
              ${worldHeaderHtml}
              ${worldQuickStatsHtml}
              ${worldMetaFormHtml}
              ${worldOptionsHtml}
              ${storageBreakdownHtml}
              ${playerRosterHtml}
              <div id="doctor-deep-scan-results" style="display: flex; flex-direction: column; gap: 14px; flex: 1;">
                ${deepScanResultsHtml}
              </div>
            ` : `
              <div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); font-size: 13px;">
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
