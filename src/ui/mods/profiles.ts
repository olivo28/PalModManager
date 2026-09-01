import { getState, updateState } from '../../state';
import { renderModsView } from './renderer';
import { loadMods } from './loader';
import { loadDependencies } from './dependencies';
import { loadLibrary } from './library';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { switchProfile, clearProfile, getProfiles, getCurrentProfile, getSettings } from '../../api';
import { suppressWatcherRefresh } from '../editor/watcher';
import { mainDom, editorDom, bus } from '../../framework';

export async function loadProfiles(): Promise<void> {
  try {
    const [profiles, currentProfile, settings] = await Promise.all([
      getProfiles(),
      getCurrentProfile().catch(() => null),
      getSettings().catch(() => null)
    ]);

    const activeProfile = currentProfile || profiles[0] || null;
    const mode = settings?.folderExpandMode || 'always_expanded';
    let initialCollapsed = new Set<string>();

    if (mode === 'always_collapsed') {
      initialCollapsed = new Set((activeProfile?.mod_folders || []).map(f => f.id));
    } else if (mode === 'remember') {
      try {
        const raw = localStorage.getItem('palmodmanager_collapsed_folders');
        if (raw) initialCollapsed = new Set(JSON.parse(raw));
      } catch {}
    }

    updateState({
      profiles,
      currentProfileId: activeProfile?.id || 'default',
      currentProfile: activeProfile,
      collapsedFolderIds: initialCollapsed,
      ...(settings ? { currentSettings: settings } : {})
    });
    updateActiveProfileLabel();
    renderProfileList();
    const { updateLoadTabVisibility } = await import('../loadView');
    updateLoadTabVisibility();
  } catch (e) {
    console.error('Failed to load profiles:', e);
  }
}

function updateActiveProfileLabel(): void {
  const label = mainDom.elMaybe('profile-active-label');
  if (!label) return;
  const { profiles, currentProfileId } = getState();
  const current = profiles.find(p => p.id === currentProfileId);
  label.textContent = t('mods.profile_label', { name: current ? current.name : 'Default' });
}

