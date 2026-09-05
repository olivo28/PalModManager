import { createScope } from '../core/query';

export interface ScannerDomMap {
  'scanner-view': HTMLElement;
  'scanner-start-btn': HTMLButtonElement;
  'scanner-start-hotkeys-btn': HTMLButtonElement;
  'scanner-rescan-hotkeys-btn': HTMLButtonElement;
  'scanner-re-run-btn': HTMLButtonElement;
  'scanner-inspector-root': HTMLElement;
  'btn-convert-all-gamepass': HTMLButtonElement;
  'btn-scanner-download-altermatic': HTMLButtonElement;
  'btn-open-patch-builder': HTMLButtonElement;
  'btn-open-existing-patches': HTMLButtonElement;
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

  // Backups Vault modal
  'pmm-backups-vault-modal': HTMLElement;

  // Patch Builder modal
  'pmm-patch-builder-modal-overlay': HTMLElement;
  'btn-close-patch-builder': HTMLButtonElement;
  'btn-cancel-patch-builder': HTMLButtonElement;
  'btn-view-managed-patches': HTMLButtonElement;
  'btn-build-patch-confirm': HTMLButtonElement;
  'patch-filename-input': HTMLInputElement;
  'patch-gamepass-toggle': HTMLInputElement;
  'patch-build-progress-box': HTMLElement;
  'pmm-existing-patches-modal': HTMLElement;
  'btn-close-existing-patches': HTMLButtonElement;
  'btn-close-existing-patches-footer': HTMLButtonElement;
}

export const scannerDom = createScope<ScannerDomMap>('scanner');
