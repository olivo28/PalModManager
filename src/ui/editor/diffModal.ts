import { editorDom } from '../../framework';
import { readModFile, restoreModBackup, mergeModBackup } from '../../api';
import { showToast } from '../toast';
import { escapeHtml, formatBytes } from '../../utils/helpers';
import { t, updateDOMTranslations } from '../../utils/i18n';
import { showConfirm } from '../confirm';
import { refreshEditorFileTree } from './tree';
import { loadFileContent } from './viewer';
import { getState } from '../../state';

interface DiffLine {
  type: 'same' | 'add' | 'del';
  leftLine?: number;
  leftText?: string;
  rightLine?: number;
  rightText?: string;
}

let _isInitialized = false;
let _currentModId: string | null = null;
let _currentBackupPath: string | null = null;
let _currentTargetPath: string | null = null;

export function initDiffModal(): void {
  if (_isInitialized) return;
  _isInitialized = true;

  const modal = editorDom.elMaybe('diff-modal');
  const closeX = editorDom.elMaybe('diff-modal-close-x');
  const closeBtn = editorDom.elMaybe('diff-modal-close-btn');
  const restoreBtn = editorDom.elMaybe('diff-modal-restore-btn');
  const mergeBtn = editorDom.elMaybe('diff-modal-merge-btn');

  if (closeX) closeX.addEventListener('click', hideDiffModal);
  if (closeBtn) closeBtn.addEventListener('click', hideDiffModal);

  if (modal) {
    modal.addEventListener('click', (e) => {
      if (e.target === modal) hideDiffModal();
    });
  }

  if (restoreBtn) {
    restoreBtn.addEventListener('click', handleRestoreCurrentBackup);
  }

  if (mergeBtn) {
    mergeBtn.addEventListener('click', handleMergeCurrentBackup);
  }

  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      const m = editorDom.elMaybe('diff-modal');
      if (m && m.classList.contains('visible')) {
        hideDiffModal();
      }
    }
  });
}

export function hideDiffModal(): void {
  const modal = editorDom.elMaybe('diff-modal');
  if (modal) {
    modal.classList.remove('visible');
  }
}

/**
 * Compute line-by-line diff between two strings.
 */
function computeLineDiff(oldText: string, newText: string): DiffLine[] {
  const oldLines = oldText.split(/\r?\n/);
  const newLines = newText.split(/\r?\n/);

  // Simple Longest Common Subsequence algorithm
  const n = oldLines.length;
  const m = newLines.length;

  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0));

  for (let i = 0; i < n; i++) {
    for (let j = 0; j < m; j++) {
      if (oldLines[i] === newLines[j]) {
        dp[i + 1][j + 1] = dp[i][j] + 1;
      } else {
        dp[i + 1][j + 1] = Math.max(dp[i + 1][j], dp[i][j + 1]);
      }
    }
  }

  const diff: DiffLine[] = [];
  let i = n;
  let j = m;

  const stack: DiffLine[] = [];

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldLines[i - 1] === newLines[j - 1]) {
      stack.push({
        type: 'same',
        leftLine: i,
        leftText: oldLines[i - 1],
        rightLine: j,
        rightText: newLines[j - 1],
      });
      i--;
      j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      stack.push({
        type: 'add',
        rightLine: j,
        rightText: newLines[j - 1],
      });
      j--;
    } else if (i > 0 && (j === 0 || dp[i][j - 1] < dp[i - 1][j])) {
      stack.push({
        type: 'del',
        leftLine: i,
        leftText: oldLines[i - 1],
      });
      i--;
    }
  }

  while (stack.length > 0) {
    diff.push(stack.pop()!);
  }

  return diff;
}

