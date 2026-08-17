/**
 * i18n.ts - Ultra-lightweight reactive internationalization engine for PalModManager.
 * Zero external dependencies. Fast JSON-based nested dictionary lookups with fallback.
 */

import en from '../locales/en.json';
import es from '../locales/es.json';
import pt from '../locales/pt.json';
import zhCN from '../locales/zh-CN.json';
import ja from '../locales/ja.json';
import ko from '../locales/ko.json';

export type SupportedLocale = 'en' | 'es' | 'pt' | 'zh-CN' | 'ja' | 'ko';

type TranslationTree = Record<string, any>;

const localeDictionaries: Record<string, TranslationTree> = {
  en,
  es,
  pt,
  'zh-CN': zhCN,
  ja,
  ko,
};

let currentLocale: SupportedLocale = 'en';

/**
 * Initializes i18n language preference from localStorage, AppSettings or browser language.
 */
export function initI18n(preferredLocale?: string | null): void {
  const saved = (preferredLocale || localStorage.getItem('pmm_locale')) as SupportedLocale | null;
  if (saved && localeDictionaries[saved]) {
    currentLocale = saved;
  } else {
    const fullLang = navigator.language?.toLowerCase();
    const navLang = fullLang?.split('-')[0];
    if (navLang === 'es') {
      currentLocale = 'es';
    } else if (navLang === 'pt') {
      currentLocale = 'pt';
    } else if (navLang === 'zh') {
      currentLocale = 'zh-CN';
    } else if (navLang === 'ja') {
      currentLocale = 'ja';
    } else if (navLang === 'ko') {
      currentLocale = 'ko';
    } else {
      currentLocale = 'en';
    }
  }
  localStorage.setItem('pmm_locale', currentLocale);
  document.documentElement.lang = currentLocale;
  updateDOMTranslations();
}

/**
 * Gets the current active locale code (e.g. 'en' or 'es').
 */
export function getLocale(): SupportedLocale {
  return currentLocale;
}

/**
 * Sets the active locale, saves preference, persists to backend DB, and triggers live DOM updates.
 */
export function setLocale(locale: SupportedLocale): void {
  if (!localeDictionaries[locale]) {
    console.warn(`Locale '${locale}' is not supported. Falling back to 'en'.`);
    locale = 'en';
  }
  currentLocale = locale;
  localStorage.setItem('pmm_locale', locale);
  document.documentElement.lang = locale;
  updateDOMTranslations();

  // Re-render active view and dynamic components immediately
  try {
    import('../state').then(({ getState }) => {
      const state = getState();
      // Re-render mods view
      import('../ui/modsView').then(m => {
        m.renderModsView();
        m.renderProfileList();
        if (state.dependencies) {
          m.renderDependencyBadges(state.dependencies);
        }
      }).catch(() => {});

      // Re-render active tab view
      if (state.activeTab === 'library') {
        import('../ui/modsView').then(m => m.renderLibraryView()).catch(() => {});
      } else if (state.activeTab === 'load') {
        import('../ui/loadView').then(m => m.renderLoadView()).catch(() => {});
      } else if (state.activeTab === 'scanner') {
        import('../ui/scannerView').then(m => m.renderScannerView()).catch(() => {});
      } else if (state.activeTab === 'db') {
        import('../ui/dbView').then(m => m.renderDbView()).catch(() => {});
      }
    }).catch(() => {});
  } catch (err) {
    console.error('Error refreshing views on locale change:', err);
  }

  // Persist to backend DB AppSettings
  import('../api').then(({ setLanguage }) => {
    setLanguage(locale).catch(err => console.error('Failed to persist language to backend DB:', err));
  }).catch(() => {});
}

/**
 * Translates a key (e.g. 'mods.btn_install_title') with optional interpolation parameters.
 * Automatically falls back to English if the key is missing in the active locale.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');

  // 1. Look up in current locale
  let val: any = localeDictionaries[currentLocale];
  for (const k of keys) {
    if (val && typeof val === 'object' && k in val) {
      val = val[k];
    } else {
      val = undefined;
      break;
    }
  }

  // 2. Fallback to English if missing
  if (typeof val !== 'string') {
    let fallback: any = localeDictionaries['en'];
    for (const k of keys) {
      if (fallback && typeof fallback === 'object' && k in fallback) {
        fallback = fallback[k];
      } else {
        fallback = undefined;
        break;
      }
    }
    val = typeof fallback === 'string' ? fallback : key;
  }

  // 3. Interpolate parameters (e.g. {count}, {name})
  if (params && typeof val === 'string') {
    for (const [pKey, pVal] of Object.entries(params)) {
      val = val.replace(new RegExp(`\\{${pKey}\\}`, 'g'), String(pVal));
    }
  }

  return typeof val === 'string' ? val : key;
}

/**
 * Automatically scans and translates all declarative data-i18n attributes in the DOM.
 */
export function updateDOMTranslations(root: HTMLElement | Document = document): void {
  // Text contents
  root.querySelectorAll<HTMLElement>('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');
    if (key) {
      el.textContent = t(key);
    }
  });

  // Input Placeholders
  root.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>('[data-i18n-placeholder]').forEach((el) => {
    const key = el.getAttribute('data-i18n-placeholder');
    if (key) {
      el.placeholder = t(key);
    }
  });

  // Tooltips & Titles
  root.querySelectorAll<HTMLElement>('[data-i18n-title]').forEach((el) => {
    const key = el.getAttribute('data-i18n-title');
    if (key) {
      el.title = t(key);
    }
  });
}
