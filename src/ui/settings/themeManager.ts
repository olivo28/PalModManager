// Theme Manager UI Controller for Settings Modal
import {
  getAllThemes,
  findThemeById,
  getActiveThemeId,
  saveCustomTheme,
  deleteCustomTheme,
  downloadThemeFile,
  parseThemeFromJson,
  applyTheme,
  initThemeEngine,
} from '../../app/theme/index';
import type { PmmTheme, PmmThemeColors, PmmThemeImages, PmmImageCustomAdjust } from '../../app/theme/types';
import { settingsDom } from '../../framework';
import { showToast } from '../toast';
import { showConfirm, showPrompt } from '../confirm';
import { t } from '../../utils/i18n';
import {
  compressImageToDataUrl,
  updateImageThumb,
  updateImageBadge,
  expandHex,
  setColorField,
  setSliderField,
  setupCategoryTabs,
  updateAdjustButtonVisibility,
  bindLiveColor,
  bindLiveSlider,
} from './themeHelpers';
import { openThemeImageAdjustModal } from './themeImageAdjuster';

let _activeDraft: PmmTheme | null = null;

export function setupThemeSettingsUI(): void {
  renderThemeSelectOptions();
  attachThemeEventListeners();
}

export function renderThemeSelectOptions(): void {
  const select = document.getElementById('settings-theme-select') as HTMLSelectElement | null;
  if (!select) return;

  const themes = getAllThemes();
  const activeId = getActiveThemeId();

  select.innerHTML = '';

  const builtinGroup = document.createElement('optgroup');
  builtinGroup.label = t('theme.group_official') || 'Official Themes';

  const customGroup = document.createElement('optgroup');
  customGroup.label = t('theme.group_custom') || 'Custom Themes';

  let hasCustom = false;

  for (const theme of themes) {
    const opt = document.createElement('option');
    opt.value = theme.id;
    opt.textContent = theme.name;
    if (theme.id === activeId) opt.selected = true;

    if (theme.isBuiltin) {
      builtinGroup.appendChild(opt);
    } else {
      customGroup.appendChild(opt);
      hasCustom = true;
    }
  }

  select.appendChild(builtinGroup);
  if (hasCustom) {
    select.appendChild(customGroup);
  }

  updateDeleteButtonVisibility(activeId);
  syncDraftWithActiveTheme();
}

function updateDeleteButtonVisibility(activeId: string): void {
  const deleteBtn = document.getElementById('theme-delete-btn') as HTMLButtonElement | null;
  if (!deleteBtn) return;
  const current = findThemeById(activeId);
  deleteBtn.style.display = current.isBuiltin ? 'none' : 'inline-block';
}

function syncDraftWithActiveTheme(): void {
  const activeId = getActiveThemeId();
  const theme = findThemeById(activeId);
  _activeDraft = JSON.parse(JSON.stringify(theme));
  populateCustomizerFields(_activeDraft!);
}

