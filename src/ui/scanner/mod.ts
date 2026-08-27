import { invoke } from '@tauri-apps/api/core';
import { updateModHotkey } from '../../api';
import type { ModHotkey } from '../../api';
import { showToast } from '../toast';
import { t } from '../../utils/i18n';

export interface ConflictingMod {
  modId: string;
  modName: string;
  filePath: string;
  lineNumber: number;
  detail: string;
}

export interface TableRowConflict {
  tableName: string;
  rowName: string;
  mods: ConflictingMod[];
}

export interface HookConflict {
  hookTarget: string;
  hookFn: string;
  mods: ConflictingMod[];
}

export interface ModSummary {
  modId: string;
  modName: string;
  modType: string;
  palschemaRows: string[];
  ue4ssHooks: string[];
  pakFiles?: string[];
}

export interface PakModSource {
  modId: string;
  modName: string;
  pakFilename: string;
  pakPath: string;
}

export interface PakConflict {
  internalPath: string;
  assetName: string;
  assetType: string;
  mods: PakModSource[];
}

export interface GamePassPakNotice {
  modId: string;
  modName: string;
  pakFilename: string;
  pakPath: string;
  missingContainers: string[];
}

export interface ScanResult {
  totalScanned: number;
  palschemaScanned: number;
  ue4ssScanned: number;
  pakScanned?: number;
  tableConflicts: TableRowConflict[];
  hookConflicts: HookConflict[];
  pakConflicts?: PakConflict[];
  internalTableConflicts: TableRowConflict[];
  internalHookConflicts: HookConflict[];
  warnings: string[];
  modSummaries: ModSummary[];
  gamepassNotices?: GamePassPakNotice[];
  isGamepass?: boolean;
}

import { runScan, renderConflictsPanel } from './conflicts';
import { runHotkeysScan, renderHotkeysPanel } from './hotkeys';
import { customStyles, escapeHtml } from './rendering';
export { runScan, runHotkeysScan };

export let lastScanResult: ScanResult | null = null;
export let lastHotkeysResult: ModHotkey[] | null = null;
export let isScanning = false;
export let isScanningHotkeys = false;
export let activeSubTab: 'conflicts' | 'hotkeys' | 'saves' = 'conflicts';
export let editingHotkeyKey: string | null = null;
export let hotkeyFilter = '';
export let registryFilterType: 'all' | 'pak' | 'ue4ss' | 'palschema' | 'hybrid' = 'all';
export let registrySearchQuery = '';
export let selectedRegistryModId: string | null = null;

// Shared setters to allow inner files to modify states
export function setLastScanResult(val: ScanResult | null): void { lastScanResult = val; }
export function setLastHotkeysResult(val: ModHotkey[] | null): void { lastHotkeysResult = val; }
export function setIsScanning(val: boolean): void { isScanning = val; }
export function setIsScanningHotkeys(val: boolean): void { isScanningHotkeys = val; }
export function setActiveSubTab(val: 'conflicts' | 'hotkeys' | 'saves'): void { activeSubTab = val; }
export function setEditingHotkeyKey(val: string | null): void { editingHotkeyKey = val; }
export function setHotkeyFilter(val: string): void { hotkeyFilter = val; }
export function setRegistryFilterType(val: 'all' | 'pak' | 'ue4ss' | 'palschema' | 'hybrid'): void { registryFilterType = val; }
export function setRegistrySearchQuery(val: string): void { registrySearchQuery = val; }
export function setSelectedRegistryModId(val: string | null): void { selectedRegistryModId = val; }

export async function renderScannerView(): Promise<void> {
  const container = document.getElementById('scanner-view');
  if (!container) return;

  const currentScrollTop = container.querySelector('.scanner-scroll-panel')?.scrollTop ?? 0;

  if (isScanning || isScanningHotkeys) {
    container.innerHTML = `
      <div style="padding: 24px; box-sizing: border-box; display: flex; align-items: center; justify-content: center; height: 100%; width: 100%;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon spinner" style="width: 48px; height: 48px; border-width: 4px;"></div>
          <div class="scanner-hero-title">${isScanning ? escapeHtml(t('scanner.hero_scanning_conflicts')) : escapeHtml(t('scanner.hero_scanning_hotkeys'))}</div>
          <div class="scanner-hero-desc">${escapeHtml(t('scanner.hero_desc'))}</div>
        </div>
      </div>
    `;
    return;
  }

  if (activeSubTab === 'conflicts') {
    await renderConflictsPanel(container);
  } else if (activeSubTab === 'hotkeys') {
    await renderHotkeysPanel(container);
  } else {
    const { renderSavesDoctorPanel } = await import('./savesDoctor');
    await renderSavesDoctorPanel(container);
  }

  const newScrollPanel = container.querySelector('.scanner-scroll-panel');
  if (newScrollPanel && currentScrollTop > 0) {
    newScrollPanel.scrollTop = currentScrollTop;
  }
}

