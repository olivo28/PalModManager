import { open } from '@tauri-apps/plugin-dialog';
import { getState } from '../../state';
import {
  getDependencyVault,
  installDependencyFromVault,
  installDependencyFromCustomZip,
  deleteDependencyVaultEntry,
  openDependencyVaultFolder,
  installUe4ss,
  installPalschema,
  uninstallUe4ss,
  uninstallPalschema,
} from '../../api';
import { dependencyDom } from '../../framework';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { t, updateDOMTranslations } from '../../utils/i18n';
import { loadDependencies } from '../mods/dependencies';
import { loadProfiles } from '../mods/profiles';
import { loadMods } from '../mods/loader';
import type { DependencyVaultEntry } from '../../types';

let _currentDepType: 'ue4ss' | 'palschema' = 'ue4ss';
let _isInitialized = false;

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatDate(isoStr: string): string {
  if (!isoStr) return '--';
  try {
    const d = new Date(isoStr);
    if (isNaN(d.getTime())) return isoStr.substring(0, 10);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  } catch {
    return isoStr.substring(0, 10);
  }
}

export function formatVersionDisplay(rawVer: string | null | undefined, depType: 'ue4ss' | 'palschema'): string {
  if (!rawVer) return '--';
  let v = rawVer.trim();
  const prefixes = [
    'palschema - ', 'palschema_', 'palschema-', 'palschema ', 'palschema.',
    'ue4ss - ', 'ue4ss_', 'ue4ss-', 'ue4ss ', 'ue4ss.',
    're-ue4ss - ', 're-ue4ss_', 're-ue4ss-', 're-ue4ss ',
  ];
  let changed = true;
  while (changed) {
    changed = false;
    const lower = v.toLowerCase();
    for (const p of prefixes) {
      if (lower.startsWith(p)) {
        v = v.substring(p.length).trim();
        changed = true;
        break;
      }
    }
  }

  if (v.toLowerCase() === 'workshop') return 'Steam Workshop';
  if (/^\d{1,2}\.\d{1,2}\.\d{4}$/.test(v)) return v;
  if (/^\d+\.\d+(\.\d+)?/.test(v)) return `v${v}`;
  return v;
}

export async function showDependencyModal(depType: 'ue4ss' | 'palschema'): Promise<void> {
  _currentDepType = depType;
  initDependencyModal();

  const modal = dependencyDom.el('dependency-modal');
  updateDOMTranslations(modal);
  modal.classList.add('visible');

  // Title and Icon
  const titleEl = dependencyDom.el('dep-modal-title');
  const iconEl = dependencyDom.el('dep-modal-icon');
  const isUe4ss = depType === 'ue4ss';
  const depName = isUe4ss ? 'UE4SS' : 'PalSchema';

  titleEl.textContent = t('dependencies.manager_title', { dep: depName });
  if (isUe4ss) {
    iconEl.innerHTML = '<span style="font-size:20px; line-height:1;">⚡</span>';
  } else {
    iconEl.innerHTML = '<img src="/palschema.png" alt="PalSchema" style="width:22px; height:22px; object-fit:contain; vertical-align:middle; border-radius:4px;" />';
  }

  await refreshVaultView(depType);
}

export function hideDependencyModal(): void {
  const modal = dependencyDom.elMaybe('dependency-modal');
  if (modal) {
    modal.classList.remove('visible');
  }
}

