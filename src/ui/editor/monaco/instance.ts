import * as monaco from 'monaco-editor';
import { editorDom } from '../../../framework';
import { registerMonacoCompletionProviders } from './completion';
import { registerMonacoLinter } from './linter';
import { registerMonacoQuickFixProvider } from './quickfix';
import {
  isMonacoInitialized,
  getMonacoEditor,
  setMonacoEditorInstance,
  getCurrentMonacoFilePath,
  setCurrentMonacoFilePath,
} from './state';

export { isMonacoInitialized, getMonacoEditor, getCurrentMonacoFilePath };

// Register and configure JSONC (JSON with Comments)
export function configureMonacoLanguages(): void {
  // Configure JSON language defaults to ignore comments & trailing commas
  const jsonLang = (monaco.languages as any).json;
  if (jsonLang && jsonLang.jsonDefaults) {
    jsonLang.jsonDefaults.setDiagnosticsOptions({
      validate: true,
      allowComments: true,
      comments: 'ignore',
      trailingCommas: 'ignore',
      schemaValidation: 'ignore',
    });
  }

  // Register dedicated 'jsonc' language
  monaco.languages.register({
    id: 'jsonc',
    extensions: ['.jsonc'],
    aliases: ['JSON with Comments', 'jsonc', 'JSONC'],
    mimetypes: ['application/json', 'application/jsonc'],
  });

  monaco.languages.setMonarchTokensProvider('jsonc', {
    defaultToken: '',
    tokenPostfix: '.json',
    tokenizer: {
      root: [
        [/\/\/.*$/, 'comment'],
        [/\/\*/, 'comment', '@comment'],
        [/[{}]/, 'delimiter.bracket'],
        [/[[\]]/, 'delimiter.array'],
        [/[:,]/, 'delimiter'],
        [/"([^"\\]|\\.)*"/, 'string'],
        [/-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/, 'number'],
        [/\b(?:true|false|null)\b/, 'keyword'],
      ],
      comment: [
        [/[^/*]+/, 'comment'],
        [/\*\//, 'comment', '@pop'],
        [/[/*]/, 'comment'],
      ],
    },
  });

  monaco.languages.setLanguageConfiguration('jsonc', {
    comments: {
      lineComment: '//',
      blockComment: ['/*', '*/'],
    },
    brackets: [
      ['{', '}'],
      ['[', ']'],
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '"', close: '"' },
    ],
  });
}

// Standard Vite Web Worker loader for Monaco
(self as any).MonacoEnvironment = {
  getWorker(_: unknown, label: string) {
    if (label === 'json' || label === 'jsonc') {
      return new Worker(
        new URL('../../../../node_modules/monaco-editor/esm/vs/language/json/json.worker.js', import.meta.url),
        { type: 'module' }
      );
    }
    return new Worker(
      new URL('../../../../node_modules/monaco-editor/esm/vs/editor/editor.worker.js', import.meta.url),
      { type: 'module' }
    );
  },
};

let _providersRegistered = false;

export function definePmmTheme(): void {
  monaco.editor.defineTheme('pmm-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: '', foreground: 'd4d4d8', background: '121418' },
      { token: 'comment', foreground: '64748b', fontStyle: 'italic' },
      { token: 'keyword', foreground: '38bdf8', fontStyle: 'bold' },
      { token: 'keyword.control', foreground: 'f43f5e' },
      { token: 'string', foreground: '4ade80' },
      { token: 'string.escape', foreground: 'a7f3d0' },
      { token: 'number', foreground: 'fbbf24' },
      { token: 'type', foreground: 'c084fc' },
      { token: 'function', foreground: '60a5fa' },
      { token: 'identifier', foreground: 'e2e8f0' },
      { token: 'delimiter', foreground: '94a3b8' },
      { token: 'delimiter.bracket', foreground: 'e2e8f0' },
      { token: 'delimiter.array', foreground: 'e2e8f0' },
      { token: 'key', foreground: '38bdf8' },
    ],
    colors: {
      'editor.background': '#121418',
      'editor.foreground': '#d4d4d8',
      'editor.lineHighlightBackground': '#1a1f26',
      'editor.selectionBackground': '#0284c733',
      'editorInactiveSelection.background': '#0284c71a',
      'editorCursor.foreground': '#38bdf8',
      'editorLineNumber.foreground': '#475569',
      'editorLineNumber.activeForeground': '#38bdf8',
      'editorGutter.background': '#15181e',
      'editorWidget.background': '#181b22',
      'editorWidget.border': '#334155',
      'editorSuggestWidget.background': '#181b22',
      'editorSuggestWidget.border': '#334155',
      'editorSuggestWidget.foreground': '#f1f5f9',
      'editorSuggestWidget.selectedBackground': '#0369a1',
      'editorSuggestWidget.highlightForeground': '#38bdf8',
      'editorHoverWidget.background': '#181b22',
      'editorHoverWidget.border': '#334155',
      'scrollbarSlider.background': '#33415544',
      'scrollbarSlider.hoverBackground': '#33415588',
      'scrollbarSlider.activeBackground': '#38bdf866',
    },
  });
}