export function subTabHeader(): string {
  let title = t('scanner.title_conflicts');
  if (activeSubTab === 'hotkeys') title = t('scanner.title_hotkeys');
  if (activeSubTab === 'saves') title = t('scanner.title_saves_doctor') || 'Save Health Doctor';

  return `
    ${customStyles}
    <!-- Top Fixed Dashboard Bar -->
    <div style="display:flex;align-items:center;justify-content:space-between;padding: 16px 24px; border-bottom: 1px solid var(--border); background: var(--bg-secondary); flex-shrink: 0;">
      <div style="display:flex;align-items:center;gap:20px;">
        <div style="font-size:16px;font-weight:700;color:var(--text-primary);">${escapeHtml(title)}</div>
        <div class="scanner-sub-tabs">
          <button class="scanner-sub-tab ${activeSubTab === 'conflicts' ? 'active' : ''}" data-subtab="conflicts">${escapeHtml(t('scanner.subtab_conflicts'))}</button>
          <button class="scanner-sub-tab ${activeSubTab === 'hotkeys' ? 'active' : ''}" data-subtab="hotkeys">${escapeHtml(t('scanner.subtab_hotkeys'))}</button>
          <button class="scanner-sub-tab ${activeSubTab === 'saves' ? 'active' : ''}" data-subtab="saves">${escapeHtml(t('scanner.subtab_saves') || 'Saves Doctor')}</button>
        </div>
      </div>
      ${activeSubTab !== 'saves' ? `
      <button id="scanner-re-run-btn" class="scanner-btn-run" style="padding: 6px 14px; font-size:12px;">
        <span>↻ ${escapeHtml(t('common.refresh'))}</span>
      </button>
      ` : ''}
    </div>
  `;
}

export function attachInspectUassetListeners(): void {
  document.querySelectorAll('.inspect-uasset-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const modId = (btn as HTMLElement).dataset.modId;
      const assetPath = (btn as HTMLElement).dataset.assetPath;
      const drawerId = (btn as HTMLElement).dataset.drawerId;
      if (!modId || !assetPath || !drawerId) return;

      const drawer = document.getElementById(drawerId);
      if (!drawer) return;

      if (drawer.style.display !== 'none') {
        drawer.style.display = 'none';
        return;
      }

      drawer.style.display = 'block';
      drawer.innerHTML = `<span style="color: var(--accent);">${escapeHtml(t('scanner.inspecting_uasset'))}</span>`;

      try {
        const rows = await invoke<string[]>('inspect_pak_asset', {
          modId,
          assetInternalPath: assetPath,
        });

        if (rows && rows.length > 0) {
          drawer.innerHTML = `
            <div style="font-weight: 700; color: #81c784; margin-bottom: 4px;">${rows.length} Exports / Objects:</div>
            <ul style="margin: 0; padding-left: 14px; display: flex; flex-direction: column; gap: 2px; font-family: monospace;">
              ${rows.map(r => `<li>${escapeHtml(r)}</li>`).join('')}
            </ul>
          `;
        } else {
          drawer.innerHTML = `<span style="color: var(--text-muted);">${escapeHtml(t('scanner.uasset_no_exports'))}</span>`;
        }
      } catch (err: any) {
        drawer.innerHTML = `<span style="color: var(--danger);">Error: ${escapeHtml(String(err))}</span>`;
      }
    });
  });
}

export function attachMasterListListeners(): void {
  document.querySelectorAll('.scanner-mod-list-item').forEach(item => {
    item.addEventListener('click', async () => {
      const modId = (item as HTMLElement).dataset.modId || null;
      if (!modId) return;

      setSelectedRegistryModId(modId);

      // Update active highlight in left list
      document.querySelectorAll('.scanner-mod-list-item').forEach(el => {
        if ((el as HTMLElement).dataset.modId === modId) {
          el.classList.add('active');
          const span = el.querySelector('span');
          if (span) (span as HTMLElement).style.color = 'var(--accent)';
        } else {
          el.classList.remove('active');
          const span = el.querySelector('span');
          if (span) (span as HTMLElement).style.color = 'var(--text-primary)';
        }
      });

      // Update right inspector in-place without re-rendering or resetting scroll!
      const inspectorRoot = document.getElementById('scanner-inspector-root');
      if (inspectorRoot && lastScanResult?.modSummaries) {
        const activeMod = lastScanResult.modSummaries.find(m => m.modId === modId) || null;
        const { buildInspectorContent } = await import('./conflicts');
        inspectorRoot.innerHTML = buildInspectorContent(activeMod);
        attachInspectUassetListeners();
        attachGamePassConversionListeners();
      }
    });
  });
}

