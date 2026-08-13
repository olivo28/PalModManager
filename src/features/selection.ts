import { getState, updateState } from '../state';
import { renderModsView, loadMods } from '../ui/modsView';
import { enableMod, disableMod, removeMod, setModProfileState } from '../api';
import { showToast } from '../ui/toast';
import { showConfirm } from '../ui/confirm';

let isDragging = false;
let startX = 0;
let startY = 0;
let lastSelectedId: string | null = null; // For Shift-click range selection

let activeDragTargetContainer: HTMLElement | null = null;

export function setupSelection(): void {
  const modsContainer = document.getElementById('mods-container');
  const libContainer = document.getElementById('library-container');
  const dragBox = document.getElementById('drag-select-box');
  if (!dragBox) return;

  const handleMouseDown = (e: MouseEvent, targetContainer: HTMLElement, isLibrary: boolean) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;

    if (target.classList.contains('library-card-checkbox') || target.classList.contains('card-checkbox')) {
      const card = target.closest('.mod-card') as HTMLElement | null;
      if (card) {
        const id = card.dataset.id!;
        if (isLibrary) {
          const selectedIds = new Set(getState().selectedLibraryIds);
          if (selectedIds.has(id)) {
            selectedIds.delete(id);
          } else {
            selectedIds.add(id);
          }
          updateState({ selectedLibraryIds: selectedIds });
          const libCards = document.querySelectorAll('.library-card');
          libCards.forEach((cardEl) => {
            const cId = (cardEl as HTMLElement).dataset.id!;
            const isSel = selectedIds.has(cId);
            cardEl.classList.toggle('selected', isSel);
            const chk = cardEl.querySelector('.library-card-checkbox') as HTMLInputElement | null;
            if (chk) chk.checked = isSel;
          });
          import('../ui/modsView').then(({ updateLibraryBulkBar }) => {
            updateLibraryBulkBar();
          });
        } else {
          const selectedIds = new Set(getState().selectedModIds);
          if (selectedIds.has(id)) {
            selectedIds.delete(id);
          } else {
            selectedIds.add(id);
          }
          updateSelection(selectedIds);
        }
      }
      e.stopPropagation();
      e.preventDefault();
      return;
    }

    if (target.closest('.toggle-switch') || target.closest('.card-remove-btn') || target.closest('select') || target.closest('input') || target.closest('button')) {
      return;
    }

    const card = target.closest('.mod-card') as HTMLElement | null;
    if (card) {
      if (isLibrary) {
        handleLibraryCardClick(card, e);
      } else {
        handleCardClick(card, e);
      }
      return;
    }

    if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
      clearSelection();
    }

    isDragging = true;
    activeDragTargetContainer = targetContainer;
    startX = e.pageX;
    startY = e.pageY;

    dragBox.style.left = `${startX}px`;
    dragBox.style.top = `${startY}px`;
    dragBox.style.width = '0px';
    dragBox.style.height = '0px';
    dragBox.style.display = 'block';

    e.preventDefault();
  };

  if (modsContainer) {
    modsContainer.addEventListener('mousedown', (e) => handleMouseDown(e, modsContainer, false));
  }
  if (libContainer) {
    libContainer.addEventListener('mousedown', (e) => handleMouseDown(e, libContainer, true));
  }

  document.addEventListener('mousemove', (e) => {
    if (!isDragging || !dragBox || !activeDragTargetContainer) return;

    const currentX = e.pageX;
    const currentY = e.pageY;

    const left = Math.min(startX, currentX);
    const top = Math.min(startY, currentY);
    const width = Math.abs(startX - currentX);
    const height = Math.abs(startY - currentY);

    dragBox.style.left = `${left}px`;
    dragBox.style.top = `${top}px`;
    dragBox.style.width = `${width}px`;
    dragBox.style.height = `${height}px`;

    const boxRect = {
      left,
      top,
      right: left + width,
      bottom: top + height
    };

    const isLibrary = activeDragTargetContainer.id === 'library-container';
    const cards = activeDragTargetContainer.querySelectorAll(isLibrary ? '.library-card' : '.mod-card:not(.library-card)');
    const selectedIds = new Set(e.ctrlKey || e.metaKey ? (isLibrary ? getState().selectedLibraryIds : getState().selectedModIds) : []);

    cards.forEach((cardEl) => {
      const card = cardEl as HTMLElement;
      const id = card.dataset.id!;
      const rect = card.getBoundingClientRect();
      const cardRect = {
        left: rect.left + window.scrollX,
        top: rect.top + window.scrollY,
        right: rect.left + window.scrollX + rect.width,
        bottom: rect.top + window.scrollY + rect.height
      };

      const isIntersecting = !(
        boxRect.right < cardRect.left ||
        boxRect.left > cardRect.right ||
        boxRect.bottom < cardRect.top ||
        boxRect.top > cardRect.bottom
      );

      if (isIntersecting) {
        selectedIds.add(id);
      } else if (!e.ctrlKey && !e.metaKey) {
        selectedIds.delete(id);
      }
    });

    if (isLibrary) {
      updateState({ selectedLibraryIds: selectedIds });
      const libCards = activeDragTargetContainer.querySelectorAll('.library-card');
      libCards.forEach((cardEl) => {
        const card = cardEl as HTMLElement;
        const id = card.dataset.id!;
        const isSel = selectedIds.has(id);
        card.classList.toggle('selected', isSel);
        const chk = card.querySelector('.library-card-checkbox') as HTMLInputElement | null;
        if (chk) chk.checked = isSel;
      });
      import('../ui/modsView').then(({ updateLibraryBulkBar }) => {
        updateLibraryBulkBar();
      });
    } else {
      updateSelection(selectedIds);
    }
  });

  document.addEventListener('mouseup', () => {
    if (isDragging) {
      isDragging = false;
      activeDragTargetContainer = null;
      if (dragBox) dragBox.style.display = 'none';
    }
  });

  // Setup Bulk Action button handlers
  document.getElementById('bulk-enable-btn')?.addEventListener('click', () => handleBulkEnable(true));
  document.getElementById('bulk-disable-btn')?.addEventListener('click', () => handleBulkEnable(false));
  document.getElementById('bulk-remove-btn')?.addEventListener('click', handleBulkRemove);
  document.getElementById('bulk-clear-btn')?.addEventListener('click', () => clearSelection());
}

