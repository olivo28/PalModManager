import { createScope } from '../core/query';

export interface InstallerDomMap {
  'install-modal': HTMLElement;
  'install-modal-title': HTMLElement;
  'modal-close-x': HTMLButtonElement;
  'modal-content': HTMLElement;
  'modal-status': HTMLElement;
  'modal-cancel': HTMLButtonElement;
  'modal-confirm': HTMLButtonElement;
  'modal-install-deps-retry': HTMLButtonElement;
  'view-all-files-btn': HTMLButtonElement;
  'update-mode-btn': HTMLButtonElement;
  'install-new-mode-btn': HTMLButtonElement;
  'mod-folder-name-input': HTMLInputElement;
  'config-diff-container': HTMLElement;
  'config-diff-summary-text': HTMLElement;
  'view-config-diff-btn': HTMLButtonElement;
  'batch-table-wrapper': HTMLElement;
  'mod-type-select': HTMLSelectElement;
  'mod-name-input': HTMLInputElement;
  'mod-version-input': HTMLInputElement;
}

export const installerDom = createScope<InstallerDomMap>('installer');
