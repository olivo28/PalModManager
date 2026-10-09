// Theme Engine: Injects dynamic CSS variables and handles live theme updates
import type { PmmTheme, PmmImageCustomAdjust } from './types';
import { findThemeById, getActiveThemeId, setActiveThemeId } from './storage';
import { mainDom, bus } from '../../framework';
import { t } from '../../utils/i18n';

const OVERRIDE_PROPERTIES = [
  '--accent',
  '--accent-hover',
  '--accent-dim',
  '--bg-primary',
  '--bg-secondary',
  '--bg-tertiary',
  '--bg-card',
  '--bg-hover',
  '--border',
  '--border-light',
  '--text-primary',
  '--text-secondary',
  '--text-muted',
  '--selection-bg',
  '--selection-input-bg',
  '--success',
  '--success-dim',
  '--warning',
  '--warning-dim',
  '--danger',
  '--danger-dim',
  '--info',
  '--info-dim',
  '--type-pak',
  '--type-pak-dim',
  '--type-ue4ss',
  '--type-ue4ss-dim',
  '--type-palschema',
  '--type-palschema-dim',
  '--type-logicmods',
  '--type-logicmods-dim',
  '--pmm-app-bg-image',
  '--pmm-app-bg-opacity',
  '--pmm-app-bg-blur',
  '--pmm-app-bg-fit',
  '--pmm-app-bg-pos',
  '--pmm-app-bg-repeat',
  '--pmm-sidebar-bg-image',
  '--pmm-sidebar-bg-opacity',
  '--pmm-sidebar-bg-fit',
  '--pmm-sidebar-bg-pos',
  '--pmm-sidebar-bg-repeat',
  '--pmm-editor-bg-image',
  '--pmm-editor-bg-opacity',
  '--pmm-editor-bg-fit',
  '--pmm-editor-bg-pos',
  '--pmm-editor-bg-repeat',
];

export function applyTheme(themeOrId: PmmTheme | string): void {
  const theme = typeof themeOrId === 'string' ? findThemeById(themeOrId) : themeOrId;
  const root = document.documentElement;

  // Set mode attribute for default base rules
  root.dataset.theme = theme.mode;
  setActiveThemeId(theme.id);
  localStorage.setItem('pmm-theme', theme.mode); // Legacy fallback

  const style = root.style;

  if (theme.isBuiltin) {
    // Clear color overrides so base CSS variables apply cleanly
    for (const prop of OVERRIDE_PROPERTIES) {
      style.removeProperty(prop);
    }
  } else {
    // Inject custom colors
    const colors = theme.colors;
    style.setProperty('--accent', colors.accent);
    style.setProperty('--accent-hover', colors.accentHover || colors.accent);
    style.setProperty('--accent-dim', colors.accentDim || hexToRgba(colors.accent, 0.15));

    style.setProperty('--bg-primary', colors.bgPrimary);
    style.setProperty('--bg-secondary', colors.bgSecondary);
    if (colors.bgTertiary) style.setProperty('--bg-tertiary', colors.bgTertiary);
    style.setProperty('--bg-card', colors.bgCard);
    style.setProperty('--border', colors.border);

    style.setProperty('--text-primary', colors.textPrimary);
    if (colors.textSecondary) style.setProperty('--text-secondary', colors.textSecondary);
    if (colors.textMuted) style.setProperty('--text-muted', colors.textMuted);

    // Semantic status colors
    if (colors.success) {
      style.setProperty('--success', colors.success);
      style.setProperty('--success-dim', hexToRgba(colors.success, 0.15));
    }
    if (colors.warning) {
      style.setProperty('--warning', colors.warning);
      style.setProperty('--warning-dim', hexToRgba(colors.warning, 0.15));
    }
    if (colors.danger) {
      style.setProperty('--danger', colors.danger);
      style.setProperty('--danger-dim', hexToRgba(colors.danger, 0.15));
    }
    if (colors.info) {
      style.setProperty('--info', colors.info);
      style.setProperty('--info-dim', hexToRgba(colors.info, 0.15));
    }

    // Mod type badge colors
    if (colors.typePak) {
      style.setProperty('--type-pak', colors.typePak);
      style.setProperty('--type-pak-dim', hexToRgba(colors.typePak, 0.15));
    }
    if (colors.typeUe4ss) {
      style.setProperty('--type-ue4ss', colors.typeUe4ss);
      style.setProperty('--type-ue4ss-dim', hexToRgba(colors.typeUe4ss, 0.15));
    }
    if (colors.typePalschema) {
      style.setProperty('--type-palschema', colors.typePalschema);
      style.setProperty('--type-palschema-dim', hexToRgba(colors.typePalschema, 0.15));
    }
    if (colors.typeLogicmods) {
      style.setProperty('--type-logicmods', colors.typeLogicmods);
      style.setProperty('--type-logicmods-dim', hexToRgba(colors.typeLogicmods, 0.15));
    }

    // Auto-derived interactive states
    style.setProperty('--bg-hover', colors.bgTertiary || hexToRgba(colors.accent, 0.12));
    style.setProperty('--border-light', colors.border);
    style.setProperty('--selection-bg', hexToRgba(colors.accent, 0.35));
    style.setProperty('--selection-input-bg', colors.accent);
  }

  // Handle custom background images
  if (theme.images?.appBackground) {
    const isRepeat = theme.images.appBackgroundFit === 'repeat';
    const fitVal = resolveImageFit(theme.images.appBackgroundFit, theme.images.appBackgroundAdjust, 'cover');
    const posVal = resolveImagePos(theme.images.appBackgroundPosition, theme.images.appBackgroundAdjust, 'center center');
    style.setProperty('--pmm-app-bg-image', `url("${theme.images.appBackground}")`);
    style.setProperty('--pmm-app-bg-opacity', String(theme.images.appBackgroundOpacity ?? 0.15));
    style.setProperty('--pmm-app-bg-blur', `${theme.images.appBackgroundBlur ?? 0}px`);
    style.setProperty('--pmm-app-bg-fit', isRepeat ? 'auto' : fitVal);
    style.setProperty('--pmm-app-bg-repeat', isRepeat ? 'repeat' : (theme.images.appBackgroundRepeat ?? 'no-repeat'));
    style.setProperty('--pmm-app-bg-pos', posVal);
  } else {
    style.removeProperty('--pmm-app-bg-image');
    style.removeProperty('--pmm-app-bg-opacity');
    style.removeProperty('--pmm-app-bg-blur');
    style.removeProperty('--pmm-app-bg-fit');
    style.removeProperty('--pmm-app-bg-repeat');
    style.removeProperty('--pmm-app-bg-pos');
  }

  if (theme.images?.sidebarBackground) {
    const isRepeat = theme.images.sidebarBackgroundFit === 'repeat';
    const fitVal = resolveImageFit(theme.images.sidebarBackgroundFit, theme.images.sidebarBackgroundAdjust, 'cover');
    const posVal = resolveImagePos(theme.images.sidebarBackgroundPosition, theme.images.sidebarBackgroundAdjust, 'center bottom');
    style.setProperty('--pmm-sidebar-bg-image', `url("${theme.images.sidebarBackground}")`);
    style.setProperty('--pmm-sidebar-bg-opacity', String(theme.images.sidebarBackgroundOpacity ?? 0.2));
    style.setProperty('--pmm-sidebar-bg-fit', isRepeat ? 'auto' : fitVal);
    style.setProperty('--pmm-sidebar-bg-repeat', isRepeat ? 'repeat' : 'no-repeat');
    style.setProperty('--pmm-sidebar-bg-pos', posVal);
  } else {
    style.removeProperty('--pmm-sidebar-bg-image');
    style.removeProperty('--pmm-sidebar-bg-opacity');
    style.removeProperty('--pmm-sidebar-bg-fit');
    style.removeProperty('--pmm-sidebar-bg-repeat');
    style.removeProperty('--pmm-sidebar-bg-pos');
  }

  if (theme.images?.editorBackground) {
    const isRepeat = theme.images.editorBackgroundFit === 'repeat';
    const fitVal = resolveImageFit(theme.images.editorBackgroundFit, theme.images.editorBackgroundAdjust, 'contain');
    const posVal = resolveImagePos(theme.images.editorBackgroundPosition, theme.images.editorBackgroundAdjust, 'right bottom');
    style.setProperty('--pmm-editor-bg-image', `url("${theme.images.editorBackground}")`);
    style.setProperty('--pmm-editor-bg-opacity', String(theme.images.editorBackgroundOpacity ?? 0.15));
    style.setProperty('--pmm-editor-bg-fit', isRepeat ? 'auto' : fitVal);
    style.setProperty('--pmm-editor-bg-repeat', isRepeat ? 'repeat' : 'no-repeat');
    style.setProperty('--pmm-editor-bg-pos', posVal);
  } else {
    style.removeProperty('--pmm-editor-bg-image');
    style.removeProperty('--pmm-editor-bg-opacity');
    style.removeProperty('--pmm-editor-bg-fit');
    style.removeProperty('--pmm-editor-bg-repeat');
    style.removeProperty('--pmm-editor-bg-pos');
  }

  updateLegacyThemeToggleBtn();
  bus.emit('theme:changed', { theme });
}

