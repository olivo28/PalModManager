import * as monaco from 'monaco-editor';
import { EditorDiagnostic, scanWorkspaceProblems } from '../../../api';
import { editorDom } from '../../../framework';
import { getState } from '../../../state';
import { t } from '../../../utils/i18n';
import { loadFileContent } from '../viewer';
import { getCurrentMonacoFilePath, getMonacoEditor } from './state';

const STORAGE_HEIGHT_KEY = 'pmm_editor_problems_height';
const STORAGE_OPEN_KEY = 'pmm_editor_problems_open';
const STORAGE_TAB_KEY = 'pmm_editor_problems_tab';
const MIN_PANEL_HEIGHT = 120;
const MAX_PANEL_HEIGHT = 500;
const DEFAULT_PANEL_HEIGHT = 220;

import { updateStatusBarProblems } from './statusBar';

let _isInitialized = false;
let _isOpen = false;
let _activeTab: 'file' | 'workspace' = 'file';
let _currentHeight = DEFAULT_PANEL_HEIGHT;

let _currentFileDiagnostics: EditorDiagnostic[] = [];
let _workspaceDiagnosticsByMod: Record<string, Record<string, EditorDiagnostic[]>> = {};
let _workspaceDiagnostics: Record<string, EditorDiagnostic[]> = {};
let _isScanningWorkspace = false;

export function initProblemsPanel(): void {
  if (_isInitialized) return;

  const panel = editorDom.elMaybe('editor-problems-panel');
  const resizeHandle = editorDom.elMaybe('editor-problems-resize-handle');
  const closeBtn = editorDom.elMaybe('editor-problems-close');
  const tabFile = editorDom.elMaybe('editor-problems-tab-file');
  const tabWorkspace = editorDom.elMaybe('editor-problems-tab-workspace');
  const scanBtn = editorDom.elMaybe('editor-problems-scan-btn');

  if (!panel || !resizeHandle || !closeBtn || !tabFile || !tabWorkspace) return;

  // Restore saved height
  const savedHeight = localStorage.getItem(STORAGE_HEIGHT_KEY);
  if (savedHeight) {
    const parsed = parseInt(savedHeight, 10);
    if (!isNaN(parsed) && parsed >= MIN_PANEL_HEIGHT && parsed <= MAX_PANEL_HEIGHT) {
      _currentHeight = parsed;
    }
  }
  panel.style.height = `${_currentHeight}px`;

  // Restore active tab
  const savedTab = localStorage.getItem(STORAGE_TAB_KEY);
  if (savedTab === 'workspace') {
    switchProblemsTab('workspace');
  } else {
    switchProblemsTab('file');
  }

  // Close button
  closeBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    toggleProblemsPanel(false);
  });

  // Tab switching
  tabFile.addEventListener('click', () => {
    switchProblemsTab('file');
  });

  tabWorkspace.addEventListener('click', () => {
    switchProblemsTab('workspace');
    const currentModId = getState().editorModId;
    if (currentModId && _workspaceDiagnosticsByMod[currentModId]) {
      renderWorkspaceProblemsView();
    } else {
      triggerWorkspaceScan();
    }
  });

  if (scanBtn) {
    scanBtn.addEventListener('click', () => {
      triggerWorkspaceScan(true);
    });
  }

  // Drag to resize handle
  let isDragging = false;
  let startY = 0;
  let startHeight = _currentHeight;

  resizeHandle.addEventListener('mousedown', (e) => {
    isDragging = true;
    startY = e.clientY;
    startHeight = panel.offsetHeight;
    document.body.style.cursor = 'row-resize';
    document.body.style.userSelect = 'none';

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (!isDragging) return;
      const deltaY = startY - moveEvent.clientY;
      let newHeight = startHeight + deltaY;
      newHeight = Math.max(MIN_PANEL_HEIGHT, Math.min(MAX_PANEL_HEIGHT, newHeight));
      _currentHeight = newHeight;
      panel.style.height = `${newHeight}px`;
      localStorage.setItem(STORAGE_HEIGHT_KEY, String(newHeight));

      const editor = getMonacoEditor();
      if (editor) {
        editor.layout();
      }
    };

    const onMouseUp = () => {
      isDragging = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);

      const editor = getMonacoEditor();
      if (editor) {
        editor.layout();
      }
    };

    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  });

  _isInitialized = true;
}

