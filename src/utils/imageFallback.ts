import { fetchAndCacheImage } from '../api';
import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * Universal fallback handler for any remote <img> that fails to load via WebView2.
 * Resolves the image via Rust's DoH (Cloudflare 1.1.1.1 / Google 8.8.8.8), caches to disk,
 * and updates the img.src with a local file URI.
 */
export async function handleUniversalImageFallback(
  img: HTMLImageElement,
  fallbackSrc?: string
): Promise<void> {
  img.onerror = null;
  const originalUrl = img.dataset.originalSrc || img.getAttribute('data-original-src') || img.src;
  
  if (!originalUrl || !originalUrl.startsWith('http')) {
    if (fallbackSrc) {
      img.src = fallbackSrc;
    } else {
      img.style.display = 'none';
      if (img.nextElementSibling) {
        (img.nextElementSibling as HTMLElement).style.display = 'flex';
      }
    }
    return;
  }

  try {
    const cachedPath = await fetchAndCacheImage(originalUrl);
    if (cachedPath) {
      img.src = convertFileSrc(cachedPath);
      return;
    }
  } catch (e) {
    console.error('[ImageFallback] DoH proxy fetch failed for:', originalUrl, e);
  }

  if (fallbackSrc) {
    img.src = fallbackSrc;
  } else {
    img.style.display = 'none';
    if (img.nextElementSibling) {
      (img.nextElementSibling as HTMLElement).style.display = 'flex';
    }
  }
}

if (typeof window !== 'undefined') {
  (window as any).handleUniversalImageFallback = handleUniversalImageFallback;
}
