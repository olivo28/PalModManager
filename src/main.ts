import 'highlight.js/styles/github-dark.css';
import './utils/imageFallback';
import { initI18n } from './utils/i18n';
import { loadAppTemplates } from './ui/templateLoader';
import { bootstrapApp } from './app';

loadAppTemplates();
initI18n();

// Immediately reveal the Tauri window with the splash screen so the user never sees a black screen
import('@tauri-apps/api/window').then(({ getCurrentWindow }) => {
  getCurrentWindow().show().catch(() => {});
}).catch(() => {});

bootstrapApp();
