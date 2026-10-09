import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import type { UnifiedCrashReport, RawDmpModule } from '../../../api';
import { showToast } from '../../toast';

interface CallstackFrame {
  index: number;
  module: string;
  baseAddress: string;
  offset: string;
}

// Tokenizes continuous UE5 callstack strings into clean execution frames
function parseCallstackFrames(raw: string): CallstackFrame[] {
  if (!raw || !raw.trim()) return [];

  const frames: CallstackFrame[] = [];
  // Matches tokens like: "Palworld-Win64-Shipping 0x00007ff7a1840000 + 34f3f8c" or "ntdll 0x00007ffff6f40000 + 160e44"
  const regex = /([a-zA-Z0-9_\-\.]+)\s+(0x[0-9a-fA-F]+)\s*\+\s*([0-9a-fA-F]+)/g;
  let match: RegExpExecArray | null;
  let idx = 0;

  while ((match = regex.exec(raw)) !== null) {
    frames.push({
      index: idx++,
      module: match[1],
      baseAddress: match[2],
      offset: match[3],
    });
  }

  // Fallback for newline-delimited basic symbol traces (e.g. Palworld_Win64_Shipping\nkernel32...)
  if (frames.length === 0 && raw.includes('\n')) {
    const lines = raw.split(/\r?\n/).map(l => l.trim()).filter(Boolean);
    lines.forEach((line, i) => {
      frames.push({
        index: i,
        module: line,
        baseAddress: '',
        offset: '',
      });
    });
  }

  return frames;
}

