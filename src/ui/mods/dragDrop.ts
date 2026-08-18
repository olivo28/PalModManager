import { getState, updateState } from '../../state';
import { loadMods } from './loader';
import { showToast } from '../toast';
import { handleAddMultipleModsToFolder } from './events';
import { reorderModFolders } from '../../api';

export function setupCardDragToFolder(container: HTMLElement): void {
  const modCards = Array.from(container.querySelectorAll('.mod-card:not(.folder-card)')) as HTMLElement[];
  const folderCards = Array.from(container.querySelectorAll('.mod-card.folder-card')) as HTMLElement[];
  const rootDropZone = container.querySelector('#mod-root-drop-zone') as HTMLElement | null;

  if (folderCards.length === 0 && !rootDropZone && modCards.length === 0) return;

  let draggingId: string | null = null;
  let draggingType: 'mod' | 'folder' | null = null;
  let draggingIds: string[] = [];
  let ghost: HTMLElement | null = null;
  let pendingEl: HTMLElement | null = null;
  let dragActive = false;

  let pointerDownX = 0;
  let pointerDownY = 0;
  const DRAG_THRESHOLD = 6;

  let activeFolderTarget: HTMLElement | null = null;
  let activeReorderTarget: { el: HTMLElement; before: boolean } | null = null;
  let activeRootTarget: HTMLElement | null = null;

  function clearAllHighlights(): void {
    if (activeFolderTarget) {
      activeFolderTarget.classList.remove('drag-target-hover');
      activeFolderTarget = null;
    }
    if (activeReorderTarget) {
      activeReorderTarget.el.classList.remove('drag-reorder-before', 'drag-reorder-after');
      activeReorderTarget = null;
    }
    if (activeRootTarget) {
      activeRootTarget.classList.remove('drag-target-hover');
      activeRootTarget = null;
    }
  }

  function cleanup(): void {
    if (ghost) {
      ghost.remove();
      ghost = null;
    }
    clearAllHighlights();
    if (pendingEl) {
      pendingEl.classList.remove('dragging');
      pendingEl = null;
    }
    const state = getState();
    if (draggingIds.length > 1) {
      state.selectedModIds.forEach(id => {
        const el = container.querySelector(`.mod-card[data-id="${id}"]`);
        if (el) el.classList.remove('dragging');
      });
    }
    draggingId = null;
    draggingType = null;
    draggingIds = [];
    dragActive = false;
    document.body.removeAttribute('data-card-dragging');
  }

  // ─── Attach pointerdown on all draggable cards ───
  const allCards = Array.from(container.querySelectorAll('.mod-card')) as HTMLElement[];
  allCards.forEach(card => {
    card.addEventListener('pointerdown', (e: PointerEvent) => {
      // Left-click only
      if (e.button !== 0) return;

      const target = e.target as HTMLElement;
      // Do not drag when interacting with switches, checkboxes, buttons, inputs, context menus, actions
      if (target.closest('.toggle-switch, .card-toggle-input, .folder-toggle-input, .mod-folder-btn, .card-remove-btn, button, input, a, select, .folder-card-actions')) {
        return;
      }

      const id = card.dataset.id;
      if (!id) return;

      pointerDownX = e.clientX;
      pointerDownY = e.clientY;
      pendingEl = card;
      draggingId = id;
      draggingType = card.dataset.type === 'folder' ? 'folder' : 'mod';

      const state = getState();
      if (draggingType === 'mod' && state.selectedModIds.has(id)) {
        draggingIds = Array.from(state.selectedModIds);
      } else {
        draggingIds = [id];
      }

      try { card.setPointerCapture(e.pointerId); } catch { }
    });
  });

  // ─── Window Pointer Move ───
  const onPointerMove = (e: PointerEvent) => {
    if (!pendingEl || !draggingId || !draggingType) return;

    const dx = e.clientX - pointerDownX;
    const dy = e.clientY - pointerDownY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (!dragActive) {
      if (dist < DRAG_THRESHOLD) return;
      dragActive = true;
      document.body.setAttribute('data-card-dragging', 'true');
      pendingEl.classList.add('dragging');

      if (draggingIds.length > 1) {
        draggingIds.forEach(id => {
          const el = container.querySelector(`.mod-card[data-id="${id}"]`);
          if (el) el.classList.add('dragging');
        });
      }

      const state = getState();
      const isMulti = draggingIds.length > 1;
      const ghostLabel = isMulti
        ? `${draggingIds.length} mods`
        : (pendingEl.querySelector('.mod-card-name')?.textContent?.trim() || (draggingType === 'folder' ? 'Folder' : 'Mod'));

      ghost = document.createElement('div');
      ghost.style.cssText = `
        position: fixed; pointer-events: none; z-index: 99999;
        background: rgba(0, 120, 212, 0.25); border: 1px solid var(--accent);
        color: var(--text-primary); font-size: 12px; padding: 6px 14px;
        border-radius: 6px; backdrop-filter: blur(8px); white-space: nowrap;
        box-shadow: 0 8px 24px rgba(0,0,0,0.6);
        font-weight: 600; display: flex; align-items: center; gap: 6px;
      `;
      
      const countBadge = isMulti
        ? `<span style="background: var(--accent); color: #fff; font-size: 10px; padding: 2px 6px; border-radius: 10px; font-weight: bold;">${draggingIds.length}</span>` 
        : '';

      ghost.innerHTML = `${draggingType === 'folder' ? '📁' : '📦'} <span>${escapeHtml(ghostLabel)}</span> ${isMulti ? '' : countBadge}`;
      document.body.appendChild(ghost);
    }

    if (ghost) {
      ghost.style.left = `${e.clientX + 14}px`;
      ghost.style.top = `${e.clientY + 10}px`;
    }

    clearAllHighlights();

    // 1. Check Root Drop Zone (for moving mods out of folder)
    if (draggingType === 'mod' && rootDropZone) {
      const rootRect = rootDropZone.getBoundingClientRect();
      if (e.clientX >= rootRect.left && e.clientX <= rootRect.right &&
        e.clientY >= rootRect.top && e.clientY <= rootRect.bottom) {
        rootDropZone.classList.add('drag-target-hover');
        activeRootTarget = rootDropZone;
        return;
      }
    }

    // 2. Check Folder Targets
    const state = getState();
    const isGrid = state.viewLayout === 'grid';

    for (const fc of folderCards) {
      const rect = fc.getBoundingClientRect();
      if (e.clientX >= rect.left && e.clientX <= rect.right &&
        e.clientY >= rect.top && e.clientY <= rect.bottom) {
        const targetFolderId = fc.dataset.id;

        // Mod dragged over Folder (Insert mod into folder)
        if (draggingType === 'mod') {
          fc.classList.add('drag-target-hover');
          activeFolderTarget = fc;
          return;
        }

        // Folder dragged over Folder (Reorder folders)
        if (draggingType === 'folder' && targetFolderId !== draggingId) {
          const isBefore = isGrid 
            ? e.clientX < (rect.left + rect.width / 2)
            : e.clientY < (rect.top + rect.height / 2);

          if (isBefore) {
            fc.classList.add('drag-reorder-before');
          } else {
            fc.classList.add('drag-reorder-after');
          }
          activeReorderTarget = { el: fc, before: isBefore };
          return;
        }
      }
    }
  };

  // ─── Window Pointer Up (Drop Action) ───
  const onPointerUp = async (e: PointerEvent) => {
    if (!pendingEl) return;

    const wasDragging = dragActive;
    const capturedId = draggingId;
    const capturedIds = [...draggingIds];
    const capturedType = draggingType;
    const capturedFolderTarget = activeFolderTarget;
    const capturedReorderTarget = activeReorderTarget;
    const capturedRootTarget = activeRootTarget;

    try { pendingEl.releasePointerCapture(e.pointerId); } catch { }

    cleanup();

    if (!wasDragging || !capturedId || !capturedType) return;
    e.preventDefault();

    // Action A: Mod dropped into Root Drop Zone (remove from current folder)
    if (capturedType === 'mod' && capturedRootTarget) {
      await handleAddMultipleModsToFolder(null, capturedIds);
      return;
    }

    // Action B: Mod dropped onto a Folder Card (assign to folder)
    if (capturedType === 'mod' && capturedFolderTarget) {
      const targetFolderId = capturedFolderTarget.dataset.id;
      if (targetFolderId) {
        await handleAddMultipleModsToFolder(targetFolderId, capturedIds);
      }
      return;
    }

    // Action C: Folder dropped onto another Folder Card (reorder folders)
    if (capturedType === 'folder' && capturedReorderTarget) {
      const targetFolderId = capturedReorderTarget.el.dataset.id;
      if (!targetFolderId || targetFolderId === capturedId) return;

      const state = getState();
      const currentProfile = state.profiles.find(p => p.id === state.currentProfileId);
      if (!currentProfile || !currentProfile.mod_folders) return;

      const folders = [...currentProfile.mod_folders];
      const sourceIndex = folders.findIndex(f => f.id === capturedId);
      const targetIndex = folders.findIndex(f => f.id === targetFolderId);

      if (sourceIndex === -1 || targetIndex === -1) return;

      const [moved] = folders.splice(sourceIndex, 1);
      const newTargetIndex = folders.findIndex(f => f.id === targetFolderId);
      const finalIndex = capturedReorderTarget.before ? newTargetIndex : newTargetIndex + 1;
      folders.splice(finalIndex, 0, moved);

      const newFolderIds = folders.map(f => f.id);
      try {
        const updatedProfile = await reorderModFolders(state.currentProfileId, newFolderIds);
        const updatedProfiles = state.profiles.map(p => p.id === state.currentProfileId ? updatedProfile : p);
        updateState({ profiles: updatedProfiles });
        await loadMods();
      } catch (err) {
        showToast(String(err), 'error');
      }
    }
  };

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', onPointerUp);
  window.addEventListener('pointercancel', () => cleanup());
}