export async function refreshVaultView(depType: 'ue4ss' | 'palschema'): Promise<void> {
  const isUe4ss = depType === 'ue4ss';
  const deps = getState().dependencies;

  // 1. Current status & installed version
  const currentVerEl = dependencyDom.el('dep-current-version');
  const statusBadgeEl = dependencyDom.el('dep-status-badge');
  const remoteVerEl = dependencyDom.el('dep-remote-version');

  const isInstalled = isUe4ss ? deps?.ue4ss_installed : deps?.palschema_installed;
  const installedVer = isUe4ss ? deps?.ue4ss_version : deps?.palschema_version;
  const needsUpdate = isUe4ss ? deps?.ue4ss_needs_update : deps?.palschema_needs_update;
  const isWorkshop = isUe4ss
    ? deps?.ue4ss_install_mode === 'Workshop'
    : deps?.palschema_version === 'Workshop';

  currentVerEl.textContent = isInstalled ? (formatVersionDisplay(installedVer, depType) || t('dependencies.installed')) : t('dependencies.not_installed');
  
  if (!isInstalled) {
    statusBadgeEl.textContent = t('dependencies.not_installed');
    statusBadgeEl.style.background = 'rgba(255, 74, 74, 0.15)';
    statusBadgeEl.style.color = '#ff4a4a';
    statusBadgeEl.style.border = '1px solid rgba(255, 74, 74, 0.3)';
  } else if (isWorkshop) {
    statusBadgeEl.textContent = 'Steam Workshop';
    statusBadgeEl.style.background = 'rgba(59, 130, 246, 0.15)';
    statusBadgeEl.style.color = '#60a5fa';
    statusBadgeEl.style.border = '1px solid rgba(59, 130, 246, 0.3)';
  } else if (needsUpdate) {
    statusBadgeEl.textContent = t('dependencies.update_available');
    statusBadgeEl.style.background = 'rgba(245, 158, 11, 0.15)';
    statusBadgeEl.style.color = '#f59e0b';
    statusBadgeEl.style.border = '1px solid rgba(245, 158, 11, 0.3)';
  } else {
    statusBadgeEl.textContent = t('dependencies.up_to_date');
    statusBadgeEl.style.background = 'rgba(34, 197, 94, 0.15)';
    statusBadgeEl.style.color = '#22c55e';
    statusBadgeEl.style.border = '1px solid rgba(34, 197, 94, 0.3)';
  }

  // 2. Remote latest version
  const latestVer = isUe4ss
    ? (deps?.ue4ss_latest_date || deps?.ue4ss_latest_tag || '--')
    : (deps?.palschema_latest_version || '--');
  remoteVerEl.textContent = formatVersionDisplay(latestVer, depType);

  // 3. Vault list
  const countEl = dependencyDom.el('dep-vault-count');
  const listEl = dependencyDom.el('dep-vault-list');
  listEl.innerHTML = `<div style="text-align:center; padding:16px; color:var(--text-muted); font-size:12px;">${t('common.loading')}...</div>`;

  try {
    const entries = await getDependencyVault(depType);
    countEl.textContent = t('dependencies.vault_count', { count: entries.length });

    if (entries.length === 0) {
      listEl.innerHTML = `
        <div style="text-align:center; padding:20px; color:var(--text-muted); font-size:12px; background:var(--bg-primary); border-radius:6px; border:1px dashed var(--border);">
          <span>📦 ${t('dependencies.no_vault_entries')}</span>
          <div style="margin-top:4px; font-size:11px; opacity:0.8;">${t('dependencies.vault_empty_hint')}</div>
        </div>
      `;
      return;
    }

    listEl.innerHTML = '';
    for (const entry of entries) {
      const card = createVaultEntryCard(entry, depType);
      listEl.appendChild(card);
    }
  } catch (err) {
    console.error('Failed to load dependency vault entries:', err);
    listEl.innerHTML = `<div style="color:#ff4a4a; font-size:12px; padding:10px;">${String(err)}</div>`;
  }
}

