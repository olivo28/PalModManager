import { setGamePath, setHideNativeMods, setDebugConsole, setCustomDataPath, setToolbarScale } from '../../api';
import { getState, updateState } from '../../state';
import { showToast } from '../toast';
import { showConfirm } from '../confirm';

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

    newUe4ss.checked = !!state.currentSettings?.forceLoadOrderUe4ss;
    newPalschema.checked = isWindows ? !!state.currentSettings?.forceLoadOrderPalschema : false;

    const handleSubChange = async (elem: HTMLInputElement, systemName: string, detailMsg: string, requireConfirm: boolean) => {
      if (elem.checked && requireConfirm) {
        const confirmed = await showConfirm(
          `Enable ${systemName} Load Order`,
          `Enabling this setting will enforce loading sequence for ${systemName} mods. ${detailMsg}<br><br>Do you want to continue?`,
          'Yes, Enable',
          'Cancel'
        );
        if (!confirmed) {
          elem.checked = false;
          return;
        }
      }
    };

    newUe4ss.addEventListener('change', () => {
      handleSubChange(newUe4ss, 'UE4SS', 'This will organize your mods via mods.txt.', true);
    });

    newPalschema.addEventListener('change', () => {
      handleSubChange(newPalschema, 'PalSchema', 'This will dynamically redirect your mods to a Storage folder and create NTFS junctions.', true);
    });
  }

  if (state.currentSettings?.gamePath) {
    pathStatus.textContent = 'Path configured';
    pathStatus.className = 'settings-path-status valid';
  } else {
    pathStatus.textContent = 'No path configured - select your Palworld folder';
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

  modal.classList.add('visible');

  requestAnimationFrame(() => {
    const modalBody = modal.querySelector('.modal-body');
    if (modalBody) {
      modalBody.scrollTop = 0;
    }
  });
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
        title: 'Select Custom Data Storage Directory',
      });
      if (selected) {
        const path = typeof selected === 'string' ? selected : selected as string;
        _tempCustomDataPath = path;
        if (display) {
          display.style.display = 'block';
          display.textContent = `Custom Folder: ${path}`;
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
      display.textContent = `Custom Folder: ${_tempCustomDataPath}`;
    }
  }
}

export async function handleSettingsBrowse(): Promise<void> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select Palworld game folder',
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
        pathStatus.textContent = 'Path configured';
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
      showToast('Migrating data files to new location...', 'info');
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

    closeSettingsModal();
    showToast('Settings saved', 'success');

    const { loadGameVersion, loadDependencies, loadMods } = await import('../modsView');
    loadGameVersion();
    await loadDependencies();
    await loadMods();
  } catch (e) {
    showToast('Failed to save settings: ' + e, 'error');
  } finally {
    saveBtn.disabled = false;
  }
}
