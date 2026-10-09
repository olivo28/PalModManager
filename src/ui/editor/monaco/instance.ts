import * as monaco from 'monaco-editor';
import { editorDom, bus } from '../../../framework';
import { t } from '../../../utils/i18n';
import { getState } from '../../../state';
import { registerMonacoCompletionProviders } from './completion';
import { registerMonacoLinter } from './linter';
import { registerMonacoQuickFixProvider } from './quickfix';
import { initializeMonacoPalSchemas } from './schemas';
import { defineDefaultMonacoThemes, syncMonacoTheme } from './themes';
import { getActiveThemeId, findThemeById } from '../../../app/theme/storage';
import {
  isMonacoInitialized,
  getMonacoEditor,
  setMonacoEditorInstance,
  getCurrentMonacoFilePath,
  setCurrentMonacoFilePath,
} from './state';
import { updateStatusBarCursor, updateStatusBarLanguage } from './statusBar';

export { isMonacoInitialized, getMonacoEditor, getCurrentMonacoFilePath, defineDefaultMonacoThemes, defineDefaultMonacoThemes as definePmmTheme, syncMonacoTheme };

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
let _themeListenerAttached = false;

export function initMonacoEditor(): monaco.editor.IStandaloneCodeEditor {
  const existing = getMonacoEditor();
  if (existing) {
    return existing;
  }

  const container = editorDom.elMaybe('editor-monaco-container');
  if (!container) {
    throw new Error('Monaco container #editor-monaco-container not found in DOM');
  }

  defineDefaultMonacoThemes();
  configureMonacoLanguages();

  // Determine active theme on initialization
  let initialTheme = 'pmm-dark';
  try {
    const activeTheme = findThemeById(getActiveThemeId());
    if (activeTheme.isBuiltin) {
      initialTheme = activeTheme.mode === 'light' ? 'pmm-light' : 'pmm-dark';
    } else {
      syncMonacoTheme(activeTheme);
      initialTheme = 'pmm-custom';
    }
  } catch {}

  const currentScale = getState().currentSettings?.uiScale || getState().currentSettings?.toolbarScale || 1.0;
  const editorInstance = monaco.editor.create(container, {
    value: '',
    language: 'lua',
    theme: initialTheme,
    automaticLayout: true,
    fontSize: Math.round(13.5 * currentScale * 10) / 10,
    fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace",
    fontLigatures: true,
    lineHeight: Math.round(22 * currentScale),
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

  if (!_themeListenerAttached) {
    bus.on('theme:changed', ({ theme }) => {
      syncMonacoTheme(theme);
    });
    _themeListenerAttached = true;
  }

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

export function setMonacoFile(filePath: string, content: string, modId?: string | null): void {
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
  const mId = modId || getState().editorModId || '';
  const modPrefix = mId ? `${encodeURIComponent(mId)}/` : '';
  const uri = cleanPath.startsWith('/') || cleanPath.includes(':/')
    ? monaco.Uri.file(cleanPath)
    : monaco.Uri.parse(`pmm:///${modPrefix}${encodeURI(cleanPath)}`);

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

export function updateMonacoScale(scale: number): void {
  const editor = getMonacoEditor();
  if (editor) {
    editor.updateOptions({
      fontSize: Math.round(13.5 * scale * 10) / 10,
      lineHeight: Math.round(22 * scale),
    });
  }
}