export function renderProfileList(): void {
  const list = mainDom.elMaybe('profile-list');
  if (!list) return;
  const { profiles, currentProfileId, dependencies } = getState();
  const isUe4ssWorkshop = dependencies?.ue4ss_version === 'Workshop';
  const isPalSchemaWorkshop = dependencies?.palschema_version === 'Workshop';

  list.innerHTML = profiles.map(p => {
    const modCount = p.enabled_mod_ids ? p.enabled_mod_ids.length : 0;
    const isActive = p.id === currentProfileId;

    const ue4ssVer = p.ue4ss_version || (isActive && dependencies?.ue4ss_installed ? dependencies.ue4ss_version : null);
    const palschemaVer = p.palschema_version || (isActive && dependencies?.palschema_installed ? dependencies.palschema_version : null);

    const isProfileWorkshop = p.dependency_mode === 'workshop';
    const ue4ssClass = `profile-badge ue4ss ${isProfileWorkshop ? 'workshop' : ''}`;
    const palschemaClass = `profile-badge palschema ${isProfileWorkshop ? 'workshop' : ''}`;

    const ue4ssVerStr = ue4ssVer && ue4ssVer !== 'Installed' && ue4ssVer !== 'None' ? ` ${ue4ssVer}` : (isProfileWorkshop ? ' Workshop' : '');
    const palschemaVerStr = palschemaVer && palschemaVer !== 'Installed' && palschemaVer !== 'None' ? ` ${palschemaVer}` : (isProfileWorkshop ? ' Workshop' : '');

    const ue4ssText = p.force_load_order_ue4ss ? `UE4SS (FLO)${ue4ssVerStr}` : `UE4SS${ue4ssVerStr}`;
    const palschemaText = p.force_load_order_palschema ? `PalSchema (FLO)${palschemaVerStr}` : `PalSchema${palschemaVerStr}`;

    const ue4ssBadge = p.ue4ss_enabled ? `<span class="${ue4ssClass}">${escapeHtml(ue4ssText)}</span>` : '';
    const palschemaBadge = p.palschema_enabled ? `<span class="${palschemaClass}">${escapeHtml(palschemaText)}</span>` : '';
    const modCountBadge = `<span class="profile-badge count">${escapeHtml(t('profiles.mod_count_badge', { count: modCount }))}</span>`;

    const altVer = p.altermatic_version;
    const uniVer = p.unipalui_version;
    const altVerStr = altVer && altVer !== 'Installed' && altVer !== 'None' ? ` v${altVer}` : '';
    const uniVerStr = uniVer && uniVer !== 'Installed' && uniVer !== 'None' ? ` v${uniVer}` : '';
    const altBadge = altVer ? `<span class="profile-badge altermatic">⚡ Altermatic${escapeHtml(altVerStr)}</span>` : '';
    const uniBadge = uniVer ? `<span class="profile-badge unipalui">🎨 UniPalUI${escapeHtml(uniVerStr)}</span>` : '';

    const patchCount = p.compatibility_patches ? p.compatibility_patches.length : 0;
    const patchBadge = patchCount > 0 ? `<span class="profile-badge patches">📦 ${patchCount} ${patchCount === 1 ? 'patch' : 'patches'}</span>` : '';

    return `
    <div class="profile-item ${isActive ? 'active' : ''}" data-id="${p.id}">
      <div style="display:flex;flex-direction:column;gap:5px;min-width:0;flex:1;">
        <div style="display:flex;align-items:center;gap:8px;">
          <span class="profile-item-name">${escapeHtml(p.name)}</span>
          ${isActive ? `<span class="profile-item-badge-active">${escapeHtml(t('profiles.active_badge'))}</span>` : ''}
        </div>
        <div style="display:flex;gap:5px;align-items:center;flex-wrap:wrap;">
          ${ue4ssBadge}
          ${palschemaBadge}
          ${altBadge}
          ${uniBadge}
          ${modCountBadge}
          ${patchBadge}
        </div>
      </div>
      <div class="profile-actions">
        <button class="btn-secondary btn-sm profile-export-btn" data-id="${p.id}" title="${escapeHtml(t('profiles.btn_export_pack_title'))}">📤 ${escapeHtml(t('profiles.btn_export_pack'))}</button>
        <button class="btn-secondary btn-sm profile-clone-btn" data-id="${p.id}">${escapeHtml(t('profiles.btn_clone'))}</button>
        <button class="btn-secondary btn-sm profile-clear-btn" data-id="${p.id}">${escapeHtml(t('profiles.btn_clear'))}</button>
        ${p.id !== currentProfileId ? `<button class="btn-secondary btn-sm profile-switch-btn" data-id="${p.id}">${escapeHtml(t('profiles.btn_switch'))}</button>` : ''}
        <button class="profile-item-delete ${p.id === 'default' ? 'disabled' : ''}" data-id="${p.id}" ${p.id === 'default' ? 'disabled' : ''}>✕</button>
      </div>
    </div>`;
  }).join('');

  list.querySelectorAll('.profile-item').forEach(item => {
    item.addEventListener('click', async (e) => {
      if (
        (e.target as HTMLElement).closest('.profile-item-delete') ||
        (e.target as HTMLElement).closest('.profile-clone-btn') ||
        (e.target as HTMLElement).closest('.profile-clear-btn') ||
        (e.target as HTMLElement).closest('.profile-export-btn')
      ) return;
      const id = (item as HTMLElement).dataset.id!;
      if (id === getState().currentProfileId) return;
      await handleProfileChange(id);
    });
  });

  list.querySelectorAll('.profile-export-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      await handleExportProfilePack(id);
    });
  });

  list.querySelectorAll('.profile-clone-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const { profiles } = getState();
      const current = profiles.find(p => p.id === id);
      const name = current ? current.name : '';

      const newName = await showInputModal(
        t('profiles.prompt_duplicate_title'),
        t('profiles.prompt_duplicate_body', { name }),
        `${name} - Copy`
      );
      if (newName === null) return;
      const trimmed = newName.trim();
      if (!trimmed) return;

      try {
        const { cloneProfile } = await import('../../api');
        await cloneProfile(id, trimmed);
        showToast(t('toasts.profile_created', { name: trimmed }), 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    });
  });

  list.querySelectorAll('.profile-item-delete:not(.disabled)').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const { profiles } = getState();
      const current = profiles.find(p => p.id === id);
      const name = current ? current.name : '';
      const confirmed = await showConfirm(t('dialogs.confirm_delete_profile', { name }));
      if (!confirmed) return;
      try {
        const { deleteProfile } = await import('../../api');
        await deleteProfile(id);
        showToast(t('toasts.profile_deleted', { name }), 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    });
  });

  list.querySelectorAll('.profile-clear-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      const { profiles } = getState();
      const current = profiles.find(p => p.id === id);
      const name = current ? current.name : '';

      const confirmed = await showConfirm(t('dialogs.confirm_clear_profile', { name }));
      if (!confirmed) return;

      try {
        const updatedProfiles = await clearProfile(id);
        updateState({ profiles: updatedProfiles });
        showToast(t('toasts.profile_cleared', { name }), 'success');

        if (id === getState().currentProfileId) {
          const { getMods } = await import('../../api');
          const mods = await getMods();
          updateState({ allMods: mods });
        }

        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
      }
    });
  });
}

