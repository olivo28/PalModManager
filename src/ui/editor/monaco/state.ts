import type * as monaco from 'monaco-editor';

let _editorInstance: monaco.editor.IStandaloneCodeEditor | null = null;
let _currentFilePath = '';

export function isMonacoInitialized(): boolean {
  return _editorInstance !== null;
}

export function getMonacoEditor(): monaco.editor.IStandaloneCodeEditor | null {
  return _editorInstance;
}

export function setMonacoEditorInstance(editor: monaco.editor.IStandaloneCodeEditor | null): void {
  _editorInstance = editor;
}

export function getCurrentMonacoFilePath(): string {
  return _currentFilePath;
}

export function setCurrentMonacoFilePath(path: string): void {
  _currentFilePath = path;
}
