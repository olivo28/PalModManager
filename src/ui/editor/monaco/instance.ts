import * as monaco from 'monaco-editor';
import { editorDom } from '../../../framework';
import { t } from '../../../utils/i18n';
import { registerMonacoCompletionProviders } from './completion';
import { registerMonacoLinter } from './linter';
import { registerMonacoQuickFixProvider } from './quickfix';
import { initializeMonacoPalSchemas } from './schemas';
import {
  isMonacoInitialized,
  getMonacoEditor,
  setMonacoEditorInstance,
  getCurrentMonacoFilePath,
  setCurrentMonacoFilePath,
} from './state';
import { updateStatusBarCursor, updateStatusBarLanguage } from './statusBar';

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
    if (typeof jsonLang.jsonDefaults.setModeConfiguration === 'function') {
      jsonLang.jsonDefaults.setModeConfiguration({
        documentFormattingEdits: true,
        documentRangeFormattingEdits: true,
        completionItems: false, // Prevent Monaco's built-in schema scraper from leaking keys from other files like modinfo.pmm.json
        hovers: true,
        documentSymbols: true,
        tokens: true,
        colors: true,
        foldingRanges: true,
        selectionRanges: true,
      });
    }
  }

  // Register dedicated 'jsonc' language
  monaco.languages.register({
    id: 'jsonc',
    extensions: ['.jsonc', '.json'],
    aliases: ['JSON with Comments', 'jsonc', 'JSONC', 'JSON'],
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
        [/"([^"\\]|\\.)*"(?=\s*:)/, 'string.key'],
        [/"([^"\\]|\\.)*"/, 'string.value'],
        [/-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/, 'number'],
        [/\b(?:true|false)\b/, 'keyword.boolean'],
        [/\bnull\b/, 'keyword.null'],
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
      { token: '', foreground: 'd4d4d4', background: '1e1e1e' },
      { token: 'comment', foreground: '6a9955', fontStyle: 'italic' },
      { token: 'keyword', foreground: '569cd6' },
      { token: 'keyword.control', foreground: 'c586c0' },
      { token: 'keyword.operator', foreground: 'd4d4d4' },
      { token: 'string', foreground: 'ce9178' },
      { token: 'string.escape', foreground: 'd7ba7d' },
      { token: 'number', foreground: 'b5cea8' },
      { token: 'type', foreground: '4ec9b0' },
      { token: 'function', foreground: 'dcdcaa' },
      { token: 'identifier', foreground: '9cdcfe' },
      { token: 'delimiter', foreground: 'd4d4d4' },
      { token: 'delimiter.bracket', foreground: 'ffd700' },
      { token: 'delimiter.array', foreground: 'd4d4d4' },
      { token: 'key', foreground: '9cdcfe' },
      { token: 'string.key', foreground: '9cdcfe' },
      { token: 'string.key.json', foreground: '9cdcfe' },
      { token: 'string.value', foreground: 'ce9178' },
      { token: 'string.value.json', foreground: 'ce9178' },
      { token: 'keyword.boolean', foreground: '569cd6', fontStyle: 'bold' },
      { token: 'keyword.null', foreground: '569cd6' },
      { token: 'keyword.json', foreground: '569cd6', fontStyle: 'bold' },
      { token: 'constant', foreground: '4fc1ff' },
      { token: 'constant.language', foreground: '569cd6' },
    ],
    colors: {
      'editor.background': '#1e1e1e',
      'editor.foreground': '#d4d4d4',
      'editor.lineHighlightBackground': '#282b30',
      'editor.selectionBackground': '#264f7880',
      'editorInactiveSelection.background': '#264f7840',
      'editorCursor.foreground': '#00d2ff',
      'editorLineNumber.foreground': '#6e7681',
      'editorLineNumber.activeForeground': '#00d2ff',
      'editorGutter.background': '#1e1e1e',
      'editorWidget.background': '#252526',
      'editorWidget.border': '#454545',
      'editorSuggestWidget.background': '#252526',
      'editorSuggestWidget.border': '#454545',
      'editorSuggestWidget.foreground': '#cccccc',
      'editorSuggestWidget.selectedBackground': '#04395e',
      'editorSuggestWidget.highlightForeground': '#00d2ff',
      'editorHoverWidget.background': '#252526',
      'editorHoverWidget.border': '#454545',
      'scrollbarSlider.background': '#79797933',
      'scrollbarSlider.hoverBackground': '#79797966',
      'scrollbarSlider.activeBackground': '#bfbfbf66',
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
      showInlineDetails: true,
      showStatusBar: true,
    },
    quickSuggestions: {
      other: true,
      comments: false,
      strings: true,
    },
    wordBasedSuggestions: 'off',
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

  // Ensure suggestion documentation details panel is auto-expanded by default
  try {
    localStorage.setItem('expandSuggestionDocs', 'true');
    localStorage.setItem('suggestWidget.detailsVisible', 'true');
  } catch {}

  setMonacoEditorInstance(editorInstance);
  editorInstance.onDidChangeModelContent(() => {
    import('../viewer').then((v) => v.updateUnsavedIndicator()).catch(() => {});
  });

  if (!_providersRegistered) {
    registerMonacoCompletionProviders();
    registerMonacoQuickFixProvider();
    _providersRegistered = true;
    setTimeout(() => {
      initializeMonacoPalSchemas().catch(console.error);
    }, 0);
  }

  registerMonacoLinter(editorInstance);

  // Custom Context Menu Actions
  editorInstance.addAction({
    id: 'pmm-save-action',
    label: t('editor.context_save') || 'Save File',
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
    contextMenuGroupId: '1_modification',
    contextMenuOrder: 1,
    run: async () => {
      const { handleEditorSave } = await import('../viewer');
      await handleEditorSave();
    },
  });

  editorInstance.addAction({
    id: 'pmm-format-action',
    label: t('editor.context_format') || 'Format Document',
    keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF],
    contextMenuGroupId: '1_modification',
    contextMenuOrder: 2,
    run: async () => {
      const { handleEditorFormat } = await import('../viewer');
      await handleEditorFormat();
    },
  });

  editorInstance.addAction({
    id: 'pmm-problems-action',
    label: t('editor.context_toggle_problems') || 'Toggle Problems Panel',
    contextMenuGroupId: '2_navigation',
    contextMenuOrder: 1,
    run: async () => {
      const { toggleProblemsPanel } = await import('./problemsPanel');
      toggleProblemsPanel();
    },
  });

  // Suppress generic VS Code command palette on F1
  editorInstance.addCommand(monaco.KeyCode.F1, () => {});

  // Live cursor position tracking for editor bottom status bar
  editorInstance.onDidChangeCursorPosition((e) => {
    updateStatusBarCursor(e.position.lineNumber, e.position.column);
  });

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

  updateStatusBarLanguage(filePath);

  const cleanPath = filePath.replace(/\\/g, '/');
  const uri = cleanPath.startsWith('/') || cleanPath.includes(':/')
    ? monaco.Uri.file(cleanPath)
    : monaco.Uri.parse(`pmm:///${encodeURI(cleanPath)}`);

  let model = monaco.editor.getModel(uri);

  if (!model) {
    model = monaco.editor.createModel(content, language, uri);
  } else {
    model.setValue(content);
    monaco.editor.setModelLanguage(model, language);
  }

  if (language === 'jsonc') {
    // Clear any stale RFC 8259 standard JSON markers emitted by json.worker
    monaco.editor.setModelMarkers(model, 'json', []);
  }

  editor.setModel(model);
}

export function getMonacoContent(): string {
  const editor = getMonacoEditor();
  if (!editor) return '';
  return editor.getValue();
}
