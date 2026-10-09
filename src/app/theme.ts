// Legacy compatibility bridge re-exporting from src/app/theme/
export * from './theme/index';

import {
  applyTheme as engineApplyTheme,
  getActiveThemeId,
  updateLegacyThemeToggleBtn,
  findThemeById,
} from './theme/index';

import type { PmmTheme } from './theme/index';

export const THEME_KEY = 'pmm-theme';

export function getPreferredTheme(): 'dark' | 'light' {
  const active = getActiveThemeId();
  const theme = findThemeById(active);
  return theme.mode;
}

export function applyTheme(theme: PmmTheme | string): void {
  engineApplyTheme(theme);
}

export function updateThemeToggleBtn(): void {
  updateLegacyThemeToggleBtn();
}
