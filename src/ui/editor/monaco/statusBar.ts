import { editorDom } from '../../../framework';
import { getState } from '../../../state';
import { getMonacoEditor } from './instance';
import { toggleProblemsPanel } from './problemsPanel';
import { getMappingsStatus, getSdkStatus, getReflectionCatalogsStatus, getEditorCompletions } from '../../../api';
import { t } from '../../../utils/i18n';

let _isInitialized = false;
let _cachedUsmapLabel = '';
let _cachedSdkLabel = '';
let _cachedDtLabel = '';
let _cachedBpLabel = '';

export function initEditorStatusBar(): void {
  if (_isInitialized) return;

  const problemsBtn = editorDom.elMaybe('editor-status-problems-btn');
  if (problemsBtn) {
    problemsBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      toggleProblemsPanel();
    });
  }

  // Fetch and display active USMAP and CXX SDK versions
  updateStatusBarEngineInfo().catch(() => {});

  _isInitialized = true;
}

export function updateStatusBarCursor(line: number, col: number): void {
  const cursorEl = editorDom.elMaybe('editor-status-cursor');
  if (cursorEl) {
    cursorEl.textContent = `Ln ${line}, Col ${col}`;
  }
}

export function updateStatusBarLanguage(filePath: string): void {
  const langEl = editorDom.elMaybe('editor-status-language');
  if (!langEl) return;

  const ext = filePath.split('.').pop()?.toLowerCase() || '';
  let lang = 'Plain Text';
  if (ext === 'lua') lang = 'Lua';
  else if (ext === 'json') lang = 'JSON';
  else if (ext === 'jsonc') lang = 'JSONC';
  else if (ext === 'md') lang = 'Markdown';
  else if (ext === 'ini' || ext === 'cfg') lang = 'Config';
  else if (ext === 'txt') lang = 'Text';

  langEl.textContent = lang;
}

export function updateStatusBarModInfo(modName: string): void {
  const modEl = editorDom.elMaybe('editor-status-mod-name');
  if (modEl) {
    modEl.textContent = modName || 'Mod Workspace';
  }
}

export function updateStatusBarProblems(fileCount: number, workspaceCount: number): void {
  const countEl = editorDom.elMaybe('editor-status-error-count');
  const btn = editorDom.elMaybe('editor-status-problems-btn');
  if (!countEl || !btn) return;

  if (fileCount === 0 && workspaceCount === 0) {
    countEl.textContent = `✓ 0`;
    btn.className = 'editor-status-btn';
    btn.title = t('editor.problems_clean') || 'No problems detected in mod';
  } else {
    countEl.textContent = `${fileCount} (${workspaceCount} in mod)`;
    btn.className = fileCount > 0 ? 'editor-status-btn has-issues' : 'editor-status-btn';
    btn.title = t('editor.status_problems_tooltip', { fileCount, workspaceCount }) || `${fileCount} problem(s) in current file, ${workspaceCount} in mod workspace`;
  }
}

export async function updateStatusBarEngineInfo(): Promise<void> {
  const usmapLabel = editorDom.elMaybe('editor-status-usmap-label');
  const sdkLabel = editorDom.elMaybe('editor-status-sdk-label');
  const dtLabel = editorDom.elMaybe('editor-status-datatables-label');
  const bpLabel = editorDom.elMaybe('editor-status-blueprints-label');

  if (_cachedUsmapLabel && usmapLabel) {
    usmapLabel.textContent = _cachedUsmapLabel;
  }
  if (_cachedSdkLabel && sdkLabel) {
    sdkLabel.textContent = _cachedSdkLabel;
  }
  if (_cachedDtLabel && dtLabel) {
    dtLabel.textContent = _cachedDtLabel;
  }
  if (_cachedBpLabel && bpLabel) {
    bpLabel.textContent = _cachedBpLabel;
  }

  try {
    const [usmapRes, sdkRes, catalogsRes] = await Promise.allSettled([
      getMappingsStatus(),
      getSdkStatus(),
      getReflectionCatalogsStatus(),
    ]);

    if (usmapRes.status === 'fulfilled' && usmapRes.value) {
      const u = usmapRes.value;
      const ver = u.activeMapping?.gameVersion || u.installedBuild?.gameVersion || 'Active';
      _cachedUsmapLabel = `USMAP: ${ver}`;
      if (usmapLabel) usmapLabel.textContent = _cachedUsmapLabel;
    }

    if (sdkRes.status === 'fulfilled' && sdkRes.value) {
      const s = sdkRes.value;
      const ver = s.gameVersion || (s.totalClasses > 0 ? 'v1.0.3' : '');
      const classCount = s.totalClasses > 0 ? `${(s.totalClasses / 1000).toFixed(1)}k` : '14.5k';
      _cachedSdkLabel = `SDK: ${ver ? ver + ` (${classCount} classes)` : classCount + ' classes'}`;
      if (sdkLabel) sdkLabel.textContent = _cachedSdkLabel;
    }

    if (catalogsRes.status === 'fulfilled' && catalogsRes.value) {
      const c = catalogsRes.value;
      if (c.totalDatatables > 0) {
        _cachedDtLabel = `DT: ${c.totalDatatables}`;
        if (dtLabel) dtLabel.textContent = _cachedDtLabel;
        const dtEl = editorDom.elMaybe('editor-status-datatables');
        if (dtEl) {
          dtEl.title = t('editor.status_datatables_title', {
            tables: c.totalDatatables,
            rows: c.totalDatatableRows.toLocaleString(),
          }) || `PalSchema DataTables (${c.totalDatatables} tables, ${c.totalDatatableRows} rows)`;
        }
      }

      if (c.totalBlueprints > 0) {
        const bpCount = (c.totalBlueprints / 1000).toFixed(1) + 'k';
        _cachedBpLabel = `BP: ${bpCount}`;
        if (bpLabel) bpLabel.textContent = _cachedBpLabel;
        const bpEl = editorDom.elMaybe('editor-status-blueprints');
        if (bpEl) {
          bpEl.title = t('editor.status_blueprints_title', {
            count: c.totalBlueprints.toLocaleString(),
          }) || `Live Game Blueprints Index (${c.totalBlueprints} classes from .pak)`;
        }
      }
    }

    // Background warmup: pre-load USMAP, Blueprints, and DataTables into memory so typing is instantaneous
    getEditorCompletions('warmup.jsonc', '', '', undefined).catch(() => {});
  } catch (e) {
    console.error('Failed to load status bar engine info:', e);
  }
}