export function initThemeEngine(): void {
  const activeId = getActiveThemeId();
  const theme = findThemeById(activeId);
  applyTheme(theme);
}

export function updateLegacyThemeToggleBtn(): void {
  const btn = mainDom.elMaybe('theme-toggle-btn');
  if (!btn) return;
  const currentMode = document.documentElement.dataset.theme || 'dark';
  btn.textContent = currentMode === 'dark' ? t('settings.btn_theme_light') : t('settings.btn_theme_dark');
}

function hexToRgba(hex: string, alpha: number): string {
  const clean = hex.replace('#', '');
  if (clean.length === 3) {
    const r = parseInt(clean[0] + clean[0], 16);
    const g = parseInt(clean[1] + clean[1], 16);
    const b = parseInt(clean[2] + clean[2], 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  } else if (clean.length === 6) {
    const r = parseInt(clean.substring(0, 2), 16);
    const g = parseInt(clean.substring(2, 4), 16);
    const b = parseInt(clean.substring(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }
  return `rgba(0, 120, 212, ${alpha})`;
}

function resolveImageFit(fit?: string, adjust?: PmmImageCustomAdjust, defaultFit = 'cover'): string {
  if (fit && fit.includes('px')) {
    return fit;
  }
  if (adjust) {
    if (adjust.stretchX != null && adjust.stretchY != null && (adjust.stretchX !== 100 || adjust.stretchY !== 100)) {
      return `${adjust.stretchX}% ${adjust.stretchY}%`;
    }
    if (adjust.scale != null) {
      return `${adjust.scale}% auto`;
    }
  }
  if (fit === 'repeat') return 'auto';
  return fit || defaultFit;
}

function resolveImagePos(pos?: string, adjust?: PmmImageCustomAdjust, defaultPos = 'center center'): string {
  if (pos && pos.includes('px')) {
    return pos;
  }
  if (adjust && adjust.posX != null && adjust.posY != null) {
    return `${adjust.posX}% ${adjust.posY}%`;
  }
  return pos || defaultPos;
}