function populateCustomizerFields(theme: PmmTheme): void {
  const c = theme.colors;
  // Group 1: Structure & Surfaces
  setColorField('theme-color-accent', 'theme-hex-accent', c.accent);
  setColorField('theme-color-bg-primary', 'theme-hex-bg-primary', c.bgPrimary);
  setColorField('theme-color-bg-secondary', 'theme-hex-bg-secondary', c.bgSecondary || '#252526');
  setColorField('theme-color-bg-tertiary', 'theme-hex-bg-tertiary', c.bgTertiary || '#181818');
  setColorField('theme-color-bg-card', 'theme-hex-bg-card', c.bgCard);
  setColorField('theme-color-border', 'theme-hex-border', c.border);
  setColorField('theme-color-text-primary', 'theme-hex-text-primary', c.textPrimary);
  setColorField('theme-color-text-muted', 'theme-hex-text-muted', c.textMuted || '#888888');

  // Group 2: Status & Health Colors
  const defSuccess = theme.mode === 'light' ? '#16a34a' : '#22c55e';
  const defWarning = theme.mode === 'light' ? '#d97706' : '#ffaa00';
  const defDanger = theme.mode === 'light' ? '#dc2626' : '#ff5f56';
  const defInfo = theme.mode === 'light' ? '#0284c7' : '#00bcff';
  setColorField('theme-color-success', 'theme-hex-success', c.success || defSuccess);
  setColorField('theme-color-warning', 'theme-hex-warning', c.warning || defWarning);
  setColorField('theme-color-danger', 'theme-hex-danger', c.danger || defDanger);
  setColorField('theme-color-info', 'theme-hex-info', c.info || defInfo);
  // Group 3: Mod Type Badges Colors
  setColorField('theme-color-type-pak', 'theme-hex-type-pak', c.typePak || '#d29922');
  setColorField('theme-color-type-ue4ss', 'theme-hex-type-ue4ss', c.typeUe4ss || '#00b4d8');
  setColorField('theme-color-type-palschema', 'theme-hex-type-palschema', c.typePalschema || '#8250df');
  setColorField('theme-color-type-logicmods', 'theme-hex-type-logicmods', c.typeLogicmods || '#2ea043');

  const img = theme.images || {};
  updateImageBadge('theme-app-bg-status', !!img.appBackground);
  updateImageBadge('theme-sidebar-bg-status', !!img.sidebarBackground);
  updateImageBadge('theme-editor-bg-status', !!img.editorBackground);
  updateImageThumb('theme-app-bg-thumb-img', 'theme-app-bg-thumb-empty', img.appBackground);
  updateImageThumb('theme-sidebar-bg-thumb-img', 'theme-sidebar-bg-thumb-empty', img.sidebarBackground);
  updateImageThumb('theme-editor-bg-thumb-img', 'theme-editor-bg-thumb-empty', img.editorBackground);
  updateAdjustButtonVisibility('theme-app-bg-adjust-btn', Boolean(img.appBackground));
  updateAdjustButtonVisibility('theme-sidebar-bg-adjust-btn', Boolean(img.sidebarBackground));
  updateAdjustButtonVisibility('theme-editor-bg-adjust-btn', Boolean(img.editorBackground));

  setSliderField('theme-app-bg-opacity', 'theme-app-bg-opacity-val', img.appBackgroundOpacity ?? 0.15, '%', 100);
  setSliderField('theme-app-bg-blur', 'theme-app-bg-blur-val', img.appBackgroundBlur ?? 0, 'px', 1);
  setSliderField('theme-sidebar-bg-opacity', 'theme-sidebar-bg-opacity-val', img.sidebarBackgroundOpacity ?? 0.2, '%', 100);
  setSliderField('theme-editor-bg-opacity', 'theme-editor-bg-opacity-val', img.editorBackgroundOpacity ?? 0.15, '%', 100);
}

