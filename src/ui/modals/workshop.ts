import { convertFileSrc } from '@tauri-apps/api/core';
import { getWorkshopState, setWorkshopGlobalEnabled } from '../../api';
import { getState } from '../../state';
import {
  checkWorkshopDependencies,
  isWorkshopDependencyMode,
  findInstalledNormalMod,
  handleWorkshopCardToggle,
} from '../mods/library/workshop';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { mainDom } from '../../framework';

export async function openWorkshopModal(): Promise<void> {
  const modal = mainDom.el('workshop-modal');
  modal.classList.add('visible');

  const closeX = mainDom.el('workshop-modal-close-x');
  const closeBtn = mainDom.el('workshop-modal-close');

  const close = () => {
    modal.classList.remove('visible');
  };

  closeX.onclick = close;
  closeBtn.onclick = close;

  await refreshWorkshopUI();
}

export async function refreshWorkshopUI(): Promise<void> {
  const masterToggle = mainDom.el('workshop-master-toggle');
  const listContainer = mainDom.el('workshop-list-container');

  try {
    const wState = await getWorkshopState();
    const isWorkshopMode = isWorkshopDependencyMode();

    masterToggle.checked = wState.globalEnabled;
    masterToggle.onchange = async () => {
      showToast(masterToggle.checked ? t('toasts.workshop_enabling') : t('toasts.workshop_disabling'), 'info');
      await setWorkshopGlobalEnabled(masterToggle.checked);
      await refreshWorkshopUI();
      const { loadMods } = await import('../modsView');
      await loadMods();
      showToast(t('toasts.workshop_state_updated'), 'success');
    };

    if (wState.mods.length === 0) {
      listContainer.innerHTML = `<div style="text-align:center; padding: 24px; color:var(--text-muted);">${escapeHtml(t('library.empty_workshop'))}</div>`;
      return;
    }

    listContainer.innerHTML = wState.mods.map((m: any) => {
      const thumb = m.thumbnailPath ? `<img src="${convertFileSrc(m.thumbnailPath)}" style="width:36px; height:36px; border-radius:4px; object-fit:cover;" />` : `<div style="width:36px; height:36px; border-radius:4px; background:var(--bg-tertiary); display:flex; align-items:center; justify-content:center; font-size:16px;">📦</div>`;
      const { isMissing, missingDeps } = checkWorkshopDependencies(m.dependencies, wState.activeModList);
      const depWarning = isMissing ? `<div style="color:#ff4a4a; font-size:10px; margin-top:2px;">${escapeHtml(t('library.missing_deps_warning', { deps: missingDeps.join(', ') }))}</div>` : '';

      const depsState = getState().dependencies;
      const isUe4ssFramework = m.packageName === 'UE4SSExperimentalPW' || m.workshopId === 3625223587;
      const isPalSchemaFramework = m.packageName === 'PalSchema' || m.workshopId === 3625280368;
      const isManagedByPmm = m.isFramework && ((isUe4ssFramework && !!depsState?.ue4ss_installed) || (isPalSchemaFramework && !!depsState?.palschema_installed));
      const managedVersion = isUe4ssFramework ? depsState?.ue4ss_version : (isPalSchemaFramework ? depsState?.palschema_version : null);

      const normalMod = !isWorkshopMode ? findInstalledNormalMod(m.packageName, m.modName, m.workshopId) : null;
      const isEnabledNormal = normalMod?.enabled ?? false;

      const badgeText = m.isFramework ? 'FRAMEWORK' : 'WORKSHOP';
      const badgeStyle = `font-size: 8px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.15)' : 'rgba(255, 157, 0, 0.15)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.3)' : 'rgba(255, 157, 0, 0.3)'}; padding: 1px 4px; border-radius: 3px;`;

      const toggleDisabled = isManagedByPmm || m.isFramework ? 'disabled' : '';
      const toggleChecked = (isWorkshopMode ? m.isActive : isEnabledNormal) ? 'checked' : '';
      const toggleSwitch = isManagedByPmm
        ? `<span style="font-size: 10px; color: #38ef7d; font-weight: 600; padding: 3px 8px; background: rgba(56, 239, 125, 0.1); border: 1px solid rgba(56, 239, 125, 0.25); border-radius: 4px; white-space: nowrap;">✓ ${escapeHtml(t('library.managed_by_pmm', { version: managedVersion ? `v${managedVersion}` : '' }))}</span>`
        : `<label class="toggle-switch ${toggleDisabled}">
          <input type="checkbox" class="workshop-item-toggle" data-package="${escapeHtml(m.packageName)}" ${toggleChecked} ${toggleDisabled} />
          <span class="toggle-slider"></span>
        </label>`;

      return `
        <div style="display:flex; align-items:center; gap:12px; background:var(--bg-tertiary); border:1px solid var(--border); padding:8px 12px; border-radius:6px;">
          ${thumb}
          <div style="flex:1;">
            <div style="display:flex; align-items:center; gap:8px;">
              <span style="font-weight:600; font-size:12px; color:var(--text-primary);">${escapeHtml(m.modName)}</span>
              <span style="${badgeStyle}">${badgeText}</span>
            </div>
            <div style="font-size:10px; color:var(--text-muted);">${escapeHtml(t('common.version'))} ${escapeHtml(normalMod?.version || m.version)} · ${escapeHtml(t('detail.author_label'))} ${escapeHtml(m.author)}</div>
            ${depWarning}
          </div>
          <div>
            ${toggleSwitch}
          </div>
        </div>
      `;
    }).join('');

    listContainer.querySelectorAll('.workshop-item-toggle').forEach(chk => {
      chk.addEventListener('change', async (e) => {
        const target = e.currentTarget as HTMLInputElement;
        const pkgName = target.dataset.package!;
        const checked = target.checked;
        const targetMod = wState.mods.find((m: any) => m.packageName === pkgName);
        if (!targetMod) return;

        target.disabled = true;
        showToast(checked ? t('toasts.workshop_activating') : t('toasts.workshop_deactivating'), 'info');
        try {
          const normalMod = !isWorkshopMode ? findInstalledNormalMod(targetMod.packageName, targetMod.modName, targetMod.workshopId) : null;
          if (!isWorkshopMode && !normalMod && checked) {
            const modal = mainDom.el('workshop-modal');
            modal.classList.remove('visible');
          }
          await handleWorkshopCardToggle(targetMod, !checked);
        } catch (err) {
          target.checked = !checked;
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        } finally {
          target.disabled = false;
          await refreshWorkshopUI();
          const { loadMods } = await import('../modsView');
          await loadMods();
        }
      });
    });

  } catch (err) {
    listContainer.innerHTML = `<div style="color:#ff4a4a; padding:12px; text-align:center;">Failed to load Workshop state: ${escapeHtml(String(err))}</div>`;
  }
}
