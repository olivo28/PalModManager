import * as monaco from 'monaco-editor';
import { getEditorCompletions, EditorCompletion } from '../../../api';
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
        const results = await getEditorCompletions(filePath, query, textUntilPosition);
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

  // 2. JSON & JSONC (PalSchema) Completion Provider
  const jsonProvider: monaco.languages.CompletionItemProvider = {
    triggerCharacters: ['"', ':', '{', ' ', '/', 'D', 'P', 'B'],
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
        const results = await getEditorCompletions(filePath, query, textUntilPosition);
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
      kind = monaco.languages.CompletionItemKind.Property;
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
