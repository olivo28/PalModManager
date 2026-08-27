import {
  listSaveWorlds,
  deepScanSaveHealth,
  repairSaveHealth,
  restoreSaveBackup,
  createWorldBackup,
  openWorldFolder,
  exportWorldZip,
  pruneWorldBackups,
  saveWorldCustomMeta,
  getProfiles,
  inspectSnapshotDetails,
} from '../../api';
import type {
  SaveWorldSummary,
  SaveHealthReport,
  WorldOptionSettings,
  PlayerSaveInfo,
  SaveStorageBreakdown,
  WorldCustomMeta,
  SaveBackupSnapshot,
} from '../../api';
import { subTabHeader } from './mod';
import { escapeHtml } from './rendering';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t } from '../../utils/i18n';

export let cachedWorlds: SaveWorldSummary[] | null = null;
export let selectedWorldDir: string | null = null;
export let currentHealthReport: SaveHealthReport | null = null;
export let customSavesPath: string = '';
export let isLoadingWorlds = false;
export let isDeepScanning = false;
export let isRepairing = false;
export let isRestoringBackup = false;
export let isCreatingBackup = false;
export let isPruningBackups = false;
export let isSavingMeta = false;
export let availableProfiles: Array<{ id: string; name: string }> = [];

export function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export function formatDeathPenalty(penalty?: string | null): string {
  if (!penalty) return 'None';
  if (penalty.includes('None')) return t('scanner.death_none') || 'Keep All';
  if (penalty.includes('ItemAndEquipment')) return t('scanner.death_equipment') || 'Drop Items & Equipment';
  if (penalty.includes('Item')) return t('scanner.death_item') || 'Drop Items Only';
  if (penalty.includes('All')) return t('scanner.death_all') || 'Drop All (Items, Gear & Pals)';
  return penalty;
}

