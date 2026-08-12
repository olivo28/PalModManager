import { getState, updateState } from '../../state';
import { syncHighlight } from './viewer';

export let findMatches: { index: number; length: number }[] = [];
export let findCurrentMatch = -1;
let findTimeout: any = null;

export function resetFindMatches(): void {
  findMatches = [];
  findCurrentMatch = -1;
  const findCount = document.getElementById('editor-find-count');
  if (findCount) findCount.textContent = '';
}

export function openFind(): void {
  const findBar = document.getElementById('editor-find-bar')!;
  const findInput = document.getElementById('editor-find-input') as HTMLInputElement;
  findBar.style.display = 'flex';
  findInput.value = '';
  findInput.focus();
  updateFindMatches();
}

export function closeFind(): void {
  const findBar = document.getElementById('editor-find-bar')!;
  findBar.style.display = 'none';
  clearFindHighlights();
}

function clearFindHighlights(): void {
  findMatches = [];
  findCurrentMatch = -1;
  const countEl = document.getElementById('editor-find-count');
  if (countEl) countEl.textContent = '';
  syncHighlight();
}

export function updateFindMatches(): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  const findInput = document.getElementById('editor-find-input') as HTMLInputElement;
  const countEl = document.getElementById('editor-find-count')!;
  const text = editorContent.value;
  const query = findInput.value;

  findMatches = [];
  findCurrentMatch = -1;

  if (!query) {
    countEl.textContent = '';
    syncHighlight();
    return;
  }

  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  let idx = 0;
  while ((idx = lowerText.indexOf(lowerQuery, idx)) !== -1) {
    findMatches.push({ index: idx, length: query.length });
    idx += query.length;
    if (findMatches.length >= 300) {
      break;
    }
  }

  if (findMatches.length > 0) {
    findCurrentMatch = 0;
    scrollToMatch(0, false);
  }
  countEl.textContent = findMatches.length > 0
    ? `${findCurrentMatch + 1} of ${findMatches.length}${findMatches.length >= 300 ? '+' : ''}`
    : 'No matches';
  syncHighlight();
}

export function scrollToMatch(idx: number, focusEditor = true): void {
  const editorContent = document.getElementById('editor-content') as HTMLTextAreaElement;
  if (idx < 0 || idx >= findMatches.length) return;
  findCurrentMatch = idx;
  const match = findMatches[idx];
  const text = editorContent.value;
  const before = text.substring(0, match.index);
  const lines = text.split('\n');
  let pos = 0;
  for (let i = 0; i < lines.length; i++) {
    if (pos + lines[i].length >= match.index) {
      if (focusEditor) {
        editorContent.focus();
      }
      editorContent.setSelectionRange(match.index, match.index + match.length);
      const lineHeight = 20;
      editorContent.scrollTop = Math.max(0, (i - 3) * lineHeight);
      break;
    }
    pos += lines[i].length + 1;
  }
  const countEl = document.getElementById('editor-find-count');
  if (countEl) countEl.textContent = `${findCurrentMatch + 1} of ${findMatches.length}`;
  syncHighlight();
}

export function findNext(): void {
  if (findMatches.length === 0) return;
  const next = (findCurrentMatch + 1) % findMatches.length;
  scrollToMatch(next);
}

export function findPrev(): void {
  if (findMatches.length === 0) return;
  const prev = (findCurrentMatch - 1 + findMatches.length) % findMatches.length;
  scrollToMatch(prev);
}

export function setupEditorFindHandlers(): void {
  const findInput = document.getElementById('editor-find-input');
  if (!findInput) return;

  findInput.addEventListener('input', () => {
    if (findTimeout) {
      clearTimeout(findTimeout);
    }
    findTimeout = setTimeout(updateFindMatches, 250);
  });
  document.getElementById('editor-find-next')!.addEventListener('click', findNext);
  document.getElementById('editor-find-prev')!.addEventListener('click', findPrev);
  document.getElementById('editor-find-close')!.addEventListener('click', closeFind);
  findInput.addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.shiftKey ? findPrev() : findNext();
    }
    if (e.key === 'Escape') {
      closeFind();
    }
  });
}

// Helper to inject find highlights text placeholders in syncHighlight
export function getFindMatchesText(text: string): { text: string; hasMatches: boolean; count: number } {
  if (findMatches.length === 0) {
    return { text, hasMatches: false, count: 0 };
  }

  const parts: string[] = [];
  let lastEnd = 0;
  for (let i = 0; i < findMatches.length; i++) {
    const m = findMatches[i];
    parts.push(text.slice(lastEnd, m.index));
    parts.push('\x00START' + i + '\x00' + text.slice(m.index, m.index + m.length) + '\x00END' + i + '\x00');
    lastEnd = m.index + m.length;
  }
  parts.push(text.slice(lastEnd));
  return { text: parts.join(''), hasMatches: true, count: findMatches.length };
}
