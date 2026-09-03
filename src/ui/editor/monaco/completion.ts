import * as monaco from 'monaco-editor';
import { getEditorCompletions, EditorCompletion } from '../../../api';
import { getState } from '../../../state';
import { getCurrentMonacoFilePath } from './state';
import { t } from '../../../utils/i18n';

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

  // 2. JSON & JSONC (PalSchema & Reflection) Completion Provider
  const jsonProvider: monaco.languages.CompletionItemProvider = {
    triggerCharacters: ['"', ':', '/', '_', 'D', 'd', 'P', 'p', 'B', 'b', 'W', 'w', 'I', 'i'],
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
            const m = lineContent.match(/["']?([a-zA-Z0-9_]+)["']?\s*:\s*\{/);
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

function localizeDocumentation(doc?: string): string | undefined {
  if (!doc) return undefined;
  return doc
    .replace(/\*\*Class Origin\*\*:/g, `**${t('editor.completions.class_origin') || 'Class Origin'}**:`)
    .replace(/\*\*Engine Type\*\*:/g, `**${t('editor.completions.engine_type') || 'Engine Type'}**:`)
    .replace(/\*\*Expected Value\*\*:/g, `**${t('editor.completions.expected_value') || 'Expected Value'}**:`)
    .replace(/\*\*Expected\*\*:/g, `**${t('editor.completions.expected') || 'Expected'}**:`)
    .replace(/\*\*Asset Type\*\*:/g, `**${t('editor.completions.asset_type') || 'Asset Type'}**:`)
    .replace(/\*\*Mount Path\*\*:/g, `**${t('editor.completions.mount_path') || 'Mount Path'}**:`)
    .replace(/\*\*Source Package\*\*:/g, `**${t('editor.completions.source_package') || 'Source Package'}**:`)
    .replace(/\*\*Row Struct\*\*:/g, `**${t('editor.completions.row_struct') || 'Row Struct'}**:`)
    .replace(/\*\*Total Rows\*\*:/g, `**${t('editor.completions.total_rows') || 'Total Rows'}**:`)
    .replace(/\*\*Package\*\*:/g, `**${t('editor.completions.package') || 'Package'}**:`)
    .replace(/\*\*Sample Rows\*\*:/g, `**${t('editor.completions.sample_rows') || 'Sample Rows'}**:`)
    .replace(/Blueprint Generated Class/g, t('editor.completions.blueprint_gen_class') || 'Blueprint Generated Class')
    .replace(/Palworld Reflection DataTable/g, t('editor.completions.reflection_table_desc') || 'Palworld Reflection DataTable');
}

function localizeDetail(detail?: string): string | undefined {
  if (!detail) return undefined;
  if (detail === 'Blueprint Class') return t('editor.completions.blueprint_class') || detail;
  if (detail === 'Reflection DataTable') return t('editor.completions.reflection_datatable') || detail;
  if (detail === 'PalSchema Patch Template' || detail === 'PalSchema DataTable Patch') return t('editor.completions.patch_template') || detail;
  if (detail.startsWith('Starter template for ')) {
    const name = detail.replace('Starter template for ', '');
    return t('editor.completions.template_desc', { name }) || detail;
  }
  return detail;
}

function mapToMonacoCompletionItem(
  item: EditorCompletion,
  range: monaco.IRange,
  sortIndex: number,
  prefix: string = ''
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

  const sortText = prefix + String(sortIndex).padStart(4, '0');
  const isSnippet = item.insertText.includes('$') || item.insertText.includes('\n');

  return {
    label: item.label,
    kind,
    detail: localizeDetail(item.detail),
    documentation: item.documentation ? {
      value: localizeDocumentation(item.documentation) || item.documentation,
      isTrusted: true,
      supportThemeIcons: true,
    } : undefined,
    insertText: item.insertText,
    insertTextRules: isSnippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
    range,
    sortText,
  };
}