export function attachGamePassConversionListeners(): void {
  document.querySelectorAll<HTMLButtonElement>('.convert-single-gamepass-btn').forEach(btn => {
    btn.onclick = async (e) => {
      e.stopPropagation();
      const modId = btn.dataset.modId;
      if (!modId) return;

      try {
        btn.disabled = true;
        btn.innerHTML = `<span>⏳</span> <span>${escapeHtml(t('scanner.converting_gamepass'))}</span>`;
        const { convertModToGamepass } = await import('../../api');
        const { showToast } = await import('../toast');
        const result = await convertModToGamepass(modId);
        showToast(t('scanner.convert_success', { count: result.length }), 'success');
        const { runScan } = await import('./conflicts');
        await runScan();
      } catch (err: any) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
        btn.disabled = false;
        btn.innerHTML = `<span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_gamepass'))}</span>`;
      }
    };
  });

  const convertAllBtn = document.getElementById('btn-convert-all-gamepass') as HTMLButtonElement | null;
  if (convertAllBtn) {
    convertAllBtn.onclick = async () => {
      try {
        convertAllBtn.disabled = true;
        convertAllBtn.innerHTML = `<span>⏳</span> <span>${escapeHtml(t('scanner.converting_gamepass'))}</span>`;
        const { convertAllGamepassMods } = await import('../../api');
        const { showToast } = await import('../toast');
        const count = await convertAllGamepassMods();
        showToast(t('scanner.convert_all_success', { count }), 'success');
        const { runScan } = await import('./conflicts');
        await runScan();
      } catch (err: any) {
        const { showToast } = await import('../toast');
        showToast(String(err), 'error');
        convertAllBtn.disabled = false;
        convertAllBtn.innerHTML = `<span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_all_gamepass', { count: '' }))}</span>`;
      }
    };
  }
}

