import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { openUrl } from '../../../api';
import { showToast } from '../../toast';
import { getActiveSyncSession, setActiveSyncSession } from './state';
import { updateFloatingSyncPill, hideFloatingSyncPill } from './floatingPill';

let dialogOverlay: HTMLElement | null = null;

function ensureDialogElement(): HTMLElement {
  if (dialogOverlay) return dialogOverlay;

  const overlay = document.createElement('div');
  overlay.id = 'profile-missing-mods-modal';
  overlay.className = 'modal-overlay';
  overlay.style.display = 'none';

  overlay.innerHTML = `
    <div class="modal" style="max-width: min(calc(680px * var(--ui-scale, 1)), 94vw); width: 100%; max-height: 85vh; display: flex; flex-direction: column;">
      <div class="modal-header" style="display: flex; align-items: center; justify-content: space-between; padding: calc(12px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1));">
        <h3 id="profile-sync-dialog-title" style="margin: 0; font-size: var(--text-lg, 16px); font-weight: 600;">
          ${escapeHtml(t('profiles.sync_dialog_title'))}
        </h3>
        <button class="modal-close-btn" id="profile-sync-dialog-close-x" style="background: none; border: none; font-size: var(--text-base, 16px); cursor: pointer; color: var(--text-muted);">✕</button>
      </div>
      <div class="modal-body" style="overflow-y: auto; flex: 1; padding: calc(14px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(12px * var(--ui-scale, 1));">
        <div id="profile-sync-dialog-desc" style="font-size: var(--text-base, 13px); color: var(--text-secondary); line-height: 1.5;"></div>
        <div id="profile-sync-missing-list" style="display: flex; flex-direction: column; gap: calc(8px * var(--ui-scale, 1));"></div>
      </div>
      <div class="modal-footer" style="display: flex; justify-content: space-between; align-items: center; padding: calc(10px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); border-top: 1px solid var(--border);">
        <button id="profile-sync-cancel-btn" class="btn-secondary" style="color: var(--danger);">
          ${escapeHtml(t('profiles.btn_cancel_sync'))}
        </button>
        <div style="display: flex; gap: calc(8px * var(--ui-scale, 1));">
          <button id="profile-sync-background-btn" class="btn-primary">
            ${escapeHtml(t('profiles.btn_sync_background'))}
          </button>
        </div>
      </div>
    </div>
  `;

  document.body.appendChild(overlay);
  dialogOverlay = overlay;

  overlay.querySelector('#profile-sync-dialog-close-x')?.addEventListener('click', () => {
    hideMissingModsDialog();
  });

  overlay.querySelector('#profile-sync-background-btn')?.addEventListener('click', () => {
    hideMissingModsDialog();
    showToast(t('profiles.sync_background_toast'), 'info');
  });

  overlay.querySelector('#profile-sync-cancel-btn')?.addEventListener('click', () => {
    setActiveSyncSession(null);
    hideFloatingSyncPill();
    hideMissingModsDialog();
    showToast(t('profiles.sync_cancelled_toast'), 'info');
  });

  return overlay;
}

export function showMissingModsDialog(): void {
  const session = getActiveSyncSession();
  if (!session) return;

  const overlay = ensureDialogElement();
  const titleEl = overlay.querySelector('#profile-sync-dialog-title');
  const descEl = overlay.querySelector('#profile-sync-dialog-desc');
  const listEl = overlay.querySelector('#profile-sync-missing-list');

  if (titleEl) {
    titleEl.textContent = t('profiles.sync_dialog_title_with_name', { name: session.profileName });
  }

  if (descEl) {
    descEl.textContent = t('profiles.sync_dialog_desc', {
      missing: session.missingMods.length,
      total: session.totalMods,
    });
  }

  if (listEl) {
    if (session.missingMods.length === 0) {
      listEl.innerHTML = `
        <div style="padding: calc(20px * var(--ui-scale, 1)); text-align: center; color: var(--success); font-weight: 500; font-size: var(--text-base, 13px);">
          ${escapeHtml(t('profiles.sync_all_downloaded'))}
        </div>
      `;
    } else {
      listEl.innerHTML = session.missingMods.map((mod, idx) => {
        const nexusUrl = mod.nexusModId
          ? `https://www.nexusmods.com/palworld/mods/${mod.nexusModId}`
          : mod.directDownloadUrl || null;

        const badgeClass = `mod-type-badge ${mod.modType.toLowerCase()}`;

        return `
          <div class="missing-mod-card" style="display: flex; align-items: center; justify-content: space-between; padding: calc(10px * var(--ui-scale, 1)) calc(14px * var(--ui-scale, 1)); background: var(--bg-card); border: 1px solid var(--border); border-radius: 6px; gap: calc(12px * var(--ui-scale, 1));">
            <div style="display: flex; align-items: center; gap: calc(10px * var(--ui-scale, 1)); min-width: 0; flex: 1;">
              <span style="font-size: var(--text-xs, 11px); font-weight: 700; color: var(--text-muted); width: 20px;">#${idx + 1}</span>
              <div style="display: flex; flex-direction: column; gap: 3px; min-width: 0;">
                <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
                  <span style="font-weight: 600; font-size: var(--text-base, 13px); color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">
                    ${escapeHtml(mod.name)}
                  </span>
                  <span class="${badgeClass}" style="font-size: var(--text-2xs, 10px); padding: 1px calc(6px * var(--ui-scale, 1)); border-radius: 3px; text-transform: uppercase;">
                    ${escapeHtml(mod.modType)}
                  </span>
                </div>
                <div style="font-size: var(--text-xs, 11px); color: var(--text-secondary);">
                  ${escapeHtml(t('profiles.version_req', { version: mod.version || '1.0.0' }))}
                </div>
              </div>
            </div>
            <div style="display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
              ${nexusUrl ? `
                <button type="button" class="btn-secondary btn-sm sync-open-url-btn" data-url="${escapeHtml(nexusUrl)}" style="font-size: var(--text-sm, 12px); display: inline-flex; align-items: center; gap: 4px;">
                  🌐 NexusMods
                </button>
              ` : `
                <button type="button" class="btn-secondary btn-sm sync-copy-name-btn" data-name="${escapeHtml(mod.name)}" style="font-size: var(--text-sm, 12px);">
                  📋 ${escapeHtml(t('common.copy_name'))}
                </button>
              `}
            </div>
          </div>
        `;
      }).join('');

      listEl.querySelectorAll('.sync-open-url-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          const url = (btn as HTMLElement).dataset.url;
          if (url) {
            openUrl(url).catch(err => console.error('Failed to open URL:', err));
          }
        });
      });

      listEl.querySelectorAll('.sync-copy-name-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
          e.stopPropagation();
          const name = (btn as HTMLElement).dataset.name;
          if (name) {
            navigator.clipboard.writeText(name);
            showToast(t('common.copied_to_clipboard'), 'info');
          }
        });
      });
    }
  }

  overlay.style.display = 'flex';
  updateFloatingSyncPill();
}

export function hideMissingModsDialog(): void {
  if (dialogOverlay) {
    dialogOverlay.style.display = 'none';
  }
}
