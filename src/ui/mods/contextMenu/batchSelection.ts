import { getState, updateState } from '../../../state';
import { showToast } from '../../toast';
import { loadMods } from '../loader';
import { showInputModal } from '../profiles';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { hideContextMenu, positionContextMenu } from './menuDom';
import { mainDom } from '../../../framework';

export function showBulkContextMenu(x: number, y: number): void {
  const menu = mainDom.el('context-menu');
  const selectedCount = getState().selectedModIds.size;

  const currentProfile = getState().profiles.find(p => p.id === getState().currentProfileId);
  const folders = currentProfile?.mod_folders || [];
  const selectedIds = Array.from(getState().selectedModIds);
  const anyInFolder = folders.some(f => selectedIds.some(id => f.mod_ids.includes(id)));
  const foldersHtml = folders.map(f => `
    <button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="${f.id}" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
      <span>📁</span> ${escapeHtml(f.name)}
    </button>
  `).join('');

  const html = `
    <div style="font-size:9px;font-weight:700;color:var(--text-muted);padding:6px 16px 2px;text-transform:uppercase">${escapeHtml(t('common.selected_count_mods', { count: selectedCount }))}</div>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="bulk-enable">
      <span class="ctx-icon">●</span>
      ${escapeHtml(t('bulk.enable_selected'))}
    </button>
    <button type="button" class="context-menu-item" data-action="bulk-disable">
      <span class="ctx-icon">◌</span>
      ${escapeHtml(t('bulk.disable_selected'))}
    </button>
    <div class="context-menu-sep"></div>
    <div class="context-menu-item has-submenu" style="position:relative;display:flex;align-items:center;width:100%;">
      <span class="ctx-icon">📁</span>
      ${escapeHtml(t('context.move_to_folder'))}
      <span style="margin-left:auto;font-size:9px;color:var(--text-muted);pointer-events:none;">▶</span>
      <div class="context-submenu" style="display:none;position:absolute;top:-4px;left:100%;background:var(--bg-primary);border:1px solid var(--border);border-radius:6px;box-shadow:0 8px 32px rgba(0,0,0,0.5);min-width:160px;z-index:4000;padding:4px 0;">
        ${foldersHtml}
        ${(folders.length > 0 && anyInFolder) ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        ${anyInFolder ? `<button type="button" class="context-submenu-item" data-action="bulk-move-to-folder" data-folder-id="none" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>❌</span> ${escapeHtml(t('context.remove_from_folders'))}
        </button>` : ''}
        ${folders.length > 0 ? `<div style="height:1px;background:var(--border);margin:4px 0;"></div>` : ''}
        <button type="button" class="context-submenu-item" data-action="bulk-move-to-new-folder" style="display:flex;align-items:center;width:100%;padding:6px 12px;background:none;border:none;color:var(--text-primary);cursor:pointer;font-size:12px;text-align:left;gap:6px;">
          <span>➕</span> ${escapeHtml(t('context.new_folder'))}
        </button>
      </div>
    </div>
    ${selectedCount === 2 ? `
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item" data-action="bulk-merge-hybrid">
      <span class="ctx-icon">⚡</span>
      ${escapeHtml(t('context.merge_as_hybrid'))}
    </button>
    ` : ''}
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="bulk-remove">
      <span class="ctx-icon">✕</span>
      ${escapeHtml(t('bulk.delete_selected'))}
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.has-submenu').forEach(item => {
    item.addEventListener('mouseenter', () => {
      const sub = item.querySelector('.context-submenu') as HTMLElement | null;
      if (!sub) return;
      const parentRect = item.getBoundingClientRect();
      const subWidth = 180;
      const spaceRight = window.innerWidth - parentRect.right;
      if (spaceRight < subWidth + 12) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
  });

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', (e) => {
      if ((btn as HTMLElement).classList.contains('has-submenu')) return;
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'bulk-enable') {
        mainDom.elMaybe('bulk-enable-btn')?.click();
      } else if (action === 'bulk-disable') {
        mainDom.elMaybe('bulk-disable-btn')?.click();
      } else if (action === 'bulk-remove') {
        mainDom.elMaybe('bulk-remove-btn')?.click();
      } else if (action === 'bulk-merge-hybrid') {
        const selected = Array.from(getState().selectedModIds);
        if (selected.length === 2) {
          (async () => {
            try {
              const { mergeModsAsHybrid } = await import('../../../api');
              const merged = await mergeModsAsHybrid(selected[0], selected[1]);
              showToast(t('toasts.merge_hybrid_success', { name: merged.name }), 'success');
              updateState({ selectedModIds: new Set([merged.id]) });
              await loadMods();
            } catch (err: any) {
              showToast(String(err), 'error');
            }
          })();
        }
      }
    });
  });

  menu.querySelectorAll('.context-submenu-item').forEach(subBtn => {
    subBtn.addEventListener('click', async (e) => {
      e.stopPropagation();
      e.preventDefault();
      const subAction = (subBtn as HTMLElement).dataset.action!;
      hideContextMenu();

      const modIds = Array.from(getState().selectedModIds);
      if (modIds.length === 0) return;

      if (subAction === 'bulk-move-to-folder') {
        const folderId = (subBtn as HTMLElement).dataset.folderId!;
        const targetFolder = folderId === 'none' ? null : folderId;
        showToast(targetFolder ? t('toasts.mod_grouped_success') : t('toasts.mod_ungrouped_success'), 'info');
        try {
          const { addModToFolder } = await import('../../../api');
          const state = getState();
          let lastProfile = null;
          for (const modId of modIds) {
            lastProfile = await addModToFolder(state.currentProfileId, targetFolder, modId);
          }
          if (lastProfile) {
            const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
            updateState({ profiles });
          }
          await loadMods();
          showToast(targetFolder ? t('toasts.mod_grouped_success') : t('toasts.mod_ungrouped_success'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else if (subAction === 'bulk-move-to-new-folder') {
        const newName = await showInputModal(
          t('dialogs.prompt_new_folder_name'),
          t('dialogs.prompt_new_folder_name'),
          'Skins'
        );
        if (newName === null) return;
        const trimmed = newName.trim();
        if (!trimmed) return;

        try {
          const { createModFolder, addModToFolder } = await import('../../../api');
          const state = getState();
          const updatedProfile = await createModFolder(state.currentProfileId, trimmed);
          const newFolder = updatedProfile.mod_folders?.find(f => f.name.toLowerCase() === trimmed.toLowerCase());
          if (newFolder) {
            let lastProfile = null;
            for (const modId of modIds) {
              lastProfile = await addModToFolder(state.currentProfileId, newFolder.id, modId);
            }
            if (lastProfile) {
              const profiles = state.profiles.map(p => p.id === state.currentProfileId ? lastProfile : p);
              updateState({ profiles });
            }
            await loadMods();
            showToast(t('toasts.folder_created'), 'success');
          }
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}