export function setupEventListeners(): void {
  document.getElementById('scanner-start-btn')?.addEventListener('click', runScan);
  document.getElementById('scanner-start-hotkeys-btn')?.addEventListener('click', runHotkeysScan);

  document.getElementById('scanner-re-run-btn')?.addEventListener('click', async () => {
    if (activeSubTab === 'conflicts') {
      runScan();
    } else if (activeSubTab === 'hotkeys') {
      runHotkeysScan();
    } else {
      const { renderSavesDoctorPanel } = await import('./savesDoctor');
      const container = document.getElementById('scanner-view');
      if (container) await renderSavesDoctorPanel(container);
    }
  });

  document.querySelectorAll('.scanner-sub-tab[data-subtab]').forEach(btn => {
    btn.addEventListener('click', () => {
      const sub = (btn as HTMLElement).dataset.subtab as 'conflicts' | 'hotkeys' | 'saves';
      if (sub && activeSubTab !== sub) {
        activeSubTab = sub;
        editingHotkeyKey = null;
        renderScannerView();
      }
    });
  });

  const search = document.getElementById('hk-search-input') as HTMLInputElement | null;
  if (search) {
    search.addEventListener('input', () => {
      hotkeyFilter = search.value;
      renderScannerView();
      const searchRef = document.getElementById('hk-search-input') as HTMLInputElement | null;
      if (searchRef) {
        searchRef.focus();
        searchRef.setSelectionRange(searchRef.value.length, searchRef.value.length);
      }
    });
  }

  // Active Mod Registries Filters & Search (Zero Flickering, In-Place DOM Update)
  document.querySelectorAll('.registry-filter-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const type = (btn as HTMLElement).dataset.type as 'all' | 'pak' | 'ue4ss' | 'palschema' | 'hybrid';
      if (registryFilterType !== type) {
        registryFilterType = type;
        const { updateMasterDetailInPlace } = await import('./conflicts');
        updateMasterDetailInPlace();
      }
    });
  });

  const regSearch = document.getElementById('registry-search-input') as HTMLInputElement | null;
  if (regSearch) {
    regSearch.addEventListener('input', async () => {
      registrySearchQuery = regSearch.value;
      const { updateMasterDetailInPlace } = await import('./conflicts');
      updateMasterDetailInPlace();
    });
  }

  attachMasterListListeners();
  attachInspectUassetListeners();
  attachGamePassConversionListeners();

  document.querySelectorAll('.hk-edit-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      editingHotkeyKey = (btn as HTMLElement).dataset.key || null;
      renderScannerView();

      const input = document.querySelector('.hotkey-edit-input') as HTMLInputElement | null;
      if (input) {
        input.focus();
        input.addEventListener('keydown', (e) => {
          e.preventDefault();
          e.stopPropagation();

          const modifiers: string[] = [];
          if (e.ctrlKey) modifiers.push('ModifierKey.CONTROL');
          if (e.shiftKey) modifiers.push('ModifierKey.SHIFT');
          if (e.altKey) modifiers.push('ModifierKey.ALT');

          let mainKey = '';
          const code = e.code;

          if (code.startsWith('Key')) {
            mainKey = `Key.${code.substring(3).toUpperCase()}`;
          } else if (code.startsWith('Digit')) {
            const digitMap: Record<string, string> = {
              'Digit0': 'ZERO', 'Digit1': 'ONE', 'Digit2': 'TWO', 'Digit3': 'THREE',
              'Digit4': 'FOUR', 'Digit5': 'FIVE', 'Digit6': 'SIX', 'Digit7': 'SEVEN',
              'Digit8': 'EIGHT', 'Digit9': 'NINE'
            };
            mainKey = `Key.${digitMap[code] || code.substring(5)}`;
          } else if (code.startsWith('F') && code.length >= 2) {
            mainKey = `Key.${code}`;
          } else if (code === 'Space') {
            mainKey = 'Key.SPACE';
          } else if (code === 'Escape') {
            mainKey = 'Key.ESCAPE';
          } else if (code === 'Enter') {
            mainKey = 'Key.ENTER';
          } else if (code === 'Tab') {
            mainKey = 'Key.TAB';
          } else if (code.startsWith('Numpad')) {
            mainKey = `Key.NUM_${code.substring(6).toUpperCase()}`;
          } else if (code === 'ArrowUp') {
            mainKey = 'Key.UP_ARROW';
          } else if (code === 'ArrowDown') {
            mainKey = 'Key.DOWN_ARROW';
          } else if (code === 'ArrowLeft') {
            mainKey = 'Key.LEFT_ARROW';
          } else if (code === 'ArrowRight') {
            mainKey = 'Key.RIGHT_ARROW';
          }

          if (e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt') {
            if (modifiers.length > 0) {
              input.value = modifiers.map(m => `{${m}}`).join(', ');
            }
            return;
          }

          if (mainKey) {
            let result = mainKey;
            if (modifiers.length > 0) {
              result = `${mainKey}, {${modifiers.join(', ')}}`;
            }
            input.value = result;
          }
        });
      }
    });
  });

  document.querySelectorAll('.hk-cancel-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      editingHotkeyKey = null;
      renderScannerView();
    });
  });

  document.querySelectorAll('.hk-save-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const idx = parseInt((btn as HTMLElement).dataset.idx || '0');
      const input = document.getElementById(`hk-input-${idx}`) as HTMLInputElement | null;
      if (!input || !lastHotkeysResult) return;

      const hk = lastHotkeysResult.find(item => `${item.absoluteFilePath}::${item.lineNumber}` === editingHotkeyKey);
      if (!hk) return;

      const newKeys = input.value.trim();
      if (!newKeys) {
        showToast(t('scanner.toast_empty_keybind'), 'error');
        return;
      }

      (btn as HTMLButtonElement).disabled = true;
      try {
        await updateModHotkey(hk.absoluteFilePath, hk.lineNumber, newKeys);
        showToast(t('scanner.toast_hotkey_saved'), 'success');
        editingHotkeyKey = null;
        await runHotkeysScan();
      } catch (err: any) {
        showToast(t('toasts.export_failed', { error: String(err) }), 'error');
        (btn as HTMLButtonElement).disabled = false;
      }
    });
  });

  document.querySelectorAll('.hk-code-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const modId = (btn as HTMLElement).dataset.modId!;
      const filePath = (btn as HTMLElement).dataset.filePath!;
      const line = parseInt((btn as HTMLElement).dataset.line || '1');
      const { openFileAtLine } = await import('../editorView');
      openFileAtLine(modId, filePath, line);
    });
  });

  document.querySelectorAll('details').forEach(el => {
    el.addEventListener('toggle', () => {
      const panel = document.querySelector('.scanner-scroll-panel');
      if (panel) {
        const maxScroll = Math.max(0, panel.scrollHeight - panel.clientHeight);
        if (panel.scrollTop > maxScroll) {
          panel.scrollTop = maxScroll;
        }
      }
    });
  });
}
