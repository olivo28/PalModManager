import hljs from 'highlight.js';

export function getLanguageFromExt(ext: string): string {
  const lower = ext.toLowerCase().trim();
  switch (lower) {
    case 'lua':
      return 'lua';
    case 'json':
    case 'jsonc':
      return 'json';
    case 'ini':
    case 'cfg':
    case 'conf':
    case 'properties':
      return 'ini';
    case 'toml':
      return 'ini';
    case 'yaml':
    case 'yml':
      return 'yaml';
    case 'xml':
    case 'html':
    case 'svg':
      return 'xml';
    case 'py':
      return 'python';
    case 'md':
    case 'markdown':
      return 'markdown';
    default:
      return 'plaintext';
  }
}

export function highlightText(text: string, ext: string): string {
  const lang = getLanguageFromExt(ext);
  const safe = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  if (lang === 'plaintext') {
    return safe;
  }

  try {
    const result = hljs.highlight(safe, { language: lang, ignoreIllegals: true });
    return result.value;
  } catch {
    return safe;
  }
}
