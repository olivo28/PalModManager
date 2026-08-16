import { getState, updateState } from '../../state';
import { checkDependencies, installUe4ss, installPalschema, uninstallUe4ss, uninstallPalschema } from '../../api';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t } from '../../utils/i18n';
import { loadMods } from './loader';
import { loadProfiles } from './profiles';

import { renderConflictBanner, removeConflictBanner } from '../conflictBanner';

let _isPromptingUe4ss = false;
let _isPromptingPalschema = false;

export async function loadDependencies(): Promise<void> {
  try {
    const deps = await checkDependencies();
    
    if (deps.has_dll_conflict && deps.conflicting_dlls && deps.conflicting_dlls.length > 0) {
      renderConflictBanner(deps.conflicting_dlls);
    } else {
      removeConflictBanner();
    }

    if (deps.ue4ss_updated_from && deps.ue4ss_version) {
      showToast(t('toasts.dep_updated', { dep: 'UE4SS', oldVer: deps.ue4ss_updated_from, newVer: deps.ue4ss_version }), 'success');
    }
    if (deps.palschema_updated_from && deps.palschema_version) {
      showToast(t('toasts.dep_updated', { dep: 'PalSchema', oldVer: deps.palschema_updated_from, newVer: deps.palschema_version }), 'success');
    }

    import('../../api').then(({ checkDependenciesFull }) => {
      checkDependenciesFull().then(fullDeps => {
        updateState({ dependencies: fullDeps });
        renderDependencyBadges(fullDeps);
        if (fullDeps.has_dll_conflict && fullDeps.conflicting_dlls && fullDeps.conflicting_dlls.length > 0) {
          renderConflictBanner(fullDeps.conflicting_dlls);
        }

        // Check if UE4SS needs update and ask user
        if (fullDeps.ue4ss_installed && fullDeps.ue4ss_needs_update && fullDeps.ue4ss_install_mode !== 'Workshop') {
          const latestTarget = fullDeps.ue4ss_latest_date || fullDeps.ue4ss_latest_tag || 'latest';
          if (!_isPromptingUe4ss && sessionStorage.getItem('dismissed_ue4ss_update') !== latestTarget) {
            _isPromptingUe4ss = true;
            sessionStorage.setItem('dismissed_ue4ss_update', latestTarget);
            showConfirm(
              t('dependencies.prompt_body_ue4ss', { installed: fullDeps.ue4ss_version || 'installed', latest: latestTarget }),
              t('dependencies.prompt_title_ue4ss')
            ).then((confirmed) => {
              _isPromptingUe4ss = false;
              if (confirmed) {
                executeInstallOrUpdate('ue4ss', true);
              }
            }).catch(() => { _isPromptingUe4ss = false; });
          }
        }

        // Check if PalSchema needs update and ask user
        if (fullDeps.palschema_installed && fullDeps.palschema_needs_update && fullDeps.palschema_version !== 'Workshop') {
          const latestVer = fullDeps.palschema_latest_version || 'latest';
          if (!_isPromptingPalschema && sessionStorage.getItem('dismissed_palschema_update') !== latestVer) {
            _isPromptingPalschema = true;
            sessionStorage.setItem('dismissed_palschema_update', latestVer);
            showConfirm(
              t('dependencies.prompt_body_palschema', { installed: fullDeps.palschema_version || 'installed', latest: latestVer }),
              t('dependencies.prompt_title_palschema')
            ).then((confirmed) => {
              _isPromptingPalschema = false;
              if (confirmed) {
                executeInstallOrUpdate('palschema', true);
              }
            }).catch(() => { _isPromptingPalschema = false; });
          }
        }
      }).catch(() => { });
    });
    updateState({ dependencies: deps });
    renderDependencyBadges(deps);
  } catch (e) {
    console.error('Failed to check dependencies:', e);
  }
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
  const deps = getState().dependencies;
  if (!deps) return;
  const isInstalled = type === 'ue4ss' ? deps.ue4ss_installed : deps.palschema_installed;
  const needsUpdate = type === 'ue4ss' ? deps.ue4ss_needs_update : deps.palschema_needs_update;

  if (type === 'palschema' && !deps.ue4ss_installed) {
    showConfirm(t('dependencies.missing_ue4ss_for_palschema'))
      .then(async (confirmed) => {
        if (!confirmed) return;
        try {
          showToast(t('toasts.installing_dep', { dep: 'UE4SS' }), 'info');
          await installUe4ss();
          showToast(t('dependencies.up_to_date'), 'success');
          await loadDependencies();

          showToast(t('toasts.installing_dep', { dep: 'PalSchema' }), 'info');
          await installPalschema();
          showToast(t('dependencies.up_to_date'), 'success');
          await loadDependencies();
          await loadMods();
        } catch (e) {
          showToast(t('toasts.export_failed', { error: String(e) }), 'error');
        }
      });
    return;
  }

  if (!isInstalled || needsUpdate) {
    const action = isInstalled ? t('common.update') : t('common.install');
    const depName = type === 'ue4ss' ? 'UE4SS' : 'PalSchema';
    const sourceInfo = type === 'ue4ss' ? 'Okaetsu/UE4SS-Palworld' : 'Okaetsu/PalSchema';

    showConfirm(
      t('dependencies.install_source_prompt_body', { action: action.toLowerCase(), depName, sourceInfo }),
      t('dependencies.install_source_prompt_title', { action, depName })
    ).then((confirmed) => {
      if (!confirmed) return;
      executeInstallOrUpdate(type, isInstalled);
    });
  }
}