function attachThemeEventListeners(): void {
  setupCategoryTabs();

  const select = document.getElementById('settings-theme-select') as HTMLSelectElement | null;
  select?.addEventListener('change', () => {
    const id = select.value;
    applyTheme(id);
    updateDeleteButtonVisibility(id);
    syncDraftWithActiveTheme();
  });

  // Toggle Customizer Panel
  const customizeToggleBtn = document.getElementById('theme-customize-toggle-btn');
  const panel = document.getElementById('theme-customizer-panel');
  customizeToggleBtn?.addEventListener('click', () => {
    if (!panel) return;
    const isHidden = panel.style.display === 'none';
    panel.style.display = isHidden ? 'flex' : 'none';
    if (isHidden) {
      syncDraftWithActiveTheme();
    }
  });

  // Group 1: Structure & Surfaces
  bindLiveColor('theme-color-accent', 'theme-hex-accent', '--accent', (val) => {
    if (_activeDraft) _activeDraft.colors.accent = val;
  });
  bindLiveColor('theme-color-bg-primary', 'theme-hex-bg-primary', '--bg-primary', (val) => {
    if (_activeDraft) _activeDraft.colors.bgPrimary = val;
  });
  bindLiveColor('theme-color-bg-secondary', 'theme-hex-bg-secondary', '--bg-secondary', (val) => {
    if (_activeDraft) _activeDraft.colors.bgSecondary = val;
  });
  bindLiveColor('theme-color-bg-tertiary', 'theme-hex-bg-tertiary', '--bg-tertiary', (val) => {
    if (_activeDraft) _activeDraft.colors.bgTertiary = val;
  });
  bindLiveColor('theme-color-bg-card', 'theme-hex-bg-card', '--bg-card', (val) => {
    if (_activeDraft) _activeDraft.colors.bgCard = val;
  });
  bindLiveColor('theme-color-border', 'theme-hex-border', '--border', (val) => {
    if (_activeDraft) _activeDraft.colors.border = val;
  });
  bindLiveColor('theme-color-text-primary', 'theme-hex-text-primary', '--text-primary', (val) => {
    if (_activeDraft) _activeDraft.colors.textPrimary = val;
  });
  bindLiveColor('theme-color-text-muted', 'theme-hex-text-muted', '--text-muted', (val) => {
    if (_activeDraft) _activeDraft.colors.textMuted = val;
  });

  // Group 2: Status & Health Colors
  bindLiveColor('theme-color-success', 'theme-hex-success', '--success', (val) => {
    if (_activeDraft) _activeDraft.colors.success = val;
  });
  bindLiveColor('theme-color-warning', 'theme-hex-warning', '--warning', (val) => {
    if (_activeDraft) _activeDraft.colors.warning = val;
  });
  bindLiveColor('theme-color-danger', 'theme-hex-danger', '--danger', (val) => {
    if (_activeDraft) _activeDraft.colors.danger = val;
  });
  bindLiveColor('theme-color-info', 'theme-hex-info', '--info', (val) => {
    if (_activeDraft) _activeDraft.colors.info = val;
  });

  // Group 3: Mod Type Badges Colors
  bindLiveColor('theme-color-type-pak', 'theme-hex-type-pak', '--type-pak', (val) => {
    if (_activeDraft) _activeDraft.colors.typePak = val;
  });
  bindLiveColor('theme-color-type-ue4ss', 'theme-hex-type-ue4ss', '--type-ue4ss', (val) => {
    if (_activeDraft) _activeDraft.colors.typeUe4ss = val;
  });
  bindLiveColor('theme-color-type-palschema', 'theme-hex-type-palschema', '--type-palschema', (val) => {
    if (_activeDraft) _activeDraft.colors.typePalschema = val;
  });
  bindLiveColor('theme-color-type-logicmods', 'theme-hex-type-logicmods', '--type-logicmods', (val) => {
    if (_activeDraft) _activeDraft.colors.typeLogicmods = val;
  });

  // Live Sliders
  bindLiveSlider('theme-app-bg-opacity', 'theme-app-bg-opacity-val', '--pmm-app-bg-opacity', '%', 100, (v) => {
    if (_activeDraft) {
      if (!_activeDraft.images) _activeDraft.images = {};
      _activeDraft.images.appBackgroundOpacity = v;
    }
  });
  bindLiveSlider('theme-app-bg-blur', 'theme-app-bg-blur-val', '--pmm-app-bg-blur', 'px', 1, (v) => {
    if (_activeDraft) {
      if (!_activeDraft.images) _activeDraft.images = {};
      _activeDraft.images.appBackgroundBlur = v;
    }
  });
  bindLiveSlider('theme-sidebar-bg-opacity', 'theme-sidebar-bg-opacity-val', '--pmm-sidebar-bg-opacity', '%', 100, (v) => {
    if (_activeDraft) {
      if (!_activeDraft.images) _activeDraft.images = {};
      _activeDraft.images.sidebarBackgroundOpacity = v;
    }
  });
  bindLiveSlider('theme-editor-bg-opacity', 'theme-editor-bg-opacity-val', '--pmm-editor-bg-opacity', '%', 100, (v) => {
    if (_activeDraft) {
      if (!_activeDraft.images) _activeDraft.images = {};
      _activeDraft.images.editorBackgroundOpacity = v;
    }
  });

  // Image File Pickers with Thumbnails, Immediate Modal Crop & Thumb Clicks
  bindImagePicker(
    'app',
    'theme-app-bg-browse-btn',
    'theme-app-bg-file',
    'theme-app-bg-clear-btn',
    'theme-app-bg-status',
    'theme-app-bg-thumb-img',
    'theme-app-bg-thumb-empty',
    'theme-app-bg-thumb',
    'theme-app-bg-adjust-btn',
    '--pmm-app-bg',
    () => _activeDraft?.images?.appBackgroundAdjust,
    (dataUrl, adjust) => {
      if (_activeDraft) {
        if (!_activeDraft.images) _activeDraft.images = {};
        _activeDraft.images.appBackground = dataUrl;
        _activeDraft.images.appBackgroundAdjust = adjust;
        const projectedPos = document.documentElement.style.getPropertyValue('--pmm-app-bg-pos');
        _activeDraft.images.appBackgroundPosition = projectedPos || `${adjust.posX}% ${adjust.posY}%`;
        const projectedFit = document.documentElement.style.getPropertyValue('--pmm-app-bg-fit');
        _activeDraft.images.appBackgroundFit = projectedFit || `${adjust.scale}% auto`;
      }
      saveActiveThemeFromCustomizer();
    },
    () => {
      if (_activeDraft?.images) {
        delete _activeDraft.images.appBackground;
        delete _activeDraft.images.appBackgroundAdjust;
      }
      saveActiveThemeFromCustomizer();
    }
  );

  bindImagePicker(
    'sidebar',
    'theme-sidebar-bg-browse-btn',
    'theme-sidebar-bg-file',
    'theme-sidebar-bg-clear-btn',
    'theme-sidebar-bg-status',
    'theme-sidebar-bg-thumb-img',
    'theme-sidebar-bg-thumb-empty',
    'theme-sidebar-bg-thumb',
    'theme-sidebar-bg-adjust-btn',
    '--pmm-sidebar-bg',
    () => _activeDraft?.images?.sidebarBackgroundAdjust,
    (dataUrl, adjust) => {
      if (_activeDraft) {
        if (!_activeDraft.images) _activeDraft.images = {};
        _activeDraft.images.sidebarBackground = dataUrl;
        _activeDraft.images.sidebarBackgroundAdjust = adjust;
        const projectedPos = document.documentElement.style.getPropertyValue('--pmm-sidebar-bg-pos');
        _activeDraft.images.sidebarBackgroundPosition = projectedPos || `${adjust.posX}% ${adjust.posY}%`;
        const projectedFit = document.documentElement.style.getPropertyValue('--pmm-sidebar-bg-fit');
        _activeDraft.images.sidebarBackgroundFit = projectedFit || `${adjust.scale}% auto`;
      }
      saveActiveThemeFromCustomizer();
    },
    () => {
      if (_activeDraft?.images) {
        delete _activeDraft.images.sidebarBackground;
        delete _activeDraft.images.sidebarBackgroundAdjust;
      }
      saveActiveThemeFromCustomizer();
    }
  );

  bindImagePicker(
    'editor',
    'theme-editor-bg-browse-btn',
    'theme-editor-bg-file',
    'theme-editor-bg-clear-btn',
    'theme-editor-bg-status',
    'theme-editor-bg-thumb-img',
    'theme-editor-bg-thumb-empty',
    'theme-editor-bg-thumb',
    'theme-editor-bg-adjust-btn',
    '--pmm-editor-bg',
    () => _activeDraft?.images?.editorBackgroundAdjust,
    (dataUrl, adjust) => {
      if (_activeDraft) {
        if (!_activeDraft.images) _activeDraft.images = {};
        _activeDraft.images.editorBackground = dataUrl;
        _activeDraft.images.editorBackgroundAdjust = adjust;
        const projectedPos = document.documentElement.style.getPropertyValue('--pmm-editor-bg-pos');
        _activeDraft.images.editorBackgroundPosition = projectedPos || `${adjust.posX}% ${adjust.posY}%`;
        const projectedFit = document.documentElement.style.getPropertyValue('--pmm-editor-bg-fit');
        _activeDraft.images.editorBackgroundFit = projectedFit || `${adjust.scale}% auto`;
      }
      saveActiveThemeFromCustomizer();
    },
    () => {
      if (_activeDraft?.images) {
        delete _activeDraft.images.editorBackground;
        delete _activeDraft.images.editorBackgroundAdjust;
      }
      saveActiveThemeFromCustomizer();
    }
  );

  // Action Buttons: Save
  document.getElementById('theme-save-btn')?.addEventListener('click', async () => {
    if (!_activeDraft) return;

    if (_activeDraft.isBuiltin) {
      // Prompt user to save as a new custom theme
      const name = await showPrompt(t('theme.prompt_theme_name') || 'Enter a name for your custom theme:', `${_activeDraft.name} (Custom)`);
      if (!name || !name.trim()) return;

      const newId = 'custom_' + Date.now();
      const newTheme: PmmTheme = {
        ..._activeDraft,
        id: newId,
        name: name.trim(),
        isBuiltin: false,
      };

      saveCustomTheme(newTheme);
      applyTheme(newTheme);
      renderThemeSelectOptions();
      showToast(t('toasts.theme_saved') || 'Custom theme saved successfully', 'success');
    } else {
      saveCustomTheme(_activeDraft);
      applyTheme(_activeDraft);
      renderThemeSelectOptions();
      showToast(t('toasts.theme_saved') || 'Theme updated successfully', 'success');
    }
  });

  // Action Buttons: Reset
  document.getElementById('theme-reset-btn')?.addEventListener('click', () => {
    syncDraftWithActiveTheme();
    if (_activeDraft) applyTheme(_activeDraft);
  });

  // Action Buttons: New Theme
  document.getElementById('theme-btn-new')?.addEventListener('click', async () => {
    const name = await showPrompt(t('theme.prompt_new_theme') || 'Name your new custom theme:', 'My Custom Theme');
    if (!name || !name.trim()) return;

    const baseTheme = findThemeById(getActiveThemeId());
    const newTheme: PmmTheme = {
      ...JSON.parse(JSON.stringify(baseTheme)),
      id: 'custom_' + Date.now(),
      name: name.trim(),
      isBuiltin: false,
    };

    saveCustomTheme(newTheme);
    applyTheme(newTheme);
    renderThemeSelectOptions();
    showToast(t('toasts.theme_created') || 'New theme created', 'success');

    // Open customizer
    const customizer = document.getElementById('theme-customizer-panel');
    if (customizer) customizer.style.display = 'flex';
  });

  // Action Buttons: Export
  document.getElementById('theme-btn-export')?.addEventListener('click', () => {
    const current = findThemeById(getActiveThemeId());
    downloadThemeFile(current);
    showToast(t('toasts.theme_exported') || 'Theme exported (.pmmtheme.json)', 'success');
  });

  // Action Buttons: Import
  const importInput = document.getElementById('theme-import-file-input') as HTMLInputElement | null;
  document.getElementById('theme-btn-import')?.addEventListener('click', () => {
    importInput?.click();
  });

  importInput?.addEventListener('change', () => {
    const file = importInput.files?.[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const text = e.target?.result as string;
        const theme = parseThemeFromJson(text);
        saveCustomTheme(theme);
        applyTheme(theme);
        renderThemeSelectOptions();
        showToast(t('toasts.theme_imported', { name: theme.name }) || `Imported theme "${theme.name}"`, 'success');
      } catch (err: any) {
        showToast(t('toasts.theme_import_failed', { error: String(err.message || err) }) || 'Failed to import theme', 'error');
      } finally {
        importInput.value = '';
      }
    };
    reader.readAsText(file);
  });

  // Action Buttons: Delete
  document.getElementById('theme-delete-btn')?.addEventListener('click', async () => {
    const activeId = getActiveThemeId();
    const current = findThemeById(activeId);
    if (current.isBuiltin) return;

    const confirmed = await showConfirm(t('theme.confirm_delete', { name: current.name }) || `Delete custom theme "${current.name}"?`);
    if (confirmed) {
      deleteCustomTheme(activeId);
      applyTheme('dark');
      renderThemeSelectOptions();
      showToast(t('toasts.theme_deleted') || 'Custom theme deleted', 'info');
    }
  });
}


