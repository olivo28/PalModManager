import { getState, updateState } from '../../state';
import { checkDependencies, installUe4ss, installPalschema, uninstallUe4ss, uninstallPalschema } from '../../api';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { loadMods } from './loader';
import { loadProfiles } from './profiles';

export async function loadDependencies(): Promise<void> {
  try {
    const deps = await checkDependencies();
    import('../../api').then(({ checkDependenciesFull }) => {
      checkDependenciesFull().then(fullDeps => {
        updateState({ dependencies: fullDeps });
        renderDependencyBadges(fullDeps);
      }).catch(() => { });
    });
    updateState({ dependencies: deps });
    renderDependencyBadges(deps);
  } catch (e) {
    console.error('Failed to check dependencies:', e);
  }
}

export function handleDepBadgeClick(type: 'ue4ss' | 'palschema'): void {
  const deps = getState().dependencies;
  if (!deps) return;
  const isInstalled = type === 'ue4ss' ? deps.ue4ss_installed : deps.palschema_installed;
  const needsUpdate = type === 'ue4ss' ? deps.ue4ss_needs_update : deps.palschema_needs_update;

  if (type === 'palschema' && !deps.ue4ss_installed) {
    showConfirm('UE4SS is not detected. PalSchema requires UE4SS to operate. Would you like to install UE4SS first?')
      .then(async (confirmed) => {
        if (!confirmed) {
          showToast('PalSchema installation cancelled: UE4SS dependency missing.', 'info');
          return;
        }
        try {
          showToast('Installing UE4SS from GitHub (Okaetsu/UE4SS-Palworld)...', 'info');
          const ue4ssMsg = await installUe4ss();
          showToast(ue4ssMsg, 'success');
          await loadDependencies();

          const action = isInstalled ? 'Updating' : 'Installing';
          showToast(`${action} PalSchema from GitHub (Okaetsu/PalSchema)...`, 'info');
          const psMsg = await installPalschema();
          showToast(psMsg, 'success');
          await loadDependencies();
          await loadMods();
        } catch (e) {
          showToast('Failed: ' + e, 'error');
        }
      });
    return;
  }

  if (!isInstalled || needsUpdate) {
    const action = isInstalled ? 'Updating' : 'Installing';
    const sourceInfo = type === 'ue4ss' ? 'UE4SS from GitHub (Okaetsu/UE4SS-Palworld)' : 'PalSchema from GitHub (Okaetsu/PalSchema)';
    showToast(`${action} ${sourceInfo}...`, 'info');
    const promise = type === 'ue4ss' ? installUe4ss() : installPalschema();
    promise.then(async (msg) => {
      showToast(msg, 'success');
      await loadProfiles();
      await loadDependencies();
      await loadMods();
    }).catch(e => showToast('Failed: ' + e, 'error'));
  }
}

export function renderDependencyBadges(deps: import('../types').DependencyStatus): void {
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
      ue4ssEl.title = `UE4SS (Steam Workshop) — Managed by Steam`;
    } else if (deps.ue4ss_needs_update) {
      const latestDisplay = deps.ue4ss_latest_date ? formatDMY(deps.ue4ss_latest_date) : '?';
      ue4ssEl.title = `Update available (${latestDisplay}) — click to update`;
    } else {
      ue4ssEl.title = `UE4SS (experimental-palworld) — Up to date`;
    }
  } else {
    ue4ssEl.textContent = 'UE4SS ✕';
    ue4ssEl.className = 'dep-badge missing';
    ue4ssEl.style.display = '';
    ue4ssEl.style.cursor = 'pointer';
    ue4ssEl.title = 'Not installed — click to install';
  }

  // PalSchema
  if (deps.palschema_installed) {
    const isWorkshop = deps.palschema_version === 'Workshop';
    const ver = isWorkshop ? ' - Workshop' : (deps.palschema_version ? ` v${deps.palschema_version}` : deps.palschema_latest_version ? ` v${deps.palschema_latest_version}` : '');
    psEl.textContent = `PalSchema${ver}${palschemaFlo}`;
    psEl.className = `dep-badge ${isWorkshop ? 'workshop' : (deps.palschema_needs_update ? 'warn' : 'ok')}`;
    psEl.style.display = '';
    psEl.style.cursor = (deps.palschema_needs_update && !isWorkshop) ? 'pointer' : 'default';
    psEl.title = isWorkshop ? 'PalSchema (Steam Workshop) — Managed by Steam' : (deps.palschema_needs_update ? 'Click to update' : 'Up to date');
  } else {
    psEl.textContent = 'PalSchema ✕';
    psEl.className = 'dep-badge missing';
    psEl.style.display = '';
    psEl.style.cursor = 'pointer';
    psEl.title = 'Not installed — click to install';
  }

  ue4ssEl.onclick = () => handleDepBadgeClick('ue4ss');
  psEl.onclick = () => handleDepBadgeClick('palschema');
}
