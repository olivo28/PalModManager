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

export function scrollToTop(): void {
  const body = document.getElementById('discovery-body');
  if (body) body.scrollTop = 0;
}
