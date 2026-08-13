import { getState, updateState } from '../../state';
import { loadMods } from './loader';
import { showToast } from '../toast';
import { handleAddModToFolder } from './events';

export function setupCardDragToFolder(container: HTMLElement): void {
  const modCards = Array.from(container.querySelectorAll('.mod-card:not(.folder-card)')) as HTMLElement[];
  const folderCards = Array.from(container.querySelectorAll('.mod-card.folder-card')) as HTMLElement[];
  const rootDropZone = container.querySelector('#mod-root-drop-zone') as HTMLElement | null;

  if (folderCards.length === 0 && !rootDropZone) return;

  let draggingModId: string | null = null;
  let ghost: HTMLElement | null = null;
  let activeFolderTarget: HTMLElement | null = null;
  let activeRootTarget: HTMLElement | null = null;

  let folderRects: { el: HTMLElement; rect: DOMRect; id: string }[] = [];
  let rootZoneRect: DOMRect | null = null;

  const DRAG_THRESHOLD = 6;
  let pointerDownX = 0;
  let pointerDownY = 0;
  let pendingModId: string | null = null;
  let pendingEl: HTMLElement | null = null;
  let dragActive = false;

  function getFolderAt(x: number, y: number): { el: HTMLElement; id: string | null } | null {
    for (const fr of folderRects) {
      if (x >= fr.rect.left && x <= fr.rect.right &&
        y >= fr.rect.top && y <= fr.rect.bottom) {
        return { el: fr.el, id: fr.id };
      }
    }
    if (rootZoneRect && rootDropZone) {
      if (x >= rootZoneRect.left && x <= rootZoneRect.right &&
        y >= rootZoneRect.top && y <= rootZoneRect.bottom) {
        return { el: rootDropZone, id: null };
      }
    }
    return null;
  }

  function clearFolderHighlights(): void {
    if (activeFolderTarget) {
      activeFolderTarget.style.outline = '';
      activeFolderTarget.style.boxShadow = '';
      activeFolderTarget = null;
    }
    if (activeRootTarget) {
      activeRootTarget.style.background = '';
      activeRootTarget.style.borderColor = '';
      activeRootTarget = null;
    }
  }

  function cleanup(): void {
    if (ghost) { ghost.remove(); ghost = null; }
    clearFolderHighlights();
    folderCards.forEach(fc => {
      fc.style.outline = '';
      fc.style.boxShadow = '';
    });
    if (rootDropZone) {
      rootDropZone.style.background = '';
      rootDropZone.style.borderColor = '';
    }
    draggingModId = null;
    pendingModId = null;
    pendingEl = null;
    dragActive = false;
    document.body.removeAttribute('data-card-dragging');
  }

  modCards.forEach(card => {
    card.addEventListener('pointerdown', (e: PointerEvent) => {
      if (e.button !== 0) return;
      if ((e.target as HTMLElement).closest('.toggle-switch, .card-toggle-input, button, a, input')) return;

      const modId = card.dataset.id;
      if (!modId || card.dataset.type === 'folder') return;

      pointerDownX = e.clientX;
      pointerDownY = e.clientY;
      pendingModId = modId;
      pendingEl = card;
    });
  });

  container.addEventListener('pointermove', (e: PointerEvent) => {
    if (!pendingModId) return;

    const dx = e.clientX - pointerDownX;
    const dy = e.clientY - pointerDownY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    if (!dragActive) {
      if (dist < DRAG_THRESHOLD) return;

      dragActive = true;
      try { container.setPointerCapture(e.pointerId); } catch { }
      draggingModId = pendingModId;
      document.body.setAttribute('data-card-dragging', 'true');

      folderRects = folderCards.map(fc => ({
        el: fc,
        rect: fc.getBoundingClientRect(),
        id: fc.dataset.id || ''
      }));

      if (rootDropZone) {
        rootZoneRect = rootDropZone.getBoundingClientRect();
      }

      const modName = pendingEl?.querySelector('.mod-card-name')?.textContent || draggingModId || 'Mod';
      ghost = document.createElement('div');
      ghost.style.cssText = `
        position: fixed; pointer-events: none; z-index: 9998;
        background: rgba(0,188,255,0.18); border: 1px solid var(--accent);
        color: var(--text-primary); font-size: 12px; padding: 5px 12px;
        border-radius: 8px; backdrop-filter: blur(6px); white-space: nowrap;
        box-shadow: 0 4px 16px rgba(0,0,0,0.45);
        font-weight: 600;
      `;
      ghost.textContent = `📦 ${modName}`;
      document.body.appendChild(ghost);

      if (pendingEl) pendingEl.style.opacity = '0.45';
    }

    if (ghost) {
      ghost.style.left = (e.clientX + 14) + 'px';
      ghost.style.top = (e.clientY + 8) + 'px';
    }

    clearFolderHighlights();
    const hit = getFolderAt(e.clientX, e.clientY);
    if (hit) {
      if (hit.id === null && rootDropZone) {
        hit.el.style.background = 'rgba(0, 188, 255, 0.1)';
        hit.el.style.borderColor = 'var(--accent)';
        activeRootTarget = hit.el;
      } else {
        hit.el.style.outline = '2px solid var(--accent)';
        hit.el.style.boxShadow = '0 0 16px rgba(0,188,255,0.3)';
        activeFolderTarget = hit.el;
      }
    }
  });

  container.addEventListener('pointerup', async (e: PointerEvent) => {
    if (!pendingModId) return;

    const wasDragging = dragActive;
    if (wasDragging) {
      e.preventDefault();
      try { container.releasePointerCapture(e.pointerId); } catch { }
    }

    const capturedModId = draggingModId;
    const capturedEl = pendingEl;
    cleanup();

    if (!capturedModId || !wasDragging) {
      if (capturedEl) capturedEl.style.opacity = '';
      return;
    }

    if (capturedEl) capturedEl.style.opacity = '';

    const hit = getFolderAt(e.clientX, e.clientY);
    if (!hit) return;

    await handleAddModToFolder(hit.id, capturedModId);
  });

  container.addEventListener('pointercancel', () => {
    if (pendingEl) pendingEl.style.opacity = '';
    cleanup();
  });
}
