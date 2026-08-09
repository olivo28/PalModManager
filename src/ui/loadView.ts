import { invoke } from '@tauri-apps/api/core';
import { ModInfo } from '../types';
import { escapeHtml } from '../utils/helpers';
import { getState } from '../state';

let _ue4ssMods: ModInfo[] = [];
let _palschemaMods: ModInfo[] = [];

let _ue4ssAbort: AbortController | null = null;
let _palschemaAbort: AbortController | null = null;

export async function renderLoadView(): Promise<void> {
  const container = document.getElementById('load-list-container');
  if (!container) return;

  const state = getState();
  const showUe4ss = !!state.currentSettings?.forceLoadOrderUe4ss;
  const showPalschema = !!state.currentSettings?.forceLoadOrderPalschema;

  container.innerHTML = `
    <!-- UE4SS Section -->
    <div id="ue4ss-load-section" class="load-section-container" style="flex: 1; display: ${showUe4ss ? 'flex' : 'none'}; flex-direction: column; gap: 10px; min-width: 320px; background: rgba(20,20,20,0.2); border: 1px solid var(--border); border-radius: 8px; padding: 16px; height: 100%;">
      <div style="display:flex; flex-direction:column; gap:4px; border-bottom:1px solid var(--border); padding-bottom:10px; flex-shrink:0;">
        <span style="font-size:14px; font-weight:700; color:var(--text-primary);">UE4SS Mods Load Order</span>
        <span style="font-size:10px; color:var(--text-muted);">Controlled via Mods/mods.txt</span>
      </div>
      <div id="ue4ss-list-subcontainer" style="flex:1; display:flex; flex-direction:column; gap:8px; overflow-y:auto; padding-right:4px;">
        <div style="color:var(--text-muted); font-size:12px; padding:8px;">Loading UE4SS order...</div>
      </div>
    </div>

    <!-- PalSchema Section -->
    <div id="palschema-load-section" class="load-section-container" style="flex: 1; display: ${showPalschema ? 'flex' : 'none'}; flex-direction: column; gap: 10px; min-width: 320px; background: rgba(20,20,20,0.2); border: 1px solid var(--border); border-radius: 8px; padding: 16px; height: 100%;">
      <div style="display:flex; flex-direction:column; gap:4px; border-bottom:1px solid var(--border); padding-bottom:10px; flex-shrink:0;">
        <span style="font-size:14px; font-weight:700; color:var(--text-primary);">PalSchema Mods Load Order <span style="font-size:9px; color:var(--accent); font-weight:normal; margin-left:2px; vertical-align:middle; border: 1px solid rgba(0,188,255,0.2); padding: 1px 4px; border-radius: 3px;">Experimental</span></span>
        <span style="font-size:10px; color:var(--text-muted);">Controlled via NTFS Junction Prefixes (Windows Only)</span>
      </div>
      <div id="palschema-list-subcontainer" style="flex:1; display:flex; flex-direction:column; gap:8px; overflow-y:auto; padding-right:4px;">
        <div style="color:var(--text-muted); font-size:12px; padding:8px;">Loading PalSchema order...</div>
      </div>
    </div>
  `;

  const ue4ssSub = document.getElementById('ue4ss-list-subcontainer')!;
  const palschemaSub = document.getElementById('palschema-list-subcontainer')!;

  // 1. Load UE4SS Mods (if enabled)
  if (showUe4ss) {
    try {
      _ue4ssMods = await invoke<ModInfo[]>('get_ue4ss_load_order');
      renderUe4ssList(ue4ssSub);
      if (_ue4ssAbort) { _ue4ssAbort.abort(); }
      _ue4ssAbort = new AbortController();
      setupPointerDrag(ue4ssSub, _ue4ssMods, 'save_ue4ss_load_order', renderUe4ssList, _ue4ssAbort.signal);
    } catch (e) {
      console.error('Failed to load UE4SS load order:', e);
      ue4ssSub.innerHTML = `<div style="color:var(--danger);font-size:12px;padding:8px;">Error: ${escapeHtml(String(e))}</div>`;
    }
  }

  // 2. Load PalSchema Mods (if enabled)
  if (showPalschema) {
    try {
      _palschemaMods = await invoke<ModInfo[]>('get_palschema_load_order');
      renderPalSchemaList(palschemaSub);
      if (_palschemaAbort) { _palschemaAbort.abort(); }
      _palschemaAbort = new AbortController();
      setupPointerDrag(palschemaSub, _palschemaMods, 'save_palschema_load_order', renderPalSchemaList, _palschemaAbort.signal);
    } catch (e) {
      console.error('Failed to load PalSchema load order:', e);
      palschemaSub.innerHTML = `<div style="color:var(--danger);font-size:12px;padding:8px;">Error: ${escapeHtml(String(e))}</div>`;
    }
  }
}

