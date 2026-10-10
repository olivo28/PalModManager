/**
 * PalModManager Unified Vector Icon System
 * Zero-dependency, theme-reactive inline SVG factory.
 */

import { ICON_PATHS, IconName } from './paths';

export interface IconOptions {
  className?: string;
  size?: number | string;
  strokeWidth?: number;
  title?: string;
  ariaHidden?: boolean;
}

/**
 * Returns an HTML string containing the rendered SVG icon.
 */
export function getIconSvg(name: IconName, options: string | IconOptions = {}): string {
  const opts: IconOptions = typeof options === 'string' ? { className: options } : options;
  const def = ICON_PATHS[name];
  if (!def) {
    console.warn(`[PMM-Icons] Icon "${String(name)}" not found in registry.`);
    return '';
  }

  const viewBox = def.viewBox || '0 0 24 24';
  const sizeAttr = opts.size
    ? `width="${opts.size}" height="${opts.size}"`
    : '';
  const strokeWidth = opts.strokeWidth || 2;
  const classes = ['pmm-icon', def.mode === 'fill' ? 'pmm-icon-fill' : '', opts.className || '']
    .filter(Boolean)
    .join(' ');

  const titleTag = opts.title ? `<title>${opts.title}</title>` : '';
  const aria = opts.ariaHidden !== false ? 'aria-hidden="true"' : '';

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}" class="${classes}" ${sizeAttr} ${aria} stroke-width="${strokeWidth}">${titleTag}${def.body}</svg>`;
}

/**
 * Creates and returns a DOM SVGSVGElement for the given icon name.
 */
export function createIcon(name: IconName, options: string | IconOptions = {}): SVGSVGElement {
  const html = getIconSvg(name, options);
  const template = document.createElement('template');
  template.innerHTML = html.trim();
  return template.content.firstElementChild as SVGSVGElement;
}

export { ICON_PATHS };
export type { IconName };