// Renders the dedicated full-height Raw Forensics & Minidump Inspector
export function renderRawDumpViewerHtml(report: UnifiedCrashReport | null): string {
  if (!report) return '';

  const dmp = report.minidump;
  const xml = report.xmlContext;
  const modulesCount = dmp?.rawModules?.length || 0;
  const callstackText = xml?.crashedThreadCallstack || xml?.callstackRaw || '';
  const xmlRawText = xml?.rawXmlContent || '';
  const parsedFrames = parseCallstackFrames(callstackText);

  return `
    <div class="raw-forensics-full-container" style="display: flex; flex-direction: column; height: 100%; min-height: 0; flex: 1; gap: calc(10px * var(--ui-scale, 1)); overflow: hidden;">
      
      <!-- Top Action Bar & Tabs -->
      <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--card-radius); padding: calc(8px * var(--ui-scale, 1)) calc(12px * var(--ui-scale, 1)); display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; flex-shrink: 0;">
        <!-- Tab Navigation -->
        <div style="display: inline-flex; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; padding: 2px; gap: 2px;">
          <button type="button" class="raw-tab-btn active" data-tab="summary" style="padding: 4px 10px; font-size: var(--text-xs, 11px); font-weight: 600; border: none; border-radius: 4px; background: var(--accent); color: #fff; cursor: pointer;">
            ${escapeHtml(t('crash_doctor.tab_raw_summary') || 'Minidump Summary')}
          </button>
          <button type="button" class="raw-tab-btn" data-tab="callstack" style="padding: 4px 10px; font-size: var(--text-xs, 11px); font-weight: 600; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); cursor: pointer;">
            ${escapeHtml(t('crash_doctor.tab_raw_callstack') || 'Crashed Callstack')}
          </button>
          <button type="button" class="raw-tab-btn" data-tab="modules" style="padding: 4px 10px; font-size: var(--text-xs, 11px); font-weight: 600; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); cursor: pointer;">
            ${escapeHtml(t('crash_doctor.tab_raw_modules', { count: String(modulesCount) }) || `Loaded Modules (${modulesCount})`)}
          </button>
          ${xmlRawText ? `
            <button type="button" class="raw-tab-btn" data-tab="xml" style="padding: 4px 10px; font-size: var(--text-xs, 11px); font-weight: 600; border: none; border-radius: 4px; background: transparent; color: var(--text-secondary); cursor: pointer;">
              ${escapeHtml(t('crash_doctor.tab_raw_xml') || 'Crash Context XML')}
            </button>
          ` : ''}
        </div>

        <!-- Copy Forensics Action -->
        <div style="display: flex; align-items: center; gap: 8px;">
          <button type="button" id="btn-copy-raw-forensics" class="btn-secondary" style="padding: 4px 10px; font-size: var(--text-xs, 11px); font-weight: 600; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-tertiary); color: var(--text-primary); cursor: pointer; display: flex; align-items: center; gap: 5px;">
            <span>📋</span>
            <span>${escapeHtml(t('crash_doctor.btn_copy_raw_dump') || 'Copy Forensics')}</span>
          </button>
        </div>
      </div>

      <!-- Tab 1: Minidump Summary Stream -->
      <div id="raw-panel-summary" class="raw-tab-panel" style="display: flex; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden;">
        <pre style="flex: 1; height: 100%; min-height: 0; margin: 0; background: var(--bg-code, #18191c); border: 1px solid var(--border); border-radius: 6px; padding: 14px; font-size: 11.5px; line-height: 1.5; font-family: 'Consolas', 'Courier New', monospace; color: var(--text-primary); overflow-y: auto; white-space: pre-wrap; word-break: break-all; box-sizing: border-box;">${escapeHtml(dmp?.rawDumpSummary || 'No minidump summary stream extracted for this crash incident.')}</pre>
      </div>

      <!-- Tab 2: Crashed Callstack with Formatted Frames & Raw Toggle -->
      <div id="raw-panel-callstack" class="raw-tab-panel" style="display: none; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden; gap: 8px;">
        <div style="display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-shrink: 0;">
          <div style="font-size: 11.5px; color: var(--text-secondary); display: flex; align-items: center; gap: 6px;">
            <span>🧵</span>
            <span>${escapeHtml(xml?.crashedThreadName ? `Crashed Thread: ${xml.crashedThreadName}` : 'Execution Callstack')}</span>
            <span style="color: var(--text-muted);">(${parsedFrames.length} frames)</span>
          </div>

          <!-- Format Switcher: Frames Table vs Plain Text -->
          <div style="display: inline-flex; background: rgba(0,0,0,0.3); border: 1px solid var(--border); border-radius: 4px; padding: 2px;">
            <button type="button" id="btn-callstack-mode-frames" class="callstack-submode-btn active" style="padding: 2px 8px; font-size: 10.5px; font-weight: 600; border: none; border-radius: 3px; background: var(--accent); color: #000; cursor: pointer;">
              ${escapeHtml(t('crash_doctor.btn_callstack_frames') || 'Frames View')}
            </button>
            <button type="button" id="btn-callstack-mode-raw" class="callstack-submode-btn" style="padding: 2px 8px; font-size: 10.5px; font-weight: 600; border: none; border-radius: 3px; background: transparent; color: var(--text-secondary); cursor: pointer;">
              ${escapeHtml(t('crash_doctor.btn_callstack_raw') || 'Raw Text')}
            </button>
          </div>
        </div>

        <!-- Callstack Sub-View 1: Structured Frames Table -->
        <div id="callstack-view-frames" style="display: flex; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow-y: auto; border: 1px solid var(--border); border-radius: 6px; background: rgba(0,0,0,0.2);">
          ${renderCallstackFramesTable(parsedFrames)}
        </div>

        <!-- Callstack Sub-View 2: Raw Monospace Text -->
        <div id="callstack-view-raw" style="display: none; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden;">
          <pre style="flex: 1; height: 100%; min-height: 0; margin: 0; background: rgba(0,0,0,0.45); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 14px; font-size: 11.5px; line-height: 1.5; font-family: 'Consolas', 'Courier New', monospace; color: #ffcc80; overflow-y: auto; white-space: pre-wrap; word-break: break-all; box-sizing: border-box;">${escapeHtml(callstackText || 'No callstack recorded in crash metadata.')}</pre>
        </div>
      </div>

      <!-- Tab 3: Loaded DLL Binaries Table -->
      <div id="raw-panel-modules" class="raw-tab-panel" style="display: none; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden; gap: 8px;">
        <div style="display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-shrink: 0;">
          <input type="text" id="raw-modules-filter" placeholder="${escapeHtml(t('crash_doctor.search_modules_placeholder') || 'Filter loaded modules...')}" style="width: 260px; padding: 6px 12px; font-size: 11px; background: rgba(0,0,0,0.3); border: 1px solid var(--border); border-radius: 4px; color: var(--text-primary);" />
          <span style="font-size: 11px; color: var(--text-muted);">${modulesCount} binaries recorded</span>
        </div>
        <div style="flex: 1; height: 100%; min-height: 0; overflow-y: auto; border: 1px solid var(--border); border-radius: 6px; background: rgba(0,0,0,0.2);">
          <table id="raw-modules-table" style="width: 100%; border-collapse: collapse; font-size: 11px; text-align: left; font-family: 'Consolas', 'Courier New', monospace;">
            <thead style="position: sticky; top: 0; background: #18191c; z-index: 2;">
              <tr style="border-bottom: 1px solid var(--border); color: var(--text-secondary);">
                <th style="padding: 8px 12px;">Module Name</th>
                <th style="padding: 8px 12px;">Base Address</th>
                <th style="padding: 8px 12px;">Image Size</th>
                <th style="padding: 8px 12px;">Category</th>
              </tr>
            </thead>
            <tbody>
              ${renderRawModulesRows(dmp?.rawModules || [])}
            </tbody>
          </table>
        </div>
      </div>

      <!-- Tab 4: Raw CrashContext XML -->
      ${xmlRawText ? `
        <div id="raw-panel-xml" class="raw-tab-panel" style="display: none; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden;">
          <pre style="flex: 1; height: 100%; min-height: 0; margin: 0; background: rgba(0,0,0,0.45); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 14px; font-size: 11px; line-height: 1.45; font-family: 'Consolas', 'Courier New', monospace; color: #a5d6a7; overflow-y: auto; white-space: pre-wrap; word-break: break-all; box-sizing: border-box;">${escapeHtml(xmlRawText)}</pre>
        </div>
      ` : ''}

    </div>
  `;
}

