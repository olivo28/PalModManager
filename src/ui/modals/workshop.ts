import { getWorkshopState, setWorkshopGlobalEnabled, activateWorkshopMod, deactivateWorkshopMod } from '../../api';
import { showToast } from '../toast';
import { escapeHtml } from '../../utils/helpers';

export async function openWorkshopModal(): Promise<void> {
  const modal = document.getElementById('workshop-modal')!;
  modal.classList.add('visible');

  const closeX = document.getElementById('workshop-modal-close-x')!;
  const closeBtn = document.getElementById('workshop-modal-close')!;

  const close = () => {
    modal.classList.remove('visible');
  };

  closeX.onclick = close;
  closeBtn.onclick = close;

  await refreshWorkshopUI();
}

export async function refreshWorkshopUI(): Promise<void> {
  const masterToggle = document.getElementById('workshop-master-toggle') as HTMLInputElement;
  const listContainer = document.getElementById('workshop-list-container')!;

  try {
    const wState = await getWorkshopState();

    masterToggle.checked = wState.globalEnabled;
    masterToggle.onchange = async () => {
      showToast(masterToggle.checked ? 'Enabling Workshop Mods...' : 'Disabling Workshop Mods...', 'info');
      await setWorkshopGlobalEnabled(masterToggle.checked);
      await refreshWorkshopUI();
      const { loadMods } = await import('../modsView');
      await loadMods();
      showToast('Workshop state updated', 'success');
    };

    if (wState.mods.length === 0) {
      listContainer.innerHTML = `<div style="text-align:center; padding: 24px; color:var(--text-muted);">No subscribed Workshop mods found. Subscribing in Steam will list them here.</div>`;
      return;
    }

    listContainer.innerHTML = wState.mods.map((m: any) => {
      const thumb = m.thumbnailPath ? `<img src="${escapeHtml(m.thumbnailPath)}" style="width:36px; height:36px; border-radius:4px; object-fit:cover;" />` : `<div style="width:36px; height:36px; border-radius:4px; background:var(--bg-tertiary); display:flex; align-items:center; justify-content:center; font-size:16px;">📦</div>`;
      const isDepMissing = m.dependencies.some((dep: string) => !wState.activeModList.includes(dep));
      const depWarning = isDepMissing ? `<div style="color:#ff4a4a; font-size:10px; margin-top:2px;">Missing dependencies: ${escapeHtml(m.dependencies.join(', '))}</div>` : '';

      const badgeText = m.isFramework ? 'FRAMEWORK' : 'WORKSHOP';
      const badgeStyle = `font-size: 8px; font-weight: bold; background: ${m.isFramework ? 'rgba(0,188,255,0.15)' : 'rgba(255, 157, 0, 0.15)'}; color: ${m.isFramework ? '#00bcff' : '#ff9d00'}; border: 1px solid ${m.isFramework ? 'rgba(0,188,255,0.3)' : 'rgba(255, 157, 0, 0.3)'}; padding: 1px 4px; border-radius: 3px;`;

      const toggleDisabled = m.isFramework ? 'disabled' : '';
      const toggleChecked = m.isActive ? 'checked' : '';
      const toggleSwitch = `<label class="toggle-switch ${toggleDisabled}">
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
            <div style="font-size:10px; color:var(--text-muted);">Version ${escapeHtml(m.version)} by ${escapeHtml(m.author)}</div>
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

        target.disabled = true;
        showToast(checked ? 'Activating Workshop mod...' : 'Deactivating Workshop mod...', 'info');
        try {
          if (checked) {
            await activateWorkshopMod(pkgName);
          } else {
            await deactivateWorkshopMod(pkgName);
          }
          showToast(checked ? 'Activated successfully' : 'Deactivated successfully', 'success');
        } catch (err) {
          target.checked = !checked;
          showToast('Failed to toggle mod: ' + err, 'error');
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