function createVaultEntryCard(entry: DependencyVaultEntry, depType: 'ue4ss' | 'palschema'): HTMLElement {
  const card = document.createElement('div');
  card.className = 'vault-entry-card';
  card.style.cssText = `
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--bg-primary);
    border: 1px solid ${entry.isInstalled ? 'rgba(34, 197, 94, 0.4)' : 'var(--border)'};
    border-radius: 6px;
    transition: border-color 0.2s ease, background 0.2s ease;
  `;

  // Left side: Version, details, badges
  const left = document.createElement('div');
  left.style.cssText = 'display: flex; flex-direction: column; gap: 3px;';

  const titleRow = document.createElement('div');
  titleRow.style.cssText = 'display: flex; align-items: center; gap: 8px;';

  const verSpan = document.createElement('span');
  verSpan.style.cssText = 'font-weight: 700; font-size: 13px; color: var(--text-primary);';
  verSpan.textContent = formatVersionDisplay(entry.version, depType);
  titleRow.appendChild(verSpan);

  if (entry.isInstalled) {
    const activeBadge = document.createElement('span');
    activeBadge.style.cssText = 'font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 3px; background: rgba(34, 197, 94, 0.2); color: #22c55e; border: 1px solid rgba(34, 197, 94, 0.4); text-transform: uppercase;';
    activeBadge.textContent = t('dependencies.active_badge');
    titleRow.appendChild(activeBadge);
  }

  if (entry.isCustom) {
    const customBadge = document.createElement('span');
    customBadge.style.cssText = 'font-size: 10px; padding: 1px 5px; border-radius: 3px; background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.3);';
    customBadge.textContent = t('dependencies.custom_badge');
    titleRow.appendChild(customBadge);
  }

  left.appendChild(titleRow);

  const subRow = document.createElement('div');
  subRow.style.cssText = 'font-size: 11px; color: var(--text-muted); display: flex; gap: 10px;';
  subRow.innerHTML = `
    <span>${formatBytes(entry.fileSize)}</span>
    <span>•</span>
    <span>${formatDate(entry.modifiedTime)}</span>
    <span>•</span>
    <span style="max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" title="${entry.filename}">${entry.filename}</span>
  `;
  left.appendChild(subRow);

  card.appendChild(left);

  // Right side: Actions
  const actions = document.createElement('div');
  actions.style.cssText = 'display: flex; align-items: center; gap: 6px;';

  if (!entry.isInstalled) {
    const installBtn = document.createElement('button');
    installBtn.className = 'btn-secondary';
    installBtn.style.cssText = 'font-size: 11px; padding: 4px 10px; border-radius: 4px;';
    installBtn.textContent = t('dependencies.rollback_install');
    installBtn.title = t('dependencies.rollback_install_hint');
    installBtn.onclick = () => handleVaultInstall(entry, depType);
    actions.appendChild(installBtn);

    const deleteBtn = document.createElement('button');
    deleteBtn.className = 'btn-icon';
    deleteBtn.style.cssText = 'font-size: 12px; padding: 4px 6px; color: var(--text-muted); background: transparent; border: none; cursor: pointer;';
    deleteBtn.textContent = '🗑️';
    deleteBtn.title = t('common.delete');
    deleteBtn.onclick = () => handleVaultDelete(entry, depType);
    actions.appendChild(deleteBtn);
  }

  card.appendChild(actions);
  return card;
}

