import { marked } from 'marked';

export function descriptionToHtml(text: string): string {
  if (!text) return '';

  // 1. NORMALIZACIÓN INICIAL (Unificar el caos de Nexus)
  let content = text
    // Limpiar entidades de corchetes y backslashes
    .replace(/&amp;#91;/g, '[').replace(/&#91;/g, '[')
    .replace(/&amp;#93;/g, ']').replace(/&#93;/g, ']')
    .replace(/&amp;#92;/g, '\\').replace(/&#92;/g, '\\')
    // Nexus mezcla <br /> de texto con saltos \n. Los unificamos todos a \n
    .replace(/<br\s*\/?>/gi, '\n')
    // Colapsar espacios verticales exagerados
    .replace(/\n{3,}/g, '\n\n');

  // 2. PROCESAR BBCODE (Recursivo/iterativo para tags anidados)
  content = bbcodeToHtml(content);

  // 3. DETECTAR Y PROCESAR MARKDOWN
  if (detectMarkdown(content)) {
    content = mdToHtml(content);
  } else {
    // Si no es MD, convertimos los \n sobrantes en <br />
    content = content.replace(/\n/g, '<br />');
    content = content.replace(/(<(?:div|blockquote|ul|ol|li|h[1-6]|hr|pre|details|section|table)[^>]*>)<br \/>/gi, '$1');
    content = content.replace(/<br \/>(<\/(?:div|blockquote|ul|ol|li|h[1-6]|hr|pre|details|section|table)>)/gi, '$1');
  }

  // 4. SANITIZACIÓN FINAL
  return sanitizeHtml(content);
}

function bbcodeToHtml(input: string): string {
  let html = input;

  // --- 1. ETIQUETAS DE BLOQUE ---

  // [heading]
  html = html.replace(/\[heading\]([\s\S]*?)\[\/heading\]/gi, '<h3 class="bbcode-heading">$1</h3>');

  // [quote]
  html = html.replace(/\[quote\]([\s\S]*?)\[\/quote\]/gi, '<blockquote class="bbcode-quote">$1</blockquote>');

  // [spoiler]
  html = html.replace(/\[spoiler(?:=([^\]]*))?\]([\s\S]*?)\[\/spoiler\]/gi, (_, title, body) => {
    const summary = title ? `Spoiler: ${title}` : 'Spoiler (Click to show)';
    return `<details class="bbcode-spoiler"><summary>${summary}</summary><div class="bbcode-spoiler-content">${body}</div></details>`;
  });

  // [center], [left], [right]
  html = html.replace(/\[center\]([\s\S]*?)\[\/center\]/gi, '<div style="text-align:center">$1</div>');
  html = html.replace(/\[left\]([\s\S]*?)\[\/left\]/gi, '<div style="text-align:left">$1</div>');
  html = html.replace(/\[right\]([\s\S]*?)\[\/right\]/gi, '<div style="text-align:right">$1</div>');

  // [code]
  html = html.replace(/\[code\]([\s\S]*?)\[\/code\]/gi, '<pre class="bbcode-code">$1</pre>');

  // [list] soportando [*] de Nexus
  html = html.replace(/\[list(?:=1)?\]([\s\S]*?)\[\/list\]/gi, (match, inner) => {
    const tag = match.toLowerCase().includes('=1') ? 'ol' : 'ul';
    const items = inner.split(/\[\*\]/).filter((i: string) => i.trim());
    const listContent = items.map((i: string) => `<li>${i.replace(/\[\/\*\]/g, '').trim()}</li>`).join('');
    return `<${tag} class="bbcode-list">${listContent}</${tag}>`;
  });

  // [line] o [hr]
  html = html.replace(/\[line\]|\[hr\]/gi, '<hr class="bbcode-hr" />');

  // [youtube]
  html = html.replace(/\[youtube\](?:https?:\/\/)?(?:www\.)?(?:youtube\.com\/watch\?v=|youtu\.be\/)?([a-zA-Z0-9_-]+)\[\/youtube\]/gi,
    '<div class="bbcode-video-wrap"><iframe src="https://www.youtube.com/embed/$1" frameborder="0" allowfullscreen></iframe></div>');

  // --- 2. ETIQUETAS INLINE ITERATIVAS (Para anidamientos profundos como [b][size=4][b]...) ---
  const sizeMap: Record<string, string> = {
    '1': '0.75rem',
    '2': '0.85rem',
    '3': '1rem',
    '4': '1.25rem',
    '5': '1.5rem',
    '6': '1.85rem',
    '7': '2.25rem',
  };

  let maxIterations = 8;
  while (maxIterations > 0) {
    const prev = html;

    // [b], [i], [u], [s]
    html = html.replace(/\[b\]([\s\S]*?)\[\/b\]/gi, '<strong>$1</strong>');
    html = html.replace(/\[i\]([\s\S]*?)\[\/i\]/gi, '<em>$1</em>');
    html = html.replace(/\[u\]([\s\S]*?)\[\/u\]/gi, '<u>$1</u>');
    html = html.replace(/\[s\]([\s\S]*?)\[\/s\]/gi, '<del>$1</del>');

    // [size]
    html = html.replace(/\[size=([0-9]+(?:px|pt)?)\]([\s\S]*?)\[\/size\]/gi, (_, size, inner) => {
      const fontSize = sizeMap[size] || (size.includes('px') || size.includes('pt') ? size : `${size}px`);
      return `<span style="font-size: ${fontSize}">${inner}</span>`;
    });

    // [color]
    html = html.replace(/\[color=([^\]]+)\]([\s\S]*?)\[\/color\]/gi, '<span style="color:$1">$2</span>');

    // [font] -> strip tag
    html = html.replace(/\[font=[^\]]+\]([\s\S]*?)\[\/font\]/gi, '$1');

    // If no changes were made in this pass, break out of loop
    if (html === prev) break;
    maxIterations--;
  }

  // [url]
  html = html.replace(/\[url=([^\]]+)\]([\s\S]*?)\[\/url\]/gi, '<a href="$1" target="_blank" rel="noopener noreferrer" class="bbcode-link">$2</a>');
  html = html.replace(/\[url\]([\s\S]*?)\[\/url\]/gi, '<a href="$1" target="_blank" rel="noopener noreferrer" class="bbcode-link">$1</a>');

  // [img] con zoom lightbox clickable
  html = html.replace(/\[img(?:[^\]]*width=([0-9]+))?\]([\s\S]*?)\[\/img\]/gi, (_, width, url) => {
    const cleanUrl = url.trim();
    const style = width ? `width:${width}px;max-width:100%;` : 'max-width:100%;';
    return `<img src="${cleanUrl}" style="${style}" class="bbcode-img cursor-zoom" loading="lazy" data-zoom-src="${cleanUrl}" />`;
  });

  // --- 3. LIMPIEZA DE ETIQUETAS HUÉRFANAS O MAL FORMATEADAS DE NEXUS ---
  html = html
    .replace(/\[\/?(?:b|i|u|s|size|color|font|center|left|right|heading|quote|spoiler|list|\*|code|line|hr|url|img)[^\]]*\]/gi, '');

  return html;
}

function detectMarkdown(text: string): boolean {
  return [/^#{1,6}\s+/m, /```[\s\S]*?```/, /\|.+\|.+\|/, /^\s*[-*+]\s+/m].some(p => p.test(text));
}

function mdToHtml(text: string): string {
  try {
    return marked.parse(text, { breaks: true, gfm: true, async: false }) as string;
  } catch {
    return text.replace(/\n/g, '<br />');
  }
}

function sanitizeHtml(html: string): string {
  return html
    .replace(/<(?:script|object|embed|applet)[\s\S]*?>[\s\S]*?<\/(?:script|object|embed|applet)>/gi, '')
    .replace(/<(?:script|object|embed|applet)[^>]*\/?>/gi, '')
    .replace(/\son[a-zA-Z]+\s*=\s*(?:'[^']*'|"[^"]*"|[^\s>]+)/gi, '')
    .replace(/href\s*=\s*['"]\s*javascript:[^'"]*['"]/gi, 'href="#blocked"');
}