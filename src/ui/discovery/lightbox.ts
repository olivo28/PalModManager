import { discState } from './state';
import { discoveryDom } from '../../framework';

export function setupLightboxPanZoom(): void {
  const wrap = discoveryDom.elMaybe('discovery-lightbox-img-wrap');
  if (!wrap) return;

  let dragOriginX = 0;
  let dragOriginY = 0;
  let startPanX = 0;
  let startPanY = 0;

  // Mouse wheel zoom
  wrap.addEventListener('wheel', (e: WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 0.87;
    const newZoom = Math.max(0.5, Math.min(6.0, discState.lightboxZoom * factor));
    updateLightboxTransform(newZoom, discState.lightboxPanX, discState.lightboxPanY);
  }, { passive: false });

  // Drag to pan
  wrap.addEventListener('mousedown', (e: MouseEvent) => {
    if (e.button !== 0) return; // Only left click
    e.preventDefault();
    discState.isPanning = true;
    dragOriginX = e.clientX;
    dragOriginY = e.clientY;
    startPanX = discState.lightboxPanX;
    startPanY = discState.lightboxPanY;
    wrap.classList.add('panning');
  });

  window.addEventListener('mousemove', (e: MouseEvent) => {
    if (!discState.isPanning) return;
    e.preventDefault();
    const dx = e.clientX - dragOriginX;
    const dy = e.clientY - dragOriginY;
    discState.lightboxPanX = startPanX + dx;
    discState.lightboxPanY = startPanY + dy;
    updateLightboxTransform(discState.lightboxZoom, discState.lightboxPanX, discState.lightboxPanY);
  });

  const stopPanning = () => {
    if (discState.isPanning) {
      discState.isPanning = false;
      wrap?.classList.remove('panning');
    }
  };

  window.addEventListener('mouseup', stopPanning);
  window.addEventListener('pointerup', stopPanning);
  window.addEventListener('mouseleave', stopPanning);
  window.addEventListener('dragend', stopPanning);

  // Double click toggle zoom
  wrap.addEventListener('dblclick', (e: MouseEvent) => {
    e.preventDefault();
    if (discState.lightboxZoom > 1.2) {
      updateLightboxTransform(1, 0, 0);
    } else {
      updateLightboxTransform(2.2, 0, 0);
    }
  });
}

export function updateLightboxTransform(zoom: number, panX: number, panY: number): void {
  discState.lightboxZoom = Math.max(0.5, Math.min(6.0, zoom));
  discState.lightboxPanX = panX;
  discState.lightboxPanY = panY;

  const img = discoveryDom.elMaybe('discovery-lightbox-img');
  if (img) {
    img.style.transform = `translate(${discState.lightboxPanX}px, ${discState.lightboxPanY}px) scale(${discState.lightboxZoom})`;
  }

  const resetBtn = discoveryDom.elMaybe('discovery-lightbox-zoom-reset');
  if (resetBtn) {
    resetBtn.textContent = `${Math.round(discState.lightboxZoom * 100)}%`;
  }
}

export function openLightbox(src: string): void {
  const lightbox = discoveryDom.elMaybe('discovery-image-modal');
  const img = discoveryDom.elMaybe('discovery-lightbox-img');
  if (!lightbox || !img) return;

  img.src = src;
  img.setAttribute('data-original-src', src);
  img.onerror = () => {
    if ((window as any).handleUniversalImageFallback) {
      (window as any).handleUniversalImageFallback(img);
    }
  };
  updateLightboxTransform(1, 0, 0);

  lightbox.classList.add('visible');
  lightbox.style.display = 'flex';
}

export function closeLightbox(): void {
  const lightbox = discoveryDom.elMaybe('discovery-image-modal');
  if (lightbox) {
    lightbox.classList.remove('visible');
    lightbox.style.display = 'none';
  }
  const img = discoveryDom.elMaybe('discovery-lightbox-img');
  if (img) img.src = '';
  updateLightboxTransform(1, 0, 0);
}
