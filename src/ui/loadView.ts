import { invoke } from '@tauri-apps/api/core';
import { ModInfo } from '../types';
import { escapeHtml } from '../utils/helpers';
import { getState } from '../state';

let _orderedMods: ModInfo[] = [];
let _draggedIndex: number | null = null;

export async function renderLoadView(): Promise<void> {
  const container = document.getElementById('load-list-container');
  if (!container) return;

  container.innerHTML = '<div style="color:var(--text-muted);font-size:12px;padding:8px;">Loading load order...</div>';

  try {
    _orderedMods = await invoke<ModInfo[]>('get_ue4ss_load_order');
    renderList();
  } catch (e) {
    console.error('Failed to load UE4SS load order:', e);
    container.innerHTML = `<div style="color:var(--danger);font-size:12px;padding:8px;">Error loading order: ${escapeHtml(String(e))}</div>`;
  }
}

function renderList(): void {
  const container = document.getElementById('load-list-container');
  if (!container) return;

  if (_orderedMods.length === 0) {
    container.innerHTML = '<div style="color:var(--text-muted);font-size:12px;padding:8px;text-align:center;">No UE4SS or Hybrid mods installed or active.</div>';
    return;
  }

  container.innerHTML = '';

  _orderedMods.forEach((mod, idx) => {
    const item = document.createElement('div');
    item.className = 'load-order-item';
    item.draggable = true;
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
    item.style.cursor = 'grab';
    item.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
    item.style.transition = 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)';

    // Hover effect
    item.addEventListener('mouseenter', () => {
      item.style.background = 'rgba(40, 40, 40, 0.7)';
      item.style.borderColor = 'var(--accent)';
      item.style.transform = 'translateY(-1px)';
      item.style.boxShadow = '0 6px 16px rgba(0,0,0,0.25), 0 0 8px rgba(0, 188, 255, 0.15)';
    });
    item.addEventListener('mouseleave', () => {
      if (_draggedIndex !== idx) {
        item.style.background = 'rgba(30, 30, 30, 0.45)';
        item.style.borderColor = 'rgba(255, 255, 255, 0.06)';
        item.style.transform = 'none';
        item.style.boxShadow = '0 4px 12px rgba(0,0,0,0.15)';
      }
    });

    const typeLower = mod.type.toLowerCase();
    const typeLabel = mod.type;

    // Draggable listeners
    item.addEventListener('dragstart', (e) => {
      _draggedIndex = idx;
      item.style.opacity = '0.5';
      if (e.dataTransfer) {
        e.dataTransfer.effectAllowed = 'move';
        e.dataTransfer.setData('text/plain', idx.toString());
      }
    });

    item.addEventListener('dragend', () => {
      _draggedIndex = null;
      item.style.opacity = '1';
      renderList();
    });

    item.addEventListener('dragover', (e) => {
      e.preventDefault();
      if (e.dataTransfer) {
        e.dataTransfer.dropEffect = 'move';
      }
      item.style.borderTop = '2px solid var(--accent)';
    });

    item.addEventListener('dragleave', () => {
      item.style.borderTop = '1px solid rgba(255, 255, 255, 0.06)';
    });

    item.addEventListener('drop', async (e) => {
      e.preventDefault();
      const fromIndexStr = e.dataTransfer?.getData('text/plain');
      if (fromIndexStr !== undefined) {
        const fromIndex = parseInt(fromIndexStr, 10);
        const toIndex = idx;
        if (fromIndex !== toIndex) {
          // Reorder the array
          const [movedItem] = _orderedMods.splice(fromIndex, 1);
          _orderedMods.splice(toIndex, 0, movedItem);
          renderList();
          await saveNewOrder();
        }
      }
    });

    // Content
    const isEnabled = mod.enabled;
    const badgeColorClass = typeLower === 'ue4ss' ? 'var(--type-ue4ss)' : 'var(--type-hybrid)';
    const badgeBgClass = typeLower === 'ue4ss' ? 'var(--type-ue4ss-dim)' : 'var(--type-hybrid-dim)';

    item.innerHTML = `
      <span class="load-order-drag-handle" style="color:var(--text-muted);font-size:16px;cursor:grab;margin-right:4px;">☰</span>
      <label class="switch" style="position: relative; display: inline-block; width: 34px; height: 20px; margin: 0;">
        <input type="checkbox" class="load-order-toggle" data-index="${idx}" ${isEnabled ? 'checked' : ''} style="opacity: 0; width: 0; height: 0;" />
        <span class="slider" style="position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #333; transition: .3s; border-radius: 20px; border: 1px solid rgba(255,255,255,0.1);"></span>
      </label>
      <div style="flex:1;display:flex;flex-direction:column;gap:2px;">
        <span style="font-size:13px;font-weight:700;color:var(--text-primary);letter-spacing:0.3px;">${escapeHtml(mod.name)}</span>
        <span style="font-size:10px;color:var(--text-muted);">Version: ${escapeHtml(mod.version)}</span>
      </div>
      <span style="font-size:10px;padding:3px 10px;border-radius:4px;border:1px solid ${badgeBgClass};color:${badgeColorClass};background:${badgeBgClass};font-weight:700;letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(typeLabel)}</span>
    `;

    // Toggle listener
    const checkbox = item.querySelector('.load-order-toggle') as HTMLInputElement;
    checkbox.addEventListener('change', async () => {
      mod.enabled = checkbox.checked;
      await saveNewOrder();
    });

    container.appendChild(item);
  });
}

async function saveNewOrder(): Promise<void> {
  const orderedItems = _orderedMods.map(m => [m.id, m.enabled] as [string, boolean]);
  try {
    await invoke('save_ue4ss_load_order', { orderedItems });
  } catch (e) {
    console.error('Failed to save UE4SS load order:', e);
  }
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
