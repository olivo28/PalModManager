import * as monaco from 'monaco-editor';
import type { PmmTheme } from '../../../app/theme/types';

let _themesDefined = false;

/**
 * Registers canonical factory Monaco themes: 'pmm-dark' and 'pmm-light'.
 */
export function defineDefaultMonacoThemes(): void {
  if (_themesDefined) return;

  // 1. PMM Dark Theme (VS Code Dark+ inspired)
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

  // 2. PMM Light Theme (VS Code Light Modern inspired)
  monaco.editor.defineTheme('pmm-light', {
    base: 'vs',
    inherit: true,
    rules: [
      { token: '', foreground: '0f172a', background: 'f8fafc' },
      { token: 'comment', foreground: '008000', fontStyle: 'italic' },
      { token: 'keyword', foreground: '0000ff' },
      { token: 'keyword.control', foreground: 'af00db' },
      { token: 'keyword.operator', foreground: '0f172a' },
      { token: 'string', foreground: 'a31515' },
      { token: 'string.escape', foreground: 'ee0000' },
      { token: 'number', foreground: '098658' },
      { token: 'type', foreground: '267f99' },
      { token: 'function', foreground: '795e26' },
      { token: 'identifier', foreground: '0f172a' },
      { token: 'delimiter', foreground: '0f172a' },
      { token: 'delimiter.bracket', foreground: '0451a5' },
      { token: 'delimiter.array', foreground: '0f172a' },
      { token: 'key', foreground: '0451a5' },
      { token: 'string.key', foreground: '0451a5' },
      { token: 'string.key.json', foreground: '0451a5' },
      { token: 'string.value', foreground: 'a31515' },
      { token: 'string.value.json', foreground: 'a31515' },
      { token: 'keyword.boolean', foreground: '0000ff', fontStyle: 'bold' },
      { token: 'keyword.null', foreground: '0000ff' },
      { token: 'keyword.json', foreground: '0000ff', fontStyle: 'bold' },
      { token: 'constant', foreground: '0070c1' },
      { token: 'constant.language', foreground: '0000ff' },
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': '#0f172a',
      'editor.lineHighlightBackground': '#f1f5f9',
      'editor.selectionBackground': '#bae6fd80',
      'editorInactiveSelection.background': '#bae6fd40',
      'editorCursor.foreground': '#0284c7',
      'editorLineNumber.foreground': '#94a3b8',
      'editorLineNumber.activeForeground': '#0284c7',
      'editorGutter.background': '#f8fafc',
      'editorWidget.background': '#f8fafc',
      'editorWidget.border': '#cbd5e1',
      'editorSuggestWidget.background': '#ffffff',
      'editorSuggestWidget.border': '#cbd5e1',
      'editorSuggestWidget.foreground': '#0f172a',
      'editorSuggestWidget.selectedBackground': '#e0f2fe',
      'editorSuggestWidget.highlightForeground': '#0284c7',
      'editorHoverWidget.background': '#ffffff',
      'editorHoverWidget.border': '#cbd5e1',
      'scrollbarSlider.background': '#94a3b833',
      'scrollbarSlider.hoverBackground': '#94a3b866',
      'scrollbarSlider.activeBackground': '#64748b66',
    },
  });

  _themesDefined = true;
}

/**
 * Synchronizes Monaco Editor with the given active PMM Theme.
 * For builtin themes, applies 'pmm-dark' or 'pmm-light'.
 * For custom themes, dynamically defines 'pmm-custom' with the theme's colors.
 */
export function syncMonacoTheme(theme: PmmTheme): void {
  defineDefaultMonacoThemes();

  if (theme.isBuiltin) {
    const targetTheme = theme.mode === 'light' ? 'pmm-light' : 'pmm-dark';
    monaco.editor.setTheme(targetTheme);
    return;
  }

  // Custom theme: define dynamic Monaco theme matching custom color palette
  const isLight = theme.mode === 'light';
  const colors = theme.colors;
  const accent = colors.accent;
  const bgPrimary = colors.bgPrimary;
  const bgSecondary = colors.bgSecondary;
  const bgCard = colors.bgCard;
  const border = colors.border;
  const textPrimary = colors.textPrimary;
  const textMuted = colors.textMuted || (isLight ? '#94a3b8' : '#6e7681');

  monaco.editor.defineTheme('pmm-custom', {
    base: isLight ? 'vs' : 'vs-dark',
    inherit: true,
    rules: [
      { token: '', foreground: textPrimary.replace('#', ''), background: bgPrimary.replace('#', '') },
    ],
    colors: {
      'editor.background': bgPrimary,
      'editor.foreground': textPrimary,
      'editor.lineHighlightBackground': bgSecondary,
      'editor.selectionBackground': `${accent}40`,
      'editorInactiveSelection.background': `${accent}20`,
      'editorCursor.foreground': accent,
      'editorLineNumber.foreground': textMuted,
      'editorLineNumber.activeForeground': accent,
      'editorGutter.background': bgPrimary,
      'editorWidget.background': bgSecondary,
      'editorWidget.border': border,
      'editorSuggestWidget.background': bgCard,
      'editorSuggestWidget.border': border,
      'editorSuggestWidget.foreground': textPrimary,
      'editorSuggestWidget.selectedBackground': `${accent}30`,
      'editorSuggestWidget.highlightForeground': accent,
      'editorHoverWidget.background': bgCard,
      'editorHoverWidget.border': border,
      'scrollbarSlider.background': `${border}40`,
      'scrollbarSlider.hoverBackground': `${border}80`,
      'scrollbarSlider.activeBackground': `${accent}80`,
    },
  });

  monaco.editor.setTheme('pmm-custom');
}
