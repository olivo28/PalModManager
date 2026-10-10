import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { getState } from '../../../state';
import { showToast } from '../../toast';
import { subTabHeader, setupEventListeners } from '../mod';
import {
  checkLatestCrash,
  listAllCrashes,
  analyzeCrashFolder,
  exportCrashSupportBundle,
  type UnifiedCrashReport,
  type CrashHistoryItem,
} from '../../../api';
import { renderRawDumpViewerHtml, attachRawViewerListeners } from './rawViewer';
import { getIconSvg } from '../../icons/mod';

let lastCrashReport: UnifiedCrashReport | null = null;
let cachedCrashHistory: CrashHistoryItem[] = [];
let selectedCrashId: string | null = null;
let isLoadingCrash = false;
let activeInspectorMode: 'summary' | 'raw' = 'summary';

export function getLastCrashReport(): UnifiedCrashReport | null {
  return lastCrashReport;
}

export function setLastCrashReport(report: UnifiedCrashReport | null): void {
  lastCrashReport = report;
}

export async function renderCrashDoctorPanel(container: HTMLElement): Promise<void> {
  const state = getState();
  let gamePath = state.currentSettings?.gamePath || '';
  if (!gamePath) {
    try {
      const { getSettings } = await import('../../../api');
      const settings = await getSettings();
      gamePath = settings.gamePath || '';
    } catch {
      // Fallback if settings query fails
    }
  }

  if (isLoadingCrash) {
    container.innerHTML = `
      <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
        ${subTabHeader()}
        <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; min-height: 400px; gap: 16px;">
          <div class="spinner" style="width: 42px; height: 42px; border-width: 4px;"></div>
          <div style="font-size: 14px; font-weight: 600; color: var(--text-primary);">
            ${escapeHtml(t('crash_doctor.scanning_dumps') || 'Inspecting crash dumps and engine logs...')}
          </div>
        </div>
      </div>
    `;
    setupEventListeners();
    return;
  }

  // Load crash history if empty
  if (gamePath && cachedCrashHistory.length === 0) {
    try {
      cachedCrashHistory = await listAllCrashes(gamePath);
      if (cachedCrashHistory.length > 0 && !selectedCrashId) {
        selectedCrashId = cachedCrashHistory[0].id;
      }
    } catch (e) {
      console.warn('Crash Doctor: Failed to list all crashes', e);
    }
  }

  // Check for the most recent crash if no report loaded yet
  if (!lastCrashReport && gamePath) {
    isLoadingCrash = true;
    try {
      if (selectedCrashId && cachedCrashHistory.length > 0) {
        const item = cachedCrashHistory.find(c => c.id === selectedCrashId);
        if (item) {
          lastCrashReport = await analyzeCrashFolder(item.folderPath, gamePath);
        }
      }
      if (!lastCrashReport) {
        lastCrashReport = await checkLatestCrash(gamePath);
        if (lastCrashReport) {
          selectedCrashId = lastCrashReport.id;
        }
      }
    } catch (e: any) {
      console.error('Crash Doctor: Failed to analyze crash', e);
    } finally {
      isLoadingCrash = false;
    }
  }

  const report = lastCrashReport;

  // Render Master/Detail container wrapping with subTabHeader
  const crashListHtml = renderCrashMasterListHtml(cachedCrashHistory, selectedCrashId, isLoadingCrash);
  const inspectorHtml = renderCrashInspectorHtml(report);

  container.innerHTML = `
    <div class="scanner-view-container" style="height: 100%; display: flex; flex-direction: column; overflow: hidden;">
      ${subTabHeader()}
      
      <div class="scanner-scroll-panel" style="padding: calc(14px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); box-sizing: border-box; display: flex; flex-direction: column; gap: calc(12px * var(--ui-scale, 1)); height: 100%; flex: 1; min-height: 0; overflow: hidden;">
        
        <!-- Header Bar -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: calc(10px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: calc(10px * var(--ui-scale, 1)); flex-shrink: 0;">
          <div style="display: flex; flex-direction: column; gap: 3px;">
            <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
              <span style="display: flex; align-items: center; color: var(--text-muted);">${getIconSvg('shieldCheck', 'pmm-icon-md')}</span>
              <span style="font-size: var(--text-md, 14.5px); font-weight: 700; color: var(--text-primary);">
                ${escapeHtml(t('scanner.title_crash_doctor') || 'Crash Doctor')}
              </span>
            </div>
            <span style="font-size: var(--text-sm, 11px); color: var(--text-muted);">
              ${escapeHtml(t('crash_doctor.history_count', { count: cachedCrashHistory.length }) || `${cachedCrashHistory.length} crash incidents detected on disk`)}
            </span>
          </div>

          <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
            <button id="btn-analyze-latest-crash" class="btn-primary" style="padding: calc(6px * var(--ui-scale, 1)) calc(14px * var(--ui-scale, 1)); font-size: var(--text-sm, 11.5px); display: flex; align-items: center; gap: calc(6px * var(--ui-scale, 1));">
              <span style="display: flex; align-items: center;">${getIconSvg('scan', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.btn_check_again') || 'Check for Crashes Now')}</span>
            </button>
          </div>
        </div>

        <!-- Main 2-Column Master/Detail Layout -->
        <div class="scanner-master-detail" style="grid-template-columns: clamp(260px, calc(320px * var(--ui-scale, 1)), 400px) 1fr; flex: 1; height: 100%; min-height: 0; display: grid; gap: calc(12px * var(--ui-scale, 1)); overflow: hidden;">
          ${crashListHtml}
          
          <div class="scanner-inspector-panel" id="crash-inspector-root" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); padding: calc(16px * var(--ui-scale, 1)) calc(18px * var(--ui-scale, 1)); box-sizing: border-box; overflow-y: auto; gap: calc(14px * var(--ui-scale, 1));">
            ${inspectorHtml}
          </div>
        </div>

      </div>
    </div>
  `;

  // Always re-bind scanner sub-tab events so user can change tabs
  setupEventListeners();

  // Attach Crash Doctor interactive listeners
  attachCrashDoctorListeners(container, gamePath);
}

