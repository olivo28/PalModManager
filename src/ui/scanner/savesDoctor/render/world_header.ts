import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { formatBytes } from '../helpers';
import { getHealthBadge } from './badges';
import type { SaveWorldSummary, WorldCustomMeta } from '../../../../api';

export function renderWorldHeaderAndActionsHtml(
  selectedWorld: SaveWorldSummary,
  customMeta: WorldCustomMeta | null,
  curCreating: boolean,
  curPruning: boolean,
  curDeep: boolean
): string {
  return `
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
  `;
}

export function renderWorldQuickStatsHtml(selectedWorld: SaveWorldSummary): string {
  return `
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
  `;
}
