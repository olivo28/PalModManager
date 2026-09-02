import * as monaco from 'monaco-editor';
import { getActiveDiagnostics } from './linter';

export function registerMonacoQuickFixProvider(): void {
  const provider: monaco.languages.CodeActionProvider = {
    provideCodeActions(model, range, context) {
      const actions: monaco.languages.CodeAction[] = [];
      const diagnostics = getActiveDiagnostics();

      for (const diag of diagnostics) {
        if (!diag.suggestion) continue;

        // Check if marker intersects current cursor range
        const isMatch = context.markers.some((m) => {
          return (
            m.startLineNumber === diag.line &&
            Math.abs(m.startColumn - diag.column) < 5
          );
        });

        if (isMatch || range.startLineNumber === diag.line) {
          const startLine = diag.line;
          const startCol = diag.column;
          const endLine = diag.endLine || diag.line;
          const endCol = diag.endColumn || (startCol + diag.target.length);

          let title = `💡 Quick Fix: Replace with "${diag.suggestion}"`;
          if (diag.category === 'ue4ss_deprecated') {
            title = `💡 Replace deprecated "${diag.target}" with "${diag.suggestion}"`;
          } else if (diag.category === 'anti_pattern') {
            title = `🛡️ Safe Error Handling: Capture "${diag.suggestion}"`;
          }

          actions.push({
            title,
            kind: 'quickfix',
            isPreferred: true,
            diagnostics: context.markers,
            edit: {
              edits: [
                {
                  resource: model.uri,
                  textEdit: {
                    range: {
                      startLineNumber: startLine,
                      startColumn: startCol,
                      endLineNumber: endLine,
                      endColumn: endCol,
                    },
                    text: diag.suggestion,
                  },
                  versionId: model.getVersionId(),
                },
              ],
            },
          });
        }
      }

      return {
        actions,
        dispose() {},
      };
    },
  };

  monaco.languages.registerCodeActionProvider('lua', provider);
  monaco.languages.registerCodeActionProvider('json', provider);
  monaco.languages.registerCodeActionProvider('jsonc', provider);
}
