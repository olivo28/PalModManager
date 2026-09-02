import { mainDom } from '../framework';
import { t } from '../utils/i18n';

export const THEME_KEY = 'pmm-theme';

export function getPreferredTheme(): 'dark' | 'light' {
  const stored = localStorage.getItem(THEME_KEY);
  if (stored === 'dark' || stored === 'light') return stored;
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

export function applyTheme(theme: 'dark' | 'light'): void {
  document.documentElement.dataset.theme = theme;
  localStorage.setItem(THEME_KEY, theme);
  updateThemeToggleBtn();
}

export function updateThemeToggleBtn(): void {
  const btn = mainDom.elMaybe('theme-toggle-btn');
  if (!btn) return;
  const current = document.documentElement.dataset.theme || 'dark';
  btn.textContent = current === 'dark' ? t('settings.btn_theme_light') : t('settings.btn_theme_dark');
}
