import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import { getState } from '../../../../state';
import type { ModInfo } from '../../../../types';
import type { ScanResult } from '../../mod';

export function buildStatCardsHtml(res: ScanResult, activeConflictsCount: number, resolvedPakCount: number, hasConflicts: boolean): string {
  return `
    <!-- Stats Row -->
    <div style="display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;flex-shrink:0;">
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_mods_scanned'))}</div>
        <div class="premium-stat-value">${res.totalScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_palschema_json'))}</div>
        <div class="premium-stat-value">${res.palschemaScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_ue4ss_lua'))}</div>
        <div class="premium-stat-value">${res.ue4ssScanned}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_pak_files'))}</div>
        <div class="premium-stat-value">${res.pakScanned ?? 0}</div>
      </div>
      <div class="premium-stat-card">
        <div style="font-size:10px;font-weight:700;color:var(--text-muted);letter-spacing:0.5px;text-transform:uppercase;">${escapeHtml(t('scanner.stat_conflicts'))}</div>
        <div class="premium-stat-value ${hasConflicts ? 'danger' : 'success'}">${hasConflicts ? activeConflictsCount : (resolvedPakCount > 0 ? `0 (✓${resolvedPakCount})` : '0')}</div>
      </div>
    </div>
  `;
}

export function buildInfoBannerHtml(): string {
  return `
    <!-- Info notice banner -->
    <div class="scanner-info-banner" style="margin-bottom: 20px; padding: 14px 18px; background: var(--bg-card); border: 1px solid var(--border); border-left: 3px solid var(--accent); border-radius: var(--card-radius); display: flex; flex-direction: column; gap: 8px; font-size: 12px; line-height: 1.5;">
      <div style="font-weight: 700; color: var(--accent); display: flex; align-items: center; gap: 8px; font-size: 13px;">
        <span style="font-size: 14px;">ℹ</span>
        <span>${escapeHtml(t('scanner.info_title'))}</span>
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_ue4ss_title'))}:</strong> ${escapeHtml(t('scanner.info_ue4ss_desc'))}
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_palschema_title'))}:</strong> ${escapeHtml(t('scanner.info_palschema_desc'))}
      </div>
      <div style="color: var(--text-secondary);">
        <strong style="color: var(--text-primary);">${escapeHtml(t('scanner.info_pak_title'))}:</strong> ${escapeHtml(t('scanner.info_pak_desc'))}
      </div>
    </div>
  `;
}

export function buildGamepassNoticeHtml(res: ScanResult): string {
  if (!res.isGamepass || !res.gamepassNotices || res.gamepassNotices.length === 0) return '';
  const gpCount = res.gamepassNotices.length;
  return `
    <div class="scanner-card-section" style="border-color: var(--border); background: var(--bg-card); margin-bottom: 20px;">
      <div class="scanner-card-header" style="background: var(--bg-primary); border-bottom: 1px solid var(--border); display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
        <span style="color: var(--accent); display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 13px;">
          <span>🎮</span> ${escapeHtml(t('scanner.gamepass_notice_title'))} (${gpCount})
        </span>
        <div style="display: flex; align-items: center; gap: 10px;">
          <span style="font-size: 10px; color: var(--text-muted); font-weight: 600;">PC Game Pass (WinGDK)</span>
          <button id="btn-convert-all-gamepass" class="btn btn-primary btn-sm" style="font-size: 10.5px; padding: 4px 12px; display: inline-flex; align-items: center; gap: 4px;">
            <span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_all_gamepass', { count: gpCount }))}</span>
          </button>
        </div>
      </div>
      <div class="scanner-card-body" style="gap: 10px; padding: 14px 16px;">
        <div style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
          ${escapeHtml(t('scanner.gamepass_notice_desc'))}
        </div>
        <div style="display: flex; flex-direction: column; gap: 6px; margin-top: 4px;">
          ${res.gamepassNotices.map(n => `
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: var(--radius); font-size: 11px; flex-wrap: wrap; gap: 8px;">
              <div style="display: flex; align-items: center; gap: 8px; overflow: hidden; min-width: 200px;">
                <span style="font-weight: 700; color: var(--text-primary);">${escapeHtml(n.modName)}</span>
                <span style="color: var(--text-muted); font-family: monospace; font-size: 10px;">(${escapeHtml(n.pakFilename)})</span>
              </div>
              <div style="display: flex; align-items: center; gap: 8px; flex-shrink: 0;">
                <span style="font-size: 9px; padding: 2px 6px; background: rgba(255, 170, 0, 0.15); color: #ffaa00; border: 1px solid rgba(255, 170, 0, 0.3); border-radius: 4px; font-weight: 700; text-transform: uppercase;">
                  ${escapeHtml(t('scanner.gamepass_missing_badge'))} (${n.missingContainers.join(', ')})
                </span>
                <button class="btn btn-secondary btn-sm convert-single-gamepass-btn" data-mod-id="${escapeHtml(n.modId)}" style="font-size: 10px; padding: 2px 8px; display: inline-flex; align-items: center; gap: 4px;">
                  <span>⚡</span> <span>${escapeHtml(t('scanner.btn_convert_gamepass'))}</span>
                </button>
              </div>
            </div>
          `).join('')}
        </div>
      </div>
    </div>
  `;
}

