import { createScope } from '../core/query';

export interface DetailDomMap {
  'detail-overlay': HTMLElement;
  'detail-panel': HTMLElement;
  'detail-header': HTMLElement;
  'detail-body': HTMLElement;
  'detail-name-header': HTMLElement;
  'detail-rename-btn': HTMLButtonElement;
  'detail-close': HTMLButtonElement;
  'detail-image-container': HTMLElement;
  'detail-image': HTMLImageElement;
  'detail-folder-select': HTMLSelectElement;
  'detail-notes-section': HTMLElement;
  'detail-custom-notes': HTMLTextAreaElement;
  'detail-notes-saved-indicator': HTMLElement;
  'detail-description': HTMLElement;
  'detail-nexus': HTMLElement;
  'detail-github': HTMLElement;
  'detail-info-tab': HTMLElement;
  'detail-tech-tab': HTMLElement;
  'detail-type': HTMLElement;
  'detail-version': HTMLElement;
  'detail-status': HTMLElement;
  'detail-pak-destination-row': HTMLElement;
  'detail-pak-destination-select': HTMLSelectElement;
  'detail-install-date': HTMLElement;
  'detail-source-zip': HTMLElement;
  'detail-components-container': HTMLElement;
  'detail-duplicate-row': HTMLElement;
  'detail-duplicate-warning': HTMLElement;
  'detail-gamepass-warning-row': HTMLElement;
  'detail-convert-gamepass-btn': HTMLButtonElement;
  'detail-config-path': HTMLElement;
  'detail-set-config': HTMLButtonElement;
  'detail-clear-config': HTMLButtonElement;
  'detail-pak-contents-container': HTMLElement;
  'detail-folder-buttons': HTMLElement;
  'detail-toggle': HTMLButtonElement;
  'detail-config': HTMLButtonElement;
  'detail-refresh': HTMLButtonElement;
  'detail-remove': HTMLButtonElement;
}

export const detailDom = createScope<DetailDomMap>('detail');
