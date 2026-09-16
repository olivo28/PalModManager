import { listSaveWorlds, getProfiles } from '../../../api';
import { subTabHeader } from '../mod';
import { escapeHtml } from '../rendering';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import {
  doctorState,
  customSavesPath,
  setCachedWorlds,
  setAvailableProfiles,
  setSelectedWorldDir,
  setIsLoadingWorlds,
} from './state';

export function renderHeroLandingHtml(isLoading: boolean = false): string {
  return `
    <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
      ${subTabHeader()}
      
      <div class="scanner-scroll-panel" style="padding: calc(24px * var(--ui-scale, 1)); display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; flex: 1; overflow-y: auto;">
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: calc(12px * var(--ui-scale, 1)); padding: calc(32px * var(--ui-scale, 1)) calc(28px * var(--ui-scale, 1)); max-width: min(calc(680px * var(--ui-scale, 1)), 92vw); width: 100%; box-shadow: 0 16px 40px rgba(0,0,0,0.3); display: flex; flex-direction: column; align-items: center; text-align: center; gap: calc(20px * var(--ui-scale, 1));">
          
          <div style="width: calc(64px * var(--ui-scale, 1)); height: calc(64px * var(--ui-scale, 1)); border-radius: calc(16px * var(--ui-scale, 1)); background: rgba(74, 246, 38, 0.1); border: 1px solid rgba(74, 246, 38, 0.25); display: flex; align-items: center; justify-content: center; font-size: calc(32px * var(--ui-scale, 1));">
            🩺
          </div>

          <div style="display: flex; flex-direction: column; gap: calc(6px * var(--ui-scale, 1));">
            <h2 style="font-size: var(--text-lg, 17px); font-weight: 700; color: var(--text-primary); margin: 0;">${escapeHtml(t('scanner.saves_doctor_hero_title') || 'Save Health Doctor & World Hub')}</h2>
            <p style="font-size: var(--text-sm, 12.5px); color: var(--text-muted); margin: 0; line-height: 1.6; max-width: calc(540px * var(--ui-scale, 1));">
              ${escapeHtml(t('scanner.saves_doctor_hero_desc') || 'Diagnose savegame integrity, detect orphaned mod references, manage auto-backups, and compare save snapshots.')}
            </p>
          </div>

          <!-- Feature Cards Preview Grid -->
          <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: calc(12px * var(--ui-scale, 1)); width: 100%; text-align: left; margin: calc(6px * var(--ui-scale, 1)) 0;">
            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: calc(8px * var(--ui-scale, 1)); padding: calc(12px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(4px * var(--ui-scale, 1));">
              <div style="font-size: var(--text-sm, 12px); font-weight: 700; color: #4af626; display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
                <span>🛡️</span>
                <span>${escapeHtml(t('scanner.saves_feature_rescue_title') || 'Rescue & Clean')}</span>
              </div>
              <span style="font-size: var(--text-xs, 10.5px); color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_rescue_desc') || 'Detect and sanitize orphaned classes from uninstalled mods.')}</span>
            </div>

            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: calc(8px * var(--ui-scale, 1)); padding: calc(12px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(4px * var(--ui-scale, 1));">
              <div style="font-size: var(--text-sm, 12px); font-weight: 700; color: #38bdf8; display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
                <span>⚖️</span>
                <span>${escapeHtml(t('scanner.saves_feature_diff_title') || 'Snapshot Comparison')}</span>
              </div>
              <span style="font-size: var(--text-xs, 10.5px); color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_diff_desc') || 'Inspect progression differences and rollback safely.')}</span>
            </div>

            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: calc(8px * var(--ui-scale, 1)); padding: calc(12px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(4px * var(--ui-scale, 1));">
              <div style="font-size: var(--text-sm, 12px); font-weight: 700; color: #ffd166; display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
                <span>👥</span>
                <span>${escapeHtml(t('scanner.saves_feature_roster_title') || 'Player Roster & Rules')}</span>
              </div>
              <span style="font-size: var(--text-xs, 10.5px); color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_roster_desc') || 'View players, levels, and WorldOption multipliers.')}</span>
            </div>
          </div>

          <!-- Action Buttons -->
          <div style="display: flex; gap: calc(12px * var(--ui-scale, 1)); align-items: center; margin-top: calc(6px * var(--ui-scale, 1)); flex-wrap: wrap; justify-content: center;">
            <button id="btn-initial-scan-saves" class="btn-primary" ${isLoading ? 'disabled' : ''} style="padding: calc(10px * var(--ui-scale, 1)) calc(24px * var(--ui-scale, 1)); font-size: var(--text-base, 13.5px); font-weight: 700; display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1)); box-shadow: 0 4px 14px rgba(74, 246, 38, 0.25); ${isLoading ? 'opacity: 0.85; cursor: wait;' : ''}">
              ${isLoading ? `
                <span class="spinner" style="width: calc(16px * var(--ui-scale, 1)); height: calc(16px * var(--ui-scale, 1)); border: 2px solid rgba(0,0,0,0.3); border-top-color: #000; border-radius: 50%; animation: spin 0.8s linear infinite;"></span>
                <span>${escapeHtml(t('scanner.saves_doctor_scanning') || 'Scanning Palworld SaveGames...')}</span>
              ` : `
                <span>🩺</span>
                <span>${escapeHtml(t('scanner.btn_scan_saves_now') || 'Scan Savegames Now')}</span>
              `}
            </button>
            <button id="btn-initial-custom-folder" class="btn-secondary" ${isLoading ? 'disabled' : ''} style="padding: calc(10px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); font-size: var(--text-base, 13.5px); display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
              <span>📁</span>
              <span>${escapeHtml(t('scanner.btn_custom_folder') || 'Choose Custom Folder')}</span>
            </button>
          </div>

        </div>
      </div>
    </div>
  `;
}

