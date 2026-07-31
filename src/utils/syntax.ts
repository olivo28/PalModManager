import hljs from 'highlight.js';

export function highlightText(text: string, ext: string): string {
  const lang = ext === 'lua' ? 'lua' : ext === 'jsonc' ? 'json' : 'json';
  const safe = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  try {
    const result = hljs.highlight(safe, { language: lang });
    return result.value;
  } catch {
    return safe;
  }
}
