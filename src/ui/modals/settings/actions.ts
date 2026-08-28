import {
  setGamePath,
  setHideNativeMods,
  setDebugConsole,
  setCustomDataPath,
  setToolbarScale,
} from '../../../api';
import { getState, updateState } from '../../../state';
import { showToast } from '../../toast';
import { t } from '../../../utils/i18n';
import { _tempCustomDataPath, setTempCustomDataPath } from './state';
import { revertDataPathSelect } from './helpers';
import { closeSettingsModal } from './modal';

export async function handleDataPathChange(): Promise<void> {
  const select = document.getElementById('settings-data-path-select') as HTMLSelectElement | null;
  const display = document.getElementById('settings-custom-data-path-display');
  if (!select) return;

  const value = select.value;
  if (value === 'default') {
    setTempCustomDataPath(null);
    if (display) display.style.display = 'none';
  } else if (value === 'portable') {
    setTempCustomDataPath('__portable__');
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
        setTempCustomDataPath(path);
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
      const { setForceLoadOrderUe4ss } = await import('../../../api');
      const settings = await setForceLoadOrderUe4ss(forceLoadOrderUe4ss);
      updateState({ currentSettings: settings });
      forceLoadOrderChanged = true;
    }

    if (forceLoadOrderPalschema !== !!state.currentSettings?.forceLoadOrderPalschema) {
      const { setForceLoadOrderPalschema } = await import('../../../api');
      const settings = await setForceLoadOrderPalschema(forceLoadOrderPalschema);
      updateState({ currentSettings: settings });
      forceLoadOrderChanged = true;
    }

    if (forceLoadOrder !== !!state.currentSettings?.forceLoadOrder || forceLoadOrderChanged) {
      const { setForceLoadOrder } = await import('../../../api');
      const settings = await setForceLoadOrder(forceLoadOrder);
      updateState({ currentSettings: settings });
      const { updateLoadTabVisibility } = await import('../../loadView');
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
      const { setLanguage } = await import('../../../api');
      const settings = await setLanguage(langSelect.value);
      updateState({ currentSettings: settings });
    }

    const dnsSelect = document.getElementById('settings-dns-resolver-select') as HTMLSelectElement | null;
    if (dnsSelect && dnsSelect.value && dnsSelect.value !== (state.currentSettings?.dnsResolver || 'auto')) {
      const { setDnsResolver } = await import('../../../api');
      const settings = await setDnsResolver(dnsSelect.value);
      updateState({ currentSettings: settings });
    }

    closeSettingsModal();
    showToast(t('toasts.settings_saved'), 'success');

    // If db view is currently active, reload snapshot immediately
    if (state.activeTab === 'db') {
      const { renderDbView } = await import('../../dbView');
      await renderDbView();
    }

    const { loadGameVersion, loadDependencies, loadMods, renderModsView, loadProfiles } = await import('../../modsView');
    loadGameVersion();
    await loadDependencies();
    await loadProfiles();
    await loadMods();
    renderModsView();
  } catch (e) {
    showToast(t('toasts.export_failed', { error: String(e) }), 'error');
  } finally {
    saveBtn.disabled = false;
  }
}