function renderCallstackFramesTable(frames: CallstackFrame[]): string {
  if (frames.length === 0) {
    return `<div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 12px;">No callstack frames recorded</div>`;
  }

  return `
    <table style="width: 100%; border-collapse: collapse; font-size: 11px; text-align: left; font-family: 'Consolas', 'Courier New', monospace;">
      <thead style="position: sticky; top: 0; background: #18191c; z-index: 2;">
        <tr style="border-bottom: 1px solid var(--border); color: var(--text-secondary);">
          <th style="padding: 8px 12px; width: 80px;">Frame</th>
          <th style="padding: 8px 12px;">Module / Binary</th>
          <th style="padding: 8px 12px;">Offset</th>
          <th style="padding: 8px 12px;">Base Address</th>
        </tr>
      </thead>
      <tbody>
        ${frames.map((f, i) => {
          const isTop = i === 0;
          const isGame = f.module.toLowerCase().includes('palworld');
          const isUe4ss = f.module.toLowerCase().includes('ue4ss');
          const modColor = isUe4ss ? '#ce93d8' : isGame ? '#ffb74d' : '#90caf9';
          const bg = isTop ? 'rgba(239, 83, 80, 0.12)' : 'transparent';
          const borderLeft = isTop ? '3px solid #ef5350' : 'none';

          return `
            <tr style="background: ${bg}; border-left: ${borderLeft}; border-bottom: 1px solid rgba(255,255,255,0.04);">
              <td style="padding: 6px 12px; color: ${isTop ? '#ef5350' : 'var(--text-muted)'}; font-weight: 700;">
                #${String(f.index).padStart(2, '0')}
              </td>
              <td style="padding: 6px 12px; color: ${modColor}; font-weight: 600;">
                ${isTop ? '💥 ' : ''}${escapeHtml(f.module)}
              </td>
              <td style="padding: 6px 12px; color: #ffab91;">
                ${f.offset ? `+0x${escapeHtml(f.offset)}` : '—'}
              </td>
              <td style="padding: 6px 12px; color: #81d4fa;">
                ${escapeHtml(f.baseAddress || '—')}
              </td>
            </tr>
          `;
        }).join('')}
      </tbody>
    </table>
  `;
}

function renderRawModulesRows(modules: RawDmpModule[]): string {
  if (modules.length === 0) {
    return `<tr><td colspan="4" style="padding: 16px; text-align: center; color: var(--text-muted);">No modules found in dump</td></tr>`;
  }

  return modules.map(m => {
    const isFaulting = m.isFaulting;
    const rowBg = isFaulting ? 'rgba(239, 83, 80, 0.15)' : 'transparent';
    const borderLeft = isFaulting ? '3px solid #ef5350' : 'none';
    const nameColor = isFaulting ? '#ef5350' : m.category === 'ue4ss' ? '#ce93d8' : m.category === 'pak_mod' ? '#ffb74d' : 'var(--text-primary)';
    const sizeKb = (m.sizeBytes / 1024).toFixed(0);

    return `
      <tr class="raw-module-row" data-name="${escapeHtml(m.name.toLowerCase())}" style="background: ${rowBg}; border-left: ${borderLeft}; border-bottom: 1px solid rgba(255,255,255,0.04);">
        <td style="padding: 6px 12px; color: ${nameColor}; font-weight: ${isFaulting ? '700' : '500'};" title="${escapeHtml(m.fullPath)}">
          ${isFaulting ? '💥 ' : ''}${escapeHtml(m.name)}
        </td>
        <td style="padding: 6px 12px; color: #90caf9;">${escapeHtml(m.baseAddressHex)}</td>
        <td style="padding: 6px 12px; color: var(--text-secondary);">${sizeKb} KB</td>
        <td style="padding: 6px 12px;">
          <span style="font-size: 9.5px; padding: 2px 7px; border-radius: 3px; background: rgba(255,255,255,0.07); text-transform: uppercase;">
            ${escapeHtml(m.category)}
          </span>
        </td>
      </tr>
    `;
  }).join('');
}

