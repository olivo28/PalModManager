import { getState, updateState } from '../state';
import { setUiScale } from '../api/settings';
import { showToast } from '../ui/toast';
import { settingsDom } from '../framework';
import { t } from './i18n';

let _activeScale = 1.0;
let _saveDebounceTimer: ReturnType<typeof setTimeout> | null = null;

export function getUiScale(): number {
  return _activeScale;
}

export function applyUiScale(scale: number, syncInputs = true): void {
  const clamped = Math.min(1.5, Math.max(0.8, Math.round(scale * 100) / 100));
  _activeScale = clamped;

  (document.documentElement.style as any).zoom = clamped.toString();
  document.documentElement.style.setProperty('--ui-scale', clamped.toString());

  if (syncInputs) {
    const slider = settingsDom.elMaybe('settings-ui-scale');
    const badge = settingsDom.elMaybe('settings-ui-scale-value');
    if (slider && parseFloat(slider.value) !== clamped) {
      slider.value = clamped.toString();
    }
    if (badge) {
      badge.textContent = `${Math.round(clamped * 100)}%`;
    }
  }
}

export async function adjustUiScale(delta: number): Promise<void> {
  const newScale = Math.min(1.5, Math.max(0.8, Math.round((_activeScale + delta) * 100) / 100));
  if (newScale === _activeScale) {
    return;
  }

  applyUiScale(newScale, true);

  const percent = Math.round(newScale * 100);
  const toastMsg = t('toasts.zoom_level', { percent }) || `🔍 Zoom: ${percent}%`;
  showToast(toastMsg, 'info');

  if (_saveDebounceTimer) {
    clearTimeout(_saveDebounceTimer);
  }
  _saveDebounceTimer = setTimeout(async () => {
    try {
      const updated = await setUiScale(newScale);
      updateState({ currentSettings: updated });
    } catch (err) {
      console.error('Failed to persist uiScale:', err);
    }
  }, 400);
}

export async function resetUiScale(): Promise<void> {
  if (_activeScale === 1.0) {
    return;
  }
  applyUiScale(1.0, true);
  const toastMsg = t('toasts.zoom_level', { percent: 100 }) || `🔍 Zoom: 100%`;
  showToast(toastMsg, 'info');

  if (_saveDebounceTimer) {
    clearTimeout(_saveDebounceTimer);
  }
  try {
    const updated = await setUiScale(1.0);
    updateState({ currentSettings: updated });
  } catch (err) {
    console.error('Failed to persist uiScale reset:', err);
  }
}
