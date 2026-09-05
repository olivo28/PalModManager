import { getState, updateState } from '../../state';
import { checkDependencies, checkDependenciesFull, installUe4ss, installPalschema, uninstallUe4ss, uninstallPalschema } from '../../api';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t } from '../../utils/i18n';
import { loadMods } from './loader';
import { loadProfiles } from './profiles';
import { bus, mainDom } from '../../framework';
import { renderConflictBanner, removeConflictBanner } from '../conflictBanner';

let _isPromptingUe4ss = false;
let _isPromptingPalschema = false;
let _loadDepsPromise: Promise<void> | null = null;
let _lastLoadDepsTime = 0;

async function waitUntilAppReady(): Promise<void> {
  const loading = mainDom.elMaybe('app-loading');
  const app = mainDom.elMaybe('app');
  const isAppVisible = app && app.style.display === 'flex' && (!loading || loading.style.display === 'none');
  if (isAppVisible) {
    return;
  }
  return new Promise((resolve) => {
    let resolved = false;
    const unsub = bus.on('app:ready', () => {
      if (!resolved) {
        resolved = true;
        unsub();
        setTimeout(resolve, 150);
      }
    });
    setTimeout(() => {
      if (!resolved) {
        resolved = true;
        unsub();
        resolve();
      }
    }, 5000);
  });
}

async function checkAndPromptUpdates(fullDeps: any): Promise<void> {
  await waitUntilAppReady();

  // Check if UE4SS needs update and ask user (only for GitHub / Standalone mode)
  if (fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop') {
    const latestTarget = fullDeps.ue4ss_latest_date || fullDeps.ue4ss_latest_tag || 'latest';
    if (!_isPromptingUe4ss && sessionStorage.getItem('dismissed_ue4ss_update') !== latestTarget) {
      _isPromptingUe4ss = true;
      sessionStorage.setItem('dismissed_ue4ss_update', latestTarget);
      try {
        const confirmed = await showConfirm(
          t('dependencies.prompt_body_ue4ss', { installed: fullDeps.ue4ss_version || 'installed', latest: latestTarget }),
          t('dependencies.prompt_title_ue4ss')
        );
        _isPromptingUe4ss = false;
        if (confirmed) {
          executeInstallOrUpdate('ue4ss', true);
          return;
        }
      } catch {
        _isPromptingUe4ss = false;
      }
    }
  }

  // Check if PalSchema needs update and ask user (only for GitHub / Standalone mode)
  if (fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop' && fullDeps.palschema_version !== 'Workshop') {
    const latestVer = fullDeps.palschema_latest_version || 'latest';
    if (!_isPromptingPalschema && sessionStorage.getItem('dismissed_palschema_update') !== latestVer) {
      _isPromptingPalschema = true;
      sessionStorage.setItem('dismissed_palschema_update', latestVer);
      try {
        const confirmed = await showConfirm(
          t('dependencies.prompt_body_palschema', { installed: fullDeps.palschema_version || 'installed', latest: latestVer }),
          t('dependencies.prompt_title_palschema')
        );
        _isPromptingPalschema = false;
        if (confirmed) {
          executeInstallOrUpdate('palschema', true);
        }
      } catch {
        _isPromptingPalschema = false;
      }
    }
  }
}

export async function loadDependencies(force = false): Promise<void> {
  const now = Date.now();
  if (!force && _loadDepsPromise) {
    return _loadDepsPromise;
  }
  if (!force && now - _lastLoadDepsTime < 3000) {
    return;
  }
  _lastLoadDepsTime = now;

  _loadDepsPromise = (async () => {
    try {
      const fullDeps = await checkDependenciesFull();
      
      if (fullDeps.has_dll_conflict && fullDeps.conflicting_dlls && fullDeps.conflicting_dlls.length > 0) {
        renderConflictBanner(fullDeps.conflicting_dlls);
      } else {
        removeConflictBanner();
      }

      if (fullDeps.ue4ss_updated_from && fullDeps.ue4ss_version) {
        showToast(t('toasts.dep_updated', { dep: 'UE4SS', oldVer: fullDeps.ue4ss_updated_from, newVer: fullDeps.ue4ss_version }), 'success');
      }
      if (fullDeps.palschema_updated_from && fullDeps.palschema_version) {
        showToast(t('toasts.dep_updated', { dep: 'PalSchema', oldVer: fullDeps.palschema_updated_from, newVer: fullDeps.palschema_version }), 'success');
      }

      updateState({ dependencies: fullDeps });
      renderDependencyBadges(fullDeps);

      // Defer interactive update prompt until the main app UI is visible and loaded
      checkAndPromptUpdates(fullDeps);
    } catch (e) {
      console.error('Failed to check dependencies:', e);
    } finally {
      _loadDepsPromise = null;
    }
  })();

  return _loadDepsPromise;
}