export async function openEditorDiffModal(modId: string, backupPath: string): Promise<void> {
  initDiffModal();

  _currentModId = modId;
  _currentBackupPath = backupPath;

  // Resolve target base file path (e.g. config.jsonc.bak -> config.jsonc)
  let targetPath = backupPath;
  if (targetPath.endsWith('.bak')) {
    targetPath = targetPath.slice(0, -4);
  } else if (targetPath.endsWith('.bak1') || targetPath.endsWith('.bak2')) {
    targetPath = targetPath.slice(0, -5);
  }
  _currentTargetPath = targetPath;

  const modal = editorDom.el('diff-modal');
  updateDOMTranslations(modal);
  modal.classList.add('visible');

  const titleEl = editorDom.el('diff-modal-title');
  const subtitleEl = editorDom.el('diff-modal-subtitle');
  const versionEl = editorDom.elMaybe('diff-modal-version');
  const leftLabel = editorDom.el('diff-left-label');
  const leftTime = editorDom.elMaybe('diff-left-time');
  const rightLabel = editorDom.el('diff-right-label');
  const rightTime = editorDom.elMaybe('diff-right-time');
  const container = editorDom.el('diff-modal-container');
  const statAdds = editorDom.el('diff-stat-additions');
  const statDels = editorDom.el('diff-stat-deletions');
  const mergeBtn = editorDom.elMaybe('diff-modal-merge-btn');

  titleEl.textContent = t('editor.diff_title') || 'Compare Backup Diff';
  subtitleEl.textContent = `${targetPath} vs ${backupPath}`;
  leftLabel.textContent = targetPath;
  rightLabel.textContent = backupPath;

  const state = getState();
  const mod = state.allMods.find(m => m.id === modId);
  const modVer = mod?.version || '--';
  if (versionEl) {
    versionEl.textContent = modVer.startsWith('v') || modVer.startsWith('V') ? modVer : `v${modVer}`;
  }

  if (leftTime) leftTime.textContent = '--';
  if (rightTime) rightTime.textContent = '--';

  // Detect mergeable config extension
  const ext = targetPath.split('.').pop()?.toLowerCase() || '';
  const isMergeableConfig = ['json', 'jsonc', 'ini', 'cfg', 'toml', 'yaml', 'yml', 'lua'].includes(ext);
  if (mergeBtn) {
    mergeBtn.style.display = isMergeableConfig ? 'flex' : 'none';
  }

  container.innerHTML = '<div style="padding: 24px; text-align: center; color: var(--text-muted);">Loading comparison...</div>';
  statAdds.textContent = '+0';
  statDels.textContent = '-0';

  try {
    const [targetRes, backupRes] = await Promise.all([
      readModFile(modId, targetPath),
      readModFile(modId, backupPath),
    ]);

    if (targetRes.modVersion && versionEl) {
      const v = targetRes.modVersion;
      versionEl.textContent = v.startsWith('v') || v.startsWith('V') ? v : `v${v}`;
    }

    if (leftTime && targetRes.modifiedTime) {
      const dateStr = new Date(targetRes.modifiedTime).toLocaleString();
      const sizeStr = targetRes.fileSize ? ` (${formatBytes(targetRes.fileSize)})` : '';
      leftTime.textContent = `${dateStr}${sizeStr}`;
    } else if (leftTime) {
      leftTime.textContent = 'Active file';
    }

    if (rightTime && backupRes.modifiedTime) {
      const dateStr = new Date(backupRes.modifiedTime).toLocaleString();
      const sizeStr = backupRes.fileSize ? ` (${formatBytes(backupRes.fileSize)})` : '';
      rightTime.textContent = `${dateStr}${sizeStr}`;
    } else if (rightTime) {
      rightTime.textContent = 'Archived backup';
    }

    const activeContent = targetRes.content || '';
    const backupContent = backupRes.content || '';

    const diffLines = computeLineDiff(activeContent, backupContent);

    let additions = 0;
    let deletions = 0;

    diffLines.forEach(line => {
      if (line.type === 'add') additions++;
      if (line.type === 'del') deletions++;
    });

    statAdds.textContent = `+${additions}`;
    statDels.textContent = `-${deletions}`;

    renderDiffRows(diffLines, container);
  } catch (err) {
    container.innerHTML = `<div style="padding: 24px; text-align: center; color: var(--danger);">Error loading diff: ${escapeHtml(String(err))}</div>`;
  }
}