export function buildSchemaNoticesHtml(res: ScanResult): string {
  const schemaNoticeCount = res.schemaNotices ? res.schemaNotices.length : 0;
  if (schemaNoticeCount === 0) return '';
  return `
    <div style="display: flex; gap: 20px; flex-wrap: wrap; width: 100%; align-items: start; margin-bottom: 20px;">
      <details class="scanner-card-section" style="flex: 1; min-width: 340px; cursor: pointer; border-color: rgba(56, 189, 248, 0.3);" open>
        <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(56, 189, 248, 0.05); border-bottom: 1px solid rgba(56, 189, 248, 0.15);">
          <span style="color: #38bdf8; display: flex; align-items: center; gap: 6px; font-weight: 700;">
            <span>⚡</span> <span>${escapeHtml(t('scanner.schema_notices_title', { count: schemaNoticeCount }) || `Engine Schema Compatibility (${schemaNoticeCount})`)}</span>
          </span>
          <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.schema_notices_desc') || 'Verified against Palworld v1.0.3 USMAP Schema')}</span>
        </summary>
        <div class="scanner-card-body" style="cursor: default; gap: 10px; padding-top: 14px;">
          ${res.schemaNotices!.map(n => `
            <div class="scanner-conflict-item" style="border-left: 2.5px solid #38bdf8;">
              <div class="scanner-conflict-header">
                <span class="scanner-conflict-title">${escapeHtml(n.modName)}</span>
                <span class="badge" style="font-size: 9.5px; padding: 2px 6px; background: rgba(56, 189, 248, 0.12); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.25); white-space: nowrap;">${escapeHtml(n.structName)}</span>
              </div>
              <div class="scanner-conflict-path" style="font-size: 10.5px; color: var(--text-secondary); margin-top: 3px;">
                <span>📄</span> <span>${escapeHtml(n.assetPath)}</span>
              </div>
              <div style="font-size: 10.5px; color: var(--text-muted); margin-top: 4px; line-height: 1.4;">
                ${escapeHtml(n.message)}
              </div>
            </div>
          `).join('')}
        </div>
      </details>
    </div>
  `;
}

export function buildFrameworkMissingHtml(): string {
  const state = getState();
  const allMods: ModInfo[] = state.allMods || [];
  const isAltermaticInstalled = state.dependencies?.altermatic_installed || 
    allMods.some((m: ModInfo) => m.enabled && (m.nexusModId === 1626 || m.name.toLowerCase().includes('altermatic - runtime') || (m.name.toLowerCase().startsWith('altermatic') && m.type === 'altermatic')));

  const altermaticMods = allMods.filter((m: ModInfo) => m.enabled && m.type === 'altermatic' && m.nexusModId !== 1626 && !m.name.toLowerCase().includes('altermatic - runtime') && !m.name.toLowerCase().startsWith('altermatic'));
  const isAltermaticMissing = altermaticMods.length > 0 && !isAltermaticInstalled;

  if (!isAltermaticMissing) return '';

  return `
    <div style="width: 100%; margin-bottom: 20px; background: rgba(255, 118, 117, 0.12); border: 1px solid rgba(255, 118, 117, 0.35); border-radius: var(--card-radius); padding: 14px 18px; display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap;">
      <div style="display: flex; align-items: center; gap: 12px;">
        <span style="font-size: 22px;">⚠️</span>
        <div style="display: flex; flex-direction: column; gap: 2px;">
          <span style="font-size: 13px; font-weight: 700; color: var(--type-altermatic);">${escapeHtml(t('installer.altermatic_missing_title') || 'Altermatic Framework Required')}</span>
          <span style="font-size: 11px; color: var(--text-secondary);">${escapeHtml(t('scanner.altermatic_missing_body', { count: altermaticMods.length, names: altermaticMods.map(m => m.name).slice(0, 3).join(', ') }) || `You have ${altermaticMods.length} active Altermatic replacer mod(s) (${altermaticMods.map(m => m.name).slice(0, 3).join(', ')}), but the base Altermatic framework is not installed.`)}</span>
        </div>
      </div>
      <button id="btn-scanner-download-altermatic" class="btn btn-secondary" style="font-size: 11px; font-weight: 700; padding: 6px 14px; border-color: var(--type-altermatic); color: var(--type-altermatic); white-space: nowrap;">
        <span>📦</span> <span>${escapeHtml(t('installer.btn_get_altermatic') || 'Get Altermatic (#1626)')}</span>
      </button>
    </div>
  `;
}

export function buildWarningsHtml(res: ScanResult): string {
  if (!res.warnings || res.warnings.length === 0) return '';
  return `
    <details class="scanner-card-section" style="margin-top: 24px; cursor: pointer;">
      <summary class="scanner-card-header" style="outline:none;">
        <span>${escapeHtml(t('scanner.warnings_title', { count: res.warnings.length }))}</span>
        <span style="font-size:10px;color:var(--warning);">${escapeHtml(t('scanner.warnings_desc'))}</span>
      </summary>
      <div class="scanner-card-body" style="cursor: default; background: rgba(0,0,0,0.15);">
        ${res.warnings.map(w => `
          <div class="scanner-warning-row" style="display: flex; gap: 8px; align-items: center; font-size: 11px; padding: 4px 0;">
            <span class="scanner-warning-icon" style="color: var(--warning);">⚠</span>
            <span>${escapeHtml(w)}</span>
          </div>
        `).join('')}
      </div>
    </details>
  `;
}
