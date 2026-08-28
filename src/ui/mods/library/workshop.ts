import { getState } from '../../../state';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import { _activeLibrarySubTab } from './state';

const WORKSHOP_TIMESTAMPS_KEY = 'pmm_workshop_mod_timestamps';
const NEW_MOD_DURATION_MS = 10 * 60 * 1000; // 10 minutes

export function syncWorkshopModTimestamps(allWorkshopMods: Array<{ packageName: string; modName: string }>): {
  newModCount: number;
  newModNames: string[];
} {
  try {
    const raw = localStorage.getItem(WORKSHOP_TIMESTAMPS_KEY);
    const now = Date.now();

    if (!raw) {
      const initMap: Record<string, number> = {};
      for (const m of allWorkshopMods) {
        initMap[m.packageName] = 0;
      }
      localStorage.setItem(WORKSHOP_TIMESTAMPS_KEY, JSON.stringify(initMap));
      return { newModCount: 0, newModNames: [] };
    }

    const map: Record<string, number> = JSON.parse(raw);
    const newModNames: string[] = [];
    let updated = false;

    for (const m of allWorkshopMods) {
      if (map[m.packageName] === undefined) {
        map[m.packageName] = now;
        newModNames.push(m.modName);
        updated = true;
      }
    }

    if (updated) {
      localStorage.setItem(WORKSHOP_TIMESTAMPS_KEY, JSON.stringify(map));
    }

    let newCount = 0;
    for (const m of allWorkshopMods) {
      const addedAt = map[m.packageName];
      if (addedAt && (now - addedAt) < NEW_MOD_DURATION_MS) {
        newCount++;
      }
    }

    return { newModCount: newCount, newModNames };
  } catch {
    return { newModCount: 0, newModNames: [] };
  }
}

export function isWorkshopModNew(packageName: string): boolean {
  try {
    const raw = localStorage.getItem(WORKSHOP_TIMESTAMPS_KEY);
    if (!raw) return false;
    const map: Record<string, number> = JSON.parse(raw);
    const addedAt = map[packageName];
    if (!addedAt) return false;
    return (Date.now() - addedAt) < NEW_MOD_DURATION_MS;
  } catch {
    return false;
  }
}

import { libraryDom, mainDom } from '../../../framework';

export function updateWorkshopBadges(newCount: number): void {
  const subtabBadge = libraryDom.elMaybe('workshop-subtab-badge');
  const sidebarBadge = mainDom.elMaybe('sidebar-library-badge');

  if (subtabBadge) {
    if (newCount > 0) {
      subtabBadge.textContent = t('library.badge_new_count', { count: newCount });
      subtabBadge.style.display = 'inline-block';
    } else {
      subtabBadge.style.display = 'none';
    }
  }

  if (sidebarBadge) {
    sidebarBadge.style.display = newCount > 0 ? 'block' : 'none';
  }
}

export function updateWorkshopTabVisibility(): void {
  const wsTabBtn = document.querySelector<HTMLButtonElement>('.library-sub-tab[data-tab="workshop"]');
  if (!wsTabBtn) return;

  const state = getState();
  const activeProfile = state.currentProfile || state.profiles?.find(p => p.id === state.currentProfileId);
  const isProfileWorkshop = activeProfile?.dependency_mode === 'workshop';

  if (!isProfileWorkshop) {
    wsTabBtn.style.display = 'none';
    if (_activeLibrarySubTab === 'workshop') {
      const localTabBtn = document.querySelector<HTMLButtonElement>('.library-sub-tab[data-tab="local"]');
      if (localTabBtn) {
        localTabBtn.click();
      }
    }
  } else {
    wsTabBtn.style.display = 'flex';
  }
}

export async function handleCheckWorkshopOnlineUpdates(): Promise<void> {
  const btn = libraryDom.elMaybe('workshop-check-updates-btn');
  if (btn) {
    btn.disabled = true;
    btn.textContent = t('library.checking_workshop_updates');
  }

  showToast(t('library.checking_workshop_updates'), 'info');

  try {
    const { checkWorkshopUpdatesOnline } = await import('../../../api');
    const { renderLibraryView } = await import('./render');
    const result = await checkWorkshopUpdatesOnline();

    if (result.totalChecked === 0) {
      showToast(t('library.empty_workshop'), 'info');
      return;
    }

    if (result.readyToInstallUpdates.length > 0) {
      showToast(t('library.toast_updates_ready_to_install', { count: result.readyToInstallUpdates.length }), 'success');
      await renderLibraryView();
      return;
    }

    if (result.pendingSteamDownloads.length > 0) {
      const { showConfirm } = await import('../../confirm');
      const modNames = result.pendingSteamDownloads.map(m => m.modName).join(', ');
      const confirmed = await showConfirm(
        t('library.confirm_force_steam_validation', { count: result.pendingSteamDownloads.length, names: modNames })
      );
      if (confirmed) {
        await handleTriggerSteamValidation(true);
      }
      return;
    }

    showToast(t('library.toast_workshop_up_to_date'), 'success');
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = t('library.btn_check_workshop_updates');
    }
  }
}

export async function handleTriggerSteamValidation(bypassConfirm: boolean = false): Promise<void> {
  try {
    if (!bypassConfirm) {
      const { showConfirm } = await import('../../confirm');
      const confirmed = await showConfirm(t('library.confirm_direct_steam_validation'));
      if (!confirmed) return;
    }

    const { triggerSteamValidation } = await import('../../../api');
    showToast(t('library.toast_starting_steam_validation'), 'info');
    await triggerSteamValidation();

    // Lock Play button safely for 2 minutes (120 seconds) while Steam downloads updates
    const playBtn = mainDom.elMaybe('launch-game-btn');
    const playLabel = playBtn?.querySelector('.sidebar-tab-label') as HTMLElement | null;

    if (playBtn) {
      playBtn.disabled = true;
      playBtn.style.opacity = '0.6';
      playBtn.style.cursor = 'not-allowed';
    }

    let remainingSeconds = 120;
    const intervalId = setInterval(async () => {
      remainingSeconds--;
      if (playBtn) {
        playBtn.title = t('library.steam_validating_tooltip', { time: remainingSeconds });
      }
      if (playLabel) {
        const mins = Math.floor(remainingSeconds / 60);
        const secs = remainingSeconds % 60;
        playLabel.textContent = `${mins}:${secs < 10 ? '0' : ''}${secs}`;
      }

      if (remainingSeconds <= 0) {
        clearInterval(intervalId);
        if (playBtn) {
          playBtn.disabled = false;
          playBtn.style.opacity = '1';
          playBtn.style.cursor = 'pointer';
          playBtn.title = t('sidebar.launch_game_title');
        }
        if (playLabel) {
          playLabel.textContent = t('sidebar.launch_game');
        }

        showToast(t('library.toast_steam_validation_completed'), 'success');

        const { loadLibrary } = await import('./listeners');
        const { loadMods } = await import('../../modsView');
        await loadLibrary();
        await loadMods();
      }
    }, 1000);
  } catch (err) {
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}
