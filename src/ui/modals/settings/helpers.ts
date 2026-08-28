import { t } from '../../../utils/i18n';
import { _tempCustomDataPath } from './state';

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
