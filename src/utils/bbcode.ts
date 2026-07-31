import { escapeHtml } from './helpers';
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
    // Colapsar espacios verticales exagerados (más de 2 saltos -> 2 saltos)
    .replace(/\n{3,}/g, '\n\n');

  // 2. PROCESAR BBCODE (Antes que Markdown)
  content = bbcodeToHtml(content);

  // 3. DETECTAR Y PROCESAR MARKDOWN
  // Solo aplicamos Markdown si detectamos patrones claros (tablas, títulos #, etc)
  if (detectMarkdown(content)) {
    content = mdToHtml(content);
  } else {
    // Si no es MD, convertimos los \n sobrantes en <br />, 
    // pero evitamos poner <br /> alrededor de etiquetas de bloque (div, blockquote, etc)
    content = content.replace(/\n/g, '<br />');
    content = content.replace(/(<(?:div|blockquote|ul|ol|li|h[1-6]|hr|pre|details|section|table)[^>]*>)<br \/>/gi, '$1');
    content = content.replace(/<br \/>(<\/(?:div|blockquote|ul|ol|li|h[1-6]|hr|pre|details|section|table)>)/gi, '$1');
  }

  // 4. SANITIZACIÓN FINAL
  return sanitizeHtml(content);
}

function bbcodeToHtml(html: string): string {
  // Nota: No escapamos el HTML al principio porque el JSON ya trae etiquetas <br /> 
  // que queremos procesar. La seguridad se maneja en el paso de sanitización final.

  // --- ETIQUETAS DE BLOQUE ---

  // [heading] -> Usado en PlayerStatLimit100
  html = html.replace(/\[heading\]([\s\S]*?)\[\/heading\]/gi, '<h3 class="text-xl font-bold my-2">$1</h3>');

  // [quote] -> Usado en AntiPhat
  html = html.replace(/\[quote\]([\s\S]*?)\[\/quote\]/gi, '<blockquote class="border-l-4 border-neutral-500 pl-4 my-2 italic text-neutral-300">$1</blockquote>');

  // [spoiler] -> Usado en AntiPhat
  html = html.replace(/\[spoiler\]([\s\S]*?)\[\/spoiler\]/gi,
    '<details class="bg-neutral-800 p-2 rounded my-2"><summary class="cursor-pointer font-bold">Spoiler (Click to show)</summary><div class="mt-2">$1</div></details>');

  // [center], [left], [right]
  html = html.replace(/\[center\]([\s\S]*?)\[\/center\]/gi, '<div style="text-align:center">$1</div>');
  html = html.replace(/\[left\]([\s\S]*?)\[\/left\]/gi, '<div style="text-align:left">$1</div>');
  html = html.replace(/\[right\]([\s\S]*?)\[\/right\]/gi, '<div style="text-align:right">$1</div>');

  // [code] -> Usado en HumanMercyBypass
  html = html.replace(/\[code\]([\s\S]*?)\[\/code\]/gi, '<pre class="bg-black/50 p-3 rounded font-mono text-sm my-2 overflow-x-auto">$1</pre>');

  // [list] soportando el formato [*] de Nexus
  html = html.replace(/\[list(?:=1)?\]([\s\S]*?)\[\/list\]/gi, (match, inner) => {
    const tag = match.toLowerCase().includes('=1') ? 'ol' : 'ul';
    const items = inner.split(/\[\*\]/).filter((i: string) => i.trim());
    const listContent = items.map((i: string) => `<li>${i.replace(/\[\/\*\]/g, '').trim()}</li>`).join('');
    return `<${tag} class="list-disc ml-6 my-2">${listContent}</${tag}>`;
  });

  // [font] -> Simplemente eliminamos la etiqueta pero dejamos el texto (ej: RandomDungeonBossMarker)
  html = html.replace(/\[font=[^\]]+\]([\s\S]*?)\[\/font\]/gi, '$1');

  // [line] o [hr]
  html = html.replace(/\[line\]|\[hr\]/gi, '<hr class="border-neutral-700 my-4" />');

  // --- ETIQUETAS INLINE ---

  // [b], [i], [u], [s]
  html = html.replace(/\[b\]([\s\S]*?)\[\/b\]/gi, '<strong>$1</strong>');
  html = html.replace(/\[i\]([\s\S]*?)\[\/i\]/gi, '<em>$1</em>');
  html = html.replace(/\[u\]([\s\S]*?)\[\/u\]/gi, '<u>$1</u>');
  html = html.replace(/\[s\]([\s\S]*?)\[\/s\]/gi, '<del>$1</del>');

  // [url]
  html = html.replace(/\[url=([^\]]+)\]([\s\S]*?)\[\/url\]/gi, '<a href="$1" target="_blank" class="text-blue-400 hover:underline">$2</a>');
  html = html.replace(/\[url\]([\s\S]*?)\[\/url\]/gi, '<a href="$1" target="_blank" class="text-blue-400 hover:underline">$1</a>');

  // [img] con soporte para width (AntiPhat usa [img width=299])
  html = html.replace(/\[img(?:[^\]]*width=([0-9]+))?\]([\s\S]*?)\[\/img\]/gi, (_, width, url) => {
    const style = width ? `width:${width}px;` : 'max-width:100%;';
    return `<img src="${url.trim()}" style="${style}" class="inline-block rounded" loading="lazy" />`;
  });

  // [size] (Nexus 1-7)
  html = html.replace(/\[size=([0-9]+)\]([\s\S]*?)\[\/size\]/gi, (_, size, inner) => {
    const sizes: any = { '1': '0.75rem', '2': '0.85rem', '3': '1rem', '4': '1.25rem', '5': '1.5rem', '6': '2rem', '7': '3rem' };
    return `<span style="font-size: ${sizes[size] || '1rem'}">${inner}</span>`;
  });

  // [color]
  html = html.replace(/\[color=([^\]]+)\]([\s\S]*?)\[\/color\]/gi, '<span style="color:$1">$2</span>');

  return html;
}

function detectMarkdown(text: string): boolean {
  // Patrones de Markdown que no suelen estar en BBCode (tablas, títulos #, bloques ```)
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
    .replace(/<script[\s\S]*?>[\s\S]*?<\/script>/gi, '') // Elimina scripts
    .replace(/\sonevent\w+\s*=\s*['"][^'"]*['"]/gi, '')  // Elimina onmouseover, onclick...
    .replace(/href\s*=\s*['"]javascript:[^'"]*['"]/gi, 'href="#blocked"');
}