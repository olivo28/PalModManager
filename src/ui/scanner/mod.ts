import { updateModHotkey } from '../../api';
import type { ModHotkey } from '../../api';
import { showToast } from '../toast';

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
}

export interface ScanResult {
  totalScanned: number;
  palschemaScanned: number;
  ue4ssScanned: number;
  tableConflicts: TableRowConflict[];
  hookConflicts: HookConflict[];
  internalTableConflicts: TableRowConflict[];
  internalHookConflicts: HookConflict[];
  warnings: string[];
  modSummaries: ModSummary[];
}

import { runScan, renderConflictsPanel } from './conflicts';
import { runHotkeysScan, renderHotkeysPanel } from './hotkeys';
import { customStyles, escapeHtml } from './rendering';
export { runScan, runHotkeysScan };

export let lastScanResult: ScanResult | null = null;
export let lastHotkeysResult: ModHotkey[] | null = null;
export let isScanning = false;
export let isScanningHotkeys = false;
export let activeSubTab: 'conflicts' | 'hotkeys' = 'conflicts';
export let editingHotkeyKey: string | null = null;
export let hotkeyFilter = '';

// Shared setters to allow inner files to modify states
export function setLastScanResult(val: ScanResult | null): void { lastScanResult = val; }
export function setLastHotkeysResult(val: ModHotkey[] | null): void { lastHotkeysResult = val; }
export function setIsScanning(val: boolean): void { isScanning = val; }
export function setIsScanningHotkeys(val: boolean): void { isScanningHotkeys = val; }
export function setActiveSubTab(val: 'conflicts' | 'hotkeys'): void { activeSubTab = val; }
export function setEditingHotkeyKey(val: string | null): void { editingHotkeyKey = val; }
export function setHotkeyFilter(val: string): void { hotkeyFilter = val; }

export async function renderScannerView(): Promise<void> {
  const container = document.getElementById('scanner-view');
  if (!container) return;

  if (isScanning || isScanningHotkeys) {
    container.innerHTML = `
      <div style="padding: 24px; box-sizing: border-box; display: flex; align-items: center; justify-content: center; height: 100%; width: 100%;">
        <div class="scanner-hero">
          <div class="scanner-hero-icon spinner" style="width: 48px; height: 48px; border-width: 4px;"></div>
          <div class="scanner-hero-title">${isScanning ? 'Scanning Mods...' : 'Scanning Lua Hotkeys...'}</div>
          <div class="scanner-hero-desc">Reading configurations and parsing files. This may take a few seconds.</div>
        </div>
      </div>
    `;
    return;
  }

  if (activeSubTab === 'conflicts') {
    await renderConflictsPanel(container);
  } else {
    await renderHotkeysPanel(container);
  }
}

export function subTabHeader(): string {
  return `
    ${customStyles}
    <!-- Top Fixed Dashboard Bar -->
    <div style="display:flex;align-items:center;justify-content:space-between;padding: 16px 24px; border-bottom: 1px solid var(--border); background: var(--bg-secondary); flex-shrink: 0;">
      <div style="display:flex;align-items:center;gap:20px;">
        <div style="font-size:16px;font-weight:700;color:var(--text-primary);">${activeSubTab === 'conflicts' ? 'Conflict Scanner' : 'Hotkeys Manager'}</div>
        <div class="scanner-sub-tabs">
          <button class="scanner-sub-tab ${activeSubTab === 'conflicts' ? 'active' : ''}" data-subtab="conflicts">Conflicts</button>
          <button class="scanner-sub-tab ${activeSubTab === 'hotkeys' ? 'active' : ''}" data-subtab="hotkeys">Hotkeys</button>
        </div>
      </div>
      <button id="scanner-re-run-btn" class="scanner-btn-run" style="padding: 6px 14px; font-size:12px;">
        <span>Run Scan</span>
      </button>
    </div>
  `;
}

export function setupEventListeners(): void {
  document.getElementById('scanner-start-btn')?.addEventListener('click', runScan);
  document.getElementById('scanner-start-hotkeys-btn')?.addEventListener('click', runHotkeysScan);

  document.getElementById('scanner-re-run-btn')?.addEventListener('click', () => {
    if (activeSubTab === 'conflicts') {
      runScan();
    } else {
      runHotkeysScan();
    }
  });

  document.querySelectorAll('.scanner-sub-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      const sub = (btn as HTMLElement).dataset.subtab as 'conflicts' | 'hotkeys';
      if (activeSubTab !== sub) {
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
        showToast('Keybind cannot be empty', 'error');
        return;
      }

      (btn as HTMLButtonElement).disabled = true;
      try {
        await updateModHotkey(hk.absoluteFilePath, hk.lineNumber, newKeys);
        showToast('Hotkey trigger modified successfully', 'success');
        editingHotkeyKey = null;
        await runHotkeysScan();
      } catch (err: any) {
        showToast(`Failed to update hotkey: ${err}`, 'error');
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
}
