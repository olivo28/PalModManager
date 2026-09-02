import { getMonacoEditor } from './monaco/instance';

export let findMatches: { index: number; length: number }[] = [];
export let findCurrentMatch = -1;

export function resetFindMatches(): void {
  findMatches = [];
  findCurrentMatch = -1;
}

export function openFind(): void {
  const editor = getMonacoEditor();
  if (editor) {
    editor.getAction('actions.find')?.run();
  }
}

export function closeFind(): void {
  findMatches = [];
  findCurrentMatch = -1;
}

export function updateFindMatches(): void {
  // Monaco natively manages find match decoration and counting
}

export function scrollToMatch(_matchIndex: number): void {
  // Monaco natively scrolls to match
}

export function findNext(): void {
  const editor = getMonacoEditor();
  if (editor) {
    editor.getAction('editor.action.nextMatchFindAction')?.run();
  }
}

export function findPrev(): void {
  const editor = getMonacoEditor();
  if (editor) {
    editor.getAction('editor.action.previousMatchFindAction')?.run();
  }
}

export function setupEditorFindHandlers(): void {
  // Monaco handles find keyboard shortcuts natively
}

export function getFindMatchesText(text: string): { text: string; hasMatches: boolean; count: number } {
  return { text, hasMatches: false, count: 0 };
}
