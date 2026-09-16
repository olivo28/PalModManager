import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { formatBytes } from '../helpers';
import { getHealthBadge } from './badges';
import type { SaveWorldSummary } from '../../../../api';

export function renderWorldsListHtml(
  curCached: SaveWorldSummary[] | null,
  curLoading: boolean,
  selectedWorld: SaveWorldSummary | null
): string {
  return `
    <div class="scanner-master-list" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); overflow: hidden;">
      <div style="padding: calc(8px * var(--ui-scale, 1)) calc(12px * var(--ui-scale, 1)); border-bottom: 1px solid var(--border); background: rgba(0,0,0,0.15); display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
        <span style="font-size: var(--text-xs, 11px); font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px;">
          ${escapeHtml(t('scanner.detected_worlds') || 'Detected Worlds')} (${curCached?.length || 0})
        </span>
      </div>

      <div class="scanner-master-items" style="padding: calc(8px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(6px * var(--ui-scale, 1)); flex: 1; min-height: 0; overflow-y: auto;">
        ${curLoading ? `
          <div style="padding: calc(24px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); text-align: center; color: var(--text-muted); font-size: var(--text-sm, 12px);">
            ⏳ ${escapeHtml(t('scanner.loading') || 'Searching for saves...')}
          </div>
        ` : (curCached && curCached.length > 0) ? curCached.map(w => {
          const displayName = w.customMeta?.nickname || w.worldName;
          const isCustom = !!w.customMeta?.nickname;
          return `
            <div class="scanner-mod-list-item world-list-card ${w.worldDir === selectedWorld?.worldDir ? 'active' : ''}" data-world-dir="${escapeHtml(w.worldDir)}" style="padding: calc(8px * var(--ui-scale, 1)) calc(10px * var(--ui-scale, 1)); cursor: pointer; border-radius: 6px; display: flex; flex-direction: column; gap: calc(4px * var(--ui-scale, 1));">
              <div style="display: flex; justify-content: space-between; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
                <span style="font-size: var(--text-base, 13px); font-weight: 700; color: ${w.worldDir === selectedWorld?.worldDir ? 'var(--accent)' : 'var(--text-primary)'}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                  🌍 ${escapeHtml(displayName)} ${isCustom ? `<span style="font-size: var(--text-2xs, 9.5px); opacity: 0.7;">(${escapeHtml(w.worldName)})</span>` : ''}
                </span>
                ${getHealthBadge(w.healthStatus, w.hasExternalEdits)}
              </div>
              
              <div style="display: flex; align-items: center; justify-content: space-between; font-size: var(--text-sm, 11px); color: var(--text-primary); opacity: 0.9;">
                <span>👤 ${escapeHtml(w.hostPlayerName || 'Host')} • <strong style="color: #4af626;">${w.playerLevel ? `Lv. ${w.playerLevel}` : 'Lv. ?'}</strong> • ${w.inGameDay ? `Day ${w.inGameDay}` : 'Day ?'}</span>
                <span style="font-size: var(--text-2xs, 10px); color: var(--text-muted);">${w.saveDate || ''}</span>
              </div>

              <div style="display: flex; align-items: center; justify-content: space-between; font-size: var(--text-2xs, 10px); color: var(--text-muted); gap: calc(6px * var(--ui-scale, 1)); white-space: nowrap;">
                <span style="overflow: hidden; text-overflow: ellipsis;">📁 ${formatBytes(w.levelSizeBytes)} (${w.playerCount} ${escapeHtml(w.playerCount === 1 ? (t('scanner.player_single') || 'Player') : (t('scanner.player_plural') || 'Players'))})</span>
                <div style="display: flex; align-items: center; gap: calc(4px * var(--ui-scale, 1)); flex-shrink: 0;">
                  <span title="${w.backupCount} Game Snapshots">💾 ${w.backupCount}</span>
                  ${w.pmmBackupCount ? `
                    <span style="opacity: 0.5;">•</span>
                    <span title="${w.pmmBackupCount} PMM Backups" style="color: #c084fc; font-weight: 600;">📦 ${w.pmmBackupCount}</span>
                  ` : ''}
                </div>
              </div>

              ${w.customMeta?.boundProfileName ? `
                <div style="font-size: var(--text-2xs, 9.5px); color: #38bdf8; background: rgba(56, 189, 248, 0.08); padding: 2px calc(6px * var(--ui-scale, 1)); border-radius: 4px; display: flex; align-items: center; gap: calc(4px * var(--ui-scale, 1)); width: fit-content;">
                  <span>🔗 Profile: ${escapeHtml(w.customMeta.boundProfileName)}</span>
                </div>
              ` : ''}
            </div>
          `;
        }).join('') : `
          <div style="padding: calc(24px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); text-align: center; color: var(--text-muted); font-size: var(--text-sm, 11.5px); display:flex; flex-direction:column; gap: calc(6px * var(--ui-scale, 1));">
            <span>🔍 No Palworld saves detected in default path.</span>
            <span style="font-size: var(--text-2xs, 10.5px);">Click "Choose Saves Folder" to locate your save directory manually.</span>
          </div>
        `}
      </div>
    </div>
  `;
}
