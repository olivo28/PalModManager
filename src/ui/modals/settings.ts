import { setGamePath, setHideNativeMods, setDebugConsole, setCustomDataPath, setToolbarScale } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';
import { escapeHtml } from '../../utils/helpers';
import { t } from '../../utils/i18n';

export let _tempCustomDataPath: string | null = null;

export function openSettingsModal(): void {
  const modal = document.getElementById('settings-modal')!;
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const forceLoadOrderUe4ssCheckbox = document.getElementById('settings-force-load-order-ue4ss')! as HTMLInputElement;
  const forceLoadOrderPalschemaCheckbox = document.getElementById('settings-force-load-order-palschema')! as HTMLInputElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  const state = getState();

  pathInput.value = state.currentSettings?.gamePath || '';
  if (hideNativeCheckbox) {
    hideNativeCheckbox.checked = !!state.currentSettings?.hideNativeMods;
  }
  if (debugConsoleCheckbox) {
    debugConsoleCheckbox.checked = !!state.currentSettings?.debugConsole;
  }

  const isWindows = navigator.userAgent.toLowerCase().includes('win');
  if (forceLoadOrderPalschemaCheckbox && !isWindows) {
    forceLoadOrderPalschemaCheckbox.disabled = true;
    forceLoadOrderPalschemaCheckbox.checked = false;
  }

  if (forceLoadOrderUe4ssCheckbox && forceLoadOrderPalschemaCheckbox) {
    const newUe4ss = forceLoadOrderUe4ssCheckbox.cloneNode(true) as HTMLInputElement;
    forceLoadOrderUe4ssCheckbox.parentNode!.replaceChild(newUe4ss, forceLoadOrderUe4ssCheckbox);

    const newPalschema = forceLoadOrderPalschemaCheckbox.cloneNode(true) as HTMLInputElement;
    forceLoadOrderPalschemaCheckbox.parentNode!.replaceChild(newPalschema, forceLoadOrderPalschemaCheckbox);

    const activeProfile = state.currentProfile || state.profiles?.find(p => p.id === state.currentProfileId);
    newUe4ss.checked = activeProfile?.force_load_order_ue4ss !== undefined && activeProfile?.force_load_order_ue4ss !== null
      ? !!activeProfile.force_load_order_ue4ss
      : !!state.currentSettings?.forceLoadOrderUe4ss;

    newPalschema.checked = isWindows
      ? (activeProfile?.force_load_order_palschema !== undefined && activeProfile?.force_load_order_palschema !== null
          ? !!activeProfile.force_load_order_palschema
          : !!state.currentSettings?.forceLoadOrderPalschema)
      : false;

    const handleSubChange = async (elem: HTMLInputElement, systemName: string, detailMsg: string, requireConfirm: boolean) => {
      if (elem.checked && requireConfirm) {
        const confirmed = await showConfirm(
          t('settings.flo_confirm_title', { systemName }),
          t('settings.flo_confirm_body', { systemName, detailMsg }),
          t('settings.flo_confirm_btn'),
          t('common.cancel')
        );
        if (!confirmed) {
          elem.checked = false;
          return;
        }
      }
    };

    newUe4ss.addEventListener('change', () => {
      handleSubChange(newUe4ss, 'UE4SS', t('settings.flo_detail_ue4ss'), true);
    });

    newPalschema.addEventListener('change', () => {
      handleSubChange(newPalschema, 'PalSchema', t('settings.flo_detail_palschema'), true);
    });
  }

  if (state.currentSettings?.gamePath) {
    pathStatus.textContent = t('settings.path_configured');
    pathStatus.className = 'settings-path-status valid';
  } else {
    pathStatus.textContent = t('settings.path_not_configured');
    pathStatus.className = 'settings-path-status invalid';
  }

  const dataPathSelect = document.getElementById('settings-data-path-select') as HTMLSelectElement | null;
  const dataPathDisplay = document.getElementById('settings-custom-data-path-display');
  _tempCustomDataPath = state.currentSettings?.customDataPath || null;

  if (dataPathSelect) {
    if (!_tempCustomDataPath) {
      dataPathSelect.value = 'default';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else if (_tempCustomDataPath === '__portable__') {
      dataPathSelect.value = 'portable';
      if (dataPathDisplay) dataPathDisplay.style.display = 'none';
    } else {
      dataPathSelect.value = 'custom';
      if (dataPathDisplay) {
        dataPathDisplay.style.display = 'block';
        dataPathDisplay.textContent = `Custom Folder: ${_tempCustomDataPath}`;
      }
    }
  }

  const scaleInput = document.getElementById('settings-toolbar-scale') as HTMLInputElement | null;
  const scaleValue = document.getElementById('settings-toolbar-scale-value');
  const initialScale = state.currentSettings?.toolbarScale || 1.0;
  if (scaleInput) {
    scaleInput.value = initialScale.toString();
    if (scaleValue) {
      scaleValue.textContent = `${Math.round(initialScale * 100)}%`;
    }

    scaleInput.addEventListener('input', () => {
      const scale = parseFloat(scaleInput.value);
      if (scaleValue) {
        scaleValue.textContent = `${Math.round(scale * 100)}%`;
      }
      document.documentElement.style.setProperty('--toolbar-scale', scale.toString());
    });
  }

  const openUe4ssBtn = document.getElementById('open-folder-ue4ss') as HTMLButtonElement | null;
  const openPalschemaBtn = document.getElementById('open-folder-palschema') as HTMLButtonElement | null;
  if (openUe4ssBtn) {
    openUe4ssBtn.style.display = !!state.dependencies?.ue4ss_installed ? '' : 'none';
  }
  if (openPalschemaBtn) {
    openPalschemaBtn.style.display = !!state.dependencies?.palschema_installed ? '' : 'none';
  }

  refreshSafetyBackupStatus();

  // Setup Sidebar Tab navigation
  setupSettingsTabs();

  // Initialize and render Nexus account section
  import('../../features/nexus_auth').then(({ renderNexusAccountUI, processOAuthCallback }) => {
    renderNexusAccountUI();

    const manualCallbackBtn = document.getElementById('btn-nexus-manual-callback');
    const manualCallbackInput = document.getElementById('nexus-manual-callback-input') as HTMLInputElement | null;
    if (manualCallbackBtn && manualCallbackInput) {
      manualCallbackBtn.onclick = async () => {
        const val = manualCallbackInput.value.trim();
        if (val) {
          await processOAuthCallback(val);
          manualCallbackInput.value = '';
        }
      };
    }
  }).catch(() => {});

  const langSelect = document.getElementById('settings-language-select') as HTMLSelectElement | null;
  if (langSelect) {
    import('../../utils/i18n').then(({ getLocale, setLocale }) => {
      langSelect.value = getLocale();
      langSelect.onchange = () => {
        setLocale(langSelect.value as any);
        import('../modsView').then(m => m.renderModsView()).catch(() => {});
      };
    });
  }

  modal.classList.add('visible');

  requestAnimationFrame(() => {
    const activePane = modal.querySelector('.settings-tab-pane.active');
    if (activePane) {
      activePane.scrollTop = 0;
    }
  });
}

function setupSettingsTabs(): void {
  const tabButtons = document.querySelectorAll<HTMLButtonElement>('.settings-tab-button');
  const panes = document.querySelectorAll<HTMLElement>('.settings-tab-pane');

  tabButtons.forEach(btn => {
    btn.onclick = () => {
      const targetTab = btn.dataset.settingsTab;
      if (!targetTab) return;

      tabButtons.forEach(b => b.classList.remove('active'));
      panes.forEach(p => p.classList.remove('active'));

      btn.classList.add('active');
      const activePane = document.getElementById(`settings-pane-${targetTab}`);
      if (activePane) {
        activePane.classList.add('active');
        activePane.scrollTop = 0;
      }
    };
  });
}

export async function refreshSafetyBackupStatus(): Promise<void> {
  const statusElem = document.getElementById('safety-backup-status-text');
  if (!statusElem) return;

  try {
    const { getSafetyBackupInfo } = await import('../../api');
    const info = await getSafetyBackupInfo();
    if (info.exists && info.timestamp) {
      const dateStr = new Date(info.timestamp).toLocaleString();
      const sizeKb = info.zipSizeBytes ? Math.round(info.zipSizeBytes / 1024) : 0;
      statusElem.innerHTML = `✅ <strong>${escapeHtml(t('settings.safety_initial_snapshot'))}:</strong> ${dateStr} (${sizeKb} KB)<br>• ${escapeHtml(t('settings.safety_tracked_summary', { ue4ss: info.ue4ssModsCount, palschema: info.palschemaModsCount }))}`;
      statusElem.style.color = 'var(--text-secondary)';
    } else {
      statusElem.textContent = t('settings.safety_none');
      statusElem.style.color = 'var(--text-muted)';
    }
  } catch (e) {
    statusElem.textContent = t('settings.safety_ready');
  }
}

export function closeSettingsModal(): void {
  const modal = document.getElementById('settings-modal');
  if (modal) modal.classList.remove('visible');
  const savedScale = getState().currentSettings?.toolbarScale || 1.0;
  document.documentElement.style.setProperty('--toolbar-scale', savedScale.toString());
}

export async function handleDataPathChange(): Promise<void> {
  const select = document.getElementById('settings-data-path-select') as HTMLSelectElement | null;
  const display = document.getElementById('settings-custom-data-path-display');
  if (!select) return;

  const value = select.value;
  if (value === 'default') {
    _tempCustomDataPath = null;
    if (display) display.style.display = 'none';
  } else if (value === 'portable') {
    _tempCustomDataPath = '__portable__';
    if (display) display.style.display = 'none';
  } else if (value === 'custom') {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('settings.dialog_browse_data_title'),
      });
      if (selected) {
        const path = typeof selected === 'string' ? selected : selected as string;
        _tempCustomDataPath = path;
        if (display) {
          display.style.display = 'block';
          display.textContent = t('settings.custom_folder_display', { path });
        }
      } else {
        revertDataPathSelect(select, display);
      }
    } catch (e) {
      console.error('Failed to open directory dialog:', e);
      revertDataPathSelect(select, display);
    }
  }
}

