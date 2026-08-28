export function getContextOverlay(): HTMLElement {
  return document.getElementById('context-overlay')!;
}

export function hideContextMenu(): void {
  const overlay = getContextOverlay();
  overlay.classList.remove('visible');
  const menu = document.getElementById('context-menu')!;
  menu.style.display = 'none';
  menu.innerHTML = '';
  document.querySelectorAll('.mod-card.context-active').forEach(el => el.classList.remove('context-active'));
}

export function positionContextMenu(x: number, y: number): void {
  const overlay = getContextOverlay();
  const menu = document.getElementById('context-menu')!;
  overlay.classList.add('visible');
  menu.style.display = 'block';
  menu.style.visibility = 'hidden';

  requestAnimationFrame(() => {
    const rect = menu.getBoundingClientRect();
    const width = rect.width || 200;
    const height = rect.height || 200;

    const posX = Math.max(10, Math.min(x, window.innerWidth - width - 10));
    const posY = Math.max(10, Math.min(y, window.innerHeight - height - 10));

    menu.style.left = `${posX}px`;
    menu.style.top = `${posY}px`;
    menu.style.visibility = 'visible';
    menu.focus();
  });
}

export function isAnyModalActive(): boolean {
  const visibleOverlay = document.querySelector('.modal-overlay.visible, .detail-overlay.visible, .confirm-overlay, .detail-overlay[style*="display: flex"]');
  return !!visibleOverlay;
}