export function switchProblemsTab(tab: 'file' | 'workspace'): void {
  _activeTab = tab;
  localStorage.setItem(STORAGE_TAB_KEY, tab);

  const tabFile = editorDom.elMaybe('editor-problems-tab-file');
  const tabWorkspace = editorDom.elMaybe('editor-problems-tab-workspace');
  const listFile = editorDom.elMaybe('editor-problems-list');
  const listWorkspace = editorDom.elMaybe('editor-problems-workspace-list');
  const scanBtn = editorDom.elMaybe('editor-problems-scan-btn');

  if (tabFile) tabFile.classList.toggle('active', tab === 'file');
  if (tabWorkspace) tabWorkspace.classList.toggle('active', tab === 'workspace');

  if (listFile) listFile.style.display = tab === 'file' ? 'flex' : 'none';
  if (listWorkspace) listWorkspace.style.display = tab === 'workspace' ? 'flex' : 'none';
  if (scanBtn) scanBtn.style.display = tab === 'workspace' ? 'inline-flex' : 'none';
}

export function isProblemsPanelOpen(): boolean {
  return _isOpen;
}

export function toggleProblemsPanel(forceState?: boolean): void {
  const panel = editorDom.elMaybe('editor-problems-panel');
  if (!panel) return;

  _isOpen = forceState !== undefined ? forceState : !_isOpen;

  if (_isOpen) {
    panel.style.display = 'flex';
    panel.style.height = `${_currentHeight}px`;
    localStorage.setItem(STORAGE_OPEN_KEY, 'true');
    if (_activeTab === 'workspace') {
      triggerWorkspaceScan();
    }
  } else {
    panel.style.display = 'none';
    localStorage.setItem(STORAGE_OPEN_KEY, 'false');
  }

  const editor = getMonacoEditor();
  if (editor) {
    setTimeout(() => {
      editor.layout();
    }, 30);
  }
}

export function updateWorkspaceBadge(): void {
  const workspaceCountBadge = editorDom.elMaybe('editor-problems-workspace-count-badge');
  const fileEntries = Object.entries(_workspaceDiagnostics);
  let totalIssues = 0;
  fileEntries.forEach(([_, diags]) => {
    totalIssues += diags ? diags.length : 0;
  });

  if (workspaceCountBadge) {
    workspaceCountBadge.textContent = String(totalIssues);
    workspaceCountBadge.className = `editor-problems-count-badge ${totalIssues === 0 ? 'clean' : 'has-issues'}`;
  }

  // Update editor footer status bar
  updateStatusBarProblems(_currentFileDiagnostics.length, totalIssues);
}

