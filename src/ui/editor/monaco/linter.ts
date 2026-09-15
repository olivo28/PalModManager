import * as monaco from 'monaco-editor';
import { validateEditorCode, EditorDiagnostic } from '../../../api';
import { getCurrentMonacoFilePath } from './state';
import { renderProblemsList } from './problemsPanel';
import { bus } from '../../../framework';
import { getState } from '../../../state';

let _activeDiagnostics: EditorDiagnostic[] = [];
let _debounceTimer: ReturnType<typeof setTimeout> | null = null;

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
    renderProblemsList([]);
    return;
  }

  const content = model.getValue();

  // Performance guard: Skip IPC validation on massive data files (>2MB)
  if (content.length > 2 * 1024 * 1024) {
    monaco.editor.setModelMarkers(model, 'palworld', []);
    _activeDiagnostics = [];
    renderProblemsList([]);
    return;
  }

  try {
    const oldDiagnostics = _activeDiagnostics;
    const diagnostics = await validateEditorCode(filePath, content);
    _activeDiagnostics = diagnostics || [];

    // Emit resolution event if any previously flagged issues disappeared
    const state = getState();
    const modId = state.editorModId;
    if (modId && oldDiagnostics.length > 0) {
      const newKeys = new Set(_activeDiagnostics.map(d => `${d.line}::${(d.target || '').trim().toLowerCase()}`));
      for (const oldD of oldDiagnostics) {
        const key = `${oldD.line}::${(oldD.target || '').trim().toLowerCase()}`;
        if (!newKeys.has(key)) {
          bus.emit('conflict:resolved', {
            modId,
            filePath,
            line: oldD.line,
            target: oldD.target,
          });
        }
      }
    }

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
    renderProblemsList(_activeDiagnostics);
  } catch (err) {
    console.error('Validation error in Monaco linter:', err);
  }
}
