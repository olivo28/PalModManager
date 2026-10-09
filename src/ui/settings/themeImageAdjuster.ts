import type { PmmImageCustomAdjust } from '../../app/theme/types';
import { applyTheme, getActiveThemeId, findThemeById } from '../../app/theme';
import { t } from '../../utils/i18n';
import themeImageAdjustModalHtml from '../../templates/modals/themeImageAdjustModal.html?raw';

interface AdjustModalState {
  target: 'app' | 'sidebar' | 'editor';
  imageUrl: string;
  posX: number;
  posY: number;
  scale: number;
  stretchX: number;
  stretchY: number;
  isStretch: boolean;
  originalPos: string;
  originalFit: string;
  originalImage: string;
  onApply: (adjust: PmmImageCustomAdjust) => void;
  onCancel?: () => void;
}

let _state: AdjustModalState | null = null;
let _activeMode: 'crop' | 'live-sidebar' = (localStorage.getItem('pmm_sidebar_adjust_mode') as 'crop' | 'live-sidebar') || 'crop';
let _isDragging = false;
let _dragStartX = 0;
let _dragStartY = 0;
let _dragStartPosX = 50;
let _dragStartPosY = 50;
let _imgNaturalW = 1920;
let _imgNaturalH = 1080;

export function setAdjustMode(mode: 'crop' | 'live-sidebar'): void {
  _activeMode = mode;
  try {
    localStorage.setItem('pmm_sidebar_adjust_mode', mode);
  } catch {}

  const modal = document.getElementById('theme-image-adjust-modal');
  const cropBtn = document.getElementById('theme-adjust-mode-crop');
  const liveBtn = document.getElementById('theme-adjust-mode-live');
  const liveBanner = document.getElementById('theme-adjust-live-banner');
  const viewportWrapper = document.getElementById('theme-adjust-viewport-wrapper');
  const hintBar = document.getElementById('theme-adjust-hint-bar');
  const liveSidebar = document.getElementById('sidebar');

  if (cropBtn) cropBtn.classList.toggle('active', mode === 'crop');
  if (liveBtn) liveBtn.classList.toggle('active', mode === 'live-sidebar');

  if (mode === 'live-sidebar') {
    modal?.setAttribute('data-mode', 'live-sidebar');
    if (liveBanner) liveBanner.style.display = 'flex';
    if (viewportWrapper) viewportWrapper.style.display = 'none';
    if (hintBar) hintBar.style.display = 'none';
    liveSidebar?.classList.add('sidebar-adjusting-active');
    document.body.classList.add('theme-adjusting-live-active');
  } else {
    modal?.removeAttribute('data-mode');
    if (liveBanner) liveBanner.style.display = 'none';
    if (viewportWrapper) viewportWrapper.style.display = 'flex';
    if (hintBar) hintBar.style.display = 'flex';
    liveSidebar?.classList.remove('sidebar-adjusting-active');
    document.body.classList.remove('theme-adjusting-live-active');
  }
}

export function closeThemeImageAdjustModal(restore = true): void {
  const modal = document.getElementById('theme-image-adjust-modal');
  if (modal) {
    modal.classList.remove('visible');
    modal.style.display = 'none';
  }
  const liveSidebar = document.getElementById('sidebar');
  liveSidebar?.classList.remove('sidebar-adjusting-active', 'dragging');
  document.body.classList.remove('theme-adjusting-live-active');

  if (restore && _state) {
    const targetPrefix = _state.target === 'app' ? '--pmm-app-bg' : _state.target === 'sidebar' ? '--pmm-sidebar-bg' : '--pmm-editor-bg';
    if (_state.originalImage) {
      document.documentElement.style.setProperty(`${targetPrefix}-image`, _state.originalImage);
      document.documentElement.style.setProperty(`${targetPrefix}-pos`, _state.originalPos);
      document.documentElement.style.setProperty(`${targetPrefix}-fit`, _state.originalFit);
    } else {
      document.documentElement.style.removeProperty(`${targetPrefix}-image`);
      document.documentElement.style.removeProperty(`${targetPrefix}-pos`);
      document.documentElement.style.removeProperty(`${targetPrefix}-fit`);
    }

    try {
      const activeTheme = findThemeById(getActiveThemeId());
      if (activeTheme) applyTheme(activeTheme);
    } catch {}

    _state.onCancel?.();
  }
  _state = null;
}

