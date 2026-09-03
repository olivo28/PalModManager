import * as monaco from 'monaco-editor';
import { getEditorCompletions, EditorCompletion } from '../../../api';
import { getState } from '../../../state';
import { getCurrentMonacoFilePath } from './state';

export function registerMonacoCompletionProviders(): void {
  // 1. Lua Completion Provider
  monaco.languages.registerCompletionItemProvider('lua', {
    triggerCharacters: ['/', '.', ':', '"', "'", '(', ' '],
    async provideCompletionItems(model, position) {
      const filePath = getCurrentMonacoFilePath() || model.uri.fsPath || 'script.lua';
      const textUntilPosition = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });

      const word = model.getWordUntilPosition(position);

      // Check if cursor is inside quotes
      const lastQuoteD = textUntilPosition.lastIndexOf('"');
      const lastQuoteS = textUntilPosition.lastIndexOf("'");
      const lastQuote = Math.max(lastQuoteD, lastQuoteS);

      const query = lastQuote !== -1
        ? textUntilPosition.substring(lastQuote + 1)
        : word.word;

      const range: monaco.IRange = lastQuote !== -1
        ? {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: lastQuote + 2,
          endColumn: position.column,
        }
        : {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

      try {
        const state = getState();
        const modId = state.editorModId || undefined;
        const results = await getEditorCompletions(filePath, query, textUntilPosition, modId);
        if (!results || results.length === 0) {
          return { suggestions: [] };
        }

        const suggestions: monaco.languages.CompletionItem[] = results.map((item, index) => {
          return mapToMonacoCompletionItem(item, range, index);
        });

        return { suggestions };
      } catch (err) {
        console.error('Monaco Lua completion provider error:', err);
        return { suggestions: [] };
      }
    },
  });

  // 2. JSON & JSONC (PalSchema & Workspace) Completion Provider
  const jsonProvider: monaco.languages.CompletionItemProvider = {
    triggerCharacters: ['"', ':', '{', ' ', '/', '_', 'D', 'd', 'P', 'p', 'B', 'b', 'W', 'w', 'I', 'i'],
    async provideCompletionItems(model, position) {
      const filePath = getCurrentMonacoFilePath() || model.uri.fsPath || 'schema.jsonc';
      const textUntilPosition = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });

      const word = model.getWordUntilPosition(position);

      // Check if cursor is inside quotes
      const lastQuoteD = textUntilPosition.lastIndexOf('"');
      const lastQuoteS = textUntilPosition.lastIndexOf("'");
      const lastQuote = Math.max(lastQuoteD, lastQuoteS);

      const query = lastQuote !== -1
        ? textUntilPosition.substring(lastQuote + 1)
        : word.word;

      const range: monaco.IRange = lastQuote !== -1
        ? {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: lastQuote + 2,
          endColumn: position.column,
        }
        : {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

      try {
        const state = getState();
        const modId = state.editorModId || undefined;

        // Detect parent block context in JSON/JSONC (e.g. inside "BP_LaserRifle_C": { ...)
        let contextPrefix = textUntilPosition;
        let depth = 0;
        for (let ln = position.lineNumber; ln >= 1; ln--) {
          const lineContent = model.getLineContent(ln);
          const openMatches = (lineContent.match(/{/g) || []).length;
          const closeMatches = (lineContent.match(/}/g) || []).length;
          depth += (closeMatches - openMatches);
          if (depth < 0) {
            const m = lineContent.match(/"([^"]+)"\s*:\s*\{/);
            if (m) {
              contextPrefix = `[context:${m[1]}] ${textUntilPosition}`;
            }
            break;
          }
        }

        const results = await getEditorCompletions(filePath, query, contextPrefix, modId);
        if (!results || results.length === 0) {
          return { suggestions: [] };
        }

        const suggestions: monaco.languages.CompletionItem[] = results.map((item, index) => {
          return mapToMonacoCompletionItem(item, range, index);
        });

        return { suggestions };
      } catch (err) {
        console.error('Monaco JSON completion provider error:', err);
        return { suggestions: [] };
      }
    },
  };

  monaco.languages.registerCompletionItemProvider('json', jsonProvider);
  monaco.languages.registerCompletionItemProvider('jsonc', jsonProvider);
}

function mapToMonacoCompletionItem(
  item: EditorCompletion,
  range: monaco.IRange,
  sortIndex: number
): monaco.languages.CompletionItem {
  let kind = monaco.languages.CompletionItemKind.Text;

  switch (item.kind) {
    case 'module':
      kind = monaco.languages.CompletionItemKind.Module;
      break;
    case 'class':
      kind = monaco.languages.CompletionItemKind.Class;
      break;
    case 'function':
      kind = monaco.languages.CompletionItemKind.Method;
      break;
    case 'delegate':
      kind = monaco.languages.CompletionItemKind.Event;
      break;
    case 'table':
      kind = monaco.languages.CompletionItemKind.Interface;
      break;
    case 'struct':
    case 'property':
      kind = monaco.languages.CompletionItemKind.Property;
      break;
    case 'field':
      kind = monaco.languages.CompletionItemKind.Field;
      break;
    case 'constant':
      kind = monaco.languages.CompletionItemKind.Constant;
      break;
    case 'value':
      kind = monaco.languages.CompletionItemKind.Value;
      break;
    case 'api':
      kind = monaco.languages.CompletionItemKind.Function;
      break;
    case 'hook':
      kind = monaco.languages.CompletionItemKind.Keyword;
      break;
  }

  const sortText = String(sortIndex).padStart(4, '0');
  const isSnippet = item.insertText.includes('$') || item.insertText.includes('\n');

  return {
    label: item.label,
    kind,
    detail: item.detail,
    documentation: item.documentation ? { value: item.documentation } : undefined,
    insertText: item.insertText,
    insertTextRules: isSnippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
    range,
    sortText,
  };
}