export function renderProblemsList(diagnostics: EditorDiagnostic[]): void {
  initProblemsPanel();
  _currentFileDiagnostics = diagnostics || [];

  const countBadge = editorDom.elMaybe('editor-problems-count-badge');
  const list = editorDom.elMaybe('editor-problems-list');
  if (!list) return;

  const count = _currentFileDiagnostics.length;
  if (countBadge) {
    countBadge.textContent = String(count);
    countBadge.className = `editor-problems-count-badge ${count === 0 ? 'clean' : 'has-issues'}`;
  }

  // Sync current file issues into workspace map and update workspace badge
  const currentFilePath = getCurrentMonacoFilePath();
  const state = getState();
  const modId = state.editorModId;
  if (currentFilePath && modId) {
    if (!_workspaceDiagnosticsByMod[modId]) {
      _workspaceDiagnosticsByMod[modId] = {};
    }
    if (_currentFileDiagnostics.length > 0) {
      _workspaceDiagnostics[currentFilePath] = _currentFileDiagnostics;
      _workspaceDiagnosticsByMod[modId][currentFilePath] = _currentFileDiagnostics;
    } else {
      delete _workspaceDiagnostics[currentFilePath];
      delete _workspaceDiagnosticsByMod[modId][currentFilePath];
    }
    updateWorkspaceBadge();
    if (_activeTab === 'workspace') {
      renderWorkspaceProblemsView();
    }
  } else {
    updateWorkspaceBadge();
  }

  if (count === 0) {
    list.innerHTML = `
      <div class="editor-problems-empty">
        <span class="empty-icon">✓</span>
        <span>${t('editor.problems_empty') || 'No problems have been detected in the current file.'}</span>
      </div>
    `;
    return;
  }

  const filePath = getCurrentMonacoFilePath() || 'file';
  const fileName = filePath.split(/[\\/]/).pop() || filePath;

  let html = '';
  _currentFileDiagnostics.forEach((diag, index) => {
    let icon = '⚠️';
    let iconClass = 'warning';
    if (diag.severity === 'error') {
      icon = '❌';
      iconClass = 'error';
    } else if (diag.severity === 'info') {
      icon = 'ℹ️';
      iconClass = 'info';
    }

    let categoryBadge = '';
    if (diag.category === 'ue4ss') {
      categoryBadge = '<span class="diag-category-badge ue4ss">UE4SS</span>';
    } else if (diag.category === 'palschema') {
      categoryBadge = '<span class="diag-category-badge palschema">PalSchema</span>';
    } else {
      categoryBadge = '<span class="diag-category-badge syntax">Syntax</span>';
    }

    const hasFix = !!diag.suggestion;

    html += `
      <div class="editor-problem-item ${iconClass}" data-index="${index}" title="Click to jump to line ${diag.line}">
        <span class="problem-icon">${icon}</span>
        <span class="problem-message">${escapeHtml(diag.message)}</span>
        <span class="problem-meta">
          ${categoryBadge}
          <span class="problem-location">${fileName} [${diag.line}, ${diag.column}]</span>
        </span>
        ${hasFix ? `<button class="problem-fix-btn" data-index="${index}" title="Apply Quick Fix: ${escapeHtml(diag.suggestion || '')}">💡 Fix</button>` : ''}
      </div>
    `;
  });

  list.innerHTML = html;

  // Add click listeners to jump and quick-fix
  list.querySelectorAll('.editor-problem-item').forEach((row) => {
    row.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target && target.classList.contains('problem-fix-btn')) {
        return; // Handled by fix button listener
      }

      const idxStr = (row as HTMLElement).getAttribute('data-index');
      if (idxStr === null) return;
      const idx = parseInt(idxStr, 10);
      const diag = _currentFileDiagnostics[idx];
      if (!diag) return;

      const editor = getMonacoEditor();
      if (editor) {
        editor.focus();
        editor.setPosition({ lineNumber: diag.line, column: diag.column });
        editor.revealPositionInCenter(
          { lineNumber: diag.line, column: diag.column },
          monaco.editor.ScrollType.Smooth
        );
      }
    });
  });

  // Quick fix buttons
  list.querySelectorAll('.problem-fix-btn').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idxStr = (btn as HTMLElement).getAttribute('data-index');
      if (idxStr === null) return;
      const idx = parseInt(idxStr, 10);
      const diag = _currentFileDiagnostics[idx];
      if (!diag || !diag.suggestion) return;

      const editor = getMonacoEditor();
      const model = editor?.getModel();
      if (!editor || !model) return;

      const startLine = Math.max(1, diag.line);
      const startCol = Math.max(1, diag.column);
      const endLine = diag.endLine ? Math.max(startLine, diag.endLine) : startLine;
      const endCol = diag.endColumn ? Math.max(startCol + 1, diag.endColumn) : startCol + Math.max(1, diag.target.length);

      const targetRange = new monaco.Range(startLine, startCol, endLine, endCol);

      editor.executeEdits('pmm-quickfix', [
        {
          range: targetRange,
          text: diag.suggestion,
          forceMoveMarkers: true,
        },
      ]);

      editor.focus();
    });
  });
}

export function loadWorkspaceDiagnosticsForMod(modId: string): void {
  _workspaceDiagnostics = _workspaceDiagnosticsByMod[modId] || {};
  updateWorkspaceBadge();
  if (_activeTab === 'workspace') {
    renderWorkspaceProblemsView();
  }
}

export function clearWorkspaceProblemsCache(): void {
  _workspaceDiagnostics = {};
  _workspaceDiagnosticsByMod = {};
  updateWorkspaceBadge();
}

export async function triggerWorkspaceScan(force = false): Promise<void> {
  const state = getState();
  const modId = state.editorModId;
  if (!modId) return;

  // Instant 0ms recovery from per-mod cache
  if (!force && _workspaceDiagnosticsByMod[modId]) {
    _workspaceDiagnostics = _workspaceDiagnosticsByMod[modId];
    updateWorkspaceBadge();
    if (_activeTab === 'workspace') {
      renderWorkspaceProblemsView();
    }
    return;
  }

  if (_isScanningWorkspace && !force) return;
  _isScanningWorkspace = true;

  const workspaceList = editorDom.elMaybe('editor-problems-workspace-list');

  // Only show the loading spinner when forced or when cache is empty for this mod
  if (workspaceList && (force || !_workspaceDiagnosticsByMod[modId])) {
    workspaceList.innerHTML = `
      <div class="editor-problems-empty">
        <span class="spinner" style="width:14px;height:14px;border:2px solid var(--accent);border-top-color:transparent;border-radius:50%;animation:spin 0.8s linear infinite;"></span>
        <span>Scanning workspace files...</span>
      </div>
    `;
  }

  try {
    const res = await scanWorkspaceProblems(modId);
    _workspaceDiagnosticsByMod[modId] = res || {};
    _workspaceDiagnostics = _workspaceDiagnosticsByMod[modId];
    updateWorkspaceBadge();
    if (_activeTab === 'workspace') {
      renderWorkspaceProblemsView();
    }
  } catch (err) {
    console.error('Failed to scan workspace problems:', err);
    if (workspaceList) {
      workspaceList.innerHTML = `
        <div class="editor-problems-empty" style="color:var(--text-danger);">
          <span>⚠️ Failed to scan workspace: ${escapeHtml(String(err))}</span>
        </div>
      `;
    }
  } finally {
    _isScanningWorkspace = false;
  }
}