export function initMonacoEditor(): monaco.editor.IStandaloneCodeEditor {
  const existing = getMonacoEditor();
  if (existing) {
    return existing;
  }

  const container = editorDom.elMaybe('editor-monaco-container');
  if (!container) {
    throw new Error('Monaco container #editor-monaco-container not found in DOM');
  }

  definePmmTheme();
  configureMonacoLanguages();

  const editorInstance = monaco.editor.create(container, {
    value: '',
    language: 'lua',
    theme: 'pmm-dark',
    automaticLayout: true,
    fontSize: 13.5,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace",
    fontLigatures: true,
    lineHeight: 22,
    tabSize: 4,
    insertSpaces: true,
    renderLineHighlight: 'all',
    minimap: {
      enabled: true,
      maxColumn: 80,
      scale: 1,
      renderCharacters: false,
    },
    bracketPairColorization: {
      enabled: true,
    },
    guides: {
      bracketPairs: true,
      indentation: true,
    },
    suggest: {
      showKeywords: true,
      showSnippets: true,
      preview: true,
      filterGraceful: true,
      localityBonus: true,
      shareSuggestSelections: true,
    },
    quickSuggestions: {
      other: true,
      comments: false,
      strings: true,
    },
    parameterHints: {
      enabled: true,
    },
    scrollBeyondLastLine: false,
    smoothScrolling: true,
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    wordWrap: 'off',
    folding: true,
    foldingHighlight: true,
  });

  setMonacoEditorInstance(editorInstance);

  if (!_providersRegistered) {
    registerMonacoCompletionProviders();
    registerMonacoQuickFixProvider();
    _providersRegistered = true;
  }

  registerMonacoLinter(editorInstance);

  return editorInstance;
}

export function setMonacoFile(filePath: string, content: string): void {
  setCurrentMonacoFilePath(filePath);
  const editor = getMonacoEditor() || initMonacoEditor();

  let language = 'plaintext';
  if (filePath.endsWith('.lua')) {
    language = 'lua';
  } else if (filePath.endsWith('.jsonc')) {
    language = 'jsonc';
  } else if (filePath.endsWith('.json')) {
    language = 'json';
  } else if (filePath.endsWith('.ini') || filePath.endsWith('.cfg')) {
    language = 'ini';
  } else if (filePath.endsWith('.md')) {
    language = 'markdown';
  }

  const uri = monaco.Uri.file(filePath);
  let model = monaco.editor.getModel(uri);

  if (!model) {
    model = monaco.editor.createModel(content, language, uri);
  } else {
    model.setValue(content);
    monaco.editor.setModelLanguage(model, language);
  }

  editor.setModel(model);
}

export function getMonacoContent(): string {
  const editor = getMonacoEditor();
  if (!editor) return '';
  return editor.getValue();
}