export function attachHeroLandingListeners(
  container: HTMLElement,
  rerenderCallback: (container: HTMLElement) => Promise<void>
): void {
  const scanBtn = container.querySelector('#btn-initial-scan-saves');
  if (scanBtn) {
    scanBtn.addEventListener('click', async () => {
      if (doctorState.isLoadingWorlds) return;
      setIsLoadingWorlds(true);
      await rerenderCallback(container);

      // Yield frame so browser renders button loading state
      await new Promise(r => setTimeout(r, 20));

      try {
        const curCustomPath = customSavesPath || doctorState.customSavesPath;
        const [worlds, profiles] = await Promise.all([
          listSaveWorlds(curCustomPath || undefined),
          getProfiles().catch(() => []),
        ]);
        setCachedWorlds(worlds);
        setAvailableProfiles(profiles || []);
        if (worlds.length > 0 && !(doctorState.selectedWorldDir)) {
          setSelectedWorldDir(worlds[0].worldDir);
        }
      } catch (err) {
        console.error('Failed to list save worlds:', err);
        setCachedWorlds([]);
      } finally {
        setIsLoadingWorlds(false);
        await rerenderCallback(container);
      }
    });
  }

  const customFolderBtn = container.querySelector('#btn-initial-custom-folder');
  if (customFolderBtn) {
    customFolderBtn.addEventListener('click', async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const selected = await open({
          directory: true,
          multiple: false,
          title: t('scanner.dialog_select_saves_folder') || 'Select Palworld SaveGames Folder',
        });
        if (selected && typeof selected === 'string') {
          doctorState.customSavesPath = selected;
          setIsLoadingWorlds(true);
          await rerenderCallback(container);

          const [worlds, profiles] = await Promise.all([
            listSaveWorlds(selected),
            getProfiles().catch(() => []),
          ]);
          setCachedWorlds(worlds);
          setAvailableProfiles(profiles || []);
          if (worlds.length > 0) {
            setSelectedWorldDir(worlds[0].worldDir);
          }
          setIsLoadingWorlds(false);
          await rerenderCallback(container);
          showToast('Loaded custom saves directory', 'success');
        }
      } catch (err) {
        setIsLoadingWorlds(false);
        await rerenderCallback(container);
        showToast(String(err), 'error');
      }
    });
  }
}
