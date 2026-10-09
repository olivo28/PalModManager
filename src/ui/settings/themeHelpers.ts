// Helper utilities for Theme Manager: image compression and UI widgets
import { t } from '../../utils/i18n';

/**
 * Resizes and compresses an image file to a lightweight WebP/JPEG DataURL
 * to avoid exceeding browser localStorage quotas (~5MB).
 */
export async function compressImageToDataUrl(
  file: File,
  maxWidth = 1920,
  maxHeight = 1080,
  quality = 0.85
): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Failed to read image file'));
    reader.onload = (e) => {
      const img = new Image();
      img.onerror = () => reject(new Error('Failed to decode image'));
      img.onload = () => {
        let width = img.naturalWidth || img.width;
        let height = img.naturalHeight || img.height;

        // Calculate proportional scale if dimensions exceed max bounds
        if (width > maxWidth || height > maxHeight) {
          const ratio = Math.min(maxWidth / width, maxHeight / height);
          width = Math.round(width * ratio);
          height = Math.round(height * ratio);
        }

        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;

        const ctx = canvas.getContext('2d');
        if (!ctx) {
          // Fallback to original DataURL if 2D context fails
          return resolve(e.target?.result as string);
        }

        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = 'high';
        ctx.drawImage(img, 0, 0, width, height);

        // Export as WebP with JPEG fallback
        try {
          const webpUrl = canvas.toDataURL('image/webp', quality);
          if (webpUrl.startsWith('data:image/webp')) {
            return resolve(webpUrl);
          }
        } catch {}

        resolve(canvas.toDataURL('image/jpeg', quality));
      };
      img.src = e.target?.result as string;
    };
    reader.readAsDataURL(file);
  });
}

export function updateImageThumb(imgId: string, emptyId: string, dataUrl?: string | null): void {
  const img = document.getElementById(imgId) as HTMLImageElement | null;
  const empty = document.getElementById(emptyId);
  if (dataUrl) {
    if (img) {
      img.src = dataUrl;
      img.style.display = 'block';
    }
    if (empty) empty.style.display = 'none';
  } else {
    if (img) {
      img.src = '';
      img.style.display = 'none';
    }
    if (empty) empty.style.display = 'block';
  }
}

export function updateImageBadge(badgeId: string, hasImage: boolean): void {
  const badge = document.getElementById(badgeId);
  if (!badge) return;
  if (hasImage) {
    badge.textContent = t('theme.status_image_loaded') || 'Image loaded';
    badge.style.background = 'var(--success-dim)';
    badge.style.color = 'var(--success)';
    badge.style.borderColor = 'var(--success-dim)';
  } else {
    badge.textContent = t('theme.status_no_image') || 'No image';
    badge.style.background = 'var(--bg-tertiary)';
    badge.style.color = 'var(--text-muted)';
    badge.style.borderColor = 'var(--border)';
  }
}

export function expandHex(hex: string): string {
  return '#' + hex[1] + hex[1] + hex[2] + hex[2] + hex[3] + hex[3];
}

export function setColorField(pickerId: string, hexId: string, value: string): void {
  const picker = document.getElementById(pickerId) as HTMLInputElement | null;
  const hex = document.getElementById(hexId) as HTMLInputElement | null;
  if (picker && value.startsWith('#') && (value.length === 7 || value.length === 4)) {
    picker.value = value.length === 4 ? expandHex(value) : value;
  }
  if (hex) hex.value = value;
}

export function setSliderField(sliderId: string, labelId: string, val: number, unit: string, mult: number): void {
  const slider = document.getElementById(sliderId) as HTMLInputElement | null;
  const label = document.getElementById(labelId);
  if (slider) slider.value = String(val);
  if (label) label.textContent = `${Math.round(val * mult)}${unit}`;
}
export function updateAdjustButtonVisibility(btnId: string, hasImage: boolean): void {
  const btn = document.getElementById(btnId);
  if (btn) btn.style.display = hasImage ? 'inline-block' : 'none';
}

export function setupCategoryTabs(): void {
  const catStructureBtn = document.getElementById('theme-cat-structure-btn');
  const catStatusBtn = document.getElementById('theme-cat-status-btn');
  const catBadgesBtn = document.getElementById('theme-cat-badges-btn');

  const groupStructure = document.getElementById('theme-group-structure');
  const groupStatus = document.getElementById('theme-group-status');
  const groupBadges = document.getElementById('theme-group-badges');

  const tabs = [
    { btn: catStructureBtn, group: groupStructure },
    { btn: catStatusBtn, group: groupStatus },
    { btn: catBadgesBtn, group: groupBadges },
  ];

  tabs.forEach(({ btn, group }) => {
    btn?.addEventListener('click', () => {
      tabs.forEach((t) => {
        t.btn?.classList.remove('active');
        if (t.group) t.group.style.display = 'none';
      });
      btn.classList.add('active');
      if (group) group.style.display = 'grid';
    });
  });
}

export function bindLiveColor(pickerId: string, hexId: string, cssVar: string, onUpdate: (val: string) => void): void {
  const picker = document.getElementById(pickerId) as HTMLInputElement | null;
  const hex = document.getElementById(hexId) as HTMLInputElement | null;

  picker?.addEventListener('input', () => {
    const val = picker.value;
    if (hex) hex.value = val;
    document.documentElement.style.setProperty(cssVar, val);
    onUpdate(val);
  });

  hex?.addEventListener('change', () => {
    let val = hex.value.trim();
    if (!val.startsWith('#')) val = '#' + val;
    if (val.length === 4 || val.length === 7) {
      if (picker) picker.value = val.length === 4 ? expandHex(val) : val;
      document.documentElement.style.setProperty(cssVar, val);
      onUpdate(val);
    }
  });
}

export function bindLiveSlider(
  sliderId: string,
  labelId: string,
  cssVar: string,
  unit: string,
  mult: number,
  onUpdate: (val: number) => void
): void {
  const slider = document.getElementById(sliderId) as HTMLInputElement | null;
  const label = document.getElementById(labelId);

  slider?.addEventListener('input', () => {
    const val = parseFloat(slider.value);
    if (label) label.textContent = `${Math.round(val * mult)}${unit}`;
    const cssVal = unit === 'px' ? `${val}px` : String(val);
    document.documentElement.style.setProperty(cssVar, cssVal);
    onUpdate(val);
  });
}