function renderDiffRows(diffLines: DiffLine[], container: HTMLElement): void {
  const hasChanges = diffLines.some(l => l.type !== 'same');
  if (diffLines.length === 0 || !hasChanges) {
    container.innerHTML = `<div style="padding: 32px; text-align: center; color: var(--text-muted); font-size: 13px;">Files are identical (No differences detected).</div>`;
    return;
  }

  // Calculate context visibility flags (show 3 unchanged lines around any diff)
  const contextSize = 3;
  const showFlags = new Array(diffLines.length).fill(false);

  for (let k = 0; k < diffLines.length; k++) {
    if (diffLines[k].type !== 'same') {
      showFlags[k] = true;
      for (let c = 1; c <= contextSize; c++) {
        if (k - c >= 0) showFlags[k - c] = true;
        if (k + c < diffLines.length) showFlags[k + c] = true;
      }
    }
  }

  let tableHtml = `
    <table style="width: 100%; border-collapse: collapse; font-family: 'Consolas', 'Courier New', monospace; font-size: 11.5px; line-height: 1.5;">
      <tbody>
  `;

  let inCollapse = false;
  let collapsedCount = 0;

  for (let k = 0; k < diffLines.length; k++) {
    if (showFlags[k]) {
      if (inCollapse) {
        tableHtml += `
          <tr style="background: rgba(0, 0, 0, 0.25); border-top: 1px dashed var(--border); border-bottom: 1px dashed var(--border);">
            <td colspan="4" style="text-align: center; padding: 6px 12px; color: var(--text-muted); font-size: 11px; font-style: italic; user-select: none;">
              ··· ${escapeHtml(t('editor.diff_collapsed_lines', { count: collapsedCount }))} ···
            </td>
          </tr>
        `;
        inCollapse = false;
        collapsedCount = 0;
      }

      const line = diffLines[k];
      let rowBg = 'transparent';
      let leftNumStr = line.leftLine ? String(line.leftLine) : '';
      let rightNumStr = line.rightLine ? String(line.rightLine) : '';
      let leftCode = escapeHtml(line.leftText ?? '');
      let rightCode = escapeHtml(line.rightText ?? '');

      if (line.type === 'add') {
        rowBg = 'rgba(74, 246, 38, 0.08)';
      } else if (line.type === 'del') {
        rowBg = 'rgba(255, 95, 86, 0.08)';
      }

      tableHtml += `
        <tr style="background: ${rowBg}; border-bottom: 1px solid rgba(255,255,255,0.02);">
          <td style="width: 40px; text-align: right; padding: 2px 8px; color: var(--text-muted); user-select: none; border-right: 1px solid var(--border);">${leftNumStr}</td>
          <td style="width: 45%; padding: 2px 10px; color: ${line.type === 'del' ? '#ff5f56' : 'var(--text-primary)'}; white-space: pre-wrap; word-break: break-all;">${leftCode}</td>
          <td style="width: 40px; text-align: right; padding: 2px 8px; color: var(--text-muted); user-select: none; border-left: 1px solid var(--border); border-right: 1px solid var(--border);">${rightNumStr}</td>
          <td style="width: 45%; padding: 2px 10px; color: ${line.type === 'add' ? '#4af626' : 'var(--text-primary)'}; white-space: pre-wrap; word-break: break-all;">${rightCode}</td>
        </tr>
      `;
    } else {
      inCollapse = true;
      collapsedCount++;
    }
  }

  if (inCollapse) {
    tableHtml += `
      <tr style="background: rgba(0, 0, 0, 0.25); border-top: 1px dashed var(--border); border-bottom: 1px dashed var(--border);">
        <td colspan="4" style="text-align: center; padding: 6px 12px; color: var(--text-muted); font-size: 11px; font-style: italic; user-select: none;">
          ··· ${escapeHtml(t('editor.diff_collapsed_lines', { count: collapsedCount }))} ···
        </td>
      </tr>
    `;
  }

  tableHtml += `
      </tbody>
    </table>
  `;

  container.innerHTML = tableHtml;
}

async function handleRestoreCurrentBackup(): Promise<void> {
  if (!_currentModId || !_currentBackupPath || !_currentTargetPath) return;

  const title = t('editor.confirm_restore_title') || 'Restore Backup';
  const body = (t('editor.confirm_restore_body') || 'Are you sure you want to restore **{backup}** over **{target}**?\n\nA safety backup of the current file will be preserved.')
    .replace('{backup}', _currentBackupPath)
    .replace('{target}', _currentTargetPath);

  const confirmed = await showConfirm(title, body);
  if (!confirmed) return;

  try {
    const res = await restoreModBackup(_currentModId, _currentBackupPath);
    if (res.success) {
      hideDiffModal();
      showToast(t('editor.toast_restored') || 'Backup restored successfully', 'success');
      await refreshEditorFileTree(_currentModId);

      const state = getState();
      if (state.editorSelectedFile === _currentTargetPath) {
        await loadFileContent(_currentTargetPath);
      }
    }
  } catch (err) {
    showToast(String(err), 'error');
  }
}

async function handleMergeCurrentBackup(): Promise<void> {
  if (!_currentModId || !_currentBackupPath || !_currentTargetPath) return;

  const title = t('editor.confirm_merge_title') || 'Merge Config Settings';
  const body = (t('editor.confirm_merge_body') || 'Intelligently merge your custom values from **{backup}** into **{target}**?\n\nThis preserves your personalized settings while keeping newly added options and structure from the active version.')
    .replace('{backup}', _currentBackupPath)
    .replace('{target}', _currentTargetPath);

  const confirmed = await showConfirm(title, body);
  if (!confirmed) return;

  try {
    const res = await mergeModBackup(_currentModId, _currentBackupPath);
    if (res.success) {
      hideDiffModal();
      showToast(t('editor.toast_merged') || 'Config settings merged successfully', 'success');
      await refreshEditorFileTree(_currentModId);

      const state = getState();
      if (state.editorSelectedFile === _currentTargetPath) {
        await loadFileContent(_currentTargetPath);
      }
    }
  } catch (err) {
    showToast(String(err), 'error');
  }
}
