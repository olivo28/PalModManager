import { escapeHtml } from '../../rendering';
import { t } from '../../../../utils/i18n';
import type { ScanResult, UsmapHookDiagnostic } from '../../mod';

export function isItemBroken(diag: UsmapHookDiagnostic): boolean {
  return diag.status === 'broken_class' ||
    diag.status === 'broken_function' ||
    diag.status === 'broken_table' ||
    diag.status === 'broken_struct';
}

export function renderHookItem(diag: UsmapHookDiagnostic): string {
  const isBroken = isItemBroken(diag);
  let statusBadge = '';
  let borderCol = 'var(--success)';

  let categoryBadge = '';
  if (diag.category === 'palschema') {
    categoryBadge = `
      <span style="font-size: 9.5px; font-weight: 700; color: var(--accent); background: rgba(0,210,255,0.1); border: 1px solid rgba(0,210,255,0.25); border-radius: 4px; padding: 2px 6px;">
        📊 ${escapeHtml(t('scanner.usmap_category_palschema') || 'PalSchema')}
      </span>
    `;
  } else if (diag.category === 'pak') {
    categoryBadge = `
      <span style="font-size: 9.5px; font-weight: 700; color: var(--warning); background: rgba(255,170,0,0.1); border: 1px solid rgba(255,170,0,0.25); border-radius: 4px; padding: 2px 6px;">
        📦 ${escapeHtml(t('scanner.usmap_category_pak') || 'Pak Asset')}
      </span>
    `;
  } else {
    categoryBadge = `
      <span style="font-size: 9.5px; font-weight: 700; color: #a78bfa; background: rgba(167,139,250,0.1); border: 1px solid rgba(167,139,250,0.25); border-radius: 4px; padding: 2px 6px;">
        ⚡ ${escapeHtml(t('scanner.usmap_category_ue4ss') || 'Lua Hook')}
      </span>
    `;
  }

  if (diag.status === 'broken_class') {
    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--danger); background: rgba(255,75,75,0.15); border: 1px solid rgba(255,75,75,0.3); border-radius: 4px; padding: 2px 6px;">
        ❌ ${escapeHtml(t('scanner.usmap_status_broken_class') || 'Missing Class')}
      </span>
    `;
    borderCol = 'var(--danger)';
  } else if (diag.status === 'broken_function') {
    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--warning); background: rgba(255,170,0,0.15); border: 1px solid rgba(255,170,0,0.3); border-radius: 4px; padding: 2px 6px;">
        ⚠️ ${escapeHtml(t('scanner.usmap_status_broken_function') || 'Missing Function')}
      </span>
    `;
    borderCol = 'var(--warning)';
  } else if (diag.status === 'broken_table') {
    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--danger); background: rgba(255,75,75,0.15); border: 1px solid rgba(255,75,75,0.3); border-radius: 4px; padding: 2px 6px;">
        ❌ ${escapeHtml(t('scanner.usmap_status_broken_table') || 'Missing DataTable')}
      </span>
    `;
    borderCol = 'var(--danger)';
  } else if (diag.status === 'broken_struct') {
    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--danger); background: rgba(255,75,75,0.15); border: 1px solid rgba(255,75,75,0.3); border-radius: 4px; padding: 2px 6px;">
        ❌ ${escapeHtml(t('scanner.usmap_status_broken_struct') || 'Unmapped Engine Struct')}
      </span>
    `;
    borderCol = 'var(--danger)';
  } else if (diag.status === 'blueprint_asset') {
    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--accent); background: rgba(0,210,255,0.12); border: 1px solid rgba(0,210,255,0.3); border-radius: 4px; padding: 2px 6px;">
        🔷 ${escapeHtml(t('scanner.usmap_status_blueprint') || 'Dynamic Blueprint Hook')}
      </span>
    `;
    borderCol = 'var(--accent)';
  } else {
    const validLabel = diag.category === 'palschema'
      ? (t('scanner.usmap_status_valid_table') || 'Valid DataTable')
      : diag.category === 'pak'
      ? (t('scanner.usmap_status_valid_pak') || 'Valid Pak Asset')
      : (t('scanner.usmap_status_valid') || 'Valid Native Class');

    statusBadge = `
      <span style="font-size: 10px; font-weight: 700; color: var(--success); background: rgba(76,175,80,0.15); border: 1px solid rgba(76,175,80,0.3); border-radius: 4px; padding: 2px 6px;">
        ✓ ${escapeHtml(validLabel)}
      </span>
    `;
    borderCol = 'var(--success)';
  }

  return `
    <div class="scanner-conflict-item" style="border-left: 3px solid ${borderCol};">
      <div class="scanner-conflict-header" style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 6px;">
        <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
          ${categoryBadge}
          <span class="scanner-conflict-title">${escapeHtml(diag.hookTarget)}</span>
          ${statusBadge}
        </div>
        ${!diag.suggestion ? `
          <button class="btn btn-secondary btn-xs scan-code-jump-btn" data-mod-id="${escapeHtml(diag.modId)}" data-file-path="${escapeHtml(diag.filePath)}" data-line="${diag.lineNumber}" style="font-size: 10px; padding: 2px 8px;">
            📝 ${escapeHtml(t('common.edit') || 'Edit in Code')}
          </button>
        ` : ''}
      </div>
      <div style="display: flex; justify-content: space-between; align-items: center; font-size: 11px; margin-top: 4px; flex-wrap: wrap; gap: 4px;">
        <span style="color: var(--text-primary); font-weight: 600;">📦 ${escapeHtml(diag.modName)} <span style="font-family: monospace; font-size: 10.5px; color: var(--text-muted); font-weight: normal;">(${escapeHtml(diag.filePath)}:L${diag.lineNumber})</span></span>
        <span style="font-size: 10.5px; color: ${isBroken ? 'var(--danger)' : 'var(--text-muted)'};">${escapeHtml(diag.reason)}</span>
      </div>
      ${diag.suggestion ? `
        <div style="background: rgba(0, 210, 255, 0.08); border: 1px solid rgba(0, 210, 255, 0.25); border-radius: 6px; padding: 6px 12px; margin-top: 8px; display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap;">
          <div style="display: flex; align-items: center; gap: 8px; font-size: 11px;">
            <span style="color: var(--accent); font-weight: 700;">💡 ${escapeHtml(t('scanner.usmap_suggested_fix') || 'Suggested Fix')}:</span>
            <code style="font-family: monospace; color: var(--accent); font-weight: 600; background: rgba(0,0,0,0.25); padding: 2px 8px; border-radius: 4px; border: 1px solid rgba(0,210,255,0.2);">${escapeHtml(diag.suggestion)}</code>
          </div>
          <button class="btn btn-primary btn-xs scan-code-jump-btn" data-mod-id="${escapeHtml(diag.modId)}" data-file-path="${escapeHtml(diag.filePath)}" data-line="${diag.lineNumber}" style="font-size: 10.5px; padding: 3px 10px; font-weight: 700;">
            📝 ${escapeHtml(t('scanner.usmap_open_and_fix') || 'Open & Fix')}
          </button>
        </div>
      ` : ''}
    </div>
  `;
}

export function buildUsmapSectionHtml(res: ScanResult): string {
  if (!res.usmapDiagnostics || !res.usmapDiagnostics.hasUsmap) return '';
  const d = res.usmapDiagnostics;
  const isClean = d.brokenHooks === 0;

  const brokenDiags = d.diagnostics.filter(isItemBroken);
  const validDiags = d.diagnostics.filter(diag => !isItemBroken(diag));

  return `
    <details class="scanner-card-section" style="width: 100%; margin-bottom: 20px; border-color: ${isClean ? 'rgba(76, 175, 80, 0.3)' : 'rgba(255, 75, 75, 0.3)'};" open>
      <summary class="scanner-card-header" style="outline: none; display: flex; align-items: center; justify-content: space-between; background: ${isClean ? 'rgba(76, 175, 80, 0.04)' : 'rgba(255, 75, 75, 0.04)'};">
        <span style="color: ${isClean ? 'var(--success)' : 'var(--danger)'}; font-weight: 700; display: flex; align-items: center; gap: 8px;">
          <span>⚡</span>
          <span>${escapeHtml(t('scanner.usmap_section_title') || 'Unreal Engine Schema & Hook Diagnostics (USMAP)')}</span>
        </span>
        <span style="font-size: 10px; color: var(--text-muted);">
          ${escapeHtml(t('scanner.usmap_version_tag', { version: d.usmapVersion || 'v0.4.x' }) || `Palworld Schema ${d.usmapVersion}`)}
        </span>
      </summary>
      <div class="scanner-card-body" style="gap: 12px; padding: 14px;">
        <div style="display: flex; gap: 12px; align-items: center; flex-wrap: wrap;">
          <div style="padding: 6px 12px; background: rgba(76, 175, 80, 0.1); border: 1px solid rgba(76, 175, 80, 0.25); border-radius: 4px; font-size: 11px; font-weight: 700; color: var(--success);">
            ✅ ${d.validHooks} ${escapeHtml(t('scanner.usmap_valid_hooks') || 'Valid Hooks')}
          </div>
          ${d.brokenHooks > 0 ? `
            <div style="padding: 6px 12px; background: rgba(255, 75, 75, 0.12); border: 1px solid rgba(255, 75, 75, 0.3); border-radius: 4px; font-size: 11px; font-weight: 700; color: var(--danger);">
              ⚠️ ${d.brokenHooks} ${escapeHtml(t('scanner.usmap_broken_hooks') || 'Broken / Obsolete Hooks Detected')}
            </div>
          ` : ''}
          <div style="font-size: 11px; color: var(--text-muted); margin-left: auto;">
            ${escapeHtml(t('scanner.usmap_total_checked', { count: d.totalHooksChecked }) || `${d.totalHooksChecked} total hooks inspected against Palworld reflection schema`)}
          </div>
        </div>

        ${brokenDiags.length > 0 ? `
          <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 6px;">
            ${brokenDiags.map(renderHookItem).join('')}
          </div>
        ` : `
          <div style="display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--success); background: rgba(76, 175, 80, 0.08); padding: 10px 14px; border-radius: var(--radius); border: 1px solid rgba(76, 175, 80, 0.2); margin-top: 4px;">
            <span style="font-size: 15px;">✅</span>
            <span>${escapeHtml(t('scanner.usmap_all_hooks_verified', { count: d.validHooks }) || `All ${d.validHooks} hooks verified against Palworld reflection schema and C++ SDK.`)}</span>
          </div>
        `}

        ${validDiags.length > 0 ? `
          <details class="scanner-card-section" style="margin-top: 10px; border-color: rgba(76, 175, 80, 0.25);">
            <summary class="scanner-card-header" style="cursor: pointer; outline: none; display: flex; align-items: center; justify-content: space-between; background: rgba(76, 175, 80, 0.04); border-bottom: 1px solid rgba(76, 175, 80, 0.1);">
              <span style="color: var(--success); font-weight: 700; display: flex; align-items: center; gap: 6px; font-size: 11.5px;">
                <span>✓</span>
                <span>${escapeHtml(t('scanner.usmap_verified_working_hooks', { count: validDiags.length }) || `Verified & Working Hooks (${validDiags.length})`)}</span>
              </span>
              <span style="font-size: 10px; color: var(--text-muted);">${escapeHtml(t('scanner.usmap_click_to_expand') || 'Click to expand')}</span>
            </summary>
            <div class="scanner-card-body" style="cursor: default; gap: 8px; padding: 12px; max-height: 400px; overflow-y: auto;">
              ${validDiags.map(renderHookItem).join('')}
            </div>
          </details>
        ` : ''}
      </div>
    </details>
  `;
}