function getOriginBadge(origin?: string): { label: string; color: string; bg: string } {
  switch (origin) {
    case 'ue4ss':
      return { label: 'UE4SS', color: '#ce93d8', bg: 'rgba(156, 39, 176, 0.15)' };
    case 'pak_mod':
      return { label: 'PAK MOD', color: '#ffb74d', bg: 'rgba(255, 152, 0, 0.15)' };
    case 'out_of_memory':
      return { label: 'OOM RAM', color: '#ef5350', bg: 'rgba(239, 83, 80, 0.18)' };
    case 'directx':
      return { label: 'DIRECTX', color: '#4fc3f7', bg: 'rgba(3, 169, 244, 0.15)' };
    case 'overlay':
      return { label: 'OVERLAY', color: '#ffd54f', bg: 'rgba(255, 193, 7, 0.15)' };
    case 'game_engine':
      return { label: 'GAME', color: '#81c784', bg: 'rgba(76, 175, 80, 0.15)' };
    default:
      return { label: 'INCIDENT', color: 'var(--text-secondary)', bg: 'rgba(255, 255, 255, 0.08)' };
  }
}

function renderCrashMasterListHtml(
  history: CrashHistoryItem[],
  activeId: string | null,
  loading: boolean
): string {
  return `
    <div class="scanner-master-list" style="height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--card-radius); overflow: hidden;">
      <div style="padding: calc(8px * var(--ui-scale, 1)) calc(12px * var(--ui-scale, 1)); border-bottom: 1px solid var(--border); background: var(--bg-secondary); display: flex; justify-content: space-between; align-items: center; flex-shrink: 0;">
        <span style="font-size: var(--text-xs, 11px); font-weight: 700; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px;">
          ${escapeHtml(t('crash_doctor.detected_crashes') || 'Detected Crashes')} (${history.length})
        </span>
      </div>

      <div class="scanner-master-items" style="padding: calc(8px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(6px * var(--ui-scale, 1)); flex: 1; min-height: 0; overflow-y: auto;">
        ${loading ? `
          <div style="padding: calc(24px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); text-align: center; color: var(--text-muted); font-size: var(--text-sm, 12px); display: flex; align-items: center; justify-content: center; gap: 6px;">
            <span>${getIconSvg('scan', 'pmm-icon-xs pmm-icon-spin')}</span>
            <span>${escapeHtml(t('crash_doctor.scanning_dumps') || 'Inspecting crash dumps and engine logs...')}</span>
          </div>
        ` : history.length > 0 ? history.map(item => {
          const isActive = item.id === activeId;
          const badge = getOriginBadge(item.origin);
          return `
            <div class="scanner-mod-list-item crash-list-card ${isActive ? 'active' : ''}" data-crash-id="${escapeHtml(item.id)}" data-folder-path="${escapeHtml(item.folderPath)}" style="padding: calc(8px * var(--ui-scale, 1)) calc(10px * var(--ui-scale, 1)); cursor: pointer; border-radius: 6px; display: flex; flex-direction: column; gap: calc(4px * var(--ui-scale, 1));">
              <div style="display: flex; justify-content: space-between; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
                <span style="font-size: var(--text-base, 12.5px); font-weight: 700; color: ${isActive ? 'var(--accent)' : 'var(--text-primary)'}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-flex; align-items: center; gap: 5px;">
                  <span>${getIconSvg('alertTriangle', 'pmm-icon-xs')}</span>
                  <span>${escapeHtml(item.folderName)}</span>
                </span>
                <div style="display: flex; align-items: center; gap: 4px; flex-shrink: 0;">
                  <span style="font-size: 9.5px; font-weight: 700; background: ${badge.bg}; color: ${badge.color}; padding: 1px 5px; border-radius: 4px; border: 1px solid ${badge.color}33;">
                    ${badge.label}
                  </span>
                  ${item.hasDmp ? `<span style="font-size: 9.5px; font-weight: 700; background: rgba(0, 188, 255, 0.15); color: var(--accent); padding: 1px 5px; border-radius: 4px;">.DMP</span>` : ''}
                </div>
              </div>

              <div style="display: flex; align-items: center; justify-content: space-between; font-size: var(--text-2xs, 10.5px); color: var(--text-secondary);">
                <span style="display: inline-flex; align-items: center; gap: 4px;">
                  <span>${getIconSvg('clock', 'pmm-icon-xs')}</span>
                  <span>${escapeHtml(item.timestamp)}</span>
                </span>
              </div>
            </div>
          `;
        }).join('') : `
          <div style="padding: calc(24px * var(--ui-scale, 1)) calc(16px * var(--ui-scale, 1)); text-align: center; color: var(--text-muted); font-size: var(--text-sm, 12px);">
            ${escapeHtml(t('crash_doctor.no_crashes_title') || 'No Recent Crashes Detected')}
          </div>
        `}
      </div>
    </div>
  `;
}