export function renderDependencyBadges(deps: import('../../types').DependencyStatus): void {
  const platformEl = document.getElementById('game-platform-badge');
  if (platformEl) {
    if (deps.game_platform && deps.game_platform !== 'Unknown') {
      platformEl.textContent = deps.game_platform;
      platformEl.style.display = '';
      platformEl.className = 'game-platform-badge ' + deps.game_platform.toLowerCase();
    } else {
      platformEl.style.display = 'none';
    }
  }

  const ue4ssEl = document.getElementById('ue4ss-badge');
  const psEl = document.getElementById('palschema-badge');
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
    const isWorkshop = deps.ue4ss_version === 'Workshop';
    const verDisplay = isWorkshop ? ' - Workshop' : (deps.ue4ss_version ? ` · ${formatDMY(deps.ue4ss_version)}` : '');
    ue4ssEl.textContent = `UE4SS${verDisplay}${ue4ssFlo}`;
    ue4ssEl.className = `dep-badge ${isWorkshop ? 'workshop' : (deps.ue4ss_needs_update ? 'warn' : 'ok')}`;
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = (deps.ue4ss_needs_update && !isWorkshop) ? 'pointer' : 'default';
    if (isWorkshop) {
      ue4ssEl.title = `UE4SS (Steam Workshop) — ${t('dependencies.managed_by_steam')}`;
    } else if (deps.ue4ss_needs_update) {
      const latestDisplay = deps.ue4ss_latest_date ? formatDMY(deps.ue4ss_latest_date) : '?';
      ue4ssEl.title = t('dependencies.update_available_ue4ss', { date: latestDisplay });
    } else {
      ue4ssEl.title = `UE4SS (experimental-palworld) — ${t('dependencies.up_to_date')}`;
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
    const isWorkshop = deps.palschema_version === 'Workshop';
    const ver = isWorkshop ? ' - Workshop' : (deps.palschema_version ? ` v${deps.palschema_version}` : deps.palschema_latest_version ? ` v${deps.palschema_latest_version}` : '');
    psEl.textContent = `PalSchema${ver}${palschemaFlo}`;
    psEl.className = `dep-badge ${isWorkshop ? 'workshop' : (deps.palschema_needs_update ? 'warn' : 'ok')}`;
    psEl.style.display = '';
    psEl.style.cursor = (deps.palschema_needs_update && !isWorkshop) ? 'pointer' : 'default';
    psEl.title = isWorkshop ? `PalSchema (Steam Workshop) — ${t('dependencies.managed_by_steam')}` : (deps.palschema_needs_update ? t('dependencies.update_available_palschema') : t('dependencies.up_to_date'));
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
