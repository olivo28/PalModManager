import { fetchAndCacheImage } from '../../api';
import { convertFileSrc } from '@tauri-apps/api/core';
import logoUrl from '../../assets/logo.png';

export async function handleDiscoveryImageError(img: HTMLImageElement, originalUrl: string): Promise<void> {
  img.onerror = null;
  if (!originalUrl || originalUrl === logoUrl || !originalUrl.startsWith('http')) {
    img.src = logoUrl;
    return;
  }
  try {
    const cachedPath = await fetchAndCacheImage(originalUrl);
    if (cachedPath) {
      img.src = convertFileSrc(cachedPath);
      return;
    }
  } catch (e) {
    console.error('DoH proxy fetch failed:', e);
  }
  img.src = logoUrl;
}

if (typeof window !== 'undefined') {
  (window as any).handleDiscoveryImageError = handleDiscoveryImageError;
}

export function formatDateDisplay(dateStr: string): string {
  if (!dateStr) return '';
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return '';
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  } catch {
    return '';
  }
}

import { discoveryDom } from '../../framework';

export function scrollToTop(): void {
  const body = discoveryDom.elMaybe('discovery-body');
  if (body) body.scrollTop = 0;
}

import type { DiscoveryFileItem } from '../../types';

/**
 * Intelligently derives a clean, non-redundant title for a mod installed from Discovery.
 * Avoids repetitive strings (e.g. "Mod - Mod 1.0.0") and cleans hardcoded versions.
 */
export function deriveDiscoveryModTitle(modName: string, file: DiscoveryFileItem): string {
  const cleanMod = (modName || '').trim();
  if (!file || !file.name) return cleanMod;

  // Category 1 is 'MAIN' in Nexus Mods file categorization
  const isMainFile = file.isPrimary || file.categoryId === 1 || file.categoryName?.toUpperCase() === 'MAIN';

  // Normalize by stripping non-alphanumeric characters for similarity check
  const normalize = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

  // Strip file extensions and common version tokens from filename
  let cleanFileName = file.name
    .replace(/\.(zip|rar|7z|pak)$/i, '')
    .trim();

  // Strip trailing version pattern e.g. "4.0.1", "v1.2", "- 1.0.0", "_v2.3"
  cleanFileName = cleanFileName
    .replace(/[-_v\s]*v?\d+(\.\d+)+[-_a-z0-9]*$/i, '')
    .trim();

  const normMod = normalize(cleanMod);
  const normFile = normalize(cleanFileName);

  // If it is the main file or if the cleaned file name matches the mod name, use the authentic mod title
  if (isMainFile || !normFile || normMod === normFile || normFile.startsWith(normMod) && normFile.length - normMod.length < 4) {
    return cleanMod;
  }

  // If the file name already starts with the mod name (e.g. "Integrated Storage Reworked Dark Icons")
  // isolate only the distinctive addon suffix
  const lowerMod = cleanMod.toLowerCase();
  const lowerFile = cleanFileName.toLowerCase();
  if (lowerFile.startsWith(lowerMod)) {
    const suffix = cleanFileName.substring(cleanMod.length).replace(/^[-_\s]+/, '').trim();
    if (suffix) {
      return `${cleanMod} - ${suffix}`;
    }
    return cleanMod;
  }

  return `${cleanMod} - ${cleanFileName}`;
}