function renderCrashInspectorHtml(report: UnifiedCrashReport | null): string {
  if (!report) {
    return `
      <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; min-height: 350px; text-align: center; gap: 14px; padding: 24px;">
        <div class="icon-inline" style="color: var(--accent);">${getIconSvg('shieldCheck', 'pmm-icon-xl')}</div>
        <div style="font-size: 16px; font-weight: 700; color: var(--text-primary);">
          ${escapeHtml(t('crash_doctor.no_crashes_title') || 'No Recent Crashes Detected')}
        </div>
        <div style="font-size: 12.5px; color: var(--text-secondary); max-width: 480px; line-height: 1.5;">
          ${escapeHtml(t('crash_doctor.select_incident') || 'Select a crash incident on the left to inspect its forensics.')}
        </div>
      </div>
    `;
  }

  const severityColor = report.severity === 'Fatal' ? '#ef5350' : report.severity === 'Critical' ? '#ff9800' : '#4caf50';
  const xml = report.xmlContext;
  const dmp = report.minidump;
  const originBadge = getOriginBadge(report.originCategory);

  return `
    <!-- Incident Alert Header -->
    <div style="background: rgba(239, 83, 80, 0.08); border: 1px solid rgba(239, 83, 80, 0.3); border-radius: 8px; padding: 14px 18px; display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-shrink: 0;">
      <div style="display: flex; gap: 14px; align-items: flex-start;">
        <div class="icon-inline" style="color: #ef5350;">${getIconSvg('alertTriangle', 'pmm-icon-lg')}</div>
        <div>
          <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px; flex-wrap: wrap;">
            <span style="font-size: 15px; font-weight: 700; color: var(--text-primary);">
              ${escapeHtml(t('crash_doctor.incident_detected') || 'Game Crash Incident Detected')}
            </span>
            <span style="background: ${severityColor}22; color: ${severityColor}; border: 1px solid ${severityColor}55; padding: 2px 7px; border-radius: 4px; font-size: 10.5px; font-weight: 700;">
              ${escapeHtml(report.severity)}
            </span>
            <span style="background: ${originBadge.bg}; color: ${originBadge.color}; border: 1px solid ${originBadge.color}44; padding: 2px 7px; border-radius: 4px; font-size: 10.5px; font-weight: 700;">
              ${originBadge.label}
            </span>
          </div>
          <div style="font-size: 11.5px; color: var(--text-secondary); margin-bottom: 6px;">
            ${escapeHtml(t('crash_doctor.logged_at') || 'Captured at')}: <strong>${escapeHtml(report.timestamp)}</strong> &nbsp;|&nbsp; 
            ${escapeHtml(t('crash_doctor.cause_tag') || 'Primary Cause')}: <code style="color: #ffb74d;">${escapeHtml(report.primaryCause)}</code>
            ${report.culpritName ? ` &nbsp;|&nbsp; ${escapeHtml(t('crash_doctor.lbl_faulting_module') || 'Culprit')}: <strong style="color: #90caf9;">${escapeHtml(report.culpritName)}</strong>` : ''}
          </div>
          <div style="font-size: 12.5px; font-weight: 600; color: var(--text-primary); line-height: 1.4;">
            ${escapeHtml(
              t(`crash_doctor.cause_${report.primaryCause}`, {
                name: report.culpritName || '',
                avail: report.xmlContext?.memoryStats.availablePhysicalGb ? report.xmlContext.memoryStats.availablePhysicalGb.toFixed(2) : '0',
              }) || report.humanSummary
            )}
          </div>
        </div>
      </div>

      <!-- Header Action Controls & Mode Switcher -->
      <div style="display: flex; flex-direction: column; align-items: flex-end; gap: 8px; flex-shrink: 0;">
        <div style="display: flex; gap: 6px;">
          <button id="btn-export-crash-bundle" class="scanner-btn-run" style="padding: 6px 12px; font-size: 11.5px; display: flex; align-items: center; gap: 6px; background: var(--accent); color: #fff; font-weight: 700; border: none; border-radius: 6px; cursor: pointer;">
            <span>${getIconSvg('package', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.btn_export_bundle') || 'Export ZIP')}</span>
          </button>
          <button id="btn-open-crash-folder" class="scanner-sub-tab" style="padding: 5px 10px; font-size: 11px; border: 1px solid var(--border); text-align: center; cursor: pointer; border-radius: 6px; display: flex; align-items: center;">
            <span>${getIconSvg('folderOpen', 'pmm-icon-xs')}</span>
          </button>
        </div>

        <!-- Mode Switcher -->
        <div style="display: inline-flex; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; padding: 2px; gap: 2px;">
          <button type="button" id="btn-mode-summary" class="inspector-mode-btn ${activeInspectorMode === 'summary' ? 'active' : ''}" style="padding: 4px 10px; font-size: 11px; font-weight: 600; border: none; border-radius: 4px; background: ${activeInspectorMode === 'summary' ? 'var(--accent)' : 'transparent'}; color: ${activeInspectorMode === 'summary' ? '#fff' : 'var(--text-secondary)'}; cursor: pointer; display: flex; align-items: center; gap: 5px;">
            <span>${getIconSvg('barChart', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.btn_mode_summary') || 'Summary')}</span>
          </button>
          <button type="button" id="btn-mode-raw" class="inspector-mode-btn ${activeInspectorMode === 'raw' ? 'active' : ''}" style="padding: 4px 10px; font-size: 11px; font-weight: 600; border: none; border-radius: 4px; background: ${activeInspectorMode === 'raw' ? 'var(--accent)' : 'transparent'}; color: ${activeInspectorMode === 'raw' ? '#fff' : 'var(--text-secondary)'}; cursor: pointer; display: flex; align-items: center; gap: 5px;">
            <span>${getIconSvg('terminal', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.btn_mode_raw') || 'Raw Dump')}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Inspector Dynamic Content (Summary Mode vs Full-Height Raw Dump Mode) -->
    ${activeInspectorMode === 'raw' ? `
      <!-- Full-Height Dedicated Raw Forensics Viewer -->
      <div style="display: flex; flex-direction: column; flex: 1; height: 100%; min-height: 0; overflow: hidden;">
        ${renderRawDumpViewerHtml(report)}
      </div>
    ` : `
      <!-- Forensic Breakdown & Solutions Grid -->
      <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 14px;">
        <!-- Technical Forensics -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 14px;">
          <div style="font-size: 13px; font-weight: 700; color: var(--text-primary); margin-bottom: 12px; display: flex; align-items: center; gap: 6px;">
            <span>${getIconSvg('search', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.sec_forensics') || 'System & Memory Forensics')}</span>
          </div>

          <div style="display: flex; flex-direction: column; gap: 8px; font-size: 11.5px;">
            ${dmp ? `
              <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_exception_code') || 'Exception Code')}:</span>
                <code style="color: #ef5350;">${escapeHtml(dmp.exceptionCodeHex)}</code>
              </div>
              <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_rip_address') || 'Faulting Instruction (RIP)')}:</span>
                <code style="color: var(--accent);">${escapeHtml(dmp.faultingAddressHex)}</code>
              </div>
              <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_faulting_module') || 'Culprit Module')}:</span>
                <strong style="color: var(--warning);">${escapeHtml(report.culpritName || dmp.faultingModule?.name || 'Palworld-Win64-Shipping.exe')}</strong>
              </div>
            ` : ''}

            ${xml ? `
              <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_uptime') || 'Game Uptime Before Crash')}:</span>
                <span>${xml.secondsSinceStart}s</span>
              </div>
              ${xml.crashedThreadName ? `
                <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                  <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_crashed_thread') || 'Crashed Thread')}:</span>
                  <span style="color: var(--warning); font-weight: 600;">${escapeHtml(xml.crashedThreadName)}</span>
                </div>
              ` : ''}
              <div style="display: flex; justify-content: space-between; border-bottom: 1px solid var(--border); padding-bottom: 4px;">
                <span style="color: var(--text-secondary);">${escapeHtml(t('crash_doctor.lbl_avail_ram') || 'Available RAM at Crash')}:</span>
                <span style="color: ${xml.memoryStats.isOomRisk ? 'var(--danger)' : 'var(--success)'}; font-weight: 600;">
                  ${xml.memoryStats.availablePhysicalGb !== undefined ? `${xml.memoryStats.availablePhysicalGb.toFixed(2)} GB ${xml.memoryStats.totalPhysicalGb ? `/ ${xml.memoryStats.totalPhysicalGb.toFixed(2)} GB` : ''}` : 'N/A'}
                </span>
              </div>
            ` : ''}
          </div>

          <div style="margin-top: 12px;">
            <div style="font-size: 10.5px; font-weight: 700; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 4px;">
              ${escapeHtml(t('crash_doctor.lbl_raw_message') || 'Raw Engine Message')}
            </div>
            <pre style="background: var(--bg-code, #18191c); border: 1px solid var(--border); border-radius: 6px; padding: 8px; font-size: 10.5px; font-family: monospace; color: var(--text-primary); white-space: pre-wrap; word-break: break-all; margin: 0; max-height: 100px; overflow-y: auto;">${escapeHtml(report.technicalDetails)}</pre>
          </div>
        </div>

        <!-- Solutions -->
        <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 14px; display: flex; flex-direction: column;">
          <div style="font-size: 13px; font-weight: 700; color: var(--text-primary); margin-bottom: 12px; display: flex; align-items: center; gap: 6px;">
            <span>${getIconSvg('sparkles', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.sec_suggestions') || 'Recommended Solutions')}</span>
          </div>

          <div style="display: flex; flex-direction: column; gap: 8px; flex: 1;">
            ${report.suggestions.map(sug => `
              <div class="crash-suggestion-card" data-action="${escapeHtml(sug.actionType)}" data-target-path="${escapeHtml(sug.targetPath || '')}" style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 6px; padding: 10px; ${sug.actionType === 'open_folder' ? 'cursor: pointer; transition: background 0.2s;' : ''}">
                <div style="font-size: 12.5px; font-weight: 700; color: var(--success); margin-bottom: 2px; display: flex; align-items: center; justify-content: space-between;">
                  <span>${escapeHtml(t(sug.labelKey) || sug.id)}</span>
                  ${sug.actionType === 'open_folder' ? `<span style="display: inline-flex; align-items: center; color: var(--text-muted);">${getIconSvg('externalLink', 'pmm-icon-xs')}</span>` : ''}
                </div>
                <div style="font-size: 11.5px; color: var(--text-secondary); line-height: 1.35;">
                  ${escapeHtml(t(sug.descriptionKey) || '')}
                </div>
              </div>
            `).join('')}
          </div>
        </div>
      </div>

      <!-- Active Mods Snapshot -->
      <div style="background: var(--bg-card); border: 1px solid var(--border); border-radius: 8px; padding: 14px;">
        <div style="font-size: 13px; font-weight: 700; color: var(--text-primary); margin-bottom: 10px; display: flex; align-items: center; justify-content: space-between;">
          <div style="display: flex; align-items: center; gap: 6px;">
            <span>${getIconSvg('mods', 'pmm-icon-xs')}</span> <span>${escapeHtml(t('crash_doctor.sec_active_mods') || 'Active Installed Mods at Crash Time')} (${report.activeMods.length})</span>
          </div>
          <button type="button" id="btn-switch-to-raw" style="font-size: 11px; color: var(--accent); background: none; border: none; cursor: pointer; text-decoration: underline; display: inline-flex; align-items: center; gap: 4px;">
            <span>${escapeHtml(t('crash_doctor.btn_view_full_raw') || 'View Raw Dump')}</span>
            <span>${getIconSvg('externalLink', 'pmm-icon-xs')}</span>
          </button>
        </div>

        <div style="max-height: 180px; overflow-y: auto; border: 1px solid var(--border); border-radius: 6px;">
          <table style="width: 100%; border-collapse: collapse; font-size: 11.5px; text-align: left;">
            <thead>
              <tr style="background: var(--bg-secondary); border-bottom: 1px solid var(--border); color: var(--text-secondary);">
                <th style="padding: 6px 10px;">${escapeHtml(t('card.table_col_name') || 'Mod Name')}</th>
                <th style="padding: 6px 10px;">${escapeHtml(t('card.table_col_type') || 'Type')}</th>
                <th style="padding: 6px 10px;">${escapeHtml(t('card.table_col_version') || 'Version')}</th>
              </tr>
            </thead>
            <tbody>
              ${report.activeMods.map(m => `
                <tr style="border-bottom: 1px solid var(--border);">
                  <td style="padding: 6px 10px; font-weight: 600; color: var(--text-primary);">${escapeHtml(m.name)}</td>
                  <td style="padding: 6px 10px;"><span class="badge badge-subtle" style="font-size: 9.5px;">${escapeHtml(m.modType)}</span></td>
                  <td style="padding: 6px 10px; color: var(--text-secondary);">${escapeHtml(m.version || '1.0')}</td>
                </tr>
              `).join('')}
            </tbody>
          </table>
        </div>
      </div>
    `}
  `;
}

function attachCrashDoctorListeners(container: HTMLElement, gamePath: string): void {
  // Master list incident selection
  container.querySelectorAll('.crash-list-card').forEach(card => {
    card.addEventListener('click', async () => {
      const crashId = card.getAttribute('data-crash-id');
      const folderPath = card.getAttribute('data-folder-path');
      if (!crashId || !folderPath || crashId === selectedCrashId) return;

      selectedCrashId = crashId;

      // Update active highlight in left list
      container.querySelectorAll('.crash-list-card').forEach(el => {
        if (el.getAttribute('data-crash-id') === crashId) {
          el.classList.add('active');
          const span = el.querySelector('span');
          if (span) (span as HTMLElement).style.color = 'var(--accent)';
        } else {
          el.classList.remove('active');
          const span = el.querySelector('span');
          if (span) (span as HTMLElement).style.color = 'var(--text-primary)';
        }
      });

      // Update inspector root smoothly
      const inspectorRoot = container.querySelector('#crash-inspector-root');
      if (inspectorRoot) {
        inspectorRoot.innerHTML = `
          <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; min-height: 250px; gap: 12px;">
            <div class="spinner" style="width: 32px; height: 32px; border-width: 3px;"></div>
            <div style="font-size: 12.5px; color: var(--text-secondary);">${escapeHtml(t('crash_doctor.scanning_dumps') || 'Inspecting crash dump...')}</div>
          </div>
        `;
      }

      try {
        lastCrashReport = await analyzeCrashFolder(folderPath, gamePath);
      } catch (err) {
        console.error('Crash Doctor: Failed to switch crash incident', err);
      } finally {
        if (inspectorRoot) {
          inspectorRoot.innerHTML = renderCrashInspectorHtml(lastCrashReport);
          attachInspectorSubListeners(container, gamePath);
        }
      }
    });
  });

  // Re-scan crashes button
  container.querySelector('#btn-analyze-latest-crash')?.addEventListener('click', async () => {
    lastCrashReport = null;
    cachedCrashHistory = [];
    selectedCrashId = null;
    await renderCrashDoctorPanel(container);
  });

  attachInspectorSubListeners(container, gamePath);
}

function attachInspectorSubListeners(container: HTMLElement, gamePath: string): void {
  // Export support bundle ZIP
  container.querySelector('#btn-export-crash-bundle')?.addEventListener('click', async () => {
    if (!lastCrashReport) return;
    const { save } = await import('@tauri-apps/plugin-dialog');
    const defaultFilename = `PalModManager_Crash_Support_${lastCrashReport.timestamp.replace(/[: -]/g, '_')}.zip`;

    try {
      const selectedPath = await save({
        defaultPath: defaultFilename,
        filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
        title: t('crash_doctor.dialog_save_bundle_title') || 'Save Crash Support Bundle (.zip)',
      });

      if (!selectedPath) return;

      showToast(t('crash_doctor.exporting_bundle') || 'Compressing support bundle...', 'info');
      const res = await exportCrashSupportBundle(lastCrashReport, selectedPath, gamePath);
      if (res.success) {
        showToast(
          t('crash_doctor.toast_bundle_success', { path: res.bundlePath }) ||
            `Support bundle created: ${res.bundlePath}`,
          'success'
        );
      } else {
        showToast(res.error || 'Failed to export bundle', 'error');
      }
    } catch (e: any) {
      showToast(String(e), 'error');
    }
  });

  // Open crash directory button
  container.querySelector('#btn-open-crash-folder')?.addEventListener('click', async () => {
    if (!lastCrashReport?.crashFolder) return;
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      await invoke('open_path', { path: lastCrashReport.crashFolder });
    } catch (e) {
      console.error('Crash Doctor: Failed to open crash folder:', e);
    }
  });

  // Interactive suggestion cards
  container.querySelectorAll('.crash-suggestion-card').forEach((card) => {
    card.addEventListener('click', async () => {
      const action = card.getAttribute('data-action');
      const targetPath = card.getAttribute('data-target-path');
      if (action === 'open_folder' && (targetPath || lastCrashReport?.crashFolder)) {
        const { invoke } = await import('@tauri-apps/api/core');
        try {
          await invoke('open_path', { path: targetPath || lastCrashReport!.crashFolder });
        } catch (e) {
          console.error(e);
        }
      }
    });
  });

  // Mode switcher listeners: Summary vs Full-Height Raw Dump
  const updateInspectorMode = (mode: 'summary' | 'raw') => {
    if (activeInspectorMode === mode) return;
    activeInspectorMode = mode;
    const inspectorRoot = container.querySelector('#crash-inspector-root');
    if (inspectorRoot && lastCrashReport) {
      inspectorRoot.innerHTML = renderCrashInspectorHtml(lastCrashReport);
      attachInspectorSubListeners(container, gamePath);
    }
  };

  container.querySelector('#btn-mode-summary')?.addEventListener('click', () => updateInspectorMode('summary'));
  container.querySelector('#btn-mode-raw')?.addEventListener('click', () => updateInspectorMode('raw'));
  container.querySelector('#btn-switch-to-raw')?.addEventListener('click', () => updateInspectorMode('raw'));

  // Attach raw minidump & callstack viewer interactive listeners
  attachRawViewerListeners(container, lastCrashReport);
}