export function showInputModal(title: string, message: string, defaultValue: string): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '1500';
    overlay.innerHTML = `
      <div class="modal" style="width: 420px; max-width: 90vw; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; box-shadow: 0 16px 40px rgba(0,0,0,0.6);">
        <div class="modal-header" style="padding: 14px 18px; border-bottom: 1px solid var(--border); background: var(--bg-primary); display: flex; align-items: center; justify-content: space-between;">
          <h3 style="margin: 0; font-size: 14px; font-weight: 700; color: var(--text-primary);">${escapeHtml(title)}</h3>
          <button class="modal-close-btn" style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 15px; padding: 2px 6px; border-radius: 4px; transition: color 0.15s;">✕</button>
        </div>
        <div class="modal-body" style="gap: 8px; padding: 18px; display: flex; flex-direction: column;">
          <label style="font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-muted);">${escapeHtml(message)}</label>
          <input type="text" class="modal-input" value="${escapeHtml(defaultValue)}" style="width: 100%; box-sizing: border-box; background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius, 4px); padding: 8px 12px; font-size: 13px; font-weight: 500; color: var(--text-primary); outline: none; transition: border-color 0.2s, box-shadow 0.2s;" />
        </div>
        <div class="modal-footer" style="padding: 12px 18px; border-top: 1px solid var(--border); background: var(--bg-primary); display: flex; justify-content: flex-end; gap: 8px;">
          <button class="modal-cancel-btn btn-secondary" style="padding: 6px 14px; border-radius: 4px; font-size: 12px; font-weight: 600; cursor: pointer;">${escapeHtml(t('common.cancel'))}</button>
          <button class="modal-confirm-btn btn-primary" style="padding: 6px 16px; border-radius: 4px; font-size: 12px; font-weight: 600; cursor: pointer;">${escapeHtml(t('common.save'))}</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const input = overlay.querySelector('.modal-input') as HTMLInputElement;
    const confirmBtn = overlay.querySelector('.modal-confirm-btn') as HTMLButtonElement;
    const cancelBtn = overlay.querySelector('.modal-cancel-btn') as HTMLButtonElement;
    const closeBtn = overlay.querySelector('.modal-close-btn') as HTMLButtonElement;

    input.addEventListener('focus', () => {
      input.style.borderColor = 'var(--accent)';
      input.style.boxShadow = '0 0 0 1px var(--accent-dim, rgba(0,120,212,0.3))';
    });
    input.addEventListener('blur', () => {
      input.style.borderColor = 'var(--border)';
      input.style.boxShadow = 'none';
    });

    closeBtn.addEventListener('mouseenter', () => { closeBtn.style.color = 'var(--text-primary)'; });
    closeBtn.addEventListener('mouseleave', () => { closeBtn.style.color = 'var(--text-muted)'; });

    input.focus();
    input.select();

    const cleanup = (val: string | null) => {
      overlay.remove();
      resolve(val);
    };

    confirmBtn.addEventListener('click', () => cleanup(input.value));
    cancelBtn.addEventListener('click', () => cleanup(null));
    closeBtn.addEventListener('click', () => cleanup(null));

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') cleanup(input.value);
      if (e.key === 'Escape') cleanup(null);
    });

    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) cleanup(null);
    });
  });
}

let _isSwitchingProfile = false;

export async function handleProfileChange(profileId: string): Promise<void> {
  const { currentProfileId, profiles } = getState();
  if (profileId === currentProfileId) return;
  if (_isSwitchingProfile) {
    showToast(t('toasts.profile_switching_busy'), 'warning');
    return;
  }
  _isSwitchingProfile = true;
  suppressWatcherRefresh(3500);

  // Disable switch buttons while processing
  document.querySelectorAll<HTMLButtonElement>('.profile-switch-btn, .profile-item').forEach(el => {
    el.style.pointerEvents = 'none';
    el.style.opacity = '0.6';
  });

  try {
    const { confirmDiscardOrSave, clearOriginalContent } = await import('../editorView');
    const proceed = await confirmDiscardOrSave();
    if (!proceed) {
      _isSwitchingProfile = false;
      document.querySelectorAll<HTMLButtonElement>('.profile-switch-btn, .profile-item').forEach(el => {
        el.style.pointerEvents = '';
        el.style.opacity = '';
      });
      return;
    }

    const targetProf = profiles.find(p => p.id === profileId);
    const targetName = targetProf ? targetProf.name : profileId;
    showToast(t('toasts.profile_switching', { name: targetName }), 'info');
    const mods = await switchProfile(profileId);

    clearOriginalContent();

    updateState({
      allMods: mods,
      currentProfileId: profileId,
      editorModId: null,
      editorFiles: [],
      editorSelectedFile: null,
      currentFolderId: null
    });
    renderModsView();

    const editorContent = editorDom.elMaybe('editor-content');
    if (editorContent) {
      editorContent.value = '';
      editorContent.disabled = true;
    }
    const editorPath = editorDom.elMaybe('editor-file-path');
    if (editorPath) editorPath.textContent = '';
    const nameEl = editorDom.elMaybe('editor-current-mod-name');
    if (nameEl) nameEl.textContent = '';
    const highlightCode = editorDom.elMaybe('editor-highlight-code');
    if (highlightCode) highlightCode.innerHTML = '';
    const fileTreeEl = editorDom.elMaybe('editor-file-tree');
    if (fileTreeEl) fileTreeEl.innerHTML = `<div class="editor-file-empty">${escapeHtml(t('editor.file_empty'))}</div>`;

    const { populateEditorModSelect, renderEditorModTree } = await import('../editorView');
    populateEditorModSelect();
    renderEditorModTree();

    await Promise.all([loadProfiles(), loadDependencies(), loadLibrary()]);
    renderModsView();
    bus.emit('profile:switched', { profileId, profileName: targetName });
    showToast(t('toasts.profile_switched', { name: targetName }), 'success');
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    _isSwitchingProfile = false;
    document.querySelectorAll<HTMLButtonElement>('.profile-switch-btn, .profile-item').forEach(el => {
      el.style.pointerEvents = '';
      el.style.opacity = '';
    });
  }
}

export async function handleCreateProfile(name: string): Promise<void> {
  try {
    const { createProfile } = await import('../../api');
    const newProfile = await createProfile(name);
    showToast(t('toasts.profile_created', { name }), 'success');
    await loadProfiles();
    if (newProfile && newProfile.id) {
      await handleProfileChange(newProfile.id);
    }
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  }
}

export async function handleExportProfilePack(profileId: string): Promise<void> {
  try {
    const { profiles } = getState();
    const profile = profiles.find(p => p.id === profileId);
    const profileName = profile ? profile.name : profileId;

    const { save } = await import('@tauri-apps/plugin-dialog');
    const defaultZipName = `PMM_Profile_${profileName.replace(/[^\w\s-]/g, '_')}_CoopPack.zip`;

    const destPath = await save({
      defaultPath: defaultZipName,
      filters: [{ name: 'Zip Archive / Co-op Pack', extensions: ['zip', 'pmmprofile'] }],
      title: t('profiles.dialog_export_title', { name: profileName }),
    });

    if (!destPath) return;

    showToast(t('profiles.exporting_pack_toast', { name: profileName }), 'info');

    const { exportProfilePack } = await import('../../api');
    const resultPath = await exportProfilePack(profileId, destPath);

    showToast(t('profiles.exported_pack_toast', { path: resultPath }), 'success');
  } catch (err) {
    console.error('Failed to export profile pack:', err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}

export async function handleImportProfilePack(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Zip Archive / Co-op Pack', extensions: ['zip', 'pmmprofile'] }],
      title: t('profiles.dialog_import_title'),
    });

    if (!selected || typeof selected !== 'string') return;

    const confirmed = await showConfirm(t('profiles.confirm_import_body'));
    if (!confirmed) return;

    showToast(t('profiles.importing_pack_toast'), 'info');

    const { importProfilePack } = await import('../../api');
    const result = await importProfilePack(selected);

    if (result && result.success) {
      await Promise.all([loadProfiles(), loadDependencies(), loadLibrary()]);
      const { getMods } = await import('../../api');
      const mods = await getMods();
      updateState({ allMods: mods });
      renderModsView();

      showToast(t('profiles.imported_pack_toast', {
        name: result.profileName,
        count: result.modCount,
      }), 'success');
    }
  } catch (err) {
    console.error('Failed to import profile pack:', err);
    showToast(t('toasts.export_failed', { error: String(err) }), 'error');
  }
}
