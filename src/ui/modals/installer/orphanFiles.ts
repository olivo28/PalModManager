import { inspectUpgradeOrphans } from '../../../api';
import type { UpgradeOrphansResult, UpgradeOrphanFile } from '../../../api';
import { escapeHtml } from '../../../utils/helpers';
import { t } from '../../../utils/i18n';
import { installerDom } from '../../../framework';

let _currentOrphans: UpgradeOrphanFile[] = [];

export function getSelectedOrphanDecisions(): { filesToPreserve: string[]; filesToPurge: string[] } {
  const container = installerDom.elMaybe('orphan-files-container');
  if (!container || _currentOrphans.length === 0) {
    return { filesToPreserve: [], filesToPurge: [] };
  }

  const preserve: string[] = [];
  const purge: string[] = [];

  const checkboxes = container.querySelectorAll<HTMLInputElement>('input[data-orphan-path]');
  checkboxes.forEach((cb) => {
    const relPath = cb.getAttribute('data-orphan-path');
    if (relPath) {
      if (cb.checked) {
        preserve.push(relPath);
      } else {
        purge.push(relPath);
      }
    }
  });

  return { filesToPreserve: preserve, filesToPurge: purge };
}

export function clearOrphanFilesState(): void {
  _currentOrphans = [];
  const container = installerDom.elMaybe('orphan-files-container');
  if (container) {
    container.style.display = 'none';
    container.innerHTML = '';
  }
}

export async function checkAndRenderUpgradeOrphans(zipPath: string, modId: string): Promise<void> {
  clearOrphanFilesState();
  try {
    const result: UpgradeOrphansResult = await inspectUpgradeOrphans(zipPath, modId);
    if (!result || !result.orphanFiles || result.orphanFiles.length === 0) {
      return;
    }

    _currentOrphans = result.orphanFiles;
    renderOrphanFilesAccordion(result.orphanFiles);
  } catch (err) {
    console.warn('[Installer] Failed to inspect upgrade orphans:', err);
  }
}

function renderOrphanFilesAccordion(files: UpgradeOrphanFile[]): void {
  const container = installerDom.elMaybe('orphan-files-container');
  if (!container) return;

  const configCount = files.filter(f => f.isConfig).length;
  const staleCount = files.length - configCount;

  let summary = t('installer.orphans_summary', { count: String(files.length) });
  if (configCount > 0 && staleCount > 0) {
    summary = t('installer.orphans_summary_mixed', { configs: String(configCount), stale: String(staleCount) });
  }

  container.style.display = 'flex';
  container.style.flexDirection = 'column';
  container.innerHTML = `
    <div style="border: 1px solid rgba(255, 170, 0, 0.35); background: rgba(255, 170, 0, 0.08); border-radius: 6px; padding: calc(8px * var(--ui-scale, 1)) calc(10px * var(--ui-scale, 1)); display: flex; flex-direction: column; gap: calc(6px * var(--ui-scale, 1));">
      <div style="display: flex; align-items: center; justify-content: space-between; cursor: pointer;" id="orphan-files-toggle-header">
        <div style="display: flex; align-items: center; gap: calc(8px * var(--ui-scale, 1));">
          <span style="font-size: calc(13px * var(--ui-scale, 1));">📂</span>
          <div style="display: flex; flex-direction: column; text-align: left;">
            <span style="font-size: var(--text-sm, 11px); font-weight: bold; color: var(--text-primary);">${escapeHtml(t('installer.orphans_title'))}</span>
            <span style="font-size: var(--text-2xs, 9px); color: var(--text-muted);">${escapeHtml(summary)}</span>
          </div>
        </div>
        <div style="display: flex; align-items: center; gap: 6px;">
          <span id="orphan-files-chevron" style="font-size: 10px; color: var(--text-secondary); transition: transform 0.2s ease;">▼</span>
        </div>
      </div>

      <div id="orphan-files-list-body" style="display: none; flex-direction: column; gap: calc(4px * var(--ui-scale, 1)); margin-top: calc(4px * var(--ui-scale, 1)); padding-top: calc(6px * var(--ui-scale, 1)); border-top: 1px dashed rgba(255, 170, 0, 0.25); max-height: calc(120px * var(--ui-scale, 1)); overflow-y: auto;">
        <div style="font-size: var(--text-2xs, 9px); color: var(--text-secondary); margin-bottom: 2px;">
          ${escapeHtml(t('installer.orphans_instructions'))}
        </div>
        ${files.map(f => {
          const badgeText = f.isConfig ? t('installer.orphans_badge_config') : t('installer.orphans_badge_stale');
          const badgeColor = f.isConfig ? '#2ecc71' : '#e74c3c';
          const checked = f.defaultKeep ? 'checked' : '';
          return `
            <label style="display: flex; align-items: center; justify-content: space-between; font-size: var(--text-xs, 10.5px); color: var(--text-primary); cursor: pointer; padding: 2px 4px; border-radius: 4px; background: rgba(0,0,0,0.15);">
              <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; min-width: 0;">
                <input type="checkbox" data-orphan-path="${escapeHtml(f.relativePath)}" ${checked} style="cursor: pointer; accent-color: var(--accent);" />
                <span title="${escapeHtml(f.relativePath)}" style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">${escapeHtml(f.relativePath)}</span>
              </div>
              <span style="font-size: var(--text-2xs, 8.5px); padding: 1px 4px; border-radius: 3px; font-weight: bold; border: 1px solid ${badgeColor}; color: ${badgeColor}; margin-left: 6px; text-transform: uppercase;">
                ${escapeHtml(badgeText)}
              </span>
            </label>
          `;
        }).join('')}
      </div>
    </div>
  `;

  const header = installerDom.elMaybe('orphan-files-toggle-header');
  const body = installerDom.elMaybe('orphan-files-list-body');
  const chevron = installerDom.elMaybe('orphan-files-chevron');
  if (header && body && chevron) {
    header.addEventListener('click', (e) => {
      e.preventDefault();
      const isHidden = body.style.display === 'none';
      body.style.display = isHidden ? 'flex' : 'none';
      chevron.style.transform = isHidden ? 'rotate(180deg)' : 'rotate(0deg)';
    });
  }
}
