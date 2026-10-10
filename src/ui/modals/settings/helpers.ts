import { t } from '../../../utils/i18n';
import { escapeHtml } from '../../../utils/helpers';
import { getIconSvg } from '../../icons/mod';
import type { IconName } from '../../icons/paths';
import { _tempCustomDataPath } from './state';

export function setBadgeWithIcon(badge: HTMLElement, iconName: IconName, label: string): void {
  badge.innerHTML = `<span style="display:inline-flex;align-items:center;gap:4px;">${getIconSvg(iconName, 'pmm-icon-xs')}<span>${escapeHtml(label)}</span></span>`;
}

export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

export function revertDataPathSelect(select: HTMLSelectElement, display: HTMLElement | null): void {
  if (!_tempCustomDataPath) {
    select.value = 'default';
    if (display) display.style.display = 'none';
  } else if (_tempCustomDataPath === '__portable__') {
    select.value = 'portable';
    if (display) display.style.display = 'none';
  } else {
    select.value = 'custom';
    if (display) {
      display.style.display = 'block';
      display.textContent = t('settings.custom_folder_display', { path: _tempCustomDataPath });
    }
  }
}