function revertDataPathSelect(select: HTMLSelectElement, display: HTMLElement | null): void {
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

export async function handleSettingsBrowse(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('settings.dialog_browse_game_title'),
    });
    if (selected) {
      const path = typeof selected === 'string' ? selected : selected as string;
      const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
      pathInput.value = path;
    }
  } catch (e) {
    console.error('Error browsing path:', e);
  }
}

export async function handleSaveSettings(): Promise<void> {
  const pathInput = document.getElementById('settings-game-path')! as HTMLInputElement;
  const hideNativeCheckbox = document.getElementById('settings-hide-native-mods')! as HTMLInputElement;
  const debugConsoleCheckbox = document.getElementById('settings-debug-console')! as HTMLInputElement;
  const saveBtn = document.getElementById('settings-save')! as HTMLButtonElement;
  const pathStatus = document.getElementById('settings-path-status')!;
  saveBtn.disabled = true;

  try {
    const newPath = pathInput.value.trim();
    const hideNative = hideNativeCheckbox ? hideNativeCheckbox.checked : false;
    const debugConsole = debugConsoleCheckbox ? debugConsoleCheckbox.checked : false;
    const forceLoadOrderUe4ssCheckbox = document.getElementById('settings-force-load-order-ue4ss')! as HTMLInputElement;
    const forceLoadOrderPalschemaCheckbox = document.getElementById('settings-force-load-order-palschema')! as HTMLInputElement;

    const forceLoadOrderUe4ss = forceLoadOrderUe4ssCheckbox ? forceLoadOrderUe4ssCheckbox.checked : false;
    const forceLoadOrderPalschema = forceLoadOrderPalschemaCheckbox ? forceLoadOrderPalschemaCheckbox.checked : false;
    const forceLoadOrder = forceLoadOrderUe4ss || forceLoadOrderPalschema;
    const state = getState();

    if (newPath && newPath !== state.currentSettings?.gamePath) {
      try {
        const settings = await setGamePath(newPath);
        updateState({ currentSettings: settings });
        pathStatus.textContent = t('settings.path_configured');
        pathStatus.className = 'settings-path-status valid';
      } catch (e) {
        pathStatus.textContent = String(e);
        pathStatus.className = 'settings-path-status invalid';
        saveBtn.disabled = false;
        return;
      }
    }

    if (hideNative !== !!state.currentSettings?.hideNativeMods) {
      const settings = await setHideNativeMods(hideNative);
      updateState({ currentSettings: settings });
    }

    if (debugConsole !== !!state.currentSettings?.debugConsole) {
      const settings = await setDebugConsole(debugConsole);
      updateState({ currentSettings: settings });
    }

    let forceLoadOrderChanged = false;
    if (forceLoadOrderUe4ss !== !!state.currentSettings?.forceLoadOrderUe4ss) {
      const { setForceLoadOrderUe4ss } = await import('../../api');
      const settings = await setForceLoadOrderUe4ss(forceLoadOrderUe4ss);
      updateState({ currentSettings: settings });
      forceLoadOrderChanged = true;
    }

    if (forceLoadOrderPalschema !== !!state.currentSettings?.forceLoadOrderPalschema) {
      const { setForceLoadOrderPalschema } = await import('../../api');
      const settings = await setForceLoadOrderPalschema(forceLoadOrderPalschema);
      updateState({ currentSettings: settings });
      forceLoadOrderChanged = true;
    }

    if (forceLoadOrder !== !!state.currentSettings?.forceLoadOrder || forceLoadOrderChanged) {
      const { setForceLoadOrder } = await import('../../api');
      const settings = await setForceLoadOrder(forceLoadOrder);
      updateState({ currentSettings: settings });
      const { updateLoadTabVisibility } = await import('../loadView');
      updateLoadTabVisibility();
    }

    if (_tempCustomDataPath !== (state.currentSettings?.customDataPath || null)) {
      showToast(t('settings.custom_data_path_desc'), 'info');
      const settings = await setCustomDataPath(_tempCustomDataPath);
      updateState({ currentSettings: settings });
    }

    const scaleInput = document.getElementById('settings-toolbar-scale') as HTMLInputElement | null;
    if (scaleInput) {
      const scale = parseFloat(scaleInput.value);
      if (scale !== (state.currentSettings?.toolbarScale || 1.0)) {
        const settings = await setToolbarScale(scale);
        updateState({ currentSettings: settings });
        document.documentElement.style.setProperty('--toolbar-scale', scale.toString());
      }
    }

    const langSelect = document.getElementById('settings-language-select') as HTMLSelectElement | null;
    if (langSelect && langSelect.value) {
      const { setLanguage } = await import('../../api');
      const settings = await setLanguage(langSelect.value);
      updateState({ currentSettings: settings });
    }

    closeSettingsModal();
    showToast(t('toasts.settings_saved'), 'success');

    // If db view is currently active, reload snapshot immediately
    if (state.activeTab === 'db') {
      const { renderDbView } = await import('../dbView');
      await renderDbView();
    }

    const { loadGameVersion, loadDependencies, loadMods } = await import('../modsView');
    loadGameVersion();
    await loadDependencies();
    await loadMods();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    saveBtn.disabled = false;
  }
}
