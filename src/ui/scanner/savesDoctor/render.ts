import type {
  WorldOptionSettings,
  WorldCustomMeta,
} from '../../../api';
import { subTabHeader } from '../mod';
import { escapeHtml } from '../rendering';
import { t } from '../../../utils/i18n';
import { formatBytes, formatDeathPenalty } from './helpers';
import { renderHeroLandingHtml, attachHeroLandingListeners } from './hero';
import { attachSavesDoctorListeners } from './listeners';
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
} from './state';

export function getHealthBadge(status: string, hasExternalEdits?: boolean): string {
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
      const { setupEventListeners } = await import('../mod');
      setupEventListeners();
      return;
    }

    // Hero Landing Screen (Zero latency tab switch)
    container.innerHTML = renderHeroLandingHtml();
    attachHeroLandingListeners(container, renderSavesDoctorPanel);
    const { setupEventListeners } = await import('../mod');
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
                ${escapeHtml(t('scanner.detected_worlds') || 'Detected Worlds')} (${curCached?.length || 0})
              </span>
            </div>

            <div class="scanner-master-items" style="padding: 10px; display: flex; flex-direction: column; gap: 8px; flex: 1; min-height: 0; overflow-y: auto;">
              ${curLoading ? `
                <div style="padding: 32px 16px; text-align: center; color: var(--text-muted); font-size: 12px;">
                  ⏳ ${escapeHtml(t('scanner.loading') || 'Searching for saves...')}
                </div>
              ` : (curCached && curCached.length > 0) ? curCached.map(w => {
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

                    <div style="display: flex; align-items: center; justify-content: space-between; font-size: 10px; color: var(--text-muted); gap: 6px; white-space: nowrap;">
                      <span style="overflow: hidden; text-overflow: ellipsis;">📁 ${formatBytes(w.levelSizeBytes)} (${w.playerCount} ${escapeHtml(w.playerCount === 1 ? (t('scanner.player_single') || 'Player') : (t('scanner.player_plural') || 'Players'))})</span>
                      <div style="display: flex; align-items: center; gap: 4px; flex-shrink: 0;">
                        <span title="${w.backupCount} Game Snapshots">💾 ${w.backupCount}</span>
                        ${w.pmmBackupCount ? `
                          <span style="opacity: 0.5;">•</span>
                          <span title="${w.pmmBackupCount} PMM Backups" style="color: #c084fc; font-weight: 600;">📦 ${w.pmmBackupCount}</span>
                        ` : ''}
                      </div>
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
                    <button id="btn-backup-world" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${curCreating ? 'disabled' : ''} title="${escapeHtml(t('scanner.btn_backup_now') || 'Create manual timestamped ZIP backup')}">
                      <span>🛡️</span>
                      <span>${curCreating ? 'Saving...' : escapeHtml(t('scanner.btn_backup_now') || 'Backup World Now')}</span>
                    </button>

                    <button id="btn-pmm-vault" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" title="${escapeHtml(t('scanner.btn_pmm_vault_title') || 'View, restore, or delete PMM manual ZIP backups')}">
                      <span>📦</span>
                      <span>${escapeHtml(t('scanner.btn_pmm_vault') || 'PMM Backups')} (${selectedWorld.pmmBackupCount || 0})</span>
                    </button>

                    <button id="btn-open-world-folder" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" title="${escapeHtml(t('scanner.btn_open_folder') || 'Open save folder in Explorer')}">
                      <span>📂</span>
                      <span>${escapeHtml(t('scanner.btn_open_folder') || 'Open in Explorer')}</span>
                    </button>

                    <button id="btn-export-world-zip" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" title="${escapeHtml(t('scanner.btn_export_zip') || 'Export save to ZIP')}">
                      <span>📤</span>
                      <span>${escapeHtml(t('scanner.btn_export_zip') || 'Export Save')}</span>
                    </button>

                    <button id="btn-prune-backups" class="btn-secondary" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${curPruning ? 'disabled' : ''} title="${escapeHtml(t('scanner.btn_prune_backups') || 'Prune old game auto-backups to free disk space')}">
                      <span>🧹</span>
                      <span>${curPruning ? 'Pruning...' : escapeHtml(t('scanner.btn_prune_backups') || 'Prune Backups')}</span>
                    </button>

                    <button id="btn-run-deep-scan" class="btn-primary" style="padding: 6px 14px; font-size: 11.5px; display: flex; align-items: center; gap: 6px;" ${curDeep ? 'disabled' : ''}>
                      <span>${curDeep ? '⏳' : '🔬'}</span>
                      <span>${curDeep ? escapeHtml(t('scanner.scanning_deep') || 'Scanning GVAS...') : escapeHtml(t('scanner.btn_deep_scan') || 'Deep Health Scan')}</span>
                    </button>
                  </div>
                </div>
              </div>

              <!-- World Quick Stats Grid -->
              <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px; flex-shrink: 0;">
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
                <div class="premium-stat-card stat-card-pmm-vault" style="cursor: pointer; transition: transform 0.15s, border-color 0.15s;" title="${escapeHtml(t('scanner.btn_pmm_vault_title') || 'View, restore, or delete PMM manual ZIP backups')}">
                  <div style="font-size: 10px; color: var(--text-muted); text-transform: uppercase; display: flex; justify-content: space-between; align-items: center;">
                    <span>${escapeHtml(t('scanner.stat_pmm_backups') || 'PMM Backups')}</span>
                    <span style="font-size: 11px; opacity: 0.7;">↗</span>
                  </div>
                  <div class="premium-stat-value" style="font-size: 15px; color: #a855f7;">
                    📦 ${selectedWorld.pmmBackupCount || 0} ${escapeHtml(t('scanner.vault_count_label') || 'Backups')}
                  </div>
                </div>
              </div>

              <!-- World Custom Nickname, Profile Binding & Notes Card -->
              <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-size: 15px;">🏷️</span>
                    <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.btn_save_meta') || 'World Profile Binding & Notes')}</span>
                  </div>
                  <button id="btn-save-meta" class="btn-primary" style="padding: 4px 12px; font-size: 11px; display: flex; align-items: center; gap: 5px;" ${curSaving ? 'disabled' : ''}>
                    <span>💾</span>
                    <span>${curSaving ? 'Saving...' : escapeHtml(t('common.save') || 'Save')}</span>
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
                      ${curProfiles.map(p => `
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
              ${curHealth?.storageBreakdown ? `
                <div style="background: rgba(0,0,0,0.18); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 16px;">📊</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.storage_title') || 'Storage & Compression Breakdown')}</span>
                    </div>
                    <span class="badge" style="font-size: 10px; padding: 2px 8px; background: rgba(74, 246, 38, 0.15); color: #4af626;">
                      Compressed to ${curHealth.storageBreakdown.compressionRatioPct}% of original size
                    </span>
                  </div>

                  <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 8px;">
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_level_sav') || 'Level.sav')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${formatBytes(curHealth.storageBreakdown.levelSavBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_players') || 'Players Folder')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${formatBytes(curHealth.storageBreakdown.playersDirBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_backups') || 'Auto-Backups')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: #38bdf8;">${formatBytes(curHealth.storageBreakdown.backupsDirBytes)}</span>
                    </div>
                    <div style="background: rgba(0,0,0,0.15); padding: 8px 10px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; gap: 2px;">
                      <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.storage_uncompressed') || 'Decompressed in RAM')}</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: #ffd166;">${formatBytes(curHealth.storageBreakdown.uncompressedLevelBytes)}</span>
                    </div>
                  </div>
                </div>
              ` : ''}

              <!-- Players & Co-op Roster Card (When available from Deep Scan) -->
              ${curHealth?.playerRoster && curHealth.playerRoster.length > 0 ? `
                <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 12px 16px; display: flex; flex-direction: column; gap: 10px; flex-shrink: 0;">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="font-size: 16px;">👥</span>
                      <span style="font-size: 12.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.players_title') || 'Players & Character Saves')} (${curHealth.playerRoster.length})</span>
                    </div>
                  </div>

                  <div style="display: flex; flex-direction: column; gap: 6px; max-height: 220px; overflow-y: auto;">
                    ${curHealth.playerRoster.map(p => `
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
                ${curHealth && curHealth.worldId === selectedWorld.worldId ? `
                  <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 12px; flex-shrink: 0;">
                    <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 10px;">
                      <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-size: 16px;">${curHealth.healthStatus === 'healthy' ? '✅' : '⚠️'}</span>
                        <span style="font-size: 13px; font-weight: 700; color: var(--text-primary);">${escapeHtml(curHealth.summaryMessage)}</span>
                      </div>
                      ${curHealth.canRepair ? `
                        <button id="btn-repair-save" class="btn-primary" style="background: #2ed573; border-color: #2ed573; color: #000; font-weight: 700; padding: 6px 14px; font-size: 12px; display: flex; align-items: center; gap: 6px;" ${curRepairing ? 'disabled' : ''}>
                          <span>🛠️</span>
                          <span>${curRepairing ? escapeHtml(t('scanner.repairing') || 'Sanitizing...') : escapeHtml(t('scanner.btn_sanitize_save') || '1-Click Rescue & Clean Save')}</span>
                        </button>
                      ` : ''}
                    </div>

                    ${(curHealth.hasExternalEdits || curHealth.externalEditDetails) ? `
                      <div style="background: linear-gradient(135deg, rgba(255, 170, 0, 0.12) 0%, rgba(255, 100, 0, 0.06) 100%); border: 1px solid rgba(255, 170, 0, 0.35); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 8px;">
                        <div style="display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px;">
                          <div style="display: flex; align-items: center; gap: 8px; font-weight: 700; color: #ffd166; font-size: 12.5px;">
                            <span style="font-size: 16px;">⚠️</span>
                            <span>${escapeHtml(t('scanner.anomaly_editor_title') || 'External Modification & State Desync')}</span>
                          </div>
                          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
                            ${curHealth.externalEditDetails?.toolName ? `
                              <span style="font-size: 10px; font-weight: 700; color: #ffd166; background: rgba(255, 209, 102, 0.15); border: 1px solid rgba(255, 209, 102, 0.3); border-radius: 10px; padding: 2px 8px;">
                                🛠️ ${escapeHtml(curHealth.externalEditDetails.toolName)}
                              </span>
                            ` : ''}
                            ${curHealth.externalEditDetails?.editorBackupCount ? `
                              <span style="font-size: 10px; font-weight: 600; color: #ffaa00; background: rgba(255, 170, 0, 0.15); border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 10px; padding: 2px 8px;">
                                📦 ${curHealth.externalEditDetails.editorBackupCount} Editor Backups
                              </span>
                            ` : ''}
                            ${curHealth.externalEditDetails?.sizeReductionPct ? `
                              <span style="font-size: 10px; font-weight: 700; color: #ff5f56; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); border-radius: 10px; padding: 2px 8px;">
                                📉 -${curHealth.externalEditDetails.sizeReductionPct}% File Size Loss
                              </span>
                            ` : ''}
                          </div>
                        </div>

                        <div style="font-size: 11.5px; color: var(--text-primary); opacity: 0.9; line-height: 1.5;">
                          ${escapeHtml(curHealth.externalEditDetails?.details || t('scanner.anomaly_editor_desc') || '')}
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
                        <span class="badge" style="font-size: 10px; padding: 2px 8px; background: ${(curHealth.totalModReferences || 0) > 0 ? (curHealth.orphanedModRefs.length > 0 ? 'rgba(255, 170, 0, 0.15)' : 'rgba(74, 246, 38, 0.15)') : 'rgba(255,255,255,0.06)'}; color: ${(curHealth.totalModReferences || 0) > 0 ? (curHealth.orphanedModRefs.length > 0 ? '#ffaa00' : '#4af626') : 'var(--text-muted)'};">
                          ${(curHealth.totalModReferences || 0) > 0 ? `${curHealth.totalModReferences} Mod References Found` : (escapeHtml(t('scanner.mod_data_none_detected') || '0 Mod Classes (100% Vanilla)'))}
                        </span>
                      </div>

                      ${(curHealth.totalModReferences || 0) === 0 ? `
                        <div style="font-size: 11px; color: var(--text-muted); line-height: 1.5; background: rgba(255,255,255,0.02); border: 1px dashed var(--border); border-radius: 6px; padding: 10px 12px; display: flex; align-items: center; gap: 8px;">
                          <span style="font-size: 14px;">ℹ️</span>
                          <span>${escapeHtml(t('scanner.mod_data_clean_desc') || 'Level.sav contains only official vanilla Palworld classes. No third-party UE4SS, Lua, or Pak mod classes are embedded in this save.')}</span>
                        </div>
                      ` : `
                        <div style="font-size: 11px; color: var(--text-secondary); line-height: 1.4;">
                          ${escapeHtml(t('scanner.mod_data_detected_desc') || 'The following custom assets and mod classes were found embedded inside Level.sav:')}
                        </div>
                        ${curHealth.orphanedModRefs.length > 0 ? `
                          <div style="display: flex; flex-direction: column; gap: 4px; max-height: 140px; overflow-y: auto;">
                            ${curHealth.orphanedModRefs.map(orphan => `
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
                            ${curHealth.rawModPathsFound.slice(0, 10).map(path => `
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
                  ${curHealth.availableBackups.length > 0 ? `
                    <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; flex: 1; min-height: 220px;">
                      <div style="display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
                        <div style="display: flex; flex-direction: column; gap: 2px;">
                          <div style="display: flex; align-items: center; gap: 8px;">
                            <span style="font-size: 16px;">💾</span>
                            <span style="font-size: 13.5px; font-weight: 700; color: var(--text-primary);">${escapeHtml(t('scanner.backups_section_title') || 'Game Auto-Backup Snapshots')} (${curHealth.availableBackups.length})</span>
                          </div>
                          <span style="font-size: 10.5px; color: var(--text-muted);">${escapeHtml(t('scanner.backups_section_desc') || 'Palworld creates snapshot backups every 10 minutes. If your world crashes, restoring a snapshot is the safest recovery method.')}</span>
                        </div>
                      </div>

                      <div style="display: flex; flex-direction: column; gap: 6px; flex: 1; overflow-y: auto; max-height: calc(100vh - 460px); min-height: 180px; padding-right: 4px;">
                        ${curHealth.availableBackups.map(snap => {
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
                                <button class="btn-secondary btn-restore-snap" data-slot="${escapeHtml(snap.slotName || (snap as any).slot_name)}" data-time="${escapeHtml(snap.timestamp)}" style="padding: 4px 10px; font-size: 11px; display: flex; align-items: center; gap: 4px;" ${curRestoring ? 'disabled' : ''}>
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
  attachSavesDoctorListeners(container, renderSavesDoctorPanel);
  const { setupEventListeners } = await import('../mod');
  setupEventListeners();
}
