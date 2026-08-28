import {
  inspectSnapshotDetails,
  restoreSaveBackup,
  deepScanSaveHealth,
  type SaveWorldSummary,
  type SaveBackupSnapshot,
} from '../../../api';
import { escapeHtml } from '../rendering';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import { formatBytes } from './helpers';
import {
  doctorState,
  cachedWorlds,
  selectedWorldDir,
  currentHealthReport,
  isRestoringBackup,
  setCachedWorlds,
  setCurrentHealthReport,
  setIsRestoringBackup,
} from './state';

export function showSnapshotComparisonModal(
  world: SaveWorldSummary,
  snap: SaveBackupSnapshot,
  parentContainer: HTMLElement,
  rerenderCallback: (container: HTMLElement) => Promise<void>
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
                  <strong style="color: var(--text-primary); font-family: monospace;">${(currentHealthReport || doctorState.currentHealthReport)?.uncompressedSize ? formatBytes((currentHealthReport || doctorState.currentHealthReport)!.uncompressedSize!) : 'N/A'}</strong>
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
                  <strong style="color: ${(currentHealthReport || doctorState.currentHealthReport)?.orphanedModRefs && (currentHealthReport || doctorState.currentHealthReport)!.orphanedModRefs.length > 0 ? '#ff5f56' : '#4af626'};">${(currentHealthReport || doctorState.currentHealthReport)?.orphanedModRefs && (currentHealthReport || doctorState.currentHealthReport)!.orphanedModRefs.length > 0 ? `⚠️ ${(currentHealthReport || doctorState.currentHealthReport)!.orphanedModRefs.length} Mod Issues` : '🟢 Clean (0 Issues)'}</strong>
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
    const curWorldDir = selectedWorldDir || doctorState.selectedWorldDir;
    const curRestoring = isRestoringBackup || doctorState.isRestoringBackup;
    if (!curWorldDir || curRestoring) return;

    const confirmed = await showConfirm(
      t('scanner.confirm_restore_backup_title') || 'Restore World Snapshot',
      (t('scanner.confirm_restore_backup_msg') || 'This will create a safety ZIP of your current world and restore snapshot \'{timestamp}\'. Proceed?').replace('{timestamp}', snap.timestamp)
    );
    if (!confirmed) return;

    setIsRestoringBackup(true);
    await rerenderCallback(parentContainer);

    try {
      await restoreSaveBackup(curWorldDir, snap.slotName);
      showToast((t('scanner.toast_restore_success') || 'World successfully restored from snapshot {timestamp}.').replace('{timestamp}', snap.timestamp), 'success');
      const rep = await deepScanSaveHealth(curWorldDir);
      setCurrentHealthReport(rep);
      setCachedWorlds(null);
    } catch (err: any) {
      showToast(`Restore failed: ${String(err)}`, 'error');
    } finally {
      setIsRestoringBackup(false);
      await rerenderCallback(parentContainer);
    }
  });
}
