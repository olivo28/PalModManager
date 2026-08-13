import { getState, updateState } from '../../state';
import { renderModsView } from './renderer';
import { loadMods } from './loader';
import { loadDependencies } from './dependencies';
import { loadLibrary } from './library';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';
import { switchProfile, clearProfile } from '../../api';

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
  label.textContent = `Profile: ${current ? current.name : 'Default'}`;
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
    const modCountBadge = `<span class="profile-badge count">${modCount} mod${modCount === 1 ? '' : 's'}</span>`;

    return `
    <div class="profile-item ${isActive ? 'active' : ''}" data-id="${p.id}">
      <div style="display:flex;flex-direction:column;gap:4px;">
        <div style="display:flex;align-items:center;gap:8px;">
          <span class="profile-item-name">${escapeHtml(p.name)}</span>
          ${isActive ? '<span class="profile-item-badge-active">ACTIVE</span>' : ''}
        </div>
        <div style="display:flex;gap:4px;align-items:center;flex-wrap:wrap;">
          ${ue4ssBadge}
          ${palschemaBadge}
          ${modCountBadge}
        </div>
      </div>
      <div class="profile-actions">
        <button class="btn-secondary btn-sm profile-clone-btn" data-id="${p.id}">Clone</button>
        <button class="btn-secondary btn-sm profile-clear-btn" data-id="${p.id}">Clear</button>
        ${p.id !== currentProfileId ? `<button class="btn-secondary btn-sm profile-switch-btn" data-id="${p.id}">Switch</button>` : ''}
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
        'Duplicate Profile',
        `Enter a name for the duplicated profile of "${name}":`,
        `${name} - Copy`
      );
      if (newName === null) return;
      const trimmed = newName.trim();
      if (!trimmed) return;

      try {
        const { cloneProfile } = await import('../../api');
        await cloneProfile(id, trimmed);
        showToast('Profile duplicated', 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast('Failed to duplicate profile: ' + err, 'error');
      }
    });
  });

  list.querySelectorAll('.profile-item-delete:not(.disabled)').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const id = (btn as HTMLElement).dataset.id!;
      try {
        const { deleteProfile } = await import('../../api');
        await deleteProfile(id);
        showToast('Profile deleted', 'success');
        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast('Failed to delete profile: ' + err, 'error');
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

      const confirmed = await showConfirm(`Are you sure you want to clear/purge all mods from the profile "${name}"? This will physically disable and clear active mods in the game (if active).`);
      if (!confirmed) return;

      try {
        const updatedProfiles = await clearProfile(id);
        updateState({ profiles: updatedProfiles });
        showToast('Profile cleared successfully', 'success');

        if (id === getState().currentProfileId) {
          const { getMods } = await import('../../api');
          const mods = await getMods();
          updateState({ allMods: mods });
        }

        await loadProfiles();
        renderModsView();
      } catch (err) {
        showToast('Failed to clear profile: ' + err, 'error');
      }
    });
  });
}

export function showInputModal(title: string, message: string, defaultValue: string): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay visible';
    overlay.style.zIndex = '3000';
    overlay.style.position = 'fixed';
    overlay.style.top = '0';
    overlay.style.left = '0';
    overlay.style.right = '0';
    overlay.style.bottom = '0';
    overlay.style.background = 'rgba(0,0,0,0.6)';
    overlay.style.backdropFilter = 'blur(4px)';
    overlay.style.display = 'flex';
    overlay.style.alignItems = 'center';
    overlay.style.justifyContent = 'center';

    overlay.innerHTML = `
      <div class="modal" style="width: 400px; max-width: 90vw; border: 1px solid var(--border); background: var(--bg-primary); border-radius: 8px; box-shadow: 0 8px 32px rgba(0,0,0,0.5);">
        <div class="modal-header" style="padding: 16px 20px; border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between;">
          <h3 style="margin: 0; font-size: 16px; font-weight: 600; color: var(--text-primary);">${escapeHtml(title)}</h3>
          <button class="modal-close-btn" id="input-modal-close-x" style="background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;">✕</button>
        </div>
        <div class="modal-body" style="padding: 20px; display: flex; flex-direction: column; gap: 12px;">
          <div style="font-size: 13px; color: var(--text-secondary);">${escapeHtml(message)}</div>
          <input type="text" id="input-modal-value" value="${escapeHtml(defaultValue)}" style="width: 100%; padding: 8px 12px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); border-radius: 4px; font-size: 13px; outline: none; box-sizing: border-box;" />
        </div>
        <div class="modal-footer" style="padding: 12px 20px; border-top: 1px solid var(--border); display: flex; justify-content: flex-end; gap: 8px;">
          <button id="input-modal-cancel" class="btn-secondary" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">Cancel</button>
          <button id="input-modal-confirm" class="btn-primary" style="padding: 6px 12px; font-size: 12px; cursor: pointer; border-radius: 4px;">Confirm</button>
        </div>
      </div>
    `;

    document.body.appendChild(overlay);

    const input = overlay.querySelector('#input-modal-value') as HTMLInputElement;
    input.focus();
    input.select();

    const cleanUp = () => {
      document.body.removeChild(overlay);
    };

    overlay.querySelector('#input-modal-close-x')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });

    overlay.querySelector('#input-modal-cancel')!.addEventListener('click', () => {
      cleanUp();
      resolve(null);
    });

    const handleConfirm = () => {
      const val = input.value.trim();
      cleanUp();
      resolve(val ? val : null);
    };

    overlay.querySelector('#input-modal-confirm')!.addEventListener('click', handleConfirm);

    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        handleConfirm();
      }
      if (e.key === 'Escape') {
        cleanUp();
        resolve(null);
      }
    });
  });
}

export async function handleProfileChange(profileId: string): Promise<void> {
  const { currentProfileId } = getState();
  if (profileId === currentProfileId) return;
  try {
    const { confirmDiscardOrSave, clearOriginalContent } = await import('../editorView');
    const proceed = await confirmDiscardOrSave();
    if (!proceed) return;

    showToast('Switching profile... Backing up current and restoring target mods...', 'info');
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
    if (fileTreeEl) fileTreeEl.innerHTML = '<div class="editor-file-empty">No files loaded</div>';

    const { populateEditorModSelect, renderEditorModTree } = await import('../editorView');
    populateEditorModSelect();
    renderEditorModTree();

    showToast('Profile switched', 'success');
    await Promise.all([loadProfiles(), loadDependencies(), loadLibrary()]);
    renderModsView();
  } catch (e) {
    showToast('Failed to switch profile: ' + e, 'error');
  }
}

export async function handleCreateProfile(name: string): Promise<void> {
  try {
    const { createProfile } = await import('../../api');
    const newProfile = await createProfile(name);
    showToast('Profile created', 'success');
    await loadProfiles();
    if (newProfile && newProfile.id) {
      await handleProfileChange(newProfile.id);
    }
  } catch (e) {
    showToast('Failed to create profile: ' + e, 'error');
  }
}
