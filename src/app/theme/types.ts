// Type definitions for PMM Themes and Custom Visual Styling

export interface PmmThemeColors {
  accent: string;
  accentHover?: string;
  accentDim?: string;
  bgPrimary: string;
  bgSecondary: string;
  bgTertiary?: string;
  bgCard: string;
  border: string;
  textPrimary: string;
  textSecondary?: string;
  textMuted?: string;

  // Semantic & Status Colors
  success?: string;
  warning?: string;
  danger?: string;
  info?: string;

  // Mod Type Badges Colors
  typePak?: string;
  typeUe4ss?: string;
  typePalschema?: string;
  typeLogicmods?: string;
}

export interface PmmImageCustomAdjust {
  posX: number;     // 0% - 100% (default 50)
  posY: number;     // 0% - 100% (default 50)
  scale: number;    // Zoom scale percentage (e.g. 100 = 100%, 150 = 150%)
  stretchX?: number; // Optional independent stretch width %
  stretchY?: number; // Optional independent stretch height %
}

export interface PmmThemeImages {
  appBackground?: string | null;
  appBackgroundOpacity?: number; // 0.05 - 1.0 (default: 0.15)
  appBackgroundBlur?: number;    // 0 - 20 (px)
  appBackgroundFit?: 'cover' | 'contain' | 'repeat' | '100% 100%' | string;
  appBackgroundPosition?: string; // e.g. '50% 50%', 'center center'
  appBackgroundRepeat?: string;   // 'no-repeat' | 'repeat'
  appBackgroundAdjust?: PmmImageCustomAdjust;
  sidebarBackground?: string | null;
  sidebarBackgroundOpacity?: number; // 0.05 - 1.0 (default: 0.2)
  sidebarBackgroundFit?: string;
  sidebarBackgroundPosition?: string;
  sidebarBackgroundAdjust?: PmmImageCustomAdjust;
  editorBackground?: string | null;
  editorBackgroundOpacity?: number;  // 0.05 - 1.0 (default: 0.15)
  editorBackgroundPosition?: 'bottom-right' | 'center' | string;
  editorBackgroundFit?: string;
  editorBackgroundAdjust?: PmmImageCustomAdjust;
}

export interface PmmTheme {
  version: 1;
  id: string;
  name: string;
  author?: string;
  isBuiltin?: boolean;
  mode: 'dark' | 'light';
  colors: PmmThemeColors;
  images?: PmmThemeImages;
}
