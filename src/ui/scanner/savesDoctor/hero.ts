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

export function renderHeroLandingHtml(): string {
  return `
    <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
      ${subTabHeader()}
      
      <div class="scanner-scroll-panel" style="padding: 24px; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; flex: 1; overflow-y: auto;">
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; padding: 32px 28px; max-width: 680px; width: 100%; box-shadow: 0 16px 40px rgba(0,0,0,0.3); display: flex; flex-direction: column; align-items: center; text-align: center; gap: 20px;">
          
          <div style="width: 64px; height: 64px; border-radius: 16px; background: rgba(74, 246, 38, 0.1); border: 1px solid rgba(74, 246, 38, 0.25); display: flex; align-items: center; justify-content: center; font-size: 32px;">
            🩺
          </div>

          <div style="display: flex; flex-direction: column; gap: 6px;">
            <h2 style="font-size: 18px; font-weight: 700; color: var(--text-primary); margin: 0;">${escapeHtml(t('scanner.saves_doctor_hero_title') || 'Save Health Doctor & World Hub')}</h2>
            <p style="font-size: 12px; color: var(--text-muted); margin: 0; line-height: 1.6; max-width: 540px;">
              ${escapeHtml(t('scanner.saves_doctor_hero_desc') || 'Diagnose savegame integrity, detect orphaned mod references, manage auto-backups, and compare save snapshots.')}
            </p>
          </div>

          <!-- Feature Cards Preview Grid -->
          <div style="display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; width: 100%; text-align: left; margin: 6px 0;">
            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <div style="font-size: 11.5px; font-weight: 700; color: #4af626; display: flex; align-items: center; gap: 6px;">
                <span>🛡️</span>
                <span>${escapeHtml(t('scanner.saves_feature_rescue_title') || 'Rescue & Clean')}</span>
              </div>
              <span style="font-size: 10px; color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_rescue_desc') || 'Detect and sanitize orphaned classes from uninstalled mods.')}</span>
            </div>

            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <div style="font-size: 11.5px; font-weight: 700; color: #38bdf8; display: flex; align-items: center; gap: 6px;">
                <span>⚖️</span>
                <span>${escapeHtml(t('scanner.saves_feature_diff_title') || 'Snapshot Comparison')}</span>
              </div>
              <span style="font-size: 10px; color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_diff_desc') || 'Inspect progression differences and rollback safely.')}</span>
            </div>

            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px; display: flex; flex-direction: column; gap: 4px;">
              <div style="font-size: 11.5px; font-weight: 700; color: #ffd166; display: flex; align-items: center; gap: 6px;">
                <span>👥</span>
                <span>${escapeHtml(t('scanner.saves_feature_roster_title') || 'Player Roster & Rules')}</span>
              </div>
              <span style="font-size: 10px; color: var(--text-muted); line-height: 1.4;">${escapeHtml(t('scanner.saves_feature_roster_desc') || 'View players, levels, and WorldOption multipliers.')}</span>
            </div>
          </div>

          <!-- Action Buttons -->
          <div style="display: flex; gap: 12px; align-items: center; margin-top: 6px;">
            <button id="btn-initial-scan-saves" class="btn-primary" style="padding: 10px 24px; font-size: 13px; font-weight: 700; display: flex; align-items: center; gap: 8px; box-shadow: 0 4px 14px rgba(74, 246, 38, 0.25);">
              <span>🩺</span>
              <span>${escapeHtml(t('scanner.btn_scan_saves_now') || 'Scan Savegames Now')}</span>
            </button>
            <button id="btn-initial-custom-folder" class="btn-secondary" style="padding: 10px 18px; font-size: 13px; display: flex; align-items: center; gap: 6px;">
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
      setIsLoadingWorlds(true);
      await rerenderCallback(container);

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
