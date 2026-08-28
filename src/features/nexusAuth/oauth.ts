import {
  startNexusOAuth,
  handleNexusOAuthCallback,
  logoutNexusAccount,
  type NexusAccountInfo,
} from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../../ui/toast';
import { t } from '../../utils/i18n';

export async function processNxmDownload(nxmUrl: string): Promise<void> {
  const currentAcc = getState().currentSettings?.nexusAccount;
  if (!currentAcc || !currentAcc.accessToken) {
    showToast(t('nexus_profile.nxm_login_required'), 'warning');
    const { openNexusProfileModal } = await import('./profileModal');
    openNexusProfileModal();
    return;
  }

  const { enqueueNxmDownload } = await import('../nxm_queue');
  await enqueueNxmDownload(nxmUrl);
}

export async function processOAuthCallback(callbackUrl: string): Promise<NexusAccountInfo | null> {
  showToast(t('toasts.nexus_authenticating'), 'info');
  try {
    const account = await handleNexusOAuthCallback(callbackUrl);
    const currentSettings = getState().currentSettings;
    if (currentSettings) {
      updateState({
        currentSettings: {
          ...currentSettings,
          nexusAccount: account,
        },
      });
    }
    const username = account.username || 'Nexus User';
    showToast(t('toasts.nexus_login_success', { user: username }), 'success');
    const { renderNexusAccountUI } = await import('./ui');
    renderNexusAccountUI();
    return account;
  } catch (err: any) {
    console.error('[NexusAuth] Login failed:', err);
    showToast(t('toasts.nexus_login_failed', { error: String(err) }), 'error');
    return null;
  }
}

export async function triggerOAuthLogin(): Promise<void> {
  try {
    showToast(t('toasts.nexus_opening_browser'), 'info');
    await startNexusOAuth();
  } catch (err: any) {
    console.error('[NexusAuth] Failed to start OAuth flow:', err);
    showToast(t('toasts.nexus_login_failed', { error: String(err) }), 'error');
  }
}

export async function triggerLogout(): Promise<void> {
  try {
    await logoutNexusAccount();
    const currentSettings = getState().currentSettings;
    if (currentSettings) {
      updateState({
        currentSettings: {
          ...currentSettings,
          nexusAccount: null,
        },
      });
    }
    showToast(t('toasts.nexus_logged_out'), 'info');
    const { renderNexusAccountUI } = await import('./ui');
    renderNexusAccountUI();
  } catch (err: any) {
    console.error('[NexusAuth] Logout failed:', err);
  }
}
