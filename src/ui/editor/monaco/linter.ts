import * as monaco from 'monaco-editor';
import { validateEditorCode, EditorDiagnostic } from '../../../api';
import { editorDom } from '../../../framework';
import { t } from '../../../utils/i18n';
import { getCurrentMonacoFilePath } from './state';
import { renderProblemsList, toggleProblemsPanel } from './problemsPanel';

let _activeDiagnostics: EditorDiagnostic[] = [];
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;
let _badgeListenerAttached = false;

export function getActiveDiagnostics(): EditorDiagnostic[] {
  return _activeDiagnostics;
}

export function registerMonacoLinter(editor: monaco.editor.IStandaloneCodeEditor): void {
  editor.onDidChangeModelContent(() => {
    scheduleValidation(editor);
  });

  editor.onDidChangeModel(() => {
    scheduleValidation(editor);
  });
}

export function scheduleValidation(editor: monaco.editor.IStandaloneCodeEditor): void {
  if (_debounceTimer) {
    clearTimeout(_debounceTimer);
  }

  _debounceTimer = setTimeout(() => {
    runValidation(editor);
  }, 220);
}

export async function runValidation(
  editor: monaco.editor.IStandaloneCodeEditor
): Promise<void> {
  const model = editor.getModel();
  if (!model) return;

  const filePath = getCurrentMonacoFilePath() || model.uri.fsPath || '';
  if (!filePath || (!filePath.endsWith('.lua') && !filePath.endsWith('.json') && !filePath.endsWith('.jsonc'))) {
    monaco.editor.setModelMarkers(model, 'palworld', []);
    _activeDiagnostics = [];
    updateHealthBadge(0);
    renderProblemsList([]);
    return;
  }

  const content = model.getValue();

  // Performance guard: Skip IPC validation on massive data files (>2MB)
  if (content.length > 2 * 1024 * 1024) {
    monaco.editor.setModelMarkers(model, 'palworld', []);
    _activeDiagnostics = [];
    updateHealthBadge(0, true);
    renderProblemsList([]);
    return;
  }

  try {
    const diagnostics = await validateEditorCode(filePath, content);
    _activeDiagnostics = diagnostics || [];

    const markers: monaco.editor.IMarkerData[] = _activeDiagnostics.map((diag) => {
      let severity = monaco.MarkerSeverity.Error;
      if (diag.severity === 'warning') {
        severity = monaco.MarkerSeverity.Warning;
      } else if (diag.severity === 'info') {
        severity = monaco.MarkerSeverity.Info;
      }

      const startLine = Math.max(1, diag.line);
      const startCol = Math.max(1, diag.column);
      const endLine = diag.endLine ? Math.max(startLine, diag.endLine) : startLine;
      const endCol = diag.endColumn ? Math.max(startCol + 1, diag.endColumn) : startCol + Math.max(1, diag.target.length);

      return {
        severity,
        startLineNumber: startLine,
        startColumn: startCol,
        endLineNumber: endLine,
        endColumn: endCol,
        message: diag.message + (diag.suggestion ? `\n💡 Quick Fix: ${diag.suggestion}` : ''),
        source: 'PalModManager',
      };
    });

    monaco.editor.setModelMarkers(model, 'palworld', markers);
    updateHealthBadge(_activeDiagnostics.length);
    renderProblemsList(_activeDiagnostics);
  } catch (err) {
    console.error('Validation error in Monaco linter:', err);
  }
}

function updateHealthBadge(issueCount: number, isLargeFile = false): void {
  const badge = editorDom.elMaybe('editor-health-badge');
  if (!badge) return;

  if (!_badgeListenerAttached) {
    badge.addEventListener('click', () => {
      toggleProblemsPanel();
    });
    badge.style.cursor = 'pointer';
    _badgeListenerAttached = true;
  }

  badge.title = t('editor.health_click_tooltip') || 'Click to toggle Problems panel';

  if (isLargeFile) {
    badge.className = 'editor-health-badge clean';
    badge.innerHTML = `<span class="health-icon">⚡</span> <span class="health-text">${t('editor.health_clean')} (&gt;2MB)</span>`;
  } else if (issueCount === 0) {
    badge.className = 'editor-health-badge clean';
    badge.innerHTML = `<span class="health-icon">✓</span> <span class="health-text">${t('editor.health_clean')}</span>`;
  } else if (issueCount === 1) {
    badge.className = 'editor-health-badge issues';
    badge.innerHTML = `<span class="health-icon">⚠️</span> <span class="health-text">${t('editor.health_single_issue', { count: 1 })}</span>`;
  } else {
    badge.className = 'editor-health-badge issues';
    badge.innerHTML = `<span class="health-icon">⚠️</span> <span class="health-text">${t('editor.health_multiple_issues', { count: issueCount })}</span>`;
  }
}