export function openThemeImageAdjustModal(
  target: 'app' | 'sidebar' | 'editor',
  imageUrl: string,
  initialAdjust: PmmImageCustomAdjust | undefined,
  onApply: (adjust: PmmImageCustomAdjust) => void,
  onCancel?: () => void
): void {
  console.info(`[ThemeImageAdjuster] openThemeImageAdjustModal called for target "${target}", url length: ${imageUrl?.length || 0}`);

  let modal = document.getElementById('theme-image-adjust-modal');
  if (!modal) {
    console.warn('[ThemeImageAdjuster] Modal not found in DOM, dynamically mounting template...');
    const root = document.getElementById('modals-root') || document.body;
    const tempDiv = document.createElement('div');
    tempDiv.innerHTML = themeImageAdjustModalHtml;
    if (tempDiv.firstElementChild) {
      root.appendChild(tempDiv.firstElementChild);
      modal = document.getElementById('theme-image-adjust-modal');
    }
  }

  const img = document.getElementById('theme-adjust-img') as HTMLImageElement | null;
  const title = document.getElementById('theme-adjust-title');
  if (!modal || !img || !imageUrl) {
    console.error('[ThemeImageAdjuster] Aborting - missing requirement:', { modal: !!modal, img: !!img, imageUrl: !!imageUrl });
    return;
  }

  const targetCssVarPrefix = target === 'app' ? '--pmm-app-bg' : target === 'sidebar' ? '--pmm-sidebar-bg' : '--pmm-editor-bg';
  const rootStyle = document.documentElement.style;
  const originalPos = rootStyle.getPropertyValue(`${targetCssVarPrefix}-pos`) || '50% 50%';
  const originalFit = rootStyle.getPropertyValue(`${targetCssVarPrefix}-fit`) || 'cover';
  const originalImage = rootStyle.getPropertyValue(`${targetCssVarPrefix}-image`) || '';

  // Immediately mirror image on live background so user sees it in PalModManager
  rootStyle.setProperty(`${targetCssVarPrefix}-image`, `url("${imageUrl}")`);

  const posX = initialAdjust?.posX ?? 50;
  const posY = initialAdjust?.posY ?? 50;
  const scale = initialAdjust?.scale ?? 100;
  const stretchX = initialAdjust?.stretchX ?? 100;
  const stretchY = initialAdjust?.stretchY ?? 100;
  const isStretch = Boolean(initialAdjust?.stretchX != null && initialAdjust?.stretchY != null);

  _state = {
    target,
    imageUrl,
    posX,
    posY,
    scale,
    stretchX,
    stretchY,
    isStretch,
    originalPos,
    originalFit,
    originalImage,
    onApply,
    onCancel,
  };

  // Set modal title based on target
  if (title) {
    const titleKey = target === 'app' ? 'theme.adjust_title_app' : target === 'sidebar' ? 'theme.adjust_title_sidebar' : 'theme.adjust_title_editor';
    const fallback = target === 'app' ? 'Adjust App Background' : target === 'sidebar' ? 'Adjust Sidebar Banner' : 'Adjust Editor Watermark';
    title.textContent = t(titleKey) || fallback;
  }

  // Configure viewport aspect ratio and overlays based on destination
  const viewport = document.getElementById('theme-adjust-viewport');
  const sidebarOverlay = document.getElementById('theme-adjust-sidebar-overlay');
  const cropFrameYellow = document.getElementById('theme-adjust-crop-frame-yellow');
  const gridOverlay = document.getElementById('theme-adjust-grid-overlay');

  if (viewport) {
    if (target === 'sidebar') {
      const viewH = 480;
      const viewW = 62;
      viewport.style.aspectRatio = 'unset';
      viewport.style.width = `${viewW}px`;
      viewport.style.height = `${viewH}px`;
      viewport.style.maxWidth = `${viewW}px`;
      viewport.style.margin = '0 auto';
      if (sidebarOverlay) sidebarOverlay.style.display = 'flex';
      if (cropFrameYellow) cropFrameYellow.style.display = 'none';
      if (gridOverlay) gridOverlay.style.display = 'none';

      // Mirror live sidebar state from main UI
      const liveDiscoveryTab = document.getElementById('sidebar-tab-discovery');
      const mockDiscoveryTab = document.getElementById('mock-tab-discovery');
      if (mockDiscoveryTab) {
        mockDiscoveryTab.style.display = (liveDiscoveryTab && liveDiscoveryTab.style.display !== 'none') ? 'flex' : 'none';
      }

      const liveNexusWidget = document.getElementById('sidebar-nexus-widget');
      const mockNexusWidget = document.getElementById('mock-sidebar-nexus-widget');
      if (mockNexusWidget) mockNexusWidget.style.display = (liveNexusWidget && liveNexusWidget.style.display !== 'none') ? 'flex' : 'none';

      const liveAvatar = document.getElementById('sidebar-nexus-avatar') as HTMLImageElement | null;
      const mockAvatar = document.getElementById('mock-sidebar-nexus-avatar') as HTMLImageElement | null;
      if (mockAvatar && liveAvatar?.src) mockAvatar.src = liveAvatar.src;

      const liveName = document.getElementById('sidebar-nexus-name');
      const mockName = document.getElementById('mock-sidebar-nexus-name');
      if (mockName && liveName?.textContent) mockName.textContent = liveName.textContent;

      const liveTier = document.getElementById('sidebar-nexus-tier');
      const mockTier = document.getElementById('mock-sidebar-nexus-tier');
      if (mockTier && liveTier?.textContent) mockTier.textContent = liveTier.textContent;

      const liveVersion = document.getElementById('sidebar-version');
      const mockVersion = document.getElementById('mock-sidebar-version');
      if (mockVersion && liveVersion?.textContent) mockVersion.textContent = liveVersion.textContent;
    } else {
      // App background or Monaco editor watermark
      viewport.style.aspectRatio = 'unset';
      viewport.style.width = '100%';
      viewport.style.height = '320px';
      viewport.style.maxHeight = '320px';
      viewport.style.maxWidth = '100%';
      viewport.style.margin = '0 auto';
      if (sidebarOverlay) sidebarOverlay.style.display = 'none';
      if (cropFrameYellow) cropFrameYellow.style.display = 'block';
      if (gridOverlay) gridOverlay.style.display = 'none';
    }
  }

  // Pre-load image and sync fields
  img.onload = () => {
    _imgNaturalW = img.naturalWidth || 1920;
    _imgNaturalH = img.naturalHeight || 1080;
    syncControlsFromState();
    renderViewport();
  };
  img.src = imageUrl;

  const modeSwitcher = document.getElementById('theme-adjust-mode-switcher');
  if (modeSwitcher) {
    if (target === 'sidebar') {
      modeSwitcher.style.display = 'flex';
      setAdjustMode(_activeMode);
    } else {
      modeSwitcher.style.display = 'none';
      setAdjustMode('crop');
    }
  }

  initModalEventsOnce();
  modal.classList.add('visible');
  modal.style.display = 'flex';
}