async function handleVaultInstall(entry: DependencyVaultEntry, depType: 'ue4ss' | 'palschema'): Promise<void> {
  const depName = depType === 'ue4ss' ? 'UE4SS' : 'PalSchema';
  const cleanVer = formatVersionDisplay(entry.version, depType);
  const confirmed = await showConfirm(
    t('dependencies.confirm_vault_install_title', { dep: depName }),
    t('dependencies.confirm_vault_install_body', { dep: depName, ver: cleanVer })
  );

  if (!confirmed) return;

  try {
    showToast(t('toasts.installing_dep', { dep: `${depName} (${cleanVer})` }), 'info');
    await installDependencyFromVault(depType, entry.filename);
    showToast(t('dependencies.up_to_date'), 'success');
    await loadProfiles();
    await loadDependencies();
    await loadMods();
    await refreshVaultView(depType);
  } catch (err) {
    console.error('Failed to install from vault:', err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

async function handleVaultDelete(entry: DependencyVaultEntry, depType: 'ue4ss' | 'palschema'): Promise<void> {
  const confirmed = await showConfirm(
    t('dependencies.confirm_vault_delete_title'),
    t('dependencies.confirm_vault_delete_body', { filename: entry.filename })
  );

  if (!confirmed) return;

  try {
    await deleteDependencyVaultEntry(depType, entry.filename);
    showToast(t('toasts.deleted_success', { item: entry.filename }), 'success');
    await refreshVaultView(depType);
  } catch (err) {
    console.error('Failed to delete vault entry:', err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export function initDependencyModal(): void {
  if (_isInitialized) return;
  _isInitialized = true;

  // Backdrop overlay click
  const modal = dependencyDom.elMaybe('dependency-modal');
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) {
      hideDependencyModal();
    }
  });

  // Close handlers
  const closeBtn = dependencyDom.elMaybe('dependency-modal-close');
  const closeX = dependencyDom.elMaybe('dependency-modal-close-x');
  closeBtn?.addEventListener('click', hideDependencyModal);
  closeX?.addEventListener('click', hideDependencyModal);

  // Download Latest button
  const dlLatestBtn = dependencyDom.elMaybe('dep-btn-download-latest');
  dlLatestBtn?.addEventListener('click', async () => {
    const depName = _currentDepType === 'ue4ss' ? 'UE4SS' : 'PalSchema';
    showToast(t('toasts.downloading_dep', { dep: depName }), 'info');
    try {
      if (_currentDepType === 'ue4ss') {
        await installUe4ss(true);
      } else {
        await installPalschema(true);
      }
      showToast(t('dependencies.up_to_date'), 'success');
      await loadProfiles();
      await loadDependencies();
      await loadMods();
      await refreshVaultView(_currentDepType);
    } catch (err) {
      console.error('Failed to download latest dependency:', err);
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });

  // Install from custom ZIP
  const customZipBtn = dependencyDom.elMaybe('dep-btn-custom-zip');
  customZipBtn?.addEventListener('click', async () => {
    const depName = _currentDepType === 'ue4ss' ? 'UE4SS' : 'PalSchema';
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
        title: t('dependencies.select_custom_zip_title', { dep: depName }),
      });

      if (!selected || typeof selected !== 'string') return;

      showToast(t('toasts.installing_dep', { dep: `${depName} (Custom ZIP)` }), 'info');
      await installDependencyFromCustomZip(_currentDepType, selected);
      showToast(t('dependencies.up_to_date'), 'success');
      await loadProfiles();
      await loadDependencies();
      await loadMods();
      await refreshVaultView(_currentDepType);
    } catch (err) {
      console.error('Failed to install from custom zip:', err);
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });

  // Open Vault Folder button
  const openVaultBtn = dependencyDom.elMaybe('dep-btn-open-vault');
  openVaultBtn?.addEventListener('click', async () => {
    try {
      await openDependencyVaultFolder(_currentDepType);
    } catch (err) {
      console.error('Failed to open vault folder:', err);
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });

  // Uninstall button
  const uninstallBtn = dependencyDom.elMaybe('dep-btn-uninstall');
  uninstallBtn?.addEventListener('click', async () => {
    const depName = _currentDepType === 'ue4ss' ? 'UE4SS' : 'PalSchema';
    const confirmed = await showConfirm(
      t('dependencies.confirm_uninstall_title', { dep: depName }),
      t('dependencies.confirm_uninstall_body', { dep: depName })
    );

    if (!confirmed) return;

    try {
      showToast(t('toasts.uninstalling_dep', { dep: depName }), 'info');
      if (_currentDepType === 'ue4ss') {
        await uninstallUe4ss();
      } else {
        await uninstallPalschema();
      }
      showToast(t('toasts.uninstalled_success', { dep: depName }), 'success');
      await loadProfiles();
      await loadDependencies();
      await loadMods();
      await refreshVaultView(_currentDepType);
    } catch (err) {
      console.error('Failed to uninstall dependency:', err);
      showToast(t('toasts.export_failed', { error: String(err) }), 'error');
    }
  });
}
