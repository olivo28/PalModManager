import { initI18n } from '../utils/i18n';
import { getSettings, logFromJs } from '../api';
import { getState, updateState } from '../state';
import { openSettingsModal } from '../ui/modal';
import { loadMods, loadGameVersion, loadProfiles, loadLibrary, loadDependencies } from '../ui/modsView';
import { setupEditorFsWatcher } from '../ui/editorView';
import { autoFetchNexusInfo } from '../features/nexus';
import { initPackerView } from '../ui/packerView';
import { mainDom, bus } from '../framework';
import { getPreferredTheme, applyTheme } from './theme';
import { setupEventListeners } from './listeners';

export function showApp(): void {
  const loading = document.getElementById('app-loading');
  const app = mainDom.elMaybe('app');
  if (app) app.style.display = 'flex';
  if (loading) {
    loading.classList.add('fade-out');
    setTimeout(() => {
      loading.style.display = 'none';
      bus.emit('app:ready', undefined);
    }, 380);
  } else {
    bus.emit('app:ready', undefined);
  }
}

export async function bootstrapApp(): Promise<void> {
  const startTime = Date.now();
  console.time('init');
  applyTheme(getPreferredTheme());

  try {
    await logFromJs("⚡ PMM-Core Reactive Engine: Frontend runtime initialized (Zero-VDOM, 11 scopes, Svelte-like)");
    console.info('%c⚡ PMM-Core Reactive Engine initialized (Zero-VDOM, 11 scopes)', 'color: #38bdf8; font-weight: bold; font-size: 13px;');
    const settings = await getSettings();
    updateState({ currentSettings: settings });
    if (settings.language) {
      initI18n(settings.language);
    }
    const { updateLoadTabVisibility } = await import('../ui/loadView');
    updateLoadTabVisibility();
    const scale = settings.toolbarScale || 1.0;
    document.documentElement.style.setProperty('--toolbar-scale', scale.toString());

    if (settings.gamePath) {
      console.time('startupSequence');
      await loadGameVersion();
      await loadProfiles();
      await loadDependencies();
      await loadLibrary();
      await loadMods();
      console.timeEnd('startupSequence');

      const hasMissingMetadata = getState().allMods.some(m => !m.nexusModId && m.version === 'unknown');
      if (hasMissingMetadata) {
        console.log('Some mods missing metadata');
      }
      autoFetchNexusInfo();
    } else {
      openSettingsModal();
    }

    setupEventListeners();
    setupEditorFsWatcher();
    initPackerView();

    // Initialize Nexus OAuth & deep link listener
    import('../features/nexus_auth').then(({ initNexusAuth }) => {
      initNexusAuth();
    }).catch(err => console.warn('Failed to init Nexus Auth:', err));

    // Ensure a smooth, pleasant splash display duration (~1.2s minimum) before transitioning to main view
    const elapsed = Date.now() - startTime;
    const remainingTime = Math.max(0, 1200 - elapsed);
    if (remainingTime > 0) {
      await new Promise(resolve => setTimeout(resolve, remainingTime));
    }
    showApp();
  } catch (e) {
    console.error('Error initializing:', e);
    showApp();
  }
  console.timeEnd('init');
}