export async function renderSavesDoctorPanel(container: HTMLElement): Promise<void> {
  if (cachedWorlds === null) {
    if (isLoadingWorlds) {
      container.innerHTML = `
        <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
          ${subTabHeader()}
          <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px;">
            <div class="spinner" style="width: 36px; height: 36px; border: 3px solid rgba(255,255,255,0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
            <div style="font-size: 13.5px; font-weight: 600; color: var(--text-primary);">${escapeHtml(t('scanner.saves_doctor_scanning') || 'Scanning Palworld SaveGames...')}</div>
          </div>
        </div>
      `;
      const { setupEventListeners } = await import('./mod');
      setupEventListeners();
      return;
    }

    // Hero Landing Screen (Zero latency tab switch)
    container.innerHTML = `
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

    attachHeroLandingListeners(container);
    const { setupEventListeners } = await import('./mod');
    setupEventListeners();
    return;
  }

  const selectedWorld = cachedWorlds?.find(w => w.worldDir === selectedWorldDir) || cachedWorlds?.[0] || null;
  const worldOptions: WorldOptionSettings | null = currentHealthReport?.worldOptions || selectedWorld?.worldOptions || null;
  const customMeta: WorldCustomMeta | null = currentHealthReport?.customMeta || selectedWorld?.customMeta || null;

  function getHealthBadge(status: string, hasExternalEdits?: boolean): string {
    let html = '';
    if (status === 'corrupt') {
      html = `<span style="font-size: 10px; font-weight: 700; color: #ff5f56; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🔴 ${escapeHtml(t('scanner.status_corrupt') || 'Corrupted')}</span>`;
    } else if (status === 'warning') {
      html = `<span style="font-size: 10px; font-weight: 700; color: #ffaa00; background: rgba(255, 170, 0, 0.15); border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🟡 ${escapeHtml(t('scanner.status_warning') || 'Mod Issues')}</span>`;
    } else if (status === 'external_edits' || (!status && hasExternalEdits)) {
      html = `<span style="font-size: 10px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">⚠️ ${escapeHtml(t('scanner.status_external_edits') || 'External Edits')}</span>`;
    } else {
      html = `<span style="font-size: 10px; font-weight: 700; color: #4af626; background: rgba(74, 246, 38, 0.15); border: 1px solid rgba(74, 246, 38, 0.3); border-radius: 12px; padding: 2px 8px; text-transform: uppercase;">🟢 ${escapeHtml(t('scanner.status_healthy') || 'Healthy')}</span>`;
    }

    if (hasExternalEdits && status === 'warning') {
      html += ` <span style="font-size: 9.5px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 12px; padding: 2px 6px; text-transform: uppercase;">⚠️ ${escapeHtml(t('scanner.badge_external_edits') || 'External Edits')}</span>`;
    }

    return html;
  }

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
          <div class="scanner-master-list" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); overflow: hidden;">
            <div style="padding: 10px 14px; border-bottom: 1px solid var(--border); background: rgba(0,0,0,0.15); display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
              <span style="font-size: 11px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px;">
                ${escapeHtml(t('scanner.detected_worlds') || 'Detected Worlds')} (${cachedWorlds?.length || 0})
              </span>
            </div>

            <div class="scanner-master-items" style="padding: 10px; display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; overflow-y: auto;">
              ${isLoadingWorlds ? `
                <div style="padding: 32px 16px; text-align: center; color: var(--text-muted); font-size: 12px;">
                  ⏳ ${escapeHtml(t('scanner.loading') || 'Searching for saves...')}
                </div>
              ` : (cachedWorlds && cachedWorlds.length > 0) ? cachedWorlds.map(w => {
                const displayName = w.customMeta?.nickname || w.worldName;
                const isCustom = !!w.customMeta?.nickname;
                return `
                  <div class="scanner-mod-list-item world-list-card ${w.worldDir === selectedWorld?.worldDir ? 'active' : ''}" data-world-dir="${escapeHtml(w.worldDir)}" style="padding: 10px 12px; cursor: pointer; border-radius: 6px; display: flex; flex-direction: column; gap: 4px;">
                    <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
                      <span style="font-size: 13px; font-weight: 700; color: ${w.worldDir === selectedWorld?.worldDir ? 'var(--accent)' : 'var(--text-primary)'}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        🌍 ${escapeHtml(displayName)} ${isCustom ? `<span style="font-size: 9.5px; opacity: 0.7;">(${escapeHtml(w.worldName)})</span>` : ''}
                      </span>
                      ${getHealthBadge(w.healthStatus, w.hasExternalEdits)}
                    </div>
                    
                    <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: var(--text-primary); opacity: 0.9;">
                      <span>👤 ${escapeHtml(w.hostPlayerName || 'Host')} • <strong style="color: #4af626;">${w.playerLevel ? `Lv. ${w.playerLevel}` : 'Lv. ?'}</strong> • ${w.inGameDay ? `Day ${w.inGameDay}` : 'Day ?'}</span>
                      <span style="font-size: 10px; color: var(--text-muted);">${w.saveDate || ''}</span>
                    </div>

                    <div style="display: flex; align-items: center; justify-content: space-between; font-size: 10px; color: var(--text-muted);">
                      <span>📁 ${formatBytes(w.levelSizeBytes)} (${w.playerCount} ${escapeHtml(t('scanner.players_title') || 'Players')})</span>
                      <span>💾 ${w.backupCount} Snapshots</span>
                    </div>

                    ${w.customMeta?.boundProfileName ? `
                      <div style="font-size: 9.5px; color: #38bdf8; background: rgba(56, 189, 248, 0.08); padding: 2px 6px; border-radius: 4px; display: flex; align-items: center; gap: 4px; width: fit-content;">
                        <span>🔗 Profile: ${escapeHtml(w.customMeta.boundProfileName)}</span>
                      </div>
                    ` : ''}
                  </div>
                `;
              }).join('') : `
                <div style="padding: 32px 16px; text-align: center; color: var(--text-muted); font-size: 11.5px; display:flex; flex-direction:column; gap:6px;">
                  <span>🔍 No Palworld saves detected in default path.</span>
                  <span style="font-size:10.5px;">Click "Choose Saves Folder" to locate your save directory manually.</span>
                </div>
              `}
            </div>
          </div>

          <!-- Right: Deep Inspector & Save Hub Panel -->
          <div class="scanner-inspector-panel" id="doctor-inspector-root" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: 18px 20px; box-sizing: border-box; overflow-y: auto; gap: 14px;">
            ${selectedWorld ? `
              <!-- World Header & Comprehensive Action Toolbar -->
              <div class="scanner-inspector-header" style="display: flex; flex-direction: column; gap: 12px; flex-shrink: 0; padding-bottom: 12px; border-bottom: 1px solid var(--border);">
                <div style="display: flex; justify-content: space-between; align-items: flex-start; flex-wrap: wrap; gap: 10px;">
                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap;">
                      <span style="font-size: 20px;">🌍</span>
                      <h3 style="margin: 0; font-size: 17px; font-weight: 700; color: var(--text-primary);">
                        ${escapeHtml(customMeta?.nickname || selectedWorld.worldName)}
                      </h3>
                      ${customMeta?.nickname ? `<span style="font-size: 12px; color: var(--text-muted); font-weight: 500;">(${escapeHtml(selectedWorld.worldName)})</span>` : ''}
                      ${getHealthBadge(selectedWorld.healthStatus, selectedWorld.hasExternalEdits)}
                    </div>
                    <span style="font-size: 10.5px; font-family: monospace; color: var(--text-muted);">${escapeHtml(selectedWorld.worldDir)}</span>
                  </div>

                  <!-- Quick Action Buttons Group -->
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <button id="btn-backup-world" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${isCreatingBackup ? 'disabled' : ''} title="${escapeHtml(t('scanner.btn_backup_now') || 'Create manual timestamped ZIP backup')}">
                      <span>🛡️</span>
                      <span>${isCreatingBackup ? 'Saving...' : escapeHtml(t('scanner.btn_backup_now') || 'Backup World Now')}</span>
                    </button>

                    <button id="btn-open-world-folder" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" title="${escapeHtml(t('scanner.btn_open_folder') || 'Open save folder in Explorer')}">
                      <span>📂</span>
                      <span>${escapeHtml(t('scanner.btn_open_folder') || 'Open in Explorer')}</span>
                    </button>

                    <button id="btn-export-world-zip" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" title="${escapeHtml(t('scanner.btn_export_zip') || 'Export save to ZIP')}">
                      <span>📤</span>
                      <span>${escapeHtml(t('scanner.btn_export_zip') || 'Export Save')}</span>
                    </button>

                    <button id="btn-prune-backups" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${isPruningBackups ? 'disabled' : ''} title="${escapeHtml(t('scanner.btn_prune_backups') || 'Prune old game auto-backups to free disk space')}">
                      <span>🧹</span>
                      <span>${isPruningBackups ? 'Pruning...' : escapeHtml(t('scanner.btn_prune_backups') || 'Prune Backups')}</span>
                    </button>

                    <button id="btn-run-deep-scan" class="btn-primary" style="padding: 6px 14px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${isDeepScanning ? 'disabled' : ''}>
                      <span>${isDeepScanning ? '⏳' : '🔬'}</span>
                      <span>${isDeepScanning ? escapeHtml(t('scanner.scanning_deep') || 'Scanning GVAS...') : escapeHtml(t('scanner.btn_deep_scan') || 'Deep Health Scan')}</span>
                    </button>
                  </div>
                </div>
              </div>

              <!-- World Quick Stats Grid -->
              <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 10px; flex-shrink: 0;">
                <div class="premium-stat-card">
                  <div style="font-size: 10px; color: var(--text-muted); text-transform: uppercase;">${escapeHtml(t('scanner.stat_in_game_day') || 'In-Game Day')}</div>
                  <div class="premium-stat-value">${selectedWorld.inGameDay ? `Day ${selectedWorld.inGameDay}` : 'N/A'}</div>
                </div>
                <div class="premium-stat-card">
                  <div style="font-size: 10px; color: var(--text-muted); text-transform: uppercase;">${escapeHtml(t('scanner.stat_host_player') || 'Host Player')}</div>
                  <div class="premium-stat-value" style="color: #4af626; font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    👤 ${escapeHtml(selectedWorld.hostPlayerName || 'Host')} <span style="font-size: 13px;">(${selectedWorld.playerLevel ? `Lv. ${selectedWorld.playerLevel}` : 'Lv. ?'})</span>
                  </div>
                </div>
                <div class="premium-stat-card">
                  <div style="font-size: 10px; color: var(--text-muted); text-transform: uppercase;">${escapeHtml(t('scanner.stat_save_size') || 'Save File Size')}</div>
                  <div class="premium-stat-value" style="font-size: 15px;">${formatBytes(selectedWorld.levelSizeBytes)}</div>
                </div>
                <div class="premium-stat-card">
                  <div style="font-size: 10px; color: var(--text-muted); text-transform: uppercase;">${escapeHtml(t('scanner.stat_auto_backups') || 'Game Auto-Backups')}</div>
                  <div class="premium-stat-value" style="font-size: 15px; color: #38bdf8;">${selectedWorld.backupCount} Snapshots</div>
                </div>
              </div>

              <!-- World Custom Nickname, Profile Binding & Notes Card -->
              <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-size: 15px;">🏷️</span>
                    <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.btn_save_meta') || 'World Profile Binding & Notes')}</span>
                  </div>
                  <button id="btn-save-meta" class="btn-primary" style="padding: 4px 12px; font-size: 11px; display: flex; align-items: center; gap: 5px;" ${isSavingMeta ? 'disabled' : ''}>
                    <span>💾</span>
                    <span>${isSavingMeta ? 'Saving...' : escapeHtml(t('common.save') || 'Save')}</span>
                  </button>
                </div>

                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">Custom World Nickname</label>
                    <input id="meta-nickname-input" type="text" class="input-text" style="font-size: 12px; padding: 6px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary);" placeholder="${escapeHtml(t('scanner.meta_nickname_placeholder') || 'e.g. Main Co-op World')}" value="${escapeHtml(customMeta?.nickname || '')}">
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 4px;">
                    <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">${escapeHtml(t('scanner.meta_bound_profile') || 'Bound Mod Profile:')}</label>
                    <select id="meta-profile-select" style="font-size: 12px; padding: 6px 10px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary);">
                      <option value="">-- ${escapeHtml(t('scanner.meta_bound_profile_none') || 'None (Use Active Profile)')} --</option>
                      ${availableProfiles.map(p => `
                        <option value="${escapeHtml(p.id)}" ${customMeta?.boundProfileId === p.id ? 'selected' : ''}>${escapeHtml(p.name)}</option>
                      `).join('')}
                    </select>
                  </div>
                </div>

                <div style="display: flex; flex-direction: column; gap: 4px;">
                  <label style="font-size: 10.5px; color: var(--text-muted); font-weight: 600;">World Notes & Private Warnings</label>
                  <textarea id="meta-notes-input" rows="2" style="font-size: 11.5px; padding: 6px 10px; background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary); resize: vertical;" placeholder="${escapeHtml(t('scanner.meta_notes_placeholder') || 'Add private notes, mod requirements, or warnings for this world...')}">${escapeHtml(customMeta?.notes || '')}</textarea>
                </div>
              </div>

              <!-- World Option Settings (WorldOption.sav) Card -->
              ${worldOptions?.exists ? `
                <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 14px; flex-shrink: 0;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 16px;">⚙️</span>
                      <span style="font-size: 13px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.world_options_title') || 'World Rules & Difficulty (WorldOption.sav)')}</span>
                    </div>
                    <span class="badge" style="font-size: 10.5px; padding: 3px 10px; background: rgba(56, 189, 248, 0.15); color: #38bdf8; font-weight: 700; border: 1px solid rgba(56, 189, 248, 0.3);">
                      ${escapeHtml(worldOptions.difficulty || 'Custom')} Mode
                    </span>
                  </div>

                  <!-- 1. General & Game Pace -->
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; font-weight: 700; color: #60a5fa; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
                      <span>🎯</span> ${escapeHtml(t('scanner.rules_cat_general') || 'General & Game Pace')}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_exp_rate') || 'EXP Rate')}:</span>
                        <strong style="color: #4af626;">${worldOptions.expRate !== null && worldOptions.expRate !== undefined ? `${worldOptions.expRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_day_speed') || 'Day Speed')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.dayTimeSpeedRate !== null && worldOptions.dayTimeSpeedRate !== undefined ? `${worldOptions.dayTimeSpeedRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_night_speed') || 'Night Speed')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.nightTimeSpeedRate !== null && worldOptions.nightTimeSpeedRate !== undefined ? `${worldOptions.nightTimeSpeedRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_work_speed') || 'Work Speed')}:</span>
                        <strong style="color: #a3e635;">${worldOptions.workSpeedRate !== null && worldOptions.workSpeedRate !== undefined ? `${worldOptions.workSpeedRate}x` : '1.0x'}</strong>
                      </div>
                    </div>
                  </div>

                  <!-- 2. Pals & Capture -->
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; font-weight: 700; color: #f59e0b; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
                      <span>🐾</span> ${escapeHtml(t('scanner.rules_cat_pals') || 'Pals & Capturing')}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_capture_rate') || 'Capture Rate')}:</span>
                        <strong style="color: #38bdf8;">${worldOptions.palCaptureRate !== null && worldOptions.palCaptureRate !== undefined ? `${worldOptions.palCaptureRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_spawn_rate') || 'Pal Spawn Rate')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.palSpawnNumRate !== null && worldOptions.palSpawnNumRate !== undefined ? `${worldOptions.palSpawnNumRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_damage') || 'Pal Damage')}:</span>
                        <strong style="color: #fb923c;">${worldOptions.palDamageRate !== null && worldOptions.palDamageRate !== undefined ? `${worldOptions.palDamageRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_egg_hatching') || 'Egg Hatching')}:</span>
                        <strong style="color: #ffd166;">${worldOptions.palEggHatchingHours !== null && worldOptions.palEggHatchingHours !== undefined ? `${worldOptions.palEggHatchingHours}h` : 'Default'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_hunger') || 'Pal Hunger')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.palStomachDecreaseRate !== null && worldOptions.palStomachDecreaseRate !== undefined ? `${worldOptions.palStomachDecreaseRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pal_stamina') || 'Pal Stamina')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.palStaminaDecreaseRate !== null && worldOptions.palStaminaDecreaseRate !== undefined ? `${worldOptions.palStaminaDecreaseRate}x` : '1.0x'}</strong>
                      </div>
                    </div>
                  </div>

                  <!-- 3. Player & Survival -->
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; font-weight: 700; color: #ef4444; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
                      <span>⚔️</span> ${escapeHtml(t('scanner.rules_cat_player') || 'Player & Survival')}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_damage') || 'Player Damage')}:</span>
                        <strong style="color: #4af626;">${worldOptions.playerDamageRate !== null && worldOptions.playerDamageRate !== undefined ? `${worldOptions.playerDamageRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_death_penalty') || 'Death Penalty')}:</span>
                        <strong style="color: ${worldOptions.deathPenalty?.includes('None') ? '#4af626' : '#ffaa00'}; font-size: 10.5px;">${formatDeathPenalty(worldOptions.deathPenalty)}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_hunger') || 'Player Hunger')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.playerStomachDecreaseRate !== null && worldOptions.playerStomachDecreaseRate !== undefined ? `${worldOptions.playerStomachDecreaseRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_player_stamina') || 'Player Stamina')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.playerStaminaDecreaseRate !== null && worldOptions.playerStaminaDecreaseRate !== undefined ? `${worldOptions.playerStaminaDecreaseRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_pvp') || 'PvP')}:</span>
                        <strong style="color: ${worldOptions.enablePlayerToPlayerDamage ? '#ff5f56' : '#a3e635'};">${worldOptions.enablePlayerToPlayerDamage ? 'Enabled' : 'Disabled'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_friendly_fire') || 'Friendly Fire')}:</span>
                        <strong style="color: ${worldOptions.enableFriendlyFire ? '#ff5f56' : '#a3e635'};">${worldOptions.enableFriendlyFire ? 'Enabled' : 'Disabled'}</strong>
                      </div>
                    </div>
                  </div>

                  <!-- 4. Base & Guilds -->
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; font-weight: 700; color: #10b981; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
                      <span>🏰</span> ${escapeHtml(t('scanner.rules_cat_bases') || 'Base & Guilds')}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_base_workers') || 'Max Base Pals')}:</span>
                        <strong style="color: #a3e635;">${worldOptions.baseCampWorkerMaxNum !== null && worldOptions.baseCampWorkerMaxNum !== undefined ? `${worldOptions.baseCampWorkerMaxNum} Pals` : '15'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_max_bases') || 'Max Bases')}:</span>
                        <strong style="color: #38bdf8;">${worldOptions.baseCampMaxNum !== null && worldOptions.baseCampMaxNum !== undefined ? `${worldOptions.baseCampMaxNum}` : '3'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_invaders') || 'Base Raids')}:</span>
                        <strong style="color: ${worldOptions.enableInvaderEnemy !== false ? '#4af626' : '#ff5f56'};">${worldOptions.enableInvaderEnemy !== false ? 'Enabled' : 'Disabled'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_decay_rate') || 'Structure Decay')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.buildObjectDeteriorationDamageRate !== null && worldOptions.buildObjectDeteriorationDamageRate !== undefined ? `${worldOptions.buildObjectDeteriorationDamageRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_guild_max_players') || 'Guild Max Players')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.guildPlayerMaxNum !== null && worldOptions.guildPlayerMaxNum !== undefined ? `${worldOptions.guildPlayerMaxNum}` : '20'}</strong>
                      </div>
                    </div>
                  </div>

                  <!-- 5. Loot, Drops & World -->
                  <div style="display: flex; flex-direction: column; gap: 6px;">
                    <div style="font-size: 11px; font-weight: 700; color: #a855f7; text-transform: uppercase; letter-spacing: 0.5px; display: flex; align-items: center; gap: 5px;">
                      <span>☄️</span> ${escapeHtml(t('scanner.rules_cat_drops') || 'Loot, Drops & World')}
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 6px;">
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_collection_drop') || 'Drop Multiplier')}:</span>
                        <strong style="color: #4af626;">${worldOptions.collectionDropRate !== null && worldOptions.collectionDropRate !== undefined ? `${worldOptions.collectionDropRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_enemy_drop') || 'Enemy Drops')}:</span>
                        <strong style="color: #38bdf8;">${worldOptions.enemyDropItemRate !== null && worldOptions.enemyDropItemRate !== undefined ? `${worldOptions.enemyDropItemRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_resource_respawn') || 'Respawn Speed')}:</span>
                        <strong style="color: var(--text-primary);">${worldOptions.collectionObjectRespawnSpeedRate !== null && worldOptions.collectionObjectRespawnSpeedRate !== undefined ? `${worldOptions.collectionObjectRespawnSpeedRate}x` : '1.0x'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_supply_drop_span') || 'Supply Drops')}:</span>
                        <strong style="color: #ffd166;">${worldOptions.supplyDropSpan !== null && worldOptions.supplyDropSpan !== undefined ? `${worldOptions.supplyDropSpan} min` : '180 min'}</strong>
                      </div>
                      <div style="background: rgba(0,0,0,0.18); padding: 6px 10px; border-radius: 5px; border: 1px solid rgba(255,255,255,0.05); display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <span style="color: var(--text-muted);">${escapeHtml(t('scanner.opt_fast_travel') || 'Fast Travel')}:</span>
                        <strong style="color: ${worldOptions.enableFastTravel !== false ? '#4af626' : '#ff5f56'};">${worldOptions.enableFastTravel !== false ? 'Enabled' : 'Disabled'}</strong>
                      </div>
                    </div>
                  </div>
                </div>
              ` : ''}

              <!-- Storage & Compression Breakdown Card (When available from Deep Scan) -->
              ${currentHealthReport?.storageBreakdown ? `
                <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 16px;">📊</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.storage_title') || 'Storage & Compression Breakdown')}</span>
                    </div>
                    <span class="badge" style="font-size: 10px; padding: 2px 8px; background: rgba(74, 246, 38, 0.15); color: #4af626;">
                      Compressed to ${currentHealthReport.storageBreakdown.compressionRatioPct}% of original size
                    </span>
                  </div>

                  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 8px;">
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_level_sav') || 'Level.sav')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${formatBytes(currentHealthReport.storageBreakdown.levelSavBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_players') || 'Players Folder')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${formatBytes(currentHealthReport.storageBreakdown.playersDirBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_backups') || 'Auto-Backups')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: #38bdf8;">${formatBytes(currentHealthReport.storageBreakdown.backupsDirBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_uncompressed') || 'Decompressed in RAM')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: #ffd166;">${formatBytes(currentHealthReport.storageBreakdown.uncompressedLevelBytes)}</span>
                    </div>
                  </div>
                </div>
              ` : ''}

              <!-- Players & Co-op Roster Card (When available from Deep Scan) -->
              ${currentHealthReport?.playerRoster && currentHealthReport.playerRoster.length > 0 ? `
                <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 16px;">👥</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.players_title') || 'Players & Character Saves')} (${currentHealthReport.playerRoster.length})</span>
                    </div>
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow-y: auto;">
                    ${currentHealthReport.playerRoster.map(p => `
                      <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                        <div style="display: flex; align-items: center; gap: 10px; overflow: hidden;">
                          <span style="font-size: 16px;">${p.isHost ? '👑' : '👤'}</span>
                          <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                            <div style="display: flex; align-items: center; gap: 6px;">
                              <span style="font-weight: 700; color: ${p.isHost ? '#4af626' : 'var(--text-primary)'};">
                                ${escapeHtml(p.playerName || (p.isHost ? 'Host Player' : 'Player'))}
                              </span>
                              ${p.playerLevel ? `<span class="badge" style="font-size: 9px; padding: 1px 5px; background: rgba(74, 246, 38, 0.15); color: #4af626;">Lv. ${p.playerLevel}</span>` : ''}
                              ${p.isHost ? '<span style="font-size: 9.5px; opacity: 0.7; color: var(--text-muted);">(Host)</span>' : ''}
                            </div>
                            <div style="display: flex; align-items: center; gap: 8px; font-size: 9.5px; color: var(--text-muted);">
                              <span style="font-family: monospace;">${escapeHtml(p.playerUid)}</span>
                            </div>
                          </div>
                        </div>

                        <div style="display: flex; align-items: center; gap: 12px; flex-shrink: 0;">
                          <div style="display: flex; flex-direction: column; align-items: flex-end; gap: 1px; font-size: 10px; color: var(--text-muted);">
                            <span>${formatBytes(p.fileSizeBytes)}</span>
                            <span>${p.lastPlayedDate || ''}</span>
                          </div>
                          ${p.isCorrupt ? `
                            <span style="font-size: 9.5px; font-weight: 700; color: #ff5f56; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); border-radius: 10px; padding: 2px 6px;">🔴 ${escapeHtml(t('scanner.player_corrupt_badge') || 'Corrupt')}</span>
                          ` : `
                            <span style="font-size: 9.5px; font-weight: 700; color: #4af626; background: rgba(74, 246, 38, 0.15); border: 1px solid rgba(74, 246, 38, 0.3); border-radius: 10px; padding: 2px 6px;">🟢 ${escapeHtml(t('scanner.player_healthy_badge') || 'Healthy')}</span>
                          `}
                        </div>
                      </div>
                    `).join('')}
                  </div>
                </div>
              ` : ''}

              <!-- Deep Scan Results Area -->
              <div id="doctor-deep-scan-results" style="display: flex; flex-direction: column; gap: 14px; flex: 1;">
                ${currentHealthReport && currentHealthReport.worldId === selectedWorld.worldId ? `
                  <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 12px; flex-shrink: 0;">
                    <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 16px;">${currentHealthReport.healthStatus === 'healthy' ? '✅' : '⚠️'}</span>
                        <span style="font-size: 13px; font-weight: 700; color: var(--text-primary);">${escapeHtml(currentHealthReport.summaryMessage)}</span>
                      </div>
                      ${currentHealthReport.canRepair ? `
                        <button id="btn-repair-save" class="btn-primary" style="background: #2ed573; border-color: #2ed573; color: #000; font-weight: 700; padding: 6px 14px; font-size: 12px; display: flex; align-items: center; gap: 6px;" ${isRepairing ? 'disabled' : ''}>
                          <span>🛠️</span>
                          <span>${isRepairing ? escapeHtml(t('scanner.repairing') || 'Sanitizing...') : escapeHtml(t('scanner.btn_sanitize_save') || '1-Click Rescue & Clean Save')}</span>
                        </button>
                      ` : ''}
                    </div>

                    ${(currentHealthReport.hasExternalEdits || currentHealthReport.externalEditDetails) ? `
                      <div style="background: linear-gradient(135deg, rgba(255, 170, 0, 0.12) 0%, rgba(255, 100, 0, 0.06) 100%); border: 1px solid rgba(255, 170, 0, 0.35); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 8px;">
                        <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
                          <div style="display: flex; align-items: center; gap: 8px; font-weight: 700; color: #ffd166; font-size: 12.5px;">
                            <span style="font-size: 16px;">⚠️</span>
                            <span>${escapeHtml(t('scanner.anomaly_editor_title') || 'External Modification & State Desync')}</span>
                          </div>
                          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
                            ${currentHealthReport.externalEditDetails?.toolName ? `
                              <span style="font-size: 10px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 10px; padding: 2px 8px;">
                                🛠️ ${escapeHtml(currentHealthReport.externalEditDetails.toolName)}
                              </span>
                            ` : ''}
                            ${currentHealthReport.externalEditDetails?.editorBackupCount ? `
                              <span style="font-size: 10px; font-weight: 600; color: #ffaa00; background: rgba(255, 170, 0, 0.15); border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 10px; padding: 2px 8px;">
                                📦 ${currentHealthReport.externalEditDetails.editorBackupCount} Editor Backups
                              </span>
                            ` : ''}
                            ${currentHealthReport.externalEditDetails?.sizeReductionPct ? `
                              <span style="font-size: 10px; font-weight: 700; color: #ff5f56; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); border-radius: 10px; padding: 2px 8px;">
                                📉 -${currentHealthReport.externalEditDetails.sizeReductionPct}% File Size Loss
                              </span>
                            ` : ''}
                          </div>
                        </div>

                        <div style="font-size: 11.5px; color: var(--text-primary); opacity: 0.9; line-height: 1.5;">
                          ${escapeHtml(currentHealthReport.externalEditDetails?.details || t('scanner.anomaly_editor_desc') || '')}
                        </div>

                        <div style="font-size: 11px; color: #ffd166; display: flex; align-items: center; gap: 6px; padding-top: 4px; border-top: 1px solid rgba(255, 170, 0, 0.2);">
                          <span>💡</span>
                          <span>${escapeHtml(t('scanner.anomaly_solution_tip') || 'To fix crash-on-load, restore one of the intact Palworld automatic snapshots below.')}</span>
                        </div>
                      </div>
                    ` : ''}

                    <!-- Mod Data & Custom Classes Section -->
                    <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 8px;">
                      <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
                        <div style="display: flex; align-items: center; gap: 8px;">
                          <span style="font-size: 16px;">📦</span>
                          <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.mod_data_title') || 'Mod Data & Custom Classes')}</span>
                        </div>
                        <span class="badge" style="font-size: 10px; padding: 2px 8px; background: ${(currentHealthReport.totalModReferences || 0) > 0 ? (currentHealthReport.orphanedModRefs.length > 0 ? 'rgba(255, 170, 0, 0.15)' : 'rgba(74, 246, 38, 0.15)') : 'rgba(255,255,255,0.06)'}; color: ${(currentHealthReport.totalModReferences || 0) > 0 ? (currentHealthReport.orphanedModRefs.length > 0 ? '#ffaa00' : '#4af626') : 'var(--text-muted)'};">
                          ${(currentHealthReport.totalModReferences || 0) > 0 ? `${currentHealthReport.totalModReferences} Mod References Found` : (escapeHtml(t('scanner.mod_data_none_detected') || '0 Mod Classes (100% Vanilla)'))}
                        </span>
                      </div>

                      ${(currentHealthReport.totalModReferences || 0) === 0 ? `
                        <div style="font-size: 11px; color: var(--text-muted); line-height: 1.5; background: rgba(255,255,255,0.02); border: 1px dashed var(--border); border-radius: 6px; padding: 10px 12px; display: flex; align-items: center; gap: 8px;">
                          <span style="font-size: 14px;">ℹ️</span>
                          <span>${escapeHtml(t('scanner.mod_data_clean_desc') || 'Level.sav contains only official vanilla Palworld classes. No third-party UE4SS, Lua, or Pak mod classes are embedded in this save.')}</span>
                        </div>
                      ` : `
                        <div style="font-size: 11px; color: var(--text-secondary); line-height: 1.4;">
                          ${escapeHtml(t('scanner.mod_data_detected_desc') || 'The following custom assets and mod classes were found embedded inside Level.sav:')}
                        </div>
                        ${currentHealthReport.orphanedModRefs.length > 0 ? `
                          <div style="display: flex; flex-direction: column; gap: 4px; max-height: 140px; overflow-y: auto;">
                            ${currentHealthReport.orphanedModRefs.map(orphan => `
                              <div style="background: rgba(255, 170, 0, 0.06); border: 1px solid rgba(255, 170, 0, 0.2); border-radius: 4px; padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
                                <div style="display: flex; flex-direction: column; gap: 2px; overflow: hidden;">
                                  <span style="font-weight: 700; color: #ffd166;">Mod: ${escapeHtml(orphan.modHintName)}</span>
                                  <span style="font-family: monospace; font-size: 10px; color: var(--text-muted); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">${escapeHtml(orphan.assetPath)}</span>
                                </div>
                                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(255,255,255,0.06); color: var(--text-primary); flex-shrink: 0;">${orphan.occurrences} refs</span>
                              </div>
                            `).join('')}
                          </div>
                        ` : `
                          <div style="display: flex; flex-direction: column; gap: 4px; max-height: 120px; overflow-y: auto;">
                            ${currentHealthReport.rawModPathsFound.slice(0, 10).map(path => `
                              <div style="background: rgba(74, 246, 38, 0.05); border: 1px solid rgba(74, 246, 38, 0.2); border-radius: 4px; padding: 5px 8px; font-size: 10.5px; font-family: monospace; color: #a3e635; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">
                                ${escapeHtml(path)}
                              </div>
                            `).join('')}
                          </div>
                        `}
                      `}
                    </div>
                  </div>


                  <!-- Auto-Backups Snapshot Browser Table -->
                  ${currentHealthReport.availableBackups.length > 0 ? `
                    <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; flex: 1; min-height: 220px;">
                      <div style="display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
                        <div style="display: flex; flex-direction: column; gap: 2px;">
                          <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="font-size: 16px;">💾</span>
                            <span style="font-size: 13.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.backups_section_title') || 'Game Auto-Backup Snapshots')} (${currentHealthReport.availableBackups.length})</span>
                          </div>
                          <span style="font-size: 10.5px; color: var(--text-muted);">${escapeHtml(t('scanner.backups_section_desc') || 'Palworld creates snapshot backups every 10 minutes. If your world crashes, restoring a snapshot is the safest recovery method.')}</span>
                        </div>
                      </div>

                      <div style="display: flex; flex-direction: column; gap: 6px; flex: 1; overflow-y: auto; max-height: calc(100vh - 460px); min-height: 180px; padding-right: 4px;">
                        ${currentHealthReport.availableBackups.map(snap => {
                          const diffBytes = (selectedWorld.levelSizeBytes || 0) - snap.levelSizeBytes;
                          let diffTag = '';
                          if (Math.abs(diffBytes) < 512) {
                            diffTag = `<span style="font-size: 9.5px; color: #38bdf8; background: rgba(56, 189, 248, 0.1); border: 1px solid rgba(56, 189, 248, 0.25); border-radius: 4px; padding: 1px 6px;">${escapeHtml(t('scanner.snap_same_size') || 'Identical Size')}</span>`;
                          } else if (diffBytes > 0) {
                            diffTag = `<span style="font-size: 9.5px; color: #4af626; background: rgba(74, 246, 38, 0.1); border: 1px solid rgba(74, 246, 38, 0.25); border-radius: 4px; padding: 1px 6px;">+${formatBytes(diffBytes)} ${escapeHtml(t('scanner.snap_size_growth') || 'growth')}</span>`;
                          } else {
                            diffTag = `<span style="font-size: 9.5px; color: #ff5f56; background: rgba(255, 95, 86, 0.1); border: 1px solid rgba(255, 95, 86, 0.25); border-radius: 4px; padding: 1px 6px;">-${formatBytes(Math.abs(diffBytes))} ${escapeHtml(t('scanner.snap_size_loss') || 'smaller')}</span>`;
                          }

                          return `
                            <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 6px; padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; gap: 10px; transition: background 0.15s ease;">
                              <div style="display: flex; align-items: center; gap: 10px;">
                                <span style="font-size: 15px;">📦</span>
                                <div style="display: flex; flex-direction: column; gap: 2px;">
                                  <div style="display: flex; align-items: center; gap: 8px;">
                                    <span style="font-size: 12px; font-weight: 700; color: var(--text-primary); font-family: monospace;">${escapeHtml(snap.timestamp)}</span>
                                    ${diffTag}
                                  </div>
                                  <span style="font-size: 10px; color: var(--text-muted);">${formatBytes(snap.levelSizeBytes)} • ${snap.localDataExists ? 'Level + LocalData' : 'Level only'}</span>
                                </div>
                              </div>

                              <div style="display: flex; align-items: center; gap: 6px;">
                                <button class="btn-secondary btn-compare-snap" data-slot="${escapeHtml(snap.slotName || (snap as any).slot_name)}" data-time="${escapeHtml(snap.timestamp)}" data-size="${snap.levelSizeBytes}" data-local="${snap.localDataExists ? 'true' : 'false'}" data-day="${snap.inGameDay || ''}" data-level="${snap.playerLevel || ''}" data-host="${escapeHtml(snap.hostPlayerName || '')}" style="padding: 4px 10px; font-size: 11px; display: flex; align-items: center; gap: 4px;">
                                  <span>🔍</span>
                                  <span>${escapeHtml(t('scanner.btn_compare_snapshot') || 'Compare')}</span>
                                </button>
                                <button class="btn-secondary btn-restore-snap" data-slot="${escapeHtml(snap.slotName || (snap as any).slot_name)}" data-time="${escapeHtml(snap.timestamp)}" style="padding: 4px 10px; font-size: 11px; display: flex; align-items: center; gap: 4px;" ${isRestoringBackup ? 'disabled' : ''}>
                                  <span>🔄</span>
                                  <span>${escapeHtml(t('scanner.btn_restore_snapshot') || 'Restore')}</span>
                                </button>
                              </div>
                            </div>
                          `;
                        }).join('')}
                      </div>
                    </div>
                  ` : ''}

                ` : `
                  <div style="background: rgba(255, 255, 255, 0.02); border: 1px dashed var(--border); border-radius: 8px; padding: 36px 20px; text-align: center; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; flex: 1;">
                    <span style="font-size: 32px;">🔬</span>
                    <span style="font-size: 13.5px; font-weight: 600; color: var(--text-primary);">${escapeHtml(t('scanner.deep_scan_prompt_title') || 'Deep Health Scan Ready')}</span>
                    <span style="font-size: 11px; color: var(--text-muted); max-width: 480px; line-height: 1.5;">
                      ${escapeHtml(t('scanner.deep_scan_prompt_desc') || 'Click "Deep Health Scan" above to decompress Level.sav and check for missing classes from removed mods that cause loading crashes.')}
                    </span>
                  </div>
                `}
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
  attachSavesDoctorListeners(container);
  const { setupEventListeners } = await import('./mod');
  setupEventListeners();
}

function attachHeroLandingListeners(container: HTMLElement): void {
  const scanBtn = container.querySelector('#btn-initial-scan-saves');
  if (scanBtn) {
    scanBtn.addEventListener('click', async () => {
      isLoadingWorlds = true;
      await renderSavesDoctorPanel(container);

      try {
        const [worlds, profiles] = await Promise.all([
          listSaveWorlds(customSavesPath || undefined),
          getProfiles().catch(() => []),
        ]);
        cachedWorlds = worlds;
        availableProfiles = profiles || [];
        if (cachedWorlds.length > 0 && !selectedWorldDir) {
          selectedWorldDir = cachedWorlds[0].worldDir;
        }
      } catch (err) {
        console.error('Failed to list save worlds:', err);
        cachedWorlds = [];
      } finally {
        isLoadingWorlds = false;
        await renderSavesDoctorPanel(container);
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
          customSavesPath = selected;
          isLoadingWorlds = true;
          await renderSavesDoctorPanel(container);

          const [worlds, profiles] = await Promise.all([
            listSaveWorlds(customSavesPath),
            getProfiles().catch(() => []),
          ]);
          cachedWorlds = worlds;
          availableProfiles = profiles || [];
          if (cachedWorlds.length > 0) {
            selectedWorldDir = cachedWorlds[0].worldDir;
          }
          isLoadingWorlds = false;
          await renderSavesDoctorPanel(container);
          showToast('Loaded custom saves directory', 'success');
        }
      } catch (err) {
        isLoadingWorlds = false;
        await renderSavesDoctorPanel(container);
        showToast(String(err), 'error');
      }
    });
  }
}

export function attachSavesDoctorListeners(container: HTMLElement): void {
  // World selection
  container.querySelectorAll('.world-list-card').forEach(card => {
    card.addEventListener('click', async () => {
      const worldDir = (card as HTMLElement).dataset.worldDir;
      if (!worldDir || worldDir === selectedWorldDir) return;
      selectedWorldDir = worldDir;
      currentHealthReport = null;
      await renderSavesDoctorPanel(container);
    });
  });

  // Refresh button
  const refreshBtn = container.querySelector('#doctor-refresh-btn');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', async () => {
      cachedWorlds = null;
      currentHealthReport = null;
      await renderSavesDoctorPanel(container);
      showToast(t('common.refreshed') || 'Saves list refreshed', 'info');
    });
  }

  // Choose Custom Folder button
  const customFolderBtn = container.querySelector('#doctor-custom-folder-btn');
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
          customSavesPath = selected;
          cachedWorlds = null;
          selectedWorldDir = null;
          currentHealthReport = null;
          await renderSavesDoctorPanel(container);
          showToast('Loaded custom saves directory', 'success');
        }
      } catch (err) {
        showToast(String(err), 'error');
      }
    });
  }

  // Backup World Now Button
  const backupWorldBtn = container.querySelector('#btn-backup-world');
  if (backupWorldBtn) {
    backupWorldBtn.addEventListener('click', async () => {
      if (!selectedWorldDir || isCreatingBackup) return;
      isCreatingBackup = true;
      await renderSavesDoctorPanel(container);

      try {
        const backupZip = await createWorldBackup(selectedWorldDir);
        const parts = backupZip.split(/[/\\]/);
        const fileName = parts[parts.length - 1] || backupZip;
        showToast((t('scanner.toast_backup_success') || 'Backup created successfully: {name}').replace('{name}', fileName), 'success');
        cachedWorlds = null;
      } catch (err: any) {
        showToast(`Backup failed: ${String(err)}`, 'error');
      } finally {
        isCreatingBackup = false;
        await renderSavesDoctorPanel(container);
      }
    });
  }

  // Open Save Folder in Explorer Button
  const openFolderBtn = container.querySelector('#btn-open-world-folder');
  if (openFolderBtn) {
    openFolderBtn.addEventListener('click', async () => {
      if (!selectedWorldDir) return;
      try {
        await openWorldFolder(selectedWorldDir);
      } catch (err: any) {
        showToast(`Failed to open directory: ${String(err)}`, 'error');
      }
    });
  }

  // Export Save to ZIP Button
  const exportZipBtn = container.querySelector('#btn-export-world-zip');
  if (exportZipBtn) {
    exportZipBtn.addEventListener('click', async () => {
      if (!selectedWorldDir) return;
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const defaultName = `PalworldSave_${new Date().toISOString().slice(0, 10)}.zip`;
        const targetPath = await save({
          defaultPath: defaultName,
          filters: [{ name: 'Zip Archive', extensions: ['zip'] }],
          title: t('scanner.export_dialog_title') || 'Export World Save (.zip)',
        });

        if (targetPath && typeof targetPath === 'string') {
          await exportWorldZip(selectedWorldDir, targetPath);
          showToast(`Exported save world to ${targetPath}`, 'success');
        }
      } catch (err: any) {
        showToast(`Export failed: ${String(err)}`, 'error');
      }
    });
  }

  // Prune Backups Button
  const pruneBackupsBtn = container.querySelector('#btn-prune-backups');
  if (pruneBackupsBtn) {
    pruneBackupsBtn.addEventListener('click', async () => {
      if (!selectedWorldDir || isPruningBackups) return;
      const confirmed = await showConfirm(
        t('scanner.confirm_prune_title') || 'Clean Old Auto-Backups',
        t('scanner.confirm_prune_msg') || 'Palworld stores backup snapshots every 10 minutes without deleting them. This will delete old snapshots, keeping only the 5 most recent. Proceed?'
      );
      if (!confirmed) return;

      isPruningBackups = true;
      await renderSavesDoctorPanel(container);

      try {
        const deleted = await pruneWorldBackups(selectedWorldDir, 5);
        showToast((t('scanner.toast_prune_success') || 'Cleaned {count} old snapshot backups.').replace('{count}', String(deleted)), 'success');
        cachedWorlds = null;
        if (currentHealthReport) {
          currentHealthReport = await deepScanSaveHealth(selectedWorldDir);
        }
      } catch (err: any) {
        showToast(`Prune failed: ${String(err)}`, 'error');
      } finally {
        isPruningBackups = false;
        await renderSavesDoctorPanel(container);
      }
    });
  }

  // Save Custom World Meta (Nickname, Profile, Notes)
  const saveMetaBtn = container.querySelector('#btn-save-meta');
  if (saveMetaBtn) {
    saveMetaBtn.addEventListener('click', async () => {
      if (!selectedWorldDir || isSavingMeta) return;
      const nicknameInput = container.querySelector('#meta-nickname-input') as HTMLInputElement;
      const profileSelect = container.querySelector('#meta-profile-select') as HTMLSelectElement;
      const notesInput = container.querySelector('#meta-notes-input') as HTMLTextAreaElement;

      const profileId = profileSelect?.value || null;
      const profileName = profileId ? availableProfiles.find(p => p.id === profileId)?.name || null : null;

      const meta: WorldCustomMeta = {
        nickname: nicknameInput?.value?.trim() || null,
        notes: notesInput?.value?.trim() || null,
        boundProfileId: profileId,
        boundProfileName: profileName,
        tags: [],
        preLaunchBackupEnabled: false,
      };

      isSavingMeta = true;
      try {
        await saveWorldCustomMeta(selectedWorldDir, meta);
        showToast('World metadata saved successfully!', 'success');
        cachedWorlds = null;
        await renderSavesDoctorPanel(container);
      } catch (err: any) {
        showToast(`Failed to save world metadata: ${String(err)}`, 'error');
      } finally {
        isSavingMeta = false;
      }
    });
  }

  // Deep Scan Button
  const deepScanBtn = container.querySelector('#btn-run-deep-scan');
  if (deepScanBtn) {
    deepScanBtn.addEventListener('click', async () => {
      if (!selectedWorldDir || isDeepScanning) return;
      isDeepScanning = true;
      await renderSavesDoctorPanel(container);

      try {
        currentHealthReport = await deepScanSaveHealth(selectedWorldDir);
      } catch (err: any) {
        showToast(`Deep scan failed: ${String(err)}`, 'error');
      } finally {
        isDeepScanning = false;
        await renderSavesDoctorPanel(container);
      }
    });
  }

  // Repair Button
  const repairBtn = container.querySelector('#btn-repair-save');
  if (repairBtn) {
    repairBtn.addEventListener('click', async () => {
      if (!selectedWorldDir || isRepairing) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_repair_title') || 'Rescue & Clean Savegame',
        t('scanner.confirm_repair_msg') || 'This will create an automatic ZIP backup of your world and sanitize orphaned mod references. Proceed?'
      );
      if (!confirmed) return;

      isRepairing = true;
      await renderSavesDoctorPanel(container);

      try {
        const result = await repairSaveHealth(selectedWorldDir);
        showToast(result.message, 'success');
        currentHealthReport = await deepScanSaveHealth(selectedWorldDir);
        cachedWorlds = null;
      } catch (err: any) {
        showToast(`Repair failed: ${String(err)}`, 'error');
      } finally {
        isRepairing = false;
        await renderSavesDoctorPanel(container);
      }
    });
  }

  // Compare Snapshot Buttons
  container.querySelectorAll('.btn-compare-snap').forEach(btn => {
    btn.addEventListener('click', () => {
      const slot = (btn as HTMLElement).dataset.slot;
      const time = (btn as HTMLElement).dataset.time || slot;
      const sizeStr = (btn as HTMLElement).dataset.size;
      const hasLocal = (btn as HTMLElement).dataset.local === 'true';
      const dayStr = (btn as HTMLElement).dataset.day;
      const levelStr = (btn as HTMLElement).dataset.level;
      const hostStr = (btn as HTMLElement).dataset.host;
      const world = cachedWorlds?.find(w => w.worldDir === selectedWorldDir) || cachedWorlds?.[0] || null;
      if (!slot || !world) return;

      const snapSize = parseInt(sizeStr || '0', 10);
      const inGameDay = dayStr ? parseInt(dayStr, 10) : undefined;
      const playerLevel = levelStr ? parseInt(levelStr, 10) : undefined;
      
      showSnapshotComparisonModal(world, {
        slotName: slot,
        timestamp: time || slot,
        levelSizeBytes: snapSize,
        localDataExists: hasLocal,
        inGameDay,
        playerLevel,
        hostPlayerName: hostStr || undefined,
      }, container);
    });
  });

  // Restore Snapshot Buttons
  container.querySelectorAll('.btn-restore-snap').forEach(btn => {
    btn.addEventListener('click', async () => {
      if (!selectedWorldDir || isRestoringBackup) return;
      const slot = (btn as HTMLElement).dataset.slot;
      const time = (btn as HTMLElement).dataset.time || slot;
      if (!slot) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_restore_backup_title') || 'Restore World Snapshot',
        (t('scanner.confirm_restore_backup_msg') || 'This will create a safety ZIP of your current world and restore snapshot \'{timestamp}\'. Proceed?').replace('{timestamp}', time || slot)
      );
      if (!confirmed) return;

      isRestoringBackup = true;
      await renderSavesDoctorPanel(container);

      try {
        const result = await restoreSaveBackup(selectedWorldDir, slot);
        showToast((t('scanner.toast_restore_success') || 'World successfully restored from snapshot {timestamp}.').replace('{timestamp}', time || slot), 'success');
        currentHealthReport = await deepScanSaveHealth(selectedWorldDir);
        cachedWorlds = null;
      } catch (err: any) {
        showToast(`Restore failed: ${String(err)}`, 'error');
      } finally {
        isRestoringBackup = false;
        await renderSavesDoctorPanel(container);
      }
    });
  });
}

function showSnapshotComparisonModal(
  world: SaveWorldSummary,
  snap: SaveBackupSnapshot,
  parentContainer: HTMLElement
): void {
  const existingModal = document.getElementById('save-compare-modal');
  if (existingModal) existingModal.remove();

  const diffBytes = (world.levelSizeBytes || 0) - snap.levelSizeBytes;
  const diffPct = snap.levelSizeBytes > 0 ? (((Math.abs(diffBytes)) * 100) / snap.levelSizeBytes).toFixed(1) : '0';

  let diffDescription = '';
  if (Math.abs(diffBytes) < 512) {
    diffDescription = `<span style="color: #38bdf8;">🟢 ${escapeHtml(t('scanner.snap_diff_same') || 'The active world and this snapshot are virtually identical in size.')}</span>`;
  } else if (diffBytes > 0) {
    diffDescription = `<span style="color: #4af626;">📈 ${escapeHtml(t('scanner.snap_diff_growth_desc') || 'The current active save has grown by')} <strong>+${formatBytes(diffBytes)} (+${diffPct}%)</strong> ${escapeHtml(t('scanner.snap_diff_since_snap') || 'since this snapshot was taken.')}</span>`;
  } else {
    diffDescription = `<span style="color: #ff5f56;">📉 ${escapeHtml(t('scanner.snap_diff_loss_desc') || 'The current active save is smaller by')} <strong>-${formatBytes(Math.abs(diffBytes))} (-${diffPct}%)</strong>. ${escapeHtml(t('scanner.snap_diff_loss_warn') || 'If you experience crashes, this snapshot has more complete world data.')}</span>`;
  }

  const modalHtml = `
    <div id="save-compare-modal" class="modal-overlay" style="position: fixed; inset: 0; background: rgba(0,0,0,0.75); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center; z-index: 9999; animation: fadeIn 0.15s ease;">
      <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 10px; width: 700px; max-width: 95vw; box-shadow: 0 12px 36px rgba(0,0,0,0.6); display: flex; flex-direction: column; overflow: hidden;">
        
        <!-- Header -->
        <div style="background: var(--bg-secondary); padding: 14px 18px; border-bottom: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center;">
          <div style="display: flex; align-items: center; gap: 10px;">
            <span style="font-size: 20px;">⚖️</span>
            <div>
              <div style="font-size: 14px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.snap_diff_modal_title') || 'Save vs Snapshot Comparison & Diff')}</div>
              <div style="font-size: 11px; color: var(--text-muted);">${escapeHtml(world.worldName || world.worldId)}</div>
            </div>
          </div>
          <button id="btn-close-compare-modal" style="background: none; border: none; font-size: 18px; color: var(--text-muted); cursor: pointer; padding: 4px;">✕</button>
        </div>

        <!-- Body -->
        <div style="padding: 18px; display: flex; flex-direction: column; gap: 14px;">
          
          <!-- Summary Banner -->
          <div style="background: rgba(0,0,0,0.25); border: 1px solid var(--border); border-radius: 8px; padding: 12px 14px; font-size: 12px; line-height: 1.5; display: flex; flex-direction: column; gap: 4px;">
            <div>${diffDescription}</div>
            <div id="snap-internal-deltas-container" style="display: flex; gap: 14px; font-size: 11px; color: var(--text-secondary); margin-top: 4px; padding-top: 4px; border-top: 1px dashed rgba(255,255,255,0.08); flex-wrap: wrap;">
              <span id="snap-deltas-status" style="color: var(--text-muted);">⏳ Reading snapshot details...</span>
            </div>
          </div>

          <!-- Side-by-Side Comparison Grid -->
          <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 12px;">
            
            <!-- Left: Active Save -->
            <div style="background: rgba(74, 246, 38, 0.04); border: 1px solid rgba(74, 246, 38, 0.2); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px;">
              <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(74, 246, 38, 0.15); padding-bottom: 8px;">
                <div style="display: flex; align-items: center; gap: 6px; font-weight: 700; color: #4af626; font-size: 12.5px;">
                  <span>🟢</span>
                  <span>${escapeHtml(t('scanner.snap_diff_active_save') || 'Current Active Save')}</span>
                </div>
                <span class="badge" style="font-size: 9.5px; background: rgba(74, 246, 38, 0.15); color: #4af626;">Live Data</span>
              </div>

              <div style="display: flex; flex-direction: column; gap: 6px; font-size: 11.5px;">
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Timestamp:</span>
                  <strong style="color: var(--text-primary); font-family: monospace;">${escapeHtml(world.saveDate || 'Active')}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Level.sav Size:</span>
                  <strong style="color: var(--text-primary); font-family: monospace;">${formatBytes(world.levelSizeBytes)}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">${escapeHtml(t('scanner.snap_diff_ram_size') || 'RAM Memory:')}</span>
                  <strong style="color: var(--text-primary); font-family: monospace;">${currentHealthReport?.uncompressedSize ? formatBytes(currentHealthReport.uncompressedSize) : 'N/A'}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">In-Game Day:</span>
                  <strong style="color: #ffd166;">${world.inGameDay ? `Day ${world.inGameDay}` : 'N/A'}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Host Player:</span>
                  <strong style="color: #4af626;">👤 ${escapeHtml(world.hostPlayerName || 'Host')} ${world.playerLevel ? `(Lv. ${world.playerLevel})` : ''}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">${escapeHtml(t('scanner.snap_diff_mod_data') || 'Mod Data:')}</span>
                  <strong style="color: ${currentHealthReport?.orphanedModRefs && currentHealthReport.orphanedModRefs.length > 0 ? '#ff5f56' : '#4af626'};">${currentHealthReport?.orphanedModRefs && currentHealthReport.orphanedModRefs.length > 0 ? `⚠️ ${currentHealthReport.orphanedModRefs.length} Mod Issues` : '🟢 Clean (0 Issues)'}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Health Status:</span>
                  <strong style="color: ${world.healthStatus === 'healthy' ? '#4af626' : '#ffaa00'}; text-transform: capitalize;">${escapeHtml(world.healthStatus)}</strong>
                </div>
              </div>
            </div>

            <!-- Right: Snapshot Target -->
            <div style="background: rgba(56, 189, 248, 0.04); border: 1px solid rgba(56, 189, 248, 0.2); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 10px;">
              <div style="display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(56, 189, 248, 0.15); padding-bottom: 8px;">
                <div style="display: flex; align-items: center; gap: 6px; font-weight: 700; color: #38bdf8; font-size: 12.5px;">
                  <span>📦</span>
                  <span>${escapeHtml(t('scanner.snap_diff_target_snap') || 'Selected Snapshot')}</span>
                </div>
                <span class="badge" style="font-size: 9.5px; background: rgba(56, 189, 248, 0.15); color: #38bdf8;">Auto-Backup</span>
              </div>

              <div style="display: flex; flex-direction: column; gap: 6px; font-size: 11.5px;">
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Snapshot Time:</span>
                  <strong style="color: var(--text-primary); font-family: monospace;">${escapeHtml(snap.timestamp)}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Level.sav Size:</span>
                  <strong style="color: var(--text-primary); font-family: monospace;">${formatBytes(snap.levelSizeBytes)}</strong>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">${escapeHtml(t('scanner.snap_diff_ram_size') || 'RAM Memory:')}</span>
                  <span id="snap-inspect-ram"><strong style="color: var(--text-primary); font-family: monospace;">⏳ ...</strong></span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">In-Game Day:</span>
                  <span id="snap-inspect-day"><strong style="color: #ffd166;">${snap.inGameDay ? `Day ${snap.inGameDay}` : (world.inGameDay ? `Day ${world.inGameDay}` : 'N/A')}</strong></span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">Host Player:</span>
                  <span id="snap-inspect-host"><strong style="color: #38bdf8;">👤 ${escapeHtml(snap.hostPlayerName || world.hostPlayerName || 'Host')} ${snap.playerLevel ? `(Lv. ${snap.playerLevel})` : (world.playerLevel ? `(Lv. ${world.playerLevel})` : '')}</strong></span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">${escapeHtml(t('scanner.snap_diff_mod_data') || 'Mod Data:')}</span>
                  <span id="snap-inspect-mod-cleanliness"><strong style="color: var(--text-muted);">⏳ Analyzing...</strong></span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: var(--text-muted);">LocalData.sav:</span>
                  <strong style="color: ${snap.localDataExists ? '#4af626' : '#ffaa00'};">${snap.localDataExists ? 'Available' : 'Level.sav only'}</strong>
                </div>
              </div>
            </div>

          </div>
        </div>

        <!-- Footer -->
        <div style="background: var(--bg-secondary); padding: 12px 18px; border-top: 1px solid var(--border); display: flex; justify-content: space-between; gap: 10px; align-items: center;">
          <div style="font-size: 11px; color: var(--text-muted);">
            Slot: <span style="font-family: monospace; color: var(--text-secondary);">${escapeHtml(snap.slotName)}</span>
          </div>
          <div style="display: flex; gap: 10px;">
            <button id="btn-modal-close-action" class="btn-secondary" style="padding: 6px 14px; font-size: 12px;">${escapeHtml(t('common.close') || 'Close')}</button>
            <button id="btn-modal-restore-action" class="btn-primary" style="padding: 6px 16px; font-size: 12px; display: flex; align-items: center; gap: 6px;">
              <span>🔄</span>
              <span>${escapeHtml(t('scanner.btn_restore_snapshot') || 'Restore This Snapshot')}</span>
            </button>
          </div>
        </div>

      </div>
    </div>
  `;

  document.body.insertAdjacentHTML('beforeend', modalHtml);

  const modalEl = document.getElementById('save-compare-modal');
  const closeBtn = document.getElementById('btn-close-compare-modal');
  const closeActionBtn = document.getElementById('btn-modal-close-action');
  const restoreActionBtn = document.getElementById('btn-modal-restore-action');
  const deltasContainer = document.getElementById('snap-internal-deltas-container');
  const dayEl = document.getElementById('snap-inspect-day');
  const hostEl = document.getElementById('snap-inspect-host');
  const ramEl = document.getElementById('snap-inspect-ram');
  const modCleanlinessEl = document.getElementById('snap-inspect-mod-cleanliness');

  // Asynchronously inspect this single snapshot on demand
  inspectSnapshotDetails(world.worldDir, snap.slotName)
    .then((details) => {
      if (dayEl && details.inGameDay) {
        dayEl.innerHTML = `<strong style="color: #ffd166;">Day ${details.inGameDay}</strong>`;
      }
      if (hostEl) {
        const hostName = details.hostPlayerName || world.hostPlayerName || 'Host';
        const level = details.playerLevel || world.playerLevel;
        hostEl.innerHTML = `<strong style="color: #38bdf8;">👤 ${escapeHtml(hostName)} ${level ? `(Lv. ${level})` : ''}</strong>`;
      }
      if (ramEl && details.uncompressedSizeBytes) {
        ramEl.innerHTML = `<strong style="color: var(--text-primary); font-family: monospace;">${formatBytes(details.uncompressedSizeBytes)}</strong>`;
      }
      if (modCleanlinessEl) {
        if (details.isCleanVanilla) {
          modCleanlinessEl.innerHTML = `<strong style="color: #4af626;">🟢 ${escapeHtml(t('scanner.snap_diff_clean_vanilla') || 'Pure Vanilla (0 Mod Refs)')}</strong>`;
        } else if (details.modRefsCount) {
          modCleanlinessEl.innerHTML = `<strong style="color: #ffd166;">📦 ${(t('scanner.snap_diff_mod_refs') || '{count} Mod References').replace('{count}', String(details.modRefsCount))}</strong>`;
        } else {
          modCleanlinessEl.innerHTML = `<strong style="color: #4af626;">🟢 ${escapeHtml(t('scanner.snap_diff_clean_vanilla') || 'Clean (0 Mod Refs)')}</strong>`;
        }
      }

      if (deltasContainer) {
        const internalDeltas: string[] = [];
        const sDay = details.inGameDay || snap.inGameDay;
        const sLevel = details.playerLevel || snap.playerLevel;

        if (world.inGameDay && sDay) {
          if (world.inGameDay > sDay) {
            internalDeltas.push(`📅 <strong>+${world.inGameDay - sDay} Days progressed</strong> (Day ${sDay} ➔ Day ${world.inGameDay})`);
          } else if (world.inGameDay === sDay) {
            internalDeltas.push(`📅 Same in-game day (Day ${world.inGameDay})`);
          } else {
            internalDeltas.push(`📅 Snapshot was at Day ${sDay} (Current: Day ${world.inGameDay})`);
          }
        }
        if (world.playerLevel && sLevel) {
          if (world.playerLevel > sLevel) {
            internalDeltas.push(`⭐ <strong>+${world.playerLevel - sLevel} Levels gained</strong> (Lv. ${sLevel} ➔ Lv. ${world.playerLevel})`);
          } else if (world.playerLevel === sLevel) {
            internalDeltas.push(`⭐ Same character level (Lv. ${world.playerLevel})`);
          }
        }
        if (details.isCleanVanilla) {
          internalDeltas.push(`🛡️ <span style="color: #4af626;"><strong>Vanilla Snapshot:</strong> 0 custom mod classes found in snapshot.</span>`);
        } else if (details.modRefsCount) {
          internalDeltas.push(`📦 <span>Snapshot contains ${details.modRefsCount} custom mod classes.</span>`);
        }

        if (internalDeltas.length > 0) {
          deltasContainer.innerHTML = internalDeltas.map(d => `<span>${d}</span>`).join('');
        } else {
          deltasContainer.innerHTML = `<span style="color: #38bdf8;">🛡️ Intact Palworld auto-backup snapshot</span>`;
        }
      }
    })
    .catch((err) => {
      console.warn('Failed to inspect snapshot on demand:', err);
      if (deltasContainer) {
        deltasContainer.innerHTML = `<span style="color: var(--text-muted);">🛡️ Palworld auto-backup snapshot</span>`;
      }
    });

  const closeModal = () => modalEl?.remove();

  closeBtn?.addEventListener('click', closeModal);
  closeActionBtn?.addEventListener('click', closeModal);
  modalEl?.addEventListener('click', (e) => {
    if (e.target === modalEl) closeModal();
  });

  restoreActionBtn?.addEventListener('click', async () => {
    closeModal();
    if (!selectedWorldDir || isRestoringBackup) return;

    const confirmed = await showConfirm(
      t('scanner.confirm_restore_backup_title') || 'Restore World Snapshot',
      (t('scanner.confirm_restore_backup_msg') || 'This will create a safety ZIP of your current world and restore snapshot \'{timestamp}\'. Proceed?').replace('{timestamp}', snap.timestamp)
    );
    if (!confirmed) return;

    isRestoringBackup = true;
    await renderSavesDoctorPanel(parentContainer);

    try {
      await restoreSaveBackup(selectedWorldDir, snap.slotName);
      showToast((t('scanner.toast_restore_success') || 'World successfully restored from snapshot {timestamp}.').replace('{timestamp}', snap.timestamp), 'success');
      currentHealthReport = await deepScanSaveHealth(selectedWorldDir);
      cachedWorlds = null;
    } catch (err: any) {
      showToast(`Restore failed: ${String(err)}`, 'error');
    } finally {
      isRestoringBackup = false;
      await renderSavesDoctorPanel(parentContainer);
    }
  });
}