function syncControlsFromState(): void {
  if (!_state) return;
  const setPair = (sliderId: string, inputId: string, val: number) => {
    const s = document.getElementById(sliderId) as HTMLInputElement | null;
    const i = document.getElementById(inputId) as HTMLInputElement | null;
    const str = String(Math.round(val));
    if (s) s.value = str;
    if (i) i.value = str;
  };
  setPair('theme-adjust-zoom-slider', 'theme-adjust-zoom-input', _state.scale);
  setPair('theme-adjust-posx-slider', 'theme-adjust-posx-input', _state.posX);
  setPair('theme-adjust-posy-slider', 'theme-adjust-posy-input', _state.posY);
  setPair('theme-adjust-stretch-w-slider', 'theme-adjust-stretch-w-input', _state.stretchX);
  setPair('theme-adjust-stretch-h-slider', 'theme-adjust-stretch-h-input', _state.stretchY);
}

function renderViewport(): void {
  if (!_state) return;
  const viewport = document.getElementById('theme-adjust-viewport');
  const img = document.getElementById('theme-adjust-img') as HTMLImageElement | null;
  if (!viewport || !img) return;

  const vw = viewport.clientWidth || 600;
  const vh = viewport.clientHeight || 300;

  let realRenderW: number;
  let realRenderH: number;
  let modalW: number;
  let modalH: number;
  let posVal: string;

  if (_state.target === 'sidebar') {
    const liveSidebar = document.getElementById('sidebar');
    const realH = liveSidebar?.clientHeight || window.innerHeight || 900;
    const naturalRatio = _imgNaturalW / (_imgNaturalH || 1);

    // Real dimensions on the actual sidebar (covering height):
    const baseH = realH;
    const baseW = realH * naturalRatio;

    realRenderW = Math.round(baseW * (_state.scale / 100) * (_state.stretchX / 100));
    realRenderH = Math.round(baseH * (_state.scale / 100) * (_state.stretchY / 100));

    // Calculate unscaled real pixel offsets directly on native sidebar (62px x realH):
    const realLeft = Math.round((62 - realRenderW) * (_state.posX / 100));
    const realTop = Math.round((realH - realRenderH) * (_state.posY / 100));
    posVal = `${realLeft}px ${realTop}px`;

    // Exact geometric scaling factor k into the modal preview:
    const k = vh / realH;
    modalW = Math.round(realRenderW * k);
    modalH = Math.round(realRenderH * k);

    // Coordinate projections into the modal preview viewport:
    const left = Math.round(realLeft * k);
    const top = Math.round(realTop * k);

    img.style.width = `${modalW}px`;
    img.style.height = `${modalH}px`;
    img.style.left = `${left}px`;
    img.style.top = `${top}px`;
  } else {
    // App background or Monaco editor watermark
    const containerW = _state.target === 'app' ? (window.innerWidth || 1920) : 1200;
    const containerH = _state.target === 'app' ? (window.innerHeight || 1080) : 750;
    const baseScale = Math.max(containerW / _imgNaturalW, containerH / _imgNaturalH);

    realRenderW = Math.round(_imgNaturalW * baseScale * (_state.scale / 100) * (_state.stretchX / 100));
    realRenderH = Math.round(_imgNaturalH * baseScale * (_state.scale / 100) * (_state.stretchY / 100));

    // Calibrate yellow crop frame to fit within the viewport canvas
    const cropFrameYellow = document.getElementById('theme-adjust-crop-frame-yellow');
    const maxCropW = Math.max(100, vw - 36);
    const maxCropH = Math.max(100, vh - 36);
    const targetRatio = containerW / containerH;

    let cw = maxCropW;
    let ch = Math.round(cw / targetRatio);
    if (ch > maxCropH) {
      ch = maxCropH;
      cw = Math.round(ch * targetRatio);
    }

    if (cropFrameYellow) {
      cropFrameYellow.style.width = `${cw}px`;
      cropFrameYellow.style.height = `${ch}px`;
    }

    const k = cw / containerW;
    modalW = Math.round(realRenderW * k);
    modalH = Math.round(realRenderH * k);

    const cropLeft = Math.round((vw - cw) / 2);
    const cropTop = Math.round((vh - ch) / 2);

    const left = Math.round(cropLeft + (cw - modalW) * (_state.posX / 100));
    const top = Math.round(cropTop + (ch - modalH) * (_state.posY / 100));

    img.style.width = `${modalW}px`;
    img.style.height = `${modalH}px`;
    img.style.left = `${left}px`;
    img.style.top = `${top}px`;

    posVal = `${Math.round(_state.posX)}% ${Math.round(_state.posY)}%`;
  }

  // Project real-time exact pixel dimensions on app workspace
  const targetPrefix = _state.target === 'app' ? '--pmm-app-bg' : _state.target === 'sidebar' ? '--pmm-sidebar-bg' : '--pmm-editor-bg';
  const fitVal = `${realRenderW}px ${realRenderH}px`;

  document.documentElement.style.setProperty(`${targetPrefix}-pos`, posVal);
  document.documentElement.style.setProperty(`${targetPrefix}-fit`, fitVal);

  // Update real-time render telemetry badges
  const telemetryRender = document.getElementById('theme-telemetry-render');
  const telemetryOriginal = document.getElementById('theme-telemetry-original');
  if (telemetryRender) {
    telemetryRender.textContent = `📐 Render: ${realRenderW} × ${realRenderH} px`;
  }
  if (telemetryOriginal) {
    telemetryOriginal.textContent = `Original: ${_imgNaturalW} × ${_imgNaturalH} px`;
  }
}

