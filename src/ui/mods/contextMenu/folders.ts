import { getState, updateState } from '../../../state';
import { showToast } from '../../toast';
import { renderModsView } from '../renderer';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { getContextOverlay, hideContextMenu, positionContextMenu } from './menuDom';
import { mainDom } from '../../../framework';

export function showFolderContextMenu(folderId: string, x: number, y: number): void {
  const state = getState();
  const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);
  const folder = currentProfile?.mod_folders?.find(f => f.id === folderId);
  if (!folder) return;

  const overlay = getContextOverlay();
  const menu = mainDom.el('context-menu');

  const modsInFolder = state.allMods.filter(m => folder.mod_ids.includes(m.id));
  const allEnabled = modsInFolder.length > 0 && modsInFolder.every(m => m.enabled);
  const toggleLabel = allEnabled ? t('context.folder_disable_all') : t('context.folder_enable_all');
  const toggleIcon = allEnabled ? '⏸' : '▶';

  const folders = currentProfile?.mod_folders || [];
  const folderIndex = folders.findIndex(f => f.id === folderId);
  const canMoveUp = folderIndex > 0;
  const canMoveDown = folderIndex !== -1 && folderIndex < folders.length - 1;

  const html = `
    <button type="button" class="context-menu-item" data-action="enter-folder">
      <span class="ctx-icon">📂</span>
      ${escapeHtml(t('context.folder_enter'))}
    </button>
    ${canMoveUp ? `
    <button type="button" class="context-menu-item" data-action="move-folder-up">
      <span class="ctx-icon">▲</span>
      ${escapeHtml(t('context.folder_move_up'))}
    </button>` : ''}
    ${canMoveDown ? `
    <button type="button" class="context-menu-item" data-action="move-folder-down">
      <span class="ctx-icon">▼</span>
      ${escapeHtml(t('context.folder_move_down'))}
    </button>` : ''}
    <button type="button" class="context-menu-item" data-action="rename-folder">
      <span class="ctx-icon">✏</span>
      ${escapeHtml(t('context.folder_rename'))}
    </button>
    <button type="button" class="context-menu-item" data-action="toggle-folder-mods">
      <span class="ctx-icon">${toggleIcon}</span>
      ${escapeHtml(toggleLabel)}
    </button>
    <button type="button" class="context-menu-item" data-action="check-updates-mods">
      <span class="ctx-icon">↑</span>
      ${escapeHtml(t('context.check_updates'))}
    </button>
    <div class="context-menu-sep"></div>
    <button type="button" class="context-menu-item danger" data-action="delete-folder">
      <span class="ctx-icon">🗑</span>
      ${escapeHtml(t('context.folder_delete'))}
    </button>
  `;

  menu.innerHTML = html;

  menu.querySelectorAll('.context-menu-item').forEach(btn => {
    btn.addEventListener('click', async (e: Event) => {
      e.stopPropagation();
      e.preventDefault();
      const action = (btn as HTMLElement).dataset.action!;
      hideContextMenu();

      if (action === 'enter-folder') {
        updateState({ currentFolderId: folderId });
        renderModsView();
      } else if (action === 'move-folder-up' || action === 'move-folder-down') {
        const fList = [...folders];
        const idx = fList.findIndex(f => f.id === folderId);
        if (idx === -1) return;
        const targetIdx = action === 'move-folder-up' ? idx - 1 : idx + 1;
        if (targetIdx < 0 || targetIdx >= fList.length) return;
        const [moved] = fList.splice(idx, 1);
        fList.splice(targetIdx, 0, moved);
        try {
          const { reorderModFolders } = await import('../../../api');
          const updatedProfile = await reorderModFolders(state.currentProfileId, fList.map(f => f.id));
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          const { loadMods } = await import('../loader');
          await loadMods();
        } catch (err) {
          showToast(String(err), 'error');
        }
      } else if (action === 'rename-folder') {
        const renameBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .rename-btn`) as HTMLElement | null;
        renameBtn?.click();
      } else if (action === 'delete-folder') {
        const deleteBtn = document.querySelector(`.folder-card[data-id="${folderId}"] .delete-btn`) as HTMLElement | null;
        deleteBtn?.click();
      } else if (action === 'toggle-folder-mods') {
        try {
          const { toggleFolderMods } = await import('../../../api');
          const updatedProfile = await toggleFolderMods(state.currentProfileId, folderId, !allEnabled);
          const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
          updateState({ profiles: updatedProfiles });
          showToast(!allEnabled ? t('toasts.folder_mods_enabled') : t('toasts.folder_mods_disabled'), 'success');
          const { loadMods } = await import('../loader');
          await loadMods();
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      } else if (action === 'check-updates-mods') {
        if (modsInFolder.length === 0) {
          showToast(t('mods.folder_empty_desc'), 'info');
          return;
        }
        showToast(t('toasts.checking_updates', { name: folder.name }), 'info');
        try {
          const { checkForUpdates } = await import('../../../api');
          await checkForUpdates();
          const { loadMods } = await import('../loader');
          await loadMods();
          showToast(t('toasts.all_mods_up_to_date'), 'success');
        } catch (err) {
          showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        }
      }
    });
  });

  positionContextMenu(x, y);
}
