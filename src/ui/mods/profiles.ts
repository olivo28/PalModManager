import { getState, updateState } from '../../state';
import { renderModsView } from './renderer';
import { loadMods } from './loader';
import { loadDependencies } from './dependencies';
import { loadLibrary } from './library';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';
import { switchProfile, clearProfile } from '../../api';
import { suppressWatcherRefresh } from '../editor/watcher';

export async function loadProfiles(): Promise<void> {
  try {
    const { getProfiles, getCurrentProfile, getSettings } = await import('../../api');
    const [profiles, currentProfile, settings] = await Promise.all([
      getProfiles(),
      getCurrentProfile().catch(() => null),
      getSettings().catch(() => null)
    ]);

    const activeProfile = currentProfile || profiles[0] || null;
    updateState({
      profiles,
      currentProfileId: activeProfile?.id || 'default',
      currentProfile: activeProfile,
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
  const label = document.getElementById('profile-active-label');
  if (!label) return;
  const { profiles, currentProfileId } = getState();
  const current = profiles.find(p => p.id === currentProfileId);
  label.textContent = t('mods.profile_label', { name: current ? current.name : 'Default' });
}

export function renderProfileList(): void {
  const list = document.getElementById('profile-list');
  if (!list) return;
  const { profiles, currentProfileId, dependencies } = getState();
  const isUe4ssWorkshop = dependencies?.ue4ss_version === 'Workshop';
  const isPalSchemaWorkshop = dependencies?.palschema_version === 'Workshop';

  list.innerHTML = profiles.map(p => {
    const modCount = p.enabled_mod_ids ? p.enabled_mod_ids.length : 0;
    const isActive = p.id === currentProfileId;

    const ue4ssText = p.force_load_order_ue4ss ? 'UE4SS (FLO)' : 'UE4SS';
    const palschemaText = p.force_load_order_palschema ? 'PalSchema (FLO)' : 'PalSchema';

    const isProfileWorkshop = p.dependency_mode === 'workshop';
    const ue4ssClass = `profile-badge ue4ss ${isProfileWorkshop ? 'workshop' : ''}`;
    const palschemaClass = `profile-badge palschema ${isProfileWorkshop ? 'workshop' : ''}`;

    const ue4ssBadge = p.ue4ss_enabled ? `<span class="${ue4ssClass}">${ue4ssText}</span>` : '';
    const palschemaBadge = p.palschema_enabled ? `<span class="${palschemaClass}">${palschemaText}</span>` : '';
    const modCountBadge = `<span class="profile-badge count">${escapeHtml(t('profiles.mod_count_badge', { count: modCount }))}</span>`;

    return `
    <div class="profile-item ${isActive ? 'active' : ''}" data-id="${p.id}">
      <div style="display:flex;flex-direction:column;gap:4px;">
        <div style="display:flex;align-items:center;gap:8px;">
          <span class="profile-item-name">${escapeHtml(p.name)}</span>
          ${isActive ? `<span class="profile-item-badge-active">${escapeHtml(t('profiles.active_badge'))}</span>` : ''}
        </div>
        <div style="display:flex;gap:4px;align-items:center;flex-wrap:wrap;">
          ${ue4ssBadge}
          ${palschemaBadge}
          ${modCountBadge}
        </div>
      </div>
      <div class="profile-actions">
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
        (e.target as HTMLElement).closest('.profile-clear-btn')
      ) return;
      const id = (item as HTMLElement).dataset.id!;
      if (id === getState().currentProfileId) return;
      await handleProfileChange(id);
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

    const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement | null;
    if (editorContent) {
      editorContent.value = '';
      editorContent.disabled = true;
    }
    const editorPath = document.getElementById('editor-file-path');
    if (editorPath) editorPath.textContent = '';
    const nameEl = document.getElementById('editor-current-mod-name');
    if (nameEl) nameEl.textContent = '';
    const highlightCode = document.getElementById('editor-highlight-code');
    if (highlightCode) highlightCode.innerHTML = '';
    const fileTreeEl = document.getElementById('editor-file-tree');
    if (fileTreeEl) fileTreeEl.innerHTML = `<div class="editor-file-empty">${escapeHtml(t('editor.file_empty'))}</div>`;

    const { populateEditorModSelect, renderEditorModTree } = await import('../editorView');
    populateEditorModSelect();
    renderEditorModTree();

    await Promise.all([loadProfiles(), loadDependencies(), loadLibrary()]);
    renderModsView();
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