// Binds interactive listeners for tabs, module filtering, callstack mode toggle, and clipboard copy
export function attachRawViewerListeners(container: HTMLElement, report: UnifiedCrashReport | null): void {
  if (!report) return;

  // Sub-tabs switcher
  const tabButtons = container.querySelectorAll('.raw-tab-btn');
  tabButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const tabName = btn.getAttribute('data-tab');
      if (!tabName) return;

      tabButtons.forEach(b => {
        (b as HTMLElement).style.background = 'transparent';
        (b as HTMLElement).style.color = 'var(--text-secondary)';
      });
      (btn as HTMLElement).style.background = 'var(--accent)';
      (btn as HTMLElement).style.color = '#000';

      const panels = container.querySelectorAll('.raw-tab-panel');
      panels.forEach(p => ((p as HTMLElement).style.display = 'none'));

      const target = container.querySelector(`#raw-panel-${tabName}`);
      if (target) (target as HTMLElement).style.display = 'flex';
    });
  });

  // Callstack sub-mode toggle: Frames View vs Raw Text
  const btnFrames = container.querySelector('#btn-callstack-mode-frames');
  const btnRaw = container.querySelector('#btn-callstack-mode-raw');
  const viewFrames = container.querySelector('#callstack-view-frames') as HTMLElement | null;
  const viewRaw = container.querySelector('#callstack-view-raw') as HTMLElement | null;

  if (btnFrames && btnRaw && viewFrames && viewRaw) {
    btnFrames.addEventListener('click', () => {
      (btnFrames as HTMLElement).style.background = 'var(--accent)';
      (btnFrames as HTMLElement).style.color = '#000';
      (btnRaw as HTMLElement).style.background = 'transparent';
      (btnRaw as HTMLElement).style.color = 'var(--text-secondary)';
      viewFrames.style.display = 'flex';
      viewRaw.style.display = 'none';
    });

    btnRaw.addEventListener('click', () => {
      (btnRaw as HTMLElement).style.background = 'var(--accent)';
      (btnRaw as HTMLElement).style.color = '#000';
      (btnFrames as HTMLElement).style.background = 'transparent';
      (btnFrames as HTMLElement).style.color = 'var(--text-secondary)';
      viewFrames.style.display = 'none';
      viewRaw.style.display = 'flex';
    });
  }

  // Filter loaded modules in table
  const filterInput = container.querySelector('#raw-modules-filter') as HTMLInputElement | null;
  if (filterInput) {
    filterInput.addEventListener('input', () => {
      const q = filterInput.value.trim().toLowerCase();
      const rows = container.querySelectorAll('.raw-module-row');
      rows.forEach(r => {
        const name = r.getAttribute('data-name') || '';
        (r as HTMLElement).style.display = name.includes(q) ? '' : 'none';
      });
    });
  }

  // Copy raw dump forensics to clipboard
  const copyBtn = container.querySelector('#btn-copy-raw-forensics');
  if (copyBtn) {
    copyBtn.addEventListener('click', async () => {
      const dmp = report.minidump;
      const xml = report.xmlContext;
      const textToCopy = [
        dmp?.rawDumpSummary || 'No minidump summary',
        '\n--- CRASHED THREAD TRACE ---',
        xml?.crashedThreadCallstack || xml?.callstackRaw || 'No callstack recorded',
      ].join('\n');

      try {
        await navigator.clipboard.writeText(textToCopy);
        showToast(t('crash_doctor.toast_raw_copied') || 'Raw dump forensics copied to clipboard', 'success');
      } catch (err) {
        console.error('Failed to copy raw forensics to clipboard:', err);
      }
    });
  }
}
