import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { formatBytes } from '../helpers';
import type { SaveWorldSummary, SaveHealthReport } from '../../../../api';

export function renderStorageBreakdownHtml(curHealth: SaveHealthReport | null): string {
  if (!curHealth?.storageBreakdown) return '';
  return `
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
  `;
}

export function renderPlayerRosterHtml(curHealth: SaveHealthReport | null): string {
  if (!curHealth?.playerRoster || curHealth.playerRoster.length === 0) return '';
  return `
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
  `;
}

export function renderDeepScanResultsHtml(
  curHealth: SaveHealthReport | null,
  selectedWorld: SaveWorldSummary,
  curRepairing: boolean,
  curRestoring: boolean
): string {
  if (!curHealth || curHealth.worldId !== selectedWorld.worldId) {
    return `
      <div style="background: rgba(255, 255, 255, 0.02); border: 1px dashed var(--border); border-radius: 8px; padding: 36px 20px; text-align: center; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; flex: 1;">
        <span style="font-size: 32px;">🔬</span>
        <span style="font-size: 13.5px; font-weight: 600; color: var(--text-primary);">${escapeHtml(t('scanner.deep_scan_prompt_title') || 'Deep Health Scan Ready')}</span>
        <span style="font-size: 11px; color: var(--text-muted); max-width: 480px; line-height: 1.5;">
          ${escapeHtml(t('scanner.deep_scan_prompt_desc') || 'Click "Deep Health Scan" above to decompress Level.sav and check for missing classes from removed mods that cause loading crashes.')}
        </span>
      </div>
    `;
  }

  return `
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

      ${(curHealth.baseDeteriorationRate !== undefined && curHealth.baseDeteriorationRate !== null && curHealth.baseDeteriorationRate > 0) ? `
        <div style="background: rgba(56, 189, 248, 0.08); border: 1px solid rgba(56, 189, 248, 0.25); border-radius: 8px; padding: 10px 14px; display: flex; align-items: flex-start; gap: 10px;">
          <span style="font-size: 16px; flex-shrink: 0; margin-top: 1px;">🏰</span>
          <div style="display: flex; flex-direction: column; gap: 3px; font-size: 11px; line-height: 1.45;">
            <span style="font-weight: 700; color: #38bdf8;">${escapeHtml(t('scanner.save_base_deterioration_tip_title') || 'Base Camp Deterioration Active')} (${curHealth.baseDeteriorationRate}x)</span>
            <span style="color: var(--text-secondary);">${escapeHtml(t('scanner.save_base_deterioration_tip', { rate: `${curHealth.baseDeteriorationRate}x` }) || `Outer structures built with expanded base radius mods will slowly decay because structure deterioration is set to ${curHealth.baseDeteriorationRate}x. To preserve outer buildings without the mod, set deterioration rate to 0 in World Settings.`)}</span>
          </div>
        </div>
      ` : ''}

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
  `;
}
