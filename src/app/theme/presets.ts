// Official Base Theme Presets (Dark & Light)
import type { PmmTheme } from './types';

export const OFFICIAL_DARK_THEME: PmmTheme = {
  version: 1,
  id: 'dark',
  name: 'Dark',
  isBuiltin: true,
  mode: 'dark',
  colors: {
    accent: '#0078d4',
    accentHover: '#1a8ae8',
    accentDim: 'rgba(0, 120, 212, 0.15)',
    bgPrimary: '#1e1e1e',
    bgSecondary: '#252526',
    bgTertiary: '#333333',
    bgCard: '#2d2d2d',
    border: '#3c3c3c',
    textPrimary: '#cccccc',
    textSecondary: '#969696',
    textMuted: '#6e6e6e',
  },
  images: {
    appBackground: null,
    appBackgroundOpacity: 0.15,
    appBackgroundBlur: 0,
    appBackgroundFit: 'cover',
    sidebarBackground: null,
    sidebarBackgroundOpacity: 0.2,
    editorBackground: null,
    editorBackgroundOpacity: 0.15,
    editorBackgroundPosition: 'bottom-right',
  },
};

export const OFFICIAL_LIGHT_THEME: PmmTheme = {
  version: 1,
  id: 'light',
  name: 'Light',
  isBuiltin: true,
  mode: 'light',
  colors: {
    accent: '#0284c7',
    accentHover: '#0369a1',
    accentDim: 'rgba(2, 132, 199, 0.12)',
    bgPrimary: '#f8fafc',
    bgSecondary: '#f1f5f9',
    bgTertiary: '#e2e8f0',
    bgCard: '#ffffff',
    border: '#cbd5e1',
    textPrimary: '#0f172a',
    textSecondary: '#334155',
    textMuted: '#64748b',
  },
  images: {
    appBackground: null,
    appBackgroundOpacity: 0.15,
    appBackgroundBlur: 0,
    appBackgroundFit: 'cover',
    sidebarBackground: null,
    sidebarBackgroundOpacity: 0.2,
    editorBackground: null,
    editorBackgroundOpacity: 0.15,
    editorBackgroundPosition: 'bottom-right',
  },
};

export const OFFICIAL_THEMES: PmmTheme[] = [
  OFFICIAL_DARK_THEME,
  OFFICIAL_LIGHT_THEME,
];
