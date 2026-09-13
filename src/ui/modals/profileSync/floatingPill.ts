import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { bus } from '../../../framework';
import { getState, updateState } from '../../../state';
import { applyProfileManifest, getMods, getProfiles } from '../../../api';
import { showToast } from '../../toast';
import { getActiveSyncSession, setActiveSyncSession } from './state';
import { showMissingModsDialog, hideMissingModsDialog } from './dialog';

let pillElement: HTMLElement | null = null;
let isApplying = false;

export function ensureFloatingSyncPill(): HTMLElement {
  if (pillElement) return pillElement;

  const pill = document.createElement('div');
  pill.id = 'profile-sync-floating-pill';
  pill.style.position = 'fixed';
  pill.style.bottom = '24px';
  pill.style.right = '24px';
  pill.style.zIndex = '9999';
  pill.style.display = 'none';
  pill.style.alignItems = 'center';
  pill.style.gap = '10px';
  pill.style.padding = '10px 18px';
  pill.style.background = 'rgba(24, 28, 36, 0.92)';
  pill.style.backdropFilter = 'blur(12px)';
  pill.style.border = '1px solid rgba(0, 180, 216, 0.45)';
  pill.style.borderRadius = '30px';
  pill.style.boxShadow = '0 8px 32px rgba(0, 0, 0, 0.45), 0 0 16px rgba(0, 180, 216, 0.2)';
  pill.style.cursor = 'pointer';
  pill.style.userSelect = 'none';
  pill.style.transition = 'all 0.25s ease';

  pill.innerHTML = `
    <div style="display: flex; align-items: center; justify-content: center; width: 22px; height: 22px; border-radius: 50%; background: rgba(0, 180, 216, 0.2); color: #00b4d8; font-size: 12px; animation: pmmPulse 2s infinite;">
      ⏳
    </div>
    <div style="display: flex; flex-direction: column; gap: 2px;">
      <span id="profile-sync-pill-title" style="font-size: 12px; font-weight: 700; color: #ffffff;">
        Profile Sync
      </span>
      <span id="profile-sync-pill-count" style="font-size: 11px; color: #a0aec0;">
        0 mods remaining
      </span>
    </div>
    <button id="profile-sync-pill-close" style="background: none; border: none; color: #718096; font-size: 14px; cursor: pointer; padding: 2px 4px; margin-left: 4px;" title="${escapeHtml(t('common.close'))}">✕</button>
  `;

  document.body.appendChild(pill);
  pillElement = pill;

  pill.addEventListener('mouseenter', () => {
    pill.style.transform = 'translateY(-2px)';
    pill.style.borderColor = 'rgba(0, 180, 216, 0.7)';
  });
  pill.addEventListener('mouseleave', () => {
    pill.style.transform = 'translateY(0)';
    pill.style.borderColor = 'rgba(0, 180, 216, 0.45)';
  });

  pill.addEventListener('click', (e) => {
    if ((e.target as HTMLElement).id === 'profile-sync-pill-close') {
      e.stopPropagation();
      hideFloatingSyncPill();
      return;
    }
    showMissingModsDialog();
  });

  // Inject CSS keyframe animation for pulse if not already present
  if (!document.getElementById('pmm-sync-pill-styles')) {
    const style = document.createElement('style');
    style.id = 'pmm-sync-pill-styles';
    style.textContent = `
      @keyframes pmmPulse {
        0% { transform: scale(1); opacity: 0.9; }
        50% { transform: scale(1.15); opacity: 1; }
        100% { transform: scale(1); opacity: 0.9; }
      }
    `;
    document.head.appendChild(style);
  }

  // Subscribe to reactive mod lifecycle events to automatically detect new mod installations
  bus.on('mods:loaded', (mods) => {
    handleModsStateChange(mods);
  });

  bus.on('mod:installed', () => {
    getMods().then(mods => handleModsStateChange(mods)).catch(console.error);
  });

  return pill;
}

export function updateFloatingSyncPill(): void {
  const session = getActiveSyncSession();
  const pill = ensureFloatingSyncPill();

  if (!session || session.missingMods.length === 0) {
    if (session && session.missingMods.length === 0 && !isApplying) {
      finalizeSync(session);
    } else {
      pill.style.display = 'none';
    }
    return;
  }

  const titleEl = pill.querySelector('#profile-sync-pill-title');
  const countEl = pill.querySelector('#profile-sync-pill-count');

  if (titleEl) {
    titleEl.textContent = t('profiles.sync_pill_title', { name: session.profileName });
  }

  if (countEl) {
    countEl.textContent = t('profiles.sync_pill_count', { count: session.missingMods.length });
  }

  pill.style.display = 'flex';
}

export function hideFloatingSyncPill(): void {
  if (pillElement) {
    pillElement.style.display = 'none';
  }
}

function handleModsStateChange(installedMods: any[]): void {
  const session = getActiveSyncSession();
  if (!session || isApplying) return;

  let anyResolved = false;
  const remaining = session.missingMods.filter(missing => {
    const found = installedMods.some(inst => {
      if (missing.nexusModId && inst.nexusModId && missing.nexusModId === inst.nexusModId) {
        return true;
      }
      return inst.name.toLowerCase() === missing.name.toLowerCase();
    });

    if (found) {
      anyResolved = true;
      showToast(t('profiles.sync_mod_resolved_toast', { name: missing.name }), 'success');
      return false; // Remove from missing
    }
    return true;
  });

  if (anyResolved) {
    session.missingMods = remaining;
    updateFloatingSyncPill();

    // If missing mods dialog is open, re-render it
    const modal = document.getElementById('profile-missing-mods-modal');
    if (modal && modal.style.display !== 'none') {
      showMissingModsDialog();
    }

    if (remaining.length === 0) {
      finalizeSync(session);
    }
  }
}

async function finalizeSync(session: any): Promise<void> {
  if (isApplying) return;
  isApplying = true;

  try {
    const pill = ensureFloatingSyncPill();
    const countEl = pill.querySelector('#profile-sync-pill-count');
    if (countEl) {
      countEl.textContent = t('profiles.sync_applying');
    }

    showToast(t('profiles.sync_all_found_applying', { name: session.profileName }), 'info');

    const result = await applyProfileManifest(session.manifestPath, session.profileName);

    if (result.success) {
      setActiveSyncSession(null);
      hideFloatingSyncPill();
      hideMissingModsDialog();

      showToast(t('profiles.sync_complete_toast', {
        name: session.profileName,
        customizations: result.customizationsApplied,
      }), 'success');

      // Refresh application state
      const [profiles, mods] = await Promise.all([getProfiles(), getMods()]);
      updateState({ profiles, allMods: mods });
      bus.emit('profile:syncCompleted', {
        profileId: result.profileId,
        profileName: session.profileName,
      });

      const { renderModsView } = await import('../../mods/renderer');
      renderModsView();
      const { renderProfileList } = await import('../../mods/profiles');
      renderProfileList();
    }
  } catch (err) {
    console.error('Failed to finalize profile sync:', err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  } finally {
    isApplying = false;
  }
}
