import { createScope } from '../core/query';

export interface MainDomMap {
  // App root & loading
  'app-loading': HTMLElement;
  'app': HTMLElement;
  'main-content': HTMLElement;
  'modals-root': HTMLElement;
  'toast-container': HTMLElement;

  // Sidebar & Navigation
  'sidebar-version': HTMLElement;
  'sidebar-tab-discovery': HTMLButtonElement;
  'sidebar-tab-mods': HTMLButtonElement;
  'sidebar-tab-editor': HTMLButtonElement;
  'sidebar-tab-library': HTMLButtonElement;
  'sidebar-tab-scanner': HTMLButtonElement;
  'sidebar-tab-build': HTMLButtonElement;
  'sidebar-tab-load': HTMLButtonElement;
  'sidebar-library-badge': HTMLElement;
  'sidebar-nexus-widget': HTMLElement;
  'sidebar-nexus-avatar': HTMLImageElement;
  'sidebar-nexus-name': HTMLElement;

  // Mods View & Toolbar
  'discovery-view': HTMLElement;
  'mods-view': HTMLElement;
  'editor-view': HTMLElement;
  'load-view': HTMLElement;
  'library-view': HTMLElement;
  'build-view': HTMLElement;
  'scanner-view': HTMLElement;
  'db-view': HTMLElement;
  'toolbar': HTMLElement;
  'game-platform-badge': HTMLElement;
  'game-version-badge': HTMLElement;
  'ue4ss-badge': HTMLElement;
  'palschema-badge': HTMLElement;
  'quick-filter-bar': HTMLElement;
  'advanced-filter-btn': HTMLButtonElement;
  'advanced-filter-dropdown': HTMLElement;
  'filter-tags-list': HTMLElement;
  'filter-cats-list': HTMLElement;
  'status-filter-bar': HTMLElement;
  'search-wrap': HTMLElement;
  'search-input': HTMLInputElement;
  'sort-bar': HTMLElement;
  'sort-select': HTMLSelectElement;
  'layout-toggle-bar': HTMLElement;
  'layout-grid-btn': HTMLButtonElement;
  'layout-list-btn': HTMLButtonElement;
  'profile-active-label': HTMLElement;
  'profile-select': HTMLSelectElement;
  'profile-manager-btn': HTMLButtonElement;
  'launch-game-btn': HTMLButtonElement;
  'new-folder-btn': HTMLButtonElement;
  'theme-toggle-btn': HTMLButtonElement;
  'settings-btn': HTMLButtonElement;
  'toolbar-actions': HTMLElement;
  'scan-btn': HTMLButtonElement;
  'install-btn': HTMLButtonElement;
  'check-updates-btn': HTMLButtonElement;
  'open-all-updates-btn': HTMLButtonElement;
  'disable-all-btn': HTMLButtonElement;
  'enable-all-btn': HTMLButtonElement;
  'export-json-btn': HTMLButtonElement;
  'backup-btn': HTMLButtonElement;
  'restore-backup-btn': HTMLButtonElement;
  'bulk-actions-bar': HTMLElement;
  'bulk-selected-count': HTMLElement;
  'bulk-enable-btn': HTMLButtonElement;
  'bulk-disable-btn': HTMLButtonElement;
  'bulk-remove-btn': HTMLButtonElement;
  'bulk-clear-btn': HTMLButtonElement;
  'mods-container': HTMLElement;
  'empty-state': HTMLElement;

  // Modals generales
  'install-modal': HTMLElement;
  'install-modal-title': HTMLElement;
  'modal-close-x': HTMLButtonElement;
  'modal-content': HTMLElement;
  'modal-status': HTMLElement;
  'modal-cancel': HTMLButtonElement;
  'modal-install-deps-retry': HTMLButtonElement;
  'modal-confirm': HTMLButtonElement;

  'profile-modal': HTMLElement;
  'profile-modal-close-x': HTMLButtonElement;
  'profile-list': HTMLElement;
  'profile-new-name': HTMLInputElement;
  'profile-create-btn': HTMLButtonElement;
  'profile-modal-close': HTMLButtonElement;

  'about-modal': HTMLElement;
  'about-modal-close-x': HTMLButtonElement;
  'about-modal-close': HTMLButtonElement;

  'workshop-modal': HTMLElement;
  'workshop-modal-close-x': HTMLButtonElement;
  'workshop-modal-close': HTMLButtonElement;
  'workshop-master-toggle': HTMLInputElement;
  'workshop-list-container': HTMLElement;

  'context-overlay': HTMLElement;
  'context-menu': HTMLElement;

  // NXM Download Queue Tray
  'nxm-download-tray': HTMLElement;
  'nxm-tray-header-toggle': HTMLElement;
  'nxm-tray-badge-count': HTMLElement;
  'nxm-tray-toggle-collapse': HTMLButtonElement;
  'nxm-tray-close': HTMLButtonElement;
  'nxm-download-body': HTMLElement;
}

export const mainDom = createScope<MainDomMap>('main');
