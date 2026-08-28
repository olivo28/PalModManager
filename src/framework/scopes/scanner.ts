import { createScope } from '../core/query';

export interface ScannerDomMap {
  'scanner-view': HTMLElement;
  'scanner-start-btn': HTMLButtonElement;
  'scanner-start-hotkeys-btn': HTMLButtonElement;
  'scanner-re-run-btn': HTMLButtonElement;
  'scanner-inspector-root': HTMLElement;
  'btn-convert-all-gamepass': HTMLButtonElement;
  'hk-search-input': HTMLInputElement;
  'registry-search-input': HTMLInputElement;

  // Saves Doctor snapshot comparison modal
  'save-compare-modal': HTMLElement;
  'btn-close-compare-modal': HTMLButtonElement;
  'btn-modal-close-action': HTMLButtonElement;
  'btn-modal-restore-action': HTMLButtonElement;
  'snap-internal-deltas-container': HTMLElement;
  'snap-inspect-day': HTMLElement;
  'snap-inspect-host': HTMLElement;
  'snap-inspect-ram': HTMLElement;
  'snap-inspect-mod-cleanliness': HTMLElement;
}

export const scannerDom = createScope<ScannerDomMap>('scanner');