function renderWorkspaceProblemsView(): void {
  const workspaceList = editorDom.elMaybe('editor-problems-workspace-list');
  const workspaceCountBadge = editorDom.elMaybe('editor-problems-workspace-count-badge');
  if (!workspaceList) return;

  const fileEntries = Object.entries(_workspaceDiagnostics);
  let totalIssues = 0;
  fileEntries.forEach(([_, diags]) => {
    totalIssues += diags.length;
  });

  if (workspaceCountBadge) {
    workspaceCountBadge.textContent = String(totalIssues);
    workspaceCountBadge.className = `editor-problems-count-badge ${totalIssues === 0 ? 'clean' : 'has-issues'}`;
  }

  if (totalIssues === 0) {
    workspaceList.innerHTML = `
      <div class="editor-problems-empty">
        <span class="empty-icon">✓</span>
        <span>${t('editor.problems_workspace_empty') || 'No problems have been detected across the entire mod workspace.'}</span>
      </div>
    `;
    return;
  }

  let html = '';
  fileEntries.forEach(([filePath, diags]) => {
    if (!diags || diags.length === 0) return;
    const fileName = filePath.split(/[\\/]/).pop() || filePath;

    html += `
      <div class="editor-problems-file-group">
        <div class="editor-problems-file-header">
          <span class="file-icon">📁</span>
          <span class="file-title">${escapeHtml(filePath)}</span>
          <span class="file-issue-badge">${diags.length}</span>
        </div>
        <div class="editor-problems-file-items">
    `;

    diags.forEach((diag) => {
      let icon = '⚠️';
      let iconClass = 'warning';
      if (diag.severity === 'error') {
        icon = '❌';
        iconClass = 'error';
      } else if (diag.severity === 'info') {
        icon = 'ℹ️';
        iconClass = 'info';
      }

      let categoryBadge = '';
      if (diag.category === 'ue4ss') {
        categoryBadge = '<span class="diag-category-badge ue4ss">UE4SS</span>';
      } else if (diag.category === 'palschema') {
        categoryBadge = '<span class="diag-category-badge palschema">PalSchema</span>';
      } else {
        categoryBadge = '<span class="diag-category-badge syntax">Syntax</span>';
      }

      html += `
        <div class="editor-problem-item ${iconClass} workspace-item" data-path="${escapeHtml(filePath)}" data-line="${diag.line}" data-col="${diag.column}">
          <span class="problem-icon">${icon}</span>
          <span class="problem-message">${escapeHtml(diag.message)}</span>
          <span class="problem-meta">
            ${categoryBadge}
            <span class="problem-location">${fileName} [${diag.line}, ${diag.column}]</span>
          </span>
        </div>
      `;
    });

    html += `
        </div>
      </div>
    `;
  });

  workspaceList.innerHTML = html;

  // Add click listeners to open file and jump
  workspaceList.querySelectorAll('.workspace-item').forEach((row) => {
    row.addEventListener('click', async () => {
      const targetPath = row.getAttribute('data-path');
      const lineStr = row.getAttribute('data-line');
      const colStr = row.getAttribute('data-col');
      if (!targetPath || !lineStr || !colStr) return;

      const line = parseInt(lineStr, 10);
      const col = parseInt(colStr, 10);

      const currentPath = getCurrentMonacoFilePath();
      if (currentPath !== targetPath) {
        await loadFileContent(targetPath);
      }

      setTimeout(() => {
        const editor = getMonacoEditor();
        if (editor) {
          editor.focus();
          editor.setPosition({ lineNumber: line, column: col });
          editor.revealPositionInCenter(
            { lineNumber: line, column: col },
            monaco.editor.ScrollType.Smooth
          );
        }
      }, 80);
    });
  });
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
