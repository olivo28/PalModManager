import { getState } from '../../../state';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import { escapeHtml } from '../../../utils/helpers';
import { convertFileSrc } from '@tauri-apps/api/core';
import { getWorkshopState, setWorkshopGlobalEnabled, openUrl } from '../../../api';
import { _activeLibrarySubTab, _librarySearchQuery, _libraryFilterStatus, _librarySortBy } from './state';

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
  const platform = state.dependencies?.game_platform?.toLowerCase();
  const isXbox = platform === 'xbox' || platform === 'gamepass' || platform === 'wingdk';

  if (isXbox) {
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

export function checkWorkshopDependencies(
  dependencies: string[] | undefined,
  activeModList: string[]
): {
  isMissing: boolean;
  missingDeps: string[];
} {
  if (!dependencies || dependencies.length === 0) {
    return { isMissing: false, missingDeps: [] };
  }

  const depsState = getState().dependencies;
  const hasUe4ss = !!depsState?.ue4ss_installed;
  const hasPalSchema = !!depsState?.palschema_installed;
  const allMods = getState().allMods || [];

  const missing: string[] = [];

  for (const dep of dependencies) {
    const depTrimmed = dep.trim();
    if (!depTrimmed) continue;
    const depLower = depTrimmed.toLowerCase();

    // 1. Reconcile with PMM's managed UE4SS
    if (depLower === 'ue4ssexperimentalpw' || depLower === 'ue4ss' || depLower.includes('ue4ss')) {
      if (hasUe4ss || activeModList.some(m => m.toLowerCase() === depLower)) {
        continue;
      }
      missing.push(depTrimmed);
      continue;
    }

    // 2. Reconcile with PMM's managed PalSchema
    if (depLower === 'palschema' || depLower === 'palschemamod' || depLower.includes('palschema')) {
      if (hasPalSchema || activeModList.some(m => m.toLowerCase() === depLower)) {
        continue;
      }
      missing.push(depTrimmed);
      continue;
    }

    // 3. Reconcile with other workshop or local mods
    const inActiveWorkshop = activeModList.some(m => m.toLowerCase() === depLower);
    const inActiveLocal = allMods.some(m => m.enabled && (m.name.toLowerCase() === depLower || m.id.toLowerCase() === depLower));

    if (!inActiveWorkshop && !inActiveLocal) {
      missing.push(depTrimmed);
    }
  }

  return {
    isMissing: missing.length > 0,
    missingDeps: missing,
  };
}

export function isWorkshopDependencyMode(): boolean {
  const state = getState();
  const deps = state.dependencies;
  const isUe4ssWs = deps?.ue4ss_install_mode === 'Workshop' || deps?.ue4ss_version === 'Workshop';
  const isPsWs = deps?.palschema_install_mode === 'Workshop' || deps?.palschema_version === 'Workshop';
  const isProfileWs = state.currentProfile?.dependency_mode === 'workshop';
  return isUe4ssWs || isPsWs || isProfileWs;
}

export function findInstalledNormalMod(packageName: string, modName?: string, workshopId?: number) {
  const allMods = getState().allMods || [];
  const pkgLower = packageName.toLowerCase();
  const nameLower = modName ? modName.toLowerCase() : '';
  const widStr = workshopId ? String(workshopId) : '';

  return allMods.find(mod => {
    const mId = mod.id.toLowerCase();
    const mName = mod.name.toLowerCase();
    const origName = mod.originalName ? mod.originalName.toLowerCase() : '';
    if (mId === pkgLower || mName === pkgLower || origName === pkgLower) return true;
    if (nameLower && (mId === nameLower || mName === nameLower || origName === nameLower)) return true;
    if (mod.nexusSummary) {
      if (widStr && mod.nexusSummary.includes(`Workshop ID: ${widStr}`)) return true;
      if (mod.nexusSummary.includes(`Package Name: ${packageName}`)) return true;
    }
    return false;
  });
}

export async function openWorkshopInstallModal(
  m: {
    packageName: string;
    modName: string;
    version?: string;
    author?: string;
    thumbnailPath?: string;
    workshopId?: number;
    description?: string;
  }
): Promise<void> {
  const { prepareWorkshopUpdateZip } = await import('../../../api');
  const { openInstallModalForZip } = await import('../../modals/installer');

  showToast(t('installer.status_analyzing'), 'info');
  const zipPath = await prepareWorkshopUpdateZip(m.packageName);
  await openInstallModalForZip(zipPath, m.modName, undefined, m.version, {
    author: m.author,
    description: m.description,
    thumbnailPath: m.thumbnailPath,
    pictureUrl: m.thumbnailPath,
    workshopId: m.workshopId,
  });
}

export async function handleWorkshopCardToggle(
  m: {
    packageName: string;
    modName: string;
    workshopId: number;
    version?: string;
    isFramework?: boolean;
    author?: string;
    thumbnailPath?: string;
    description?: string;
  },
  isActive: boolean
): Promise<void> {
  if (isWorkshopDependencyMode()) {
    const { activateWorkshopMod, deactivateWorkshopMod } = await import('../../../api');
    if (!isActive) {
      await activateWorkshopMod(m.packageName);
    } else {
      await deactivateWorkshopMod(m.packageName);
    }
    showToast(!isActive ? t('toasts.workshop_activated') : t('toasts.workshop_deactivated'), 'success');
  } else {
    const normalMod = findInstalledNormalMod(m.packageName, m.modName, m.workshopId);
    if (normalMod) {
      const { enableMod, disableMod } = await import('../../../api');
      if (normalMod.enabled) {
        await disableMod(normalMod.id);
        showToast(t('toasts.workshop_deactivated'), 'success');
      } else {
        await enableMod(normalMod.id);
        showToast(t('toasts.workshop_activated'), 'success');
      }
    } else {
      await openWorkshopInstallModal(m);
    }
  }
}

export async function handleWorkshopCardUpdate(
  m: {
    packageName: string;
    modName: string;
    version?: string;
    author?: string;
    thumbnailPath?: string;
    workshopId?: number;
    description?: string;
  }
): Promise<void> {
  if (isWorkshopDependencyMode()) {
    const { activateWorkshopMod } = await import('../../../api');
    await activateWorkshopMod(m.packageName);
    showToast(t('toasts.mod_updated', { name: m.packageName }), 'success');
  } else {
    await openWorkshopInstallModal(m);
  }
}

export async function renderWorkshopLibraryTab(container: HTMLElement): Promise<void> {
  const masterToggleWrap = libraryDom.elMaybe('library-workshop-master-wrap');
  const wsCheckUpdatesBtn = libraryDom.elMaybe('workshop-check-updates-btn');
  const bulkBar = libraryDom.elMaybe('library-bulk-actions-bar');

  if (masterToggleWrap) masterToggleWrap.style.display = 'flex';
  if (wsCheckUpdatesBtn) wsCheckUpdatesBtn.style.display = 'inline-flex';
  if (bulkBar) bulkBar.style.display = 'none';

  try {
    const wState = await getWorkshopState();

    const masterToggle = libraryDom.elMaybe('library-workshop-master-toggle');
    if (masterToggle) {
      masterToggle.checked = wState.globalEnabled;
      masterToggle.onchange = async () => {
        showToast(masterToggle.checked ? t('toasts.workshop_enabling') : t('toasts.workshop_disabling'), 'info');
        await setWorkshopGlobalEnabled(masterToggle.checked);
        const { renderLibraryView } = await import('./render');
        await renderLibraryView();
        const { loadMods } = await import('../../modsView');
        await loadMods();
        showToast(t('toasts.workshop_state_updated'), 'success');
      };
    }

    let mods = wState.mods;
    if (_librarySearchQuery) {
      mods = mods.filter((m: any) => m.modName.toLowerCase().includes(_librarySearchQuery) || m.author.toLowerCase().includes(_librarySearchQuery));
    }

    const isWorkshopMode = isWorkshopDependencyMode();

    if (_libraryFilterStatus === 'installed') {
      mods = mods.filter((m: any) => {
        if (isWorkshopMode) return m.isInstalled || wState.activeModList.includes(m.packageName);
        return !!findInstalledNormalMod(m.packageName, m.modName, m.workshopId);
      });
    } else if (_libraryFilterStatus === 'not_installed') {
      mods = mods.filter((m: any) => {
        if (isWorkshopMode) return !m.isInstalled && !wState.activeModList.includes(m.packageName);
        return !findInstalledNormalMod(m.packageName, m.modName, m.workshopId);
      });
    } else if (_libraryFilterStatus === 'updates') {
      mods = mods.filter((m: any) => {
        if (isWorkshopMode) return m.hasPendingUpdate || (m.isInstalled && m.installedVersion && m.installedVersion !== m.version);
        const normalMod = findInstalledNormalMod(m.packageName, m.modName, m.workshopId);
        return !!(normalMod && m.version && normalMod.version && m.version !== normalMod.version && m.version !== '1.0');
      });
    }

    mods.sort((a: any, b: any) => {
      const isInstalledA = isWorkshopMode
        ? (a.isInstalled || wState.activeModList.includes(a.packageName))
        : !!findInstalledNormalMod(a.packageName, a.modName, a.workshopId);
      const isInstalledB = isWorkshopMode
        ? (b.isInstalled || wState.activeModList.includes(b.packageName))
        : !!findInstalledNormalMod(b.packageName, b.modName, b.workshopId);

      switch (_librarySortBy) {
        case 'name:asc':
          return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
        case 'name:desc':
          return (b.modName || '').localeCompare(a.modName || '', undefined, { sensitivity: 'base', numeric: true });
        case 'installed:first':
          if (isInstalledA !== isInstalledB) return isInstalledB ? 1 : -1;
          return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
        case 'not_installed:first':
          if (isInstalledA !== isInstalledB) return isInstalledA ? 1 : -1;
          return (a.modName || '').localeCompare(b.modName || '', undefined, { sensitivity: 'base', numeric: true });
        case 'date:desc':
          return (b.workshopId || 0) - (a.workshopId || 0);
        default:
          return (a.modName || '').localeCompare(b.modName || '');
      }
    });

    if (mods.length === 0) {
      container.innerHTML = `<div id="library-empty">${escapeHtml(t('library.empty_workshop'))}</div>`;
      return;
    }

    container.innerHTML = mods.map((m: any) => {
      let typeClass = 'ue4ss';
      let typeLabel = 'U';
      if (m.installType === 'palSchemaMod') {
        typeClass = 'palschema';
        typeLabel = 'PS';
      } else if (m.installType === 'pakMod') {
        typeClass = 'pak';
        typeLabel = 'PAK';
      }
      const thumb = m.thumbnailPath 
        ? `<div style="width:100%;height:100%;position:relative;"><img src="${convertFileSrc(m.thumbnailPath)}" style="width:100%;height:100%;object-fit:cover;" onerror="this.onerror=null; this.style.display='none'; if (this.nextElementSibling) this.nextElementSibling.style.display='flex';" /><div class="mod-card-image-placeholder ${typeClass}" style="display:none;width:100%;height:100%;align-items:center;justify-content:center;font-weight:bold;font-size:24px;color:#fff;">${typeLabel}</div></div>` 
        : `<div class="mod-card-image-placeholder ${typeClass}" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;font-weight:bold;font-size:24px;color:#fff;">${typeLabel}</div>`;
      const { isMissing, missingDeps } = checkWorkshopDependencies(m.dependencies, wState.activeModList);
      const depWarning = isMissing ? `<div style="color:#ff4a4a; font-size:10px; margin-top:2px; text-align:center;">${escapeHtml(t('library.missing_deps_warning', { deps: missingDeps.join(', ') }))}</div>` : '';

      const depsState = getState().dependencies;
      const isUe4ssFramework = m.packageName === 'UE4SSExperimentalPW' || m.workshopId === 3625223587;
      const isPalSchemaFramework = m.packageName === 'PalSchema' || m.workshopId === 3625280368;
      const isManagedByPmm = m.isFramework && ((isUe4ssFramework && !!depsState?.ue4ss_installed) || (isPalSchemaFramework && !!depsState?.palschema_installed));
      const managedVersion = isUe4ssFramework ? depsState?.ue4ss_version : (isPalSchemaFramework ? depsState?.palschema_version : null);

      const normalMod = !isWorkshopMode ? findInstalledNormalMod(m.packageName, m.modName, m.workshopId) : null;
      const isInstalledNormal = !!normalMod;
      const isEnabledNormal = normalMod?.enabled ?? false;

      const isNew = isWorkshopModNew(m.packageName);
      const newBadge = isNew
        ? `<span style="font-size: 7.5px; font-weight: 700; background: linear-gradient(135deg, #00bcff, #38ef7d); color: #000; padding: 2px 5px; border-radius: 8px; box-shadow: 0 0 6px rgba(0,188,255,0.5); letter-spacing: 0.3px; white-space: nowrap;">✨ ${escapeHtml(t('card.badge_new'))}</span>`
        : '';

      const badgeText = m.isFramework ? 'FRAMEWORK' : t('card.badge_workshop');
      const badgeStyle = `font-size: 7.5px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.1)' : 'rgba(255, 157, 0, 0.1)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.2)' : 'rgba(255, 157, 0, 0.2)'}; padding: 2px 4px; border-radius: 3px; white-space: nowrap;`;

      const hasUpdate = isWorkshopMode
        ? (m.hasPendingUpdate || (m.isInstalled && m.installedVersion && m.installedVersion !== m.version))
        : !!(normalMod && m.version && normalMod.version && m.version !== normalMod.version && m.version !== '1.0');

      const updateBadge = hasUpdate
        ? `<span style="font-size: 7.5px; font-weight: 700; background: rgba(255, 157, 0, 0.15); color: #ff9d00; border: 1px solid rgba(255, 157, 0, 0.4); padding: 2px 5px; border-radius: 3px; letter-spacing: 0.2px; white-space: nowrap;">▲ ${escapeHtml(t('card.badge_update_available', { version: m.version }))}</span>`
        : '';

      let versionTextHtml = '';
      if (isManagedByPmm) {
        versionTextHtml = `
          <div style="font-size:10px; color:var(--text-muted); text-align:center; display:flex; flex-direction:column; gap:2px;">
            <div>${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} &bull; ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)}</div>
            <div style="color:#38ef7d; font-weight:700; font-size:10.5px;">✓ ${escapeHtml(t('library.managed_by_pmm', { version: managedVersion ? `v${managedVersion}` : '' }))}</div>
          </div>`;
      } else if (!isWorkshopMode) {
        if (isInstalledNormal) {
          if (hasUpdate) {
            versionTextHtml = `
              <div style="font-size:10px; color:var(--text-muted); text-align:center; display:flex; flex-direction:column; gap:3px;">
                <div>${escapeHtml(t('detail.installed_label'))}: <b style="color:var(--text-primary);">v${escapeHtml(normalMod!.version || '1.0.0')}</b> &bull; Workshop: <b style="color:#00bcff;">v${escapeHtml(m.version)}</b></div>
                <div style="font-size:9px; color:var(--text-muted);">${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})</div>
              </div>`;
          } else {
            versionTextHtml = `
              <div style="font-size:10px; color:var(--text-muted); text-align:center;">
                ${escapeHtml(t('common.version'))} ${escapeHtml(normalMod!.version || m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} <span style="color:#38ef7d; font-weight:600; margin-left:2px;">(${escapeHtml(t('common.installed'))} ✓)</span>
              </div>`;
          }
        } else {
          versionTextHtml = `
            <div style="font-size:10px; color:var(--text-muted); text-align:center;">
              ${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})
            </div>`;
        }
      } else if (m.isInstalled) {
        if (hasUpdate) {
          versionTextHtml = `
            <div style="font-size:10px; color:var(--text-muted); text-align:center; display:flex; flex-direction:column; gap:3px;">
              <div>${escapeHtml(t('detail.installed_label'))}: <b style="color:var(--text-primary);">v${escapeHtml(m.installedVersion || '1.0.0')}</b> &bull; Workshop: <b style="color:#00bcff;">v${escapeHtml(m.version)}</b></div>
              <div style="font-size:9px; color:var(--text-muted);">${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})</div>
            </div>`;
        } else {
          versionTextHtml = `
            <div style="font-size:10px; color:var(--text-muted); text-align:center;">
              ${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} <span style="color:#38ef7d; font-weight:600; margin-left:2px;">(${escapeHtml(t('common.installed'))} ✓)</span>
            </div>`;
        }
      } else {
        versionTextHtml = `
          <div style="font-size:10px; color:var(--text-muted); text-align:center;">
            ${escapeHtml(t('common.version'))} ${escapeHtml(m.version)} ${escapeHtml(t('common.author'))}: ${escapeHtml(m.author)} (ID: ${m.workshopId})
          </div>`;
      }

      const isActiveInCurrentMode = isWorkshopMode ? m.isActive : isEnabledNormal;
      const toggleBtnText = isManagedByPmm
        ? `✓ ${escapeHtml(t('library.managed_by_pmm_btn'))}`
        : (isActiveInCurrentMode ? t('common.disable') : t('common.enable'));
      const toggleBtnClass = isManagedByPmm
        ? 'btn-secondary btn-sm'
        : (isActiveInCurrentMode ? 'btn-action btn-action-danger' : 'btn-primary btn-sm');
      const frameworkBtnExtra = isManagedByPmm
        ? 'style="flex:1;padding:6px;font-size:10px;cursor:default;background:rgba(56, 239, 125, 0.12);color:#38ef7d;border:1px solid rgba(56, 239, 125, 0.3);opacity:0.95;"'
        : (m.isFramework ? 'disabled style="flex:1;padding:6px;font-size:10px;cursor:not-allowed;opacity:0.5;"' : 'style="flex:1;padding:6px;font-size:10px;cursor:pointer;"');

      const updateBtn = (hasUpdate && (isWorkshopMode ? m.isActive : isInstalledNormal))
        ? `<button class="workshop-item-update-btn btn-primary btn-sm" data-package="${escapeHtml(m.packageName)}" style="padding:6px;font-size:10px;cursor:pointer;background:rgba(255, 157, 0, 0.2);color:#ff9d00;border:1px solid rgba(255, 157, 0, 0.4);" title="${escapeHtml(t('library.btn_update_to', { version: m.version }))}">▲ ${escapeHtml(t('library.btn_update_to', { version: m.version }))}</button>`
        : '';

      return `
        <div class="mod-card library-card workshop-card" data-package="${escapeHtml(m.packageName)}" style="position:relative;padding:12px;display:flex;flex-direction:column;gap:8px;border:1px solid var(--border);border-radius:var(--card-radius);background:var(--bg-secondary);">
          <div style="position:absolute;top:8px;right:8px;z-index:5;display:flex;align-items:center;gap:3px;max-width:calc(100% - 16px);flex-wrap:wrap;justify-content:flex-end;">
            <span style="${badgeStyle}">${badgeText}</span>
            ${updateBadge}
            ${newBadge}
          </div>
          <div style="padding-top:16px;display:flex;flex-direction:column;gap:8px;height:100%;justify-content:space-between;min-height:160px;">
            <div class="library-card-img-container" style="width:100%;height:80px;border-radius:4px;overflow:hidden;background:var(--bg-primary);display:flex;align-items:center;justify-content:center;margin-top:6px;">
              ${thumb}
            </div>
            <div class="mod-card-name" style="font-weight:600;font-size:12px;text-align:center;word-break:break-word;line-height:1.3;flex:1;min-height:36px;display:flex;align-items:center;justify-content:center;margin-top:4px;">
              ${escapeHtml(m.modName)}
            </div>
            ${versionTextHtml}
            ${depWarning}
            ${updateBtn ? `<div style="display:flex;flex-direction:column;margin-top:2px;">${updateBtn}</div>` : ''}
            <div style="display:flex;gap:6px;margin-top:4px;z-index:4;">
              <button class="workshop-item-toggle-btn ${toggleBtnClass}" data-package="${escapeHtml(m.packageName)}" data-active="${isActiveInCurrentMode}" ${isManagedByPmm || m.isFramework ? 'disabled' : ''} ${frameworkBtnExtra}>
                ${toggleBtnText}
              </button>
              <button class="workshop-item-folder-btn btn-secondary btn-sm" data-path="${escapeHtml(wState.workshopRoot + '/' + m.workshopId)}" style="padding:6px 8px;font-size:10px;cursor:pointer;" title="${escapeHtml(t('library.workshop_open_folder_title'))}">
                📁 ${escapeHtml(t('common.folder'))}
              </button>
            </div>
          </div>
        </div>
      `;
    }).join('');

    container.querySelectorAll('.workshop-item-update-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const target = e.currentTarget as HTMLButtonElement;
        const pkgName = target.dataset.package!;
        const targetMod = mods.find((m: any) => m.packageName === pkgName);
        if (!targetMod) return;
        target.disabled = true;
        showToast(t('toasts.preparing_workshop_update'), 'info');
        try {
          await handleWorkshopCardUpdate(targetMod);
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        } finally {
          target.disabled = false;
          const { renderLibraryView } = await import('./render');
          await renderLibraryView();
          const { loadMods } = await import('../../modsView');
          await loadMods();
        }
      });
    });

    container.querySelectorAll('.workshop-item-toggle-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        const target = e.currentTarget as HTMLButtonElement;
        const pkgName = target.dataset.package!;
        const targetMod = mods.find((m: any) => m.packageName === pkgName);
        if (!targetMod) return;
        const isActive = target.dataset.active === 'true';

        target.disabled = true;
        showToast(!isActive ? t('toasts.workshop_activating') : t('toasts.workshop_deactivating'), 'info');
        try {
          await handleWorkshopCardToggle(targetMod, isActive);
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        } finally {
          target.disabled = false;
          const { renderLibraryView } = await import('./render');
          await renderLibraryView();
          const { loadMods } = await import('../../modsView');
          await loadMods();
        }
      });
    });

    container.querySelectorAll('.workshop-item-folder-btn').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const target = e.currentTarget as HTMLButtonElement;
        const path = target.dataset.path!;
        try {
          await openUrl(path);
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      });
    });

  } catch (err) {
    container.innerHTML = `<div style="color:#ff4a4a; padding:12px; text-align:center;">Failed to load Workshop state: ${escapeHtml(String(err))}</div>`;
  }
}