function handleLibraryCardClick(card: HTMLElement, e: MouseEvent): void {
  const id = card.dataset.id!;
  const state = getState();
  const selectedIds = new Set(state.selectedLibraryIds);

  if (e.ctrlKey || e.metaKey) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
    }
  } else {
    if (selectedIds.has(id) && selectedIds.size === 1) {
      selectedIds.clear();
    } else {
      selectedIds.clear();
      selectedIds.add(id);
    }
  }

  updateState({ selectedLibraryIds: selectedIds });
  const libCards = document.querySelectorAll('.library-card');
  libCards.forEach((cardEl) => {
    const cId = (cardEl as HTMLElement).dataset.id!;
    const isSel = selectedIds.has(cId);
    cardEl.classList.toggle('selected', isSel);
    const chk = cardEl.querySelector('.library-card-checkbox') as HTMLInputElement | null;
    if (chk) chk.checked = isSel;
  });
  import('../ui/modsView').then(({ updateLibraryBulkBar }) => {
    updateLibraryBulkBar();
  });
}

function handleCardClick(card: HTMLElement, e: MouseEvent): void {
  const id = card.dataset.id!;
  const state = getState();
  const selectedIds = new Set(state.selectedModIds);

  if (e.shiftKey && lastSelectedId) {
    const cards = Array.from(document.querySelectorAll('.mod-card:not(.library-card)')) as HTMLElement[];
    const idx1 = cards.findIndex(c => c.dataset.id === lastSelectedId);
    const idx2 = cards.findIndex(c => c.dataset.id === id);
    if (idx1 !== -1 && idx2 !== -1) {
      const start = Math.min(idx1, idx2);
      const end = Math.max(idx1, idx2);
      if (!e.ctrlKey && !e.metaKey) {
        selectedIds.clear();
      }
      for (let i = start; i <= end; i++) {
        selectedIds.add(cards[i].dataset.id!);
      }
    }
  } else if (e.ctrlKey || e.metaKey) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
      lastSelectedId = id;
    }
  } else {
    if (selectedIds.has(id) && selectedIds.size === 1) {
      selectedIds.clear();
      lastSelectedId = null;
    } else {
      selectedIds.clear();
      selectedIds.add(id);
      lastSelectedId = id;
    }
  }

  updateSelection(selectedIds);
}

export function updateSelection(selectedIds: Set<string>): void {
  updateState({ selectedModIds: selectedIds });

  const cards = document.querySelectorAll('.mod-card:not(.library-card)');
  cards.forEach((cardEl) => {
    const card = cardEl as HTMLElement;
    const id = card.dataset.id!;
    card.classList.toggle('selected', selectedIds.has(id));
  });

  const bar = document.getElementById('bulk-actions-bar');
  const countSpan = document.getElementById('bulk-selected-count');
  if (bar && countSpan) {
    if (selectedIds.size > 1) {
      countSpan.textContent = selectedIds.size.toString();
      bar.style.display = 'flex';
    } else {
      bar.style.display = 'none';
    }
  }
}

export function clearSelection(): void {
  updateSelection(new Set<string>());
  updateState({ selectedLibraryIds: new Set<string>() });
  const libCards = document.querySelectorAll('.library-card');
  libCards.forEach((cardEl) => {
    cardEl.classList.remove('selected');
    const chk = cardEl.querySelector('.library-card-checkbox') as HTMLInputElement | null;
    if (chk) chk.checked = false;
  });
  import('../ui/modsView').then(({ updateLibraryBulkBar }) => {
    updateLibraryBulkBar();
  });
  lastSelectedId = null;
}

async function handleBulkEnable(enable: boolean): Promise<void> {
  const ids = Array.from(getState().selectedModIds);
  if (ids.length === 0) return;

  showToast(`${enable ? 'Enabling' : 'Disabling'} ${ids.length} mods...`, 'info');
  let successCount = 0;

  for (const id of ids) {
    try {
      if (enable) {
        await enableMod(id);
      } else {
        await disableMod(id);
      }
      try { await setModProfileState(id, enable); } catch {}
      successCount++;
    } catch (err) {
      console.error(`Failed to toggle mod ${id}:`, err);
    }
  }

  showToast(`Successfully updated ${successCount}/${ids.length} mods`, 'success');
  clearSelection();
  await loadMods();
}

async function handleBulkRemove(): Promise<void> {
  const ids = Array.from(getState().selectedModIds);
  if (ids.length === 0) return;

  const confirmed = await showConfirm(`Remove all ${ids.length} selected mods permanently?`);
  if (!confirmed) return;

  showToast(`Removing ${ids.length} mods...`, 'info');
  let successCount = 0;

  for (const id of ids) {
    try {
      await removeMod(id);
      successCount++;
    } catch (err) {
      console.error(`Failed to remove mod ${id}:`, err);
    }
  }

  showToast(`Successfully removed ${successCount}/${ids.length} mods`, 'success');
  clearSelection();
  await loadMods();
}
