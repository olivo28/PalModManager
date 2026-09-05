import { listen } from '@tauri-apps/api/event';
import { getNexusAccountStatus } from '../../api';
import { getState, updateState } from '../../state';
import { t } from '../../utils/i18n';
import { DEFAULT_AVATAR, isListening, setIsListening } from './state';
import { processOAuthCallback, processNxmDownload, triggerOAuthLogin, triggerLogout } from './oauth';
import { openNexusProfileModal } from './profileModal';
import { updateProtocolStatusUI } from './protocol';
import { mainDom, settingsDom } from '../../framework';

export async function initNexusAuth(): Promise<void> {
  // 1. Initial check of account status
  try {
    const account = await getNexusAccountStatus();
    const currentSettings = getState().currentSettings;
    if (currentSettings) {
      updateState({
        currentSettings: {
          ...currentSettings,
          nexusAccount: account,
        },
      });
    }
    renderSidebarNexusWidget();
  } catch (err) {
    console.warn('[NexusAuth] Failed to load initial account status:', err);
  }

  // 2. Setup deep-link listener if not already initialized
  if (!isListening) {
    setIsListening(true);
    try {
      await listen<string>('nexus-oauth-deep-link', async (event) => {
        const url = event.payload;
        console.log('[NexusAuth] Captured deep link:', url);
        if (url && url.startsWith('palmodmanager://oauth/callback')) {
          await processOAuthCallback(url);
        }
      });

      await listen<string>('nexus-nxm-download', async (event) => {
        const url = event.payload;
        console.log('[NexusAuth] Captured NXM download link:', url);
        if (url && url.startsWith('nxm://')) {
          await processNxmDownload(url);
        }
      });
    } catch (err) {
      console.warn('[NexusAuth] Failed to setup deep link listener:', err);
    }
  }
}

export function renderNexusAccountUI(): void {
  renderSidebarNexusWidget();

  const container = settingsDom.elMaybe('settings-nexus-account-card');
  if (!container) return;

  const state = getState();
  const account = state.currentSettings?.nexusAccount;

  if (account && account.username) {
    const isPremium = account.isPremium;
    const isSupporter = account.isSupporter;
    const tierBadgeClass = isPremium ? 'nexus-badge-premium' : isSupporter ? 'nexus-badge-supporter' : 'nexus-badge-free';
    const tierText = isPremium ? t('settings.nexus_status_premium') : isSupporter ? t('settings.nexus_status_supporter') : t('settings.nexus_status_free');
    const avatar = account.avatarUrl || DEFAULT_AVATAR;

    container.innerHTML = `
      <div class="nexus-profile-card">
        <div class="nexus-avatar-wrap" style="cursor: pointer; transition: transform 0.2s;" id="btn-nexus-open-modal-avatar" title="${t('nexus_profile.modal_title')}">
          <img src="${avatar}" data-original-src="${avatar}" alt="${account.username}" class="nexus-avatar-img" onerror="window.handleUniversalImageFallback ? window.handleUniversalImageFallback(this, '${DEFAULT_AVATAR}') : (this.src='${DEFAULT_AVATAR}')" />
        </div>
        <div class="nexus-profile-info" style="cursor: pointer;" id="btn-nexus-open-modal-info" title="${t('nexus_profile.modal_title')}">
          <div class="nexus-profile-header">
            <span class="nexus-username">${account.username}</span>
            <span class="nexus-tier-badge ${tierBadgeClass}">${tierText}</span>
          </div>
          <div class="nexus-profile-sub" style="display: flex; gap: 8px; align-items: center;">
            <span style="font-family: monospace; font-size: 11px; opacity: 0.85;">ID: #${account.userId || 'N/A'}</span>
            <span>•</span>
            <span>${t('settings.nexus_auth_oauth')}</span>
          </div>
        </div>
        <div class="nexus-profile-actions" style="display: flex; gap: 8px; align-items: center;">
          <button type="button" class="btn btn-secondary btn-sm" id="btn-nexus-open-modal" style="display: flex; align-items: center; gap: 5px;">
            <span>👤</span> ${t('nexus_profile.modal_title')}
          </button>
          <button type="button" class="btn btn-danger btn-sm" id="btn-nexus-disconnect">
            ${t('settings.nexus_disconnect_btn')}
          </button>
        </div>
      </div>
    `;

    settingsDom.elMaybe('btn-nexus-open-modal-avatar')?.addEventListener('click', () => openNexusProfileModal());
    settingsDom.elMaybe('btn-nexus-open-modal-info')?.addEventListener('click', () => openNexusProfileModal());
    settingsDom.elMaybe('btn-nexus-open-modal')?.addEventListener('click', () => openNexusProfileModal());

    settingsDom.elMaybe('btn-nexus-disconnect')?.addEventListener('click', async () => {
      await triggerLogout();
    });
  } else {
    container.innerHTML = `
      <div class="nexus-login-prompt">
        <div class="nexus-prompt-text">
          <p>${t('settings.nexus_connect_desc')}</p>
        </div>
        <div class="nexus-prompt-actions">
          <button type="button" class="btn btn-nexus-connect" id="btn-nexus-connect-oauth">
            <svg class="nexus-icon-svg" viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 14.5v-9l6 4.5-6 4.5z"/>
            </svg>
            ${t('settings.nexus_connect_btn')}
          </button>
        </div>
      </div>
    `;

    settingsDom.elMaybe('btn-nexus-connect-oauth')?.addEventListener('click', async () => {
      await triggerOAuthLogin();
    });
  }

  // Update Protocol Status Section
  updateProtocolStatusUI();
}

export function renderSidebarNexusWidget(): void {
  const widget = mainDom.elMaybe('sidebar-nexus-widget');
  if (!widget) return;

  const state = getState();
  const account = state.currentSettings?.nexusAccount;
  const discoveryTabBtn = mainDom.elMaybe('sidebar-tab-discovery');

  if (account && account.username) {
    if (discoveryTabBtn) discoveryTabBtn.style.display = 'flex';
    const avatar = account.avatarUrl || DEFAULT_AVATAR;
    const avatarEl = mainDom.elMaybe('sidebar-nexus-avatar');
    const nameEl = mainDom.elMaybe('sidebar-nexus-name');
    const tierEl = mainDom.elMaybe('sidebar-nexus-tier');

    if (avatarEl) {
      avatarEl.src = avatar;
      avatarEl.onerror = () => { avatarEl.src = DEFAULT_AVATAR; };
    }
    if (nameEl) {
      nameEl.textContent = account.username;
    }
    if (tierEl) {
      const tierShort = account.isPremium ? 'PREMIUM' : account.isSupporter ? 'SUPPORTER' : 'MEMBER';
      tierEl.textContent = tierShort;
      tierEl.className = `sidebar-nexus-tier ${account.isPremium ? 'premium' : account.isSupporter ? 'supporter' : 'free'}`;
    }

    widget.style.display = 'flex';
    widget.onclick = () => {
      openNexusProfileModal();
    };
  } else {
    if (discoveryTabBtn) discoveryTabBtn.style.display = 'none';
    if (state.activeTab === 'discovery') {
      import('../../ui/tabManager').then(({ navigateTo }) => navigateTo('mods'));
    }
    widget.style.display = 'none';
    widget.onclick = null;
  }
}