export function executeInstallOrUpdate(type: 'ue4ss' | 'palschema', isUpdate: boolean): void {
  const depName = type === 'ue4ss' ? 'UE4SS' : 'PalSchema';
  showToast(isUpdate ? t('toasts.updating_dep', { dep: depName }) : t('toasts.installing_dep', { dep: depName }), 'info');
  const promise = type === 'ue4ss' ? installUe4ss() : installPalschema();
  promise.then(async () => {
    showToast(t('dependencies.up_to_date'), 'success');
    await loadProfiles();
    await loadDependencies();
    await loadMods();
  }).catch(e => showToast(t('toasts.export_failed', { error: String(e) }), 'error'));
}

export function handleDepBadgeClick(type: 'ue4ss' | 'palschema'): void {
  import('../modals/dependencyModal').then(({ showDependencyModal }) => {
    showDependencyModal(type);
  }).catch((err) => {
    console.error('Failed to open dependency modal:', err);
  });
}

export function renderDependencyBadges(deps: import('../../types').DependencyStatus): void {
  const platformEl = mainDom.elMaybe('game-platform-badge');
  if (platformEl) {
    if (deps.game_platform && deps.game_platform !== 'Unknown') {
      platformEl.textContent = deps.game_platform;
      platformEl.style.display = '';
      platformEl.className = 'game-platform-badge ' + deps.game_platform.toLowerCase();
    } else {
      platformEl.style.display = 'none';
    }
  }

  const ue4ssEl = mainDom.elMaybe('ue4ss-badge');
  const psEl = mainDom.elMaybe('palschema-badge');
  if (!ue4ssEl || !psEl) return;

  const { currentProfile } = getState();
  const ue4ssFlo = currentProfile?.force_load_order_ue4ss ? ' (FLO)' : '';
  const palschemaFlo = currentProfile?.force_load_order_palschema ? ' (FLO)' : '';

  // UE4SS
  if (deps.ue4ss_installed) {
    const formatDMY = (dmy: string): string => {
      const parts = dmy.split('.');
      if (parts.length === 3) {
        const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
        const m = parseInt(parts[1], 10) - 1;
        return `${parseInt(parts[0], 10)} ${months[m] ?? parts[1]} ${parts[2]}`;
      }
      return dmy;
    };
    const isWorkshop = deps.ue4ss_install_mode === 'Workshop' || deps.ue4ss_version === 'Workshop';
    let verDisplay = '';
    if (deps.ue4ss_version && deps.ue4ss_version !== 'Workshop') {
      verDisplay = deps.ue4ss_version.includes('.') ? ` · ${formatDMY(deps.ue4ss_version)}` : ` v${deps.ue4ss_version}`;
    }
    const workshopSuffix = isWorkshop ? ' (Workshop)' : '';
    ue4ssEl.textContent = `UE4SS${verDisplay}${workshopSuffix}${ue4ssFlo}`;
    ue4ssEl.className = `dep-badge ${isWorkshop ? 'workshop' : (deps.ue4ss_needs_update ? 'warn' : 'ok')}`;
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = 'pointer';
    if (isWorkshop) {
      ue4ssEl.title = `UE4SS${verDisplay} (Steam Workshop) — ${t('dependencies.managed_by_steam')}`;
    } else if (deps.ue4ss_needs_update) {
      const latestDisplay = deps.ue4ss_latest_date ? formatDMY(deps.ue4ss_latest_date) : '?';
      ue4ssEl.title = t('dependencies.update_available_ue4ss', { date: latestDisplay });
    } else {
      ue4ssEl.title = `UE4SS${verDisplay} — ${t('dependencies.up_to_date')}`;
    }
  } else {
    ue4ssEl.textContent = 'UE4SS ✕';
    ue4ssEl.className = 'dep-badge missing';
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = 'pointer';
    ue4ssEl.title = t('dependencies.ue4ss_not_installed');
  }

  // PalSchema
  if (deps.palschema_installed) {
    const isWorkshop = deps.ue4ss_install_mode === 'Workshop' || deps.palschema_version === 'Workshop';
    let ver = '';
    if (deps.palschema_version && deps.palschema_version !== 'Workshop') {
      ver = ` v${deps.palschema_version.replace(/^v/i, '')}`;
    } else if (deps.palschema_latest_version) {
      ver = ` v${deps.palschema_latest_version.replace(/^v/i, '')}`;
    }
    const workshopSuffix = isWorkshop ? ' (Workshop)' : '';
    psEl.textContent = `PalSchema${ver}${workshopSuffix}${palschemaFlo}`;
    psEl.className = `dep-badge ${isWorkshop ? 'workshop' : (deps.palschema_needs_update ? 'warn' : 'ok')}`;
    psEl.style.display = '';
    psEl.style.cursor = 'pointer';
    psEl.title = isWorkshop ? `PalSchema${ver} (Steam Workshop) — ${t('dependencies.managed_by_steam')}` : (deps.palschema_needs_update ? t('dependencies.update_available_palschema') : t('dependencies.up_to_date'));
  } else {
    psEl.textContent = 'PalSchema ✕';
    psEl.className = 'dep-badge missing';
    psEl.style.display = '';
    psEl.style.cursor = 'pointer';
    psEl.title = t('dependencies.palschema_not_installed');
  }

  ue4ssEl.onclick = () => handleDepBadgeClick('ue4ss');
  psEl.onclick = () => handleDepBadgeClick('palschema');
}