let _eventsInitialized = false;

function initModalEventsOnce(): void {
  if (_eventsInitialized) return;
  _eventsInitialized = true;

  const modal = document.getElementById('theme-image-adjust-modal');
  const viewport = document.getElementById('theme-adjust-viewport');
  const closeBtn = document.getElementById('theme-adjust-close-x');
  const cancelBtn = document.getElementById('theme-adjust-cancel-btn');
  const applyBtn = document.getElementById('theme-adjust-apply-btn');

  const zoomSlider = document.getElementById('theme-adjust-zoom-slider') as HTMLInputElement | null;
  const posxSlider = document.getElementById('theme-adjust-posx-slider') as HTMLInputElement | null;
  const posySlider = document.getElementById('theme-adjust-posy-slider') as HTMLInputElement | null;
  const stretchWSlider = document.getElementById('theme-adjust-stretch-w-slider') as HTMLInputElement | null;
  const stretchHSlider = document.getElementById('theme-adjust-stretch-h-slider') as HTMLInputElement | null;

  closeBtn?.addEventListener('click', () => closeThemeImageAdjustModal(true));
  cancelBtn?.addEventListener('click', () => closeThemeImageAdjustModal(true));
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) closeThemeImageAdjustModal(true);
  });

  // Apply Handler
  applyBtn?.addEventListener('click', () => {
    if (!_state) return;
    const result: PmmImageCustomAdjust = {
      posX: Math.round(_state.posX),
      posY: Math.round(_state.posY),
      scale: Math.round(_state.scale),
      stretchX: _state.isStretch ? Math.round(_state.stretchX) : undefined,
      stretchY: _state.isStretch ? Math.round(_state.stretchY) : undefined,
    };
    _state.onApply(result);
    closeThemeImageAdjustModal(false);
  });

  const zoomInput = document.getElementById('theme-adjust-zoom-input') as HTMLInputElement | null;
  const posxInput = document.getElementById('theme-adjust-posx-input') as HTMLInputElement | null;
  const posyInput = document.getElementById('theme-adjust-posy-input') as HTMLInputElement | null;
  const stretchWInput = document.getElementById('theme-adjust-stretch-w-input') as HTMLInputElement | null;
  const stretchHInput = document.getElementById('theme-adjust-stretch-h-input') as HTMLInputElement | null;

  // Slider inputs
  zoomSlider?.addEventListener('input', () => {
    if (!_state) return;
    _state.scale = parseFloat(zoomSlider.value);
    if (zoomInput) zoomInput.value = String(Math.round(_state.scale));
    renderViewport();
  });

  posxSlider?.addEventListener('input', () => {
    if (!_state) return;
    _state.posX = parseFloat(posxSlider.value);
    if (posxInput) posxInput.value = String(Math.round(_state.posX));
    renderViewport();
  });

  posySlider?.addEventListener('input', () => {
    if (!_state) return;
    _state.posY = parseFloat(posySlider.value);
    if (posyInput) posyInput.value = String(Math.round(_state.posY));
    renderViewport();
  });

  stretchWSlider?.addEventListener('input', () => {
    if (!_state) return;
    _state.stretchX = parseFloat(stretchWSlider.value);
    _state.isStretch = true;
    if (stretchWInput) stretchWInput.value = String(Math.round(_state.stretchX));
    renderViewport();
  });

  stretchHSlider?.addEventListener('input', () => {
    if (!_state) return;
    _state.stretchY = parseFloat(stretchHSlider.value);
    _state.isStretch = true;
    if (stretchHInput) stretchHInput.value = String(Math.round(_state.stretchY));
    renderViewport();
  });

  // Manual numeric input listeners with 2-way sync
  const wireNumInput = (input: HTMLInputElement | null, slider: HTMLInputElement | null, min: number, max: number, onVal: (v: number) => void) => {
    if (!input) return;
    const update = () => {
      if (!_state) return;
      const num = parseFloat(input.value);
      if (isNaN(num)) return;
      const clamped = Math.min(max, Math.max(min, num));
      onVal(clamped);
      if (slider) slider.value = String(Math.round(clamped));
      renderViewport();
    };
    input.addEventListener('input', update);
    input.addEventListener('change', () => {
      update();
      syncControlsFromState();
    });
  };

  wireNumInput(zoomInput, zoomSlider, 20, 300, (v) => _state!.scale = v);
  wireNumInput(posxInput, posxSlider, 0, 100, (v) => _state!.posX = v);
  wireNumInput(posyInput, posySlider, 0, 100, (v) => _state!.posY = v);
  wireNumInput(stretchWInput, stretchWSlider, 20, 400, (v) => { _state!.stretchX = v; _state!.isStretch = true; });
  wireNumInput(stretchHInput, stretchHSlider, 20, 400, (v) => { _state!.stretchY = v; _state!.isStretch = true; });

  // Quick Action Buttons (Reset only)
  document.getElementById('theme-adjust-btn-reset')?.addEventListener('click', () => {
    if (!_state) return;
    _state.scale = 100;
    _state.posX = 50;
    _state.posY = 50;
    _state.stretchX = 100;
    _state.stretchY = 100;
    _state.isStretch = false;
    syncControlsFromState();
    renderViewport();
  });

  // Viewport Pointer Dragging (Pan)
  viewport?.addEventListener('pointerdown', (e) => {
    if (!_state) return;
    _isDragging = true;
    _dragStartX = e.clientX;
    _dragStartY = e.clientY;
    _dragStartPosX = _state.posX;
    _dragStartPosY = _state.posY;
    viewport.classList.add('dragging');
    viewport.setPointerCapture(e.pointerId);
  });

  viewport?.addEventListener('pointermove', (e) => {
    if (!_isDragging || !_state) return;
    const deltaX = e.clientX - _dragStartX;
    const deltaY = e.clientY - _dragStartY;

    // Invert delta because dragging the canvas right moves the offset left
    const sensitivity = 0.25;
    const newPosX = Math.min(100, Math.max(0, _dragStartPosX - deltaX * sensitivity));
    const newPosY = Math.min(100, Math.max(0, _dragStartPosY - deltaY * sensitivity));

    _state.posX = newPosX;
    _state.posY = newPosY;
    syncControlsFromState();
    renderViewport();
  });

  const stopDrag = () => {
    _isDragging = false;
    viewport?.classList.remove('dragging');
  };

  viewport?.addEventListener('pointerup', stopDrag);
  viewport?.addEventListener('pointercancel', stopDrag);

  // Wheel Zoom
  viewport?.addEventListener('wheel', (e) => {
    if (!_state) return;
    e.preventDefault();
    const step = e.deltaY < 0 ? 5 : -5;
    _state.scale = Math.min(300, Math.max(20, _state.scale + step));
    syncControlsFromState();
    renderViewport();
  }, { passive: false });

  // Mode Switcher handlers
  document.getElementById('theme-adjust-mode-crop')?.addEventListener('click', () => setAdjustMode('crop'));
  document.getElementById('theme-adjust-mode-live')?.addEventListener('click', () => setAdjustMode('live-sidebar'));

  // Live Sidebar Pointer Drag & Wheel handlers
  const liveSidebar = document.getElementById('sidebar');
  if (liveSidebar) {
    liveSidebar.addEventListener('pointerdown', (e) => {
      if (!_state || _state.target !== 'sidebar' || _activeMode !== 'live-sidebar') return;
      _isDragging = true;
      _dragStartX = e.clientX;
      _dragStartY = e.clientY;
      _dragStartPosX = _state.posX;
      _dragStartPosY = _state.posY;
      liveSidebar.classList.add('dragging');
      liveSidebar.setPointerCapture(e.pointerId);
    });

    liveSidebar.addEventListener('pointermove', (e) => {
      if (!_isDragging || !_state || _state.target !== 'sidebar' || _activeMode !== 'live-sidebar') return;
      const deltaX = e.clientX - _dragStartX;
      const deltaY = e.clientY - _dragStartY;

      const sensitivity = 0.15;
      const newPosX = Math.min(100, Math.max(0, _dragStartPosX - deltaX * sensitivity));
      const newPosY = Math.min(100, Math.max(0, _dragStartPosY - deltaY * sensitivity));

      _state.posX = newPosX;
      _state.posY = newPosY;
      syncControlsFromState();
      renderViewport();
    });

    const stopLiveDrag = () => {
      _isDragging = false;
      liveSidebar.classList.remove('dragging');
    };

    liveSidebar.addEventListener('pointerup', stopLiveDrag);
    liveSidebar.addEventListener('pointercancel', stopLiveDrag);

    liveSidebar.addEventListener('wheel', (e) => {
      if (!_state || _state.target !== 'sidebar' || _activeMode !== 'live-sidebar') return;
      e.preventDefault();
      const step = e.deltaY < 0 ? 5 : -5;
      _state.scale = Math.min(300, Math.max(20, _state.scale + step));
      syncControlsFromState();
      renderViewport();
    }, { passive: false });
  }
}
