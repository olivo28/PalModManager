// Storage and management for custom PMM themes (.pmmtheme.json)
import type { PmmTheme } from './types';
import { OFFICIAL_DARK_THEME, OFFICIAL_LIGHT_THEME, OFFICIAL_THEMES } from './presets';

const STORAGE_CUSTOM_THEMES = 'pmm-custom-themes';
const STORAGE_ACTIVE_THEME = 'pmm-active-theme-id';
const LEGACY_THEME_KEY = 'pmm-theme';

export function getCustomThemes(): PmmTheme[] {
  try {
    const raw = localStorage.getItem(STORAGE_CUSTOM_THEMES);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch (err) {
    console.error('Failed to parse custom themes from localStorage:', err);
    return [];
  }
}

export function saveCustomThemes(themes: PmmTheme[]): void {
  try {
    localStorage.setItem(STORAGE_CUSTOM_THEMES, JSON.stringify(themes));
  } catch (err) {
    console.error('Failed to save custom themes to localStorage:', err);
  }
}

export function saveCustomTheme(theme: PmmTheme): void {
  const current = getCustomThemes();
  const index = current.findIndex(t => t.id === theme.id);
  if (index >= 0) {
    current[index] = theme;
  } else {
    current.push(theme);
  }
  saveCustomThemes(current);
}

export function deleteCustomTheme(id: string): void {
  const filtered = getCustomThemes().filter(t => t.id !== id);
  saveCustomThemes(filtered);
  if (getActiveThemeId() === id) {
    setActiveThemeId('dark');
  }
}

export function getActiveThemeId(): string {
  const stored = localStorage.getItem(STORAGE_ACTIVE_THEME);
  if (stored) return stored;

  const legacy = localStorage.getItem(LEGACY_THEME_KEY);
  if (legacy === 'light' || legacy === 'dark') return legacy;

  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

export function setActiveThemeId(id: string): void {
  localStorage.setItem(STORAGE_ACTIVE_THEME, id);
}

export function getAllThemes(): PmmTheme[] {
  return [...OFFICIAL_THEMES, ...getCustomThemes()];
}

export function findThemeById(id: string): PmmTheme {
  const all = getAllThemes();
  return all.find(t => t.id === id) || OFFICIAL_DARK_THEME;
}

export function exportThemeToJson(theme: PmmTheme): string {
  const payload: PmmTheme = {
    ...theme,
    isBuiltin: false,
  };
  return JSON.stringify(payload, null, 2);
}

export function downloadThemeFile(theme: PmmTheme): void {
  const json = exportThemeToJson(theme);
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  const safeName = theme.name.toLowerCase().replace(/[^a-z0-9_-]+/g, '_');
  a.href = url;
  a.download = `${safeName || 'custom'}.pmmtheme.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export function parseThemeFromJson(jsonStr: string): PmmTheme {
  const parsed = JSON.parse(jsonStr);
  if (!parsed || typeof parsed !== 'object') {
    throw new Error('Invalid theme format');
  }

  if (!parsed.colors || typeof parsed.colors !== 'object') {
    throw new Error('Theme is missing colors specification');
  }

  const id = (parsed.id || 'theme_' + Date.now()).replace(/[^a-zA-Z0-9_-]/g, '_');
  const name = parsed.name || 'Imported Theme';
  const mode = parsed.mode === 'light' ? 'light' : 'dark';

  const theme: PmmTheme = {
    version: 1,
    id,
    name,
    author: parsed.author || 'Community',
    isBuiltin: false,
    mode,
    colors: {
      accent: parsed.colors.accent || (mode === 'light' ? '#0284c7' : '#0078d4'),
      accentHover: parsed.colors.accentHover,
      accentDim: parsed.colors.accentDim,
      bgPrimary: parsed.colors.bgPrimary || (mode === 'light' ? '#f8fafc' : '#1e1e1e'),
      bgSecondary: parsed.colors.bgSecondary || (mode === 'light' ? '#f1f5f9' : '#252526'),
      bgTertiary: parsed.colors.bgTertiary,
      bgCard: parsed.colors.bgCard || (mode === 'light' ? '#ffffff' : '#2d2d2d'),
      border: parsed.colors.border || (mode === 'light' ? '#cbd5e1' : '#3c3c3c'),
      textPrimary: parsed.colors.textPrimary || (mode === 'light' ? '#0f172a' : '#cccccc'),
      textSecondary: parsed.colors.textSecondary,
      textMuted: parsed.colors.textMuted,
    },
    images: {
      appBackground: parsed.images?.appBackground || null,
      appBackgroundOpacity: typeof parsed.images?.appBackgroundOpacity === 'number' ? parsed.images.appBackgroundOpacity : 0.15,
      appBackgroundBlur: typeof parsed.images?.appBackgroundBlur === 'number' ? parsed.images.appBackgroundBlur : 0,
      appBackgroundFit: parsed.images?.appBackgroundFit === 'contain' ? 'contain' : 'cover',
      sidebarBackground: parsed.images?.sidebarBackground || null,
      sidebarBackgroundOpacity: typeof parsed.images?.sidebarBackgroundOpacity === 'number' ? parsed.images.sidebarBackgroundOpacity : 0.2,
      editorBackground: parsed.images?.editorBackground || null,
      editorBackgroundOpacity: typeof parsed.images?.editorBackgroundOpacity === 'number' ? parsed.images.editorBackgroundOpacity : 0.15,
      editorBackgroundPosition: parsed.images?.editorBackgroundPosition === 'center' ? 'center' : 'bottom-right',
    },
  };

  return theme;
}