function renderUe4ssList(container: HTMLElement): void {
  renderGenericList(container, _ue4ssMods, 'ue4ss', 'save_ue4ss_load_order');
}

function renderPalSchemaList(container: HTMLElement): void {
  renderGenericList(container, _palschemaMods, 'palschema', 'save_palschema_load_order');
}

function renderGenericList(container: HTMLElement, modsList: ModInfo[], sectionType: 'ue4ss' | 'palschema', saveCommand: string): void {
  if (!container) return;

  if (modsList.length === 0) {
    container.innerHTML = `<div style="color:var(--text-muted);font-size:12px;padding:12px;text-align:center;background:rgba(255,255,255,0.01);border:1px dashed var(--border);border-radius:8px;">No active/installed ${sectionType === 'ue4ss' ? 'UE4SS / Hybrid' : 'PalSchema'} mods.</div>`;
    return;
  }

  container.innerHTML = '';

  modsList.forEach((mod, idx) => {
    const item = document.createElement('div');
    item.className = 'load-order-item';
    item.dataset.index = idx.toString();

    // Premium Glassmorphic design
    item.style.display = 'flex';
    item.style.alignItems = 'center';
    item.style.gap = '16px';
    item.style.background = 'rgba(30, 30, 30, 0.45)';
    item.style.backdropFilter = 'blur(8px)';
    item.style.border = '1px solid rgba(255, 255, 255, 0.06)';
    item.style.borderRadius = '8px';
    item.style.padding = '12px 20px';
    item.style.userSelect = 'none';
    item.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
    item.style.transition = 'background 0.2s, border-color 0.2s, box-shadow 0.2s';
    item.style.position = 'relative';

    const typeLower = mod.type.toLowerCase();
    const typeLabel = mod.type;
    const isEnabled = mod.enabled;
    const badgeColor = typeLower === 'ue4ss' ? 'var(--type-ue4ss)' : (typeLower === 'palschema' ? 'var(--accent)' : 'var(--type-hybrid)');
    const badgeBg = typeLower === 'ue4ss' ? 'var(--type-ue4ss-dim)' : (typeLower === 'palschema' ? 'rgba(0,188,255,0.1)' : 'var(--type-hybrid-dim)');

    item.innerHTML = `
      <span class="load-order-drag-handle" style="color:var(--text-muted);font-size:18px;cursor:grab;margin-right:4px;line-height:1;flex-shrink:0;">☰</span>
      <label class="switch" style="position: relative; display: inline-block; width: 34px; height: 20px; margin: 0; flex-shrink:0;">
        <input type="checkbox" class="load-order-toggle" data-index="${idx}" ${isEnabled ? 'checked' : ''} style="opacity: 0; width: 0; height: 0;" />
        <span class="slider" style="position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #333; transition: .3s; border-radius: 20px; border: 1px solid rgba(255,255,255,0.1);"></span>
      </label>
      <div style="flex:1;display:flex;flex-direction:column;gap:2px;min-width:0;padding-right:8px;">
        <span style="font-size:13px;font-weight:700;color:var(--text-primary);letter-spacing:0.3px;word-break:break-word;line-height:1.2;">${escapeHtml(mod.name)}</span>
        <span style="font-size:10px;color:var(--text-muted);">Version: ${escapeHtml(mod.version)}</span>
      </div>
      <span style="font-size:10px;padding:3px 10px;border-radius:4px;border:1px solid ${badgeBg};color:${badgeColor};background:${badgeBg};font-weight:700;letter-spacing:0.5px;text-transform:uppercase;flex-shrink:0;">${escapeHtml(typeLabel)}</span>
    `;

    // Toggle listener
    const checkbox = item.querySelector('.load-order-toggle') as HTMLInputElement;
    checkbox.addEventListener('change', async () => {
      mod.enabled = checkbox.checked;
      const orderedItems = modsList.map(m => [m.id, m.enabled] as [string, boolean]);
      try {
        await invoke(saveCommand, { orderedItems });
      } catch (e) {
        console.error(`Failed to toggle mod in ${saveCommand}:`, e);
      }
    });

    // Hover effects (only when not dragging)
    item.addEventListener('mouseenter', () => {
      if (!document.body.hasAttribute('data-load-dragging')) {
        item.style.background = 'rgba(40, 40, 40, 0.7)';
        item.style.borderColor = 'var(--accent)';
        item.style.boxShadow = '0 6px 16px rgba(0,0,0,0.25), 0 0 8px rgba(0, 188, 255, 0.15)';
      }
    });
    item.addEventListener('mouseleave', () => {
      item.style.background = 'rgba(30, 30, 30, 0.45)';
      item.style.borderColor = 'rgba(255, 255, 255, 0.06)';
      item.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
    });

    container.appendChild(item);
  });
}