function bindImagePicker(
  target: 'app' | 'sidebar' | 'editor',
  browseBtnId: string,
  fileInputId: string,
  clearBtnId: string,
  badgeId: string,
  thumbImgId: string,
  thumbEmptyId: string,
  thumbBoxId: string,
  adjustBtnId: string,
  cssVarPrefix: string,
  getAdjust: () => PmmImageCustomAdjust | undefined,
  onSave: (dataUrl: string, adjust: PmmImageCustomAdjust) => void,
  onClear: () => void
): void {
  const browseBtn = document.getElementById(browseBtnId);
  const fileInput = document.getElementById(fileInputId) as HTMLInputElement | null;
  const clearBtn = document.getElementById(clearBtnId);
  const adjustBtn = document.getElementById(adjustBtnId);
  const thumbBox = document.getElementById(thumbBoxId);

  browseBtn?.addEventListener('click', () => {
    fileInput?.click();
  });

  // When a file is chosen, automatically open the crop/adjust modal immediately!
  fileInput?.addEventListener('change', async () => {
    const file = fileInput.files?.[0];
    if (!file) return;

    try {
      const dataUrl = await compressImageToDataUrl(file);
      openThemeImageAdjustModal(
        target,
        dataUrl,
        getAdjust(),
        (adjust) => {
          updateImageBadge(badgeId, true);
          updateImageThumb(thumbImgId, thumbEmptyId, dataUrl);
          updateAdjustButtonVisibility(adjustBtnId, true);
          onSave(dataUrl, adjust);
        },
        () => {
          if (fileInput) fileInput.value = '';
        }
      );
    } catch (err) {
      console.error('Failed to load image:', err);
    }
  });

  // Re-open adjust modal when clicking the adjust button or thumbnail box
  const openAdjust = () => {
    if (!_activeDraft) {
      syncDraftWithActiveTheme();
    }

    let currentImg = target === 'app' ? _activeDraft?.images?.appBackground
      : target === 'sidebar' ? _activeDraft?.images?.sidebarBackground
      : _activeDraft?.images?.editorBackground;

    // Fallback 1: check thumbnail image element
    if (!currentImg) {
      const thumbImg = document.getElementById(thumbImgId) as HTMLImageElement | null;
      if (thumbImg?.src && (thumbImg.src.startsWith('data:') || thumbImg.src.startsWith('http') || thumbImg.src.startsWith('blob') || thumbImg.src.startsWith('/'))) {
        currentImg = thumbImg.src;
      }
    }

    // Fallback 2: check documentElement CSS property
    if (!currentImg) {
      const cssVal = document.documentElement.style.getPropertyValue(`${cssVarPrefix}-image`);
      if (cssVal && cssVal.includes('url(')) {
        currentImg = cssVal.replace(/^url\(["']?/, '').replace(/["']?\)$/, '');
      }
    }

    console.info(`[ThemeManager] openAdjust for "${target}", has currentImg:`, !!currentImg);
    if (!currentImg) {
      console.warn(`[ThemeManager] No image available to adjust for target "${target}"`);
      return;
    }

    openThemeImageAdjustModal(
      target,
      currentImg,
      getAdjust(),
      (adjust) => {
        onSave(currentImg!, adjust);
      }
    );
  };

  adjustBtn?.addEventListener('click', openAdjust);
  thumbBox?.addEventListener('click', openAdjust);

  clearBtn?.addEventListener('click', () => {
    if (fileInput) fileInput.value = '';
    document.documentElement.style.removeProperty(`${cssVarPrefix}-image`);
    document.documentElement.style.removeProperty(`${cssVarPrefix}-pos`);
    document.documentElement.style.removeProperty(`${cssVarPrefix}-fit`);
    updateImageBadge(badgeId, false);
    updateImageThumb(thumbImgId, thumbEmptyId, null);
    updateAdjustButtonVisibility(adjustBtnId, false);
    onClear();
  });
}

/**
 * Persists all current customizations made in the Theme Customizer.
 * If the active theme is a builtin preset, it is safely promoted to a custom theme
 * so user tweaks (images, colors) persist seamlessly across restarts.
 */
export function saveActiveThemeFromCustomizer(): void {
  if (!_activeDraft) return;

  if (_activeDraft.isBuiltin) {
    const original = findThemeById(_activeDraft.id);
    const hasImages = Boolean(
      _activeDraft.images?.appBackground ||
      _activeDraft.images?.sidebarBackground ||
      _activeDraft.images?.editorBackground
    );
    const hasColorChanges = JSON.stringify(_activeDraft.colors) !== JSON.stringify(original.colors);

    if (hasImages || hasColorChanges) {
      const newId = `custom_${_activeDraft.id}_${Date.now()}`;
      const newTheme: PmmTheme = {
        ..._activeDraft,
        id: newId,
        name: `${_activeDraft.name} (Custom)`,
        isBuiltin: false,
      };
      _activeDraft = newTheme;
      saveCustomTheme(newTheme);
      applyTheme(newTheme);
      renderThemeSelectOptions();
    }
  } else {
    saveCustomTheme(_activeDraft);
    applyTheme(_activeDraft);
    renderThemeSelectOptions();
  }
}
