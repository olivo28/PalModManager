import {
  listPmmWorldBackups,
  restorePmmWorldBackup,
  deletePmmWorldBackup,
  openPmmWorldBackupsFolder,
  deepScanSaveHealth,
  listSaveWorlds,
  type SaveWorldSummary,
  type PmmWorldBackup,
} from '../../../api';
import { escapeHtml } from '../rendering';
import { showToast } from '../../toast';
import { showConfirm } from '../../confirm';
import { t } from '../../../utils/i18n';
import { formatBytes } from './helpers';
import {
  doctorState,
  setCachedWorlds,
  setCurrentHealthReport,
  setIsRestoringBackup,
} from './state';

export async function showPmmBackupsVaultModal(
  world: SaveWorldSummary,
  parentContainer: HTMLElement,
  rerenderCallback: (container: HTMLElement) => Promise<void>
): Promise<void> {
  const existing = document.getElementById('pmm-backups-vault-modal');
  if (existing) existing.remove();

  let backups: PmmWorldBackup[] = [];
  try {
    backups = await listPmmWorldBackups(world.worldName);
  } catch (err) {
    console.error('Failed to load PMM world backups:', err);
  }

  const totalBytes = backups.reduce((acc, b) => acc + b.fileSizeBytes, 0);

  const modalHtml = `
    <div id="pmm-backups-vault-modal" class="modal-overlay active" style="z-index: 9999; display: flex; align-items: center; justify-content: center; position: fixed; inset: 0; background: rgba(0,0,0,0.75); backdrop-filter: blur(4px);">
      <div class="modal" style="max-width: 680px; width: 100%; max-height: 85vh; display: flex; flex-direction: column; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; box-shadow: 0 16px 36px rgba(0,0,0,0.6); overflow: hidden;">
        
        <!-- Header -->
        <div class="modal-header" style="display: flex; justify-content: space-between; align-items: center; padding: 14px 18px; border-bottom: 1px solid var(--border); background: rgba(0,0,0,0.2);">
          <div style="display: flex; align-items: center; gap: 10px;">
            <span style="font-size: 20px;">📦</span>
            <div>
              <h3 style="margin: 0; font-size: 15px; font-weight: 700; color: var(--text-primary);">
                ${escapeHtml(t('scanner.vault_title') || 'PMM World Backups Vault')}
              </h3>
              <span style="font-size: 11px; color: var(--text-muted);">
                ${escapeHtml(world.customMeta?.nickname || world.worldName)} • ${backups.length} ${escapeHtml(t('scanner.vault_count_label') || 'Backups')} (${formatBytes(totalBytes)})
              </span>
            </div>
          </div>
          <button class="modal-close-btn" id="pmm-vault-close-x" style="background: none; border: none; font-size: 16px; color: var(--text-muted); cursor: pointer;">✕</button>
        </div>

        <!-- Body / Content -->
        <div class="modal-body" style="padding: 16px 18px; overflow-y: auto; flex: 1; display: flex; flex-direction: column; gap: 14px;">
          
          <div style="display: flex; justify-content: space-between; align-items: center; background: rgba(0,0,0,0.15); padding: 10px 14px; border-radius: 6px; border: 1px solid var(--border);">
            <div style="font-size: 11.5px; color: var(--text-secondary);">
              <span>💡 ${escapeHtml(t('scanner.vault_desc') || 'Manual and pre-repair ZIP archives managed by PalModManager.')}</span>
            </div>
            <button id="btn-open-vault-folder" class="btn-secondary btn-sm" style="display: flex; align-items: center; gap: 6px; font-size: 11px; padding: 4px 10px;">
              <span>📁</span> <span>${escapeHtml(t('scanner.btn_open_folder') || 'Open in Explorer')}</span>
            </button>
          </div>

          <div id="pmm-vault-items-list" style="display: flex; flex-direction: column; gap: 8px;">
            ${backups.length === 0 ? `
              <div style="text-align: center; padding: 36px 16px; color: var(--text-muted); font-size: 12px; display: flex; flex-direction: column; gap: 8px;">
                <span style="font-size: 28px;">📭</span>
                <span>${escapeHtml(t('scanner.vault_empty') || 'No PMM backups found for this world.')}</span>
                <span style="font-size: 11px; opacity: 0.8;">${escapeHtml(t('scanner.vault_empty_hint') || 'Click "Backup World Now" in the Save Doctor panel to create your first safety backup.')}</span>
              </div>
            ` : backups.map(b => `
              <div class="pmm-backup-item-card" data-filepath="${escapeHtml(b.filePath)}" style="display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; background: var(--bg-card); border: 1px solid var(--border); border-radius: 6px; gap: 12px;">
                <div style="display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1;">
                  <span style="font-size: 18px;">🗜️</span>
                  <div style="min-width: 0; display: flex; flex-direction: column; gap: 2px;">
                    <span style="font-size: 12.5px; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" title="${escapeHtml(b.fileName)}">
                      ${escapeHtml(b.fileName)}
                    </span>
                    <div style="display: flex; align-items: center; gap: 8px; font-size: 10.5px; color: var(--text-muted);">
                      <span>🕒 ${escapeHtml(b.createdAt)}</span>
                      <span>•</span>
                      <span style="color: #38bdf8; font-weight: 600;">📁 ${formatBytes(b.fileSizeBytes)}</span>
                    </div>
                  </div>
                </div>

                <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                  <button class="btn-primary btn-sm btn-vault-restore" data-filepath="${escapeHtml(b.filePath)}" data-filename="${escapeHtml(b.fileName)}" style="padding: 4px 10px; font-size: 11px; display: flex; align-items: center; gap: 4px;">
                    <span>🔄</span> <span>${escapeHtml(t('scanner.btn_restore') || 'Restore')}</span>
                  </button>
                  <button class="btn-danger btn-sm btn-vault-delete" data-filepath="${escapeHtml(b.filePath)}" data-filename="${escapeHtml(b.fileName)}" style="padding: 4px 8px; font-size: 11px; background: rgba(255, 95, 86, 0.15); border: 1px solid rgba(255, 95, 86, 0.3); color: #ff5f56; border-radius: var(--radius); cursor: pointer;" title="${escapeHtml(t('common.delete') || 'Delete')}">
                    <span>🗑️</span>
                  </button>
                </div>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Footer -->
        <div class="modal-footer" style="padding: 12px 18px; border-top: 1px solid var(--border); background: rgba(0,0,0,0.15); display: flex; justify-content: flex-end;">
          <button id="pmm-vault-close-btn" class="btn-secondary" style="padding: 6px 16px; font-size: 12px;">
            ${escapeHtml(t('common.close') || 'Close')}
          </button>
        </div>
      </div>
    </div>
  `;

  document.body.insertAdjacentHTML('beforeend', modalHtml);
  const modal = document.getElementById('pmm-backups-vault-modal');
  if (!modal) return;

  const closeModal = () => modal.remove();

  modal.querySelector('#pmm-vault-close-x')?.addEventListener('click', closeModal);
  modal.querySelector('#pmm-vault-close-btn')?.addEventListener('click', closeModal);

  // Open Explorer
  modal.querySelector('#btn-open-vault-folder')?.addEventListener('click', async () => {
    try {
      await openPmmWorldBackupsFolder();
    } catch (err) {
      showToast(String(err), 'error');
    }
  });

  // Restore Handlers
  modal.querySelectorAll('.btn-vault-restore').forEach(btn => {
    btn.addEventListener('click', async () => {
      const filePath = (btn as HTMLElement).dataset.filepath;
      const fileName = (btn as HTMLElement).dataset.filename || 'backup';
      if (!filePath) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_restore_pmm_title') || 'Restore PMM World Backup',
        (t('scanner.confirm_restore_pmm_msg') || 'This will create a safety ZIP of your current world and restore \'{filename}\'. Proceed?').replace('{filename}', fileName)
      );
      if (!confirmed) return;

      closeModal();
      setIsRestoringBackup(true);
      await rerenderCallback(parentContainer);

      try {
        await restorePmmWorldBackup(world.worldDir, filePath);
        showToast((t('scanner.toast_pmm_restore_success') || 'World successfully restored from {filename}.').replace('{filename}', fileName), 'success');
        const rep = await deepScanSaveHealth(world.worldDir);
        setCurrentHealthReport(rep);
        try {
          const curCustomPath = doctorState.customSavesPath;
          const worlds = await listSaveWorlds(curCustomPath || undefined);
          setCachedWorlds(worlds);
        } catch {
          // preserve
        }
      } catch (err: any) {
        showToast(`Restore failed: ${String(err)}`, 'error');
      } finally {
        setIsRestoringBackup(false);
        await rerenderCallback(parentContainer);
      }
    });
  });

  // Delete Handlers
  modal.querySelectorAll('.btn-vault-delete').forEach(btn => {
    btn.addEventListener('click', async () => {
      const filePath = (btn as HTMLElement).dataset.filepath;
      const fileName = (btn as HTMLElement).dataset.filename || 'backup';
      if (!filePath) return;

      const confirmed = await showConfirm(
        t('scanner.confirm_delete_pmm_title') || 'Delete Backup Archive',
        (t('scanner.confirm_delete_pmm_msg') || 'Are you sure you want to permanently delete backup \'{filename}\'?').replace('{filename}', fileName)
      );
      if (!confirmed) return;

      try {
        await deletePmmWorldBackup(filePath);
        showToast((t('scanner.toast_pmm_delete_success') || 'Backup {filename} deleted successfully.').replace('{filename}', fileName), 'success');
        
        // Remove card from UI
        const card = (btn as HTMLElement).closest('.pmm-backup-item-card');
        card?.remove();

        // Refresh world summary cache
        try {
          const curCustomPath = doctorState.customSavesPath;
          const worlds = await listSaveWorlds(curCustomPath || undefined);
          setCachedWorlds(worlds);
          await rerenderCallback(parentContainer);
        } catch {
          // preserve
        }
      } catch (err: any) {
        showToast(`Delete failed: ${String(err)}`, 'error');
      }
    });
  });
}