/**
 * Generic Pointer-event based drag & drop reordering handler.
 */
function setupPointerDrag(
  container: HTMLElement,
  modsList: ModInfo[],
  saveCommand: string,
  reRenderFn: (c: HTMLElement) => void,
  signal: AbortSignal
): void {
  let draggingEl: HTMLElement | null = null;
  let ghost: HTMLElement | null = null;
  let fromIndex = -1;
  let ghostOffsetY = 0;
  let ghostHeight = 0;
  let containerTop = 0;
  let containerBottom = 0;

  // Cached per-drag: item elements and their vertical midpoints
  let cachedItems: HTMLElement[] = [];
  let cachedMidYs: number[] = [];
  let prevIndicatorIndex = -1;
  let prevIndicatorSide: 'top' | 'bottom' | null = null;

  function clearIndicator(): void {
    if (prevIndicatorIndex >= 0 && prevIndicatorIndex < cachedItems.length) {
      const el = cachedItems[prevIndicatorIndex];
      el.style.borderTop = '1px solid rgba(255, 255, 255, 0.06)';
      el.style.borderBottom = '1px solid rgba(255, 255, 255, 0.06)';
    }
    prevIndicatorIndex = -1;
    prevIndicatorSide = null;
  }

  function setIndicator(index: number, side: 'top' | 'bottom'): void {
    if (prevIndicatorIndex === index && prevIndicatorSide === side) return;
    clearIndicator();
    if (index >= 0 && index < cachedItems.length) {
      if (side === 'top') {
        cachedItems[index].style.borderTop = '2px solid var(--accent)';
        cachedItems[index].style.borderBottom = '1px solid rgba(255, 255, 255, 0.06)';
      } else {
        cachedItems[index].style.borderTop = '1px solid rgba(255, 255, 255, 0.06)';
        cachedItems[index].style.borderBottom = '2px solid var(--accent)';
      }
      prevIndicatorIndex = index;
      prevIndicatorSide = side;
    }
  }

  const opts = { signal };

  container.addEventListener('pointerdown', (e: PointerEvent) => {
    const handle = (e.target as HTMLElement).closest('.load-order-drag-handle');
    if (!handle) return;
    const item = (handle as HTMLElement).closest('.load-order-item') as HTMLElement;
    if (!item) return;
    if ((e.target as HTMLElement).tagName === 'INPUT') return;

    e.preventDefault();
    fromIndex = parseInt(item.dataset.index || '0', 10);
    draggingEl = item;

    const rect = item.getBoundingClientRect();
    ghostOffsetY = e.clientY - rect.top;
    ghostHeight = rect.height;

    const cRect = container.getBoundingClientRect();
    containerTop = cRect.top;
    containerBottom = cRect.bottom;

    // Cache items and midpoints ONCE — zero DOM work during pointermove
    cachedItems = Array.from(container.querySelectorAll('.load-order-item')) as HTMLElement[];
    cachedMidYs = cachedItems.map(el => {
      const r = el.getBoundingClientRect();
      return r.top + r.height / 2;
    });

    // Ghost clone that follows the pointer
    ghost = item.cloneNode(true) as HTMLElement;
    ghost.style.cssText = `
      position: fixed;
      left: ${rect.left}px;
      top: ${rect.top}px;
      width: ${rect.width}px;
      height: ${rect.height}px;
      opacity: 0.85;
      pointer-events: none;
      z-index: 9998;
      box-shadow: 0 12px 32px rgba(0,0,0,0.5), 0 0 12px rgba(0,188,255,0.25);
      border: 2px solid var(--accent);
      border-radius: 8px;
      transition: none;
      backdrop-filter: blur(8px);
    `;
    document.body.appendChild(ghost);

    item.style.opacity = '0.35';
    document.body.setAttribute('data-load-dragging', 'true');
    container.setPointerCapture(e.pointerId);
  }, opts);

  container.addEventListener('pointermove', (e: PointerEvent) => {
    if (!draggingEl || !ghost) return;
    e.preventDefault();

    // Move ghost — pure arithmetic, no DOM reads
    const newTop = e.clientY - ghostOffsetY;
    ghost.style.top = Math.max(containerTop, Math.min(newTop, containerBottom - ghostHeight)) + 'px';

    // Hit-test against cached midpoints — O(n) pure JS, zero DOM
    let targetIndex = cachedMidYs.length - 1; // default: last position
    for (let i = 0; i < cachedMidYs.length; i++) {
      if (e.clientY <= cachedMidYs[i]) {
        targetIndex = i;
        break;
      }
    }

    const side: 'top' | 'bottom' = e.clientY <= cachedMidYs[targetIndex] ? 'top' : 'bottom';
    setIndicator(targetIndex, side);
  }, opts);

  container.addEventListener('pointerup', async (e: PointerEvent) => {
    if (!draggingEl || !ghost) return;
    e.preventDefault();

    ghost.remove();
    ghost = null;
    draggingEl.style.opacity = '1';
    document.body.removeAttribute('data-load-dragging');

    // Compute final drop index BEFORE clearing the cache
    let toIndex = cachedMidYs.length - 1;
    for (let i = 0; i < cachedMidYs.length; i++) {
      if (e.clientY <= cachedMidYs[i]) {
        toIndex = i;
        break;
      }
    }

    const capturedFrom = fromIndex;

    // Clear indicator BEFORE zeroing cachedItems (clearIndicator reads them)
    clearIndicator();
    draggingEl = null;
    cachedItems = [];
    cachedMidYs = [];
    fromIndex = -1;

    if (toIndex !== capturedFrom) {
      const [movedItem] = modsList.splice(capturedFrom, 1);
      modsList.splice(toIndex, 0, movedItem);
      reRenderFn(container);
      
      // Save ordered items
      const orderedItems = modsList.map(m => [m.id, m.enabled] as [string, boolean]);
      try {
        await invoke(saveCommand, { orderedItems });
      } catch (e) {
        console.error(`Failed to save load order in ${saveCommand}:`, e);
      }
    } else {
      reRenderFn(container);
    }
  }, opts);

  container.addEventListener('pointercancel', () => {
    if (ghost) { ghost.remove(); ghost = null; }
    if (draggingEl) { draggingEl.style.opacity = '1'; draggingEl = null; }
    document.body.removeAttribute('data-load-dragging');
    clearIndicator();
    cachedItems = [];
    cachedMidYs = [];
    fromIndex = -1;
    reRenderFn(container);
  }, opts);
}

/**
 * Toggle the visibility of the "Load" tab button in the sidebar based on settings
 */
export function updateLoadTabVisibility(): void {
  const state = getState();
  const forceLoadOrder = !!state.currentSettings?.forceLoadOrder;
  const loadTabBtn = document.getElementById('sidebar-tab-load');
  if (loadTabBtn) {
    loadTabBtn.style.display = forceLoadOrder ? 'flex' : 'none';
  }
}
