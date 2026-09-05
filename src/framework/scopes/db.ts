import { createScope } from '../core/query';

export interface DbDomMap {
  'db-view': HTMLElement;
  'db-refresh-btn': HTMLButtonElement;
  'db-grid-panel': HTMLElement;
  'db-json-status': HTMLElement;
  'db-save-btn': HTMLButtonElement;
  'db-json-editor': HTMLTextAreaElement;
  'db-usmap-table-wrap': HTMLElement;
  'db-usmap-footer': HTMLElement;
  'db-inspector-custom-container': HTMLElement;
  'db-inspector-title': HTMLElement;
  'db-inspector-actions': HTMLElement;
  'db-inspector-toggle-visual': HTMLButtonElement;
  'db-inspector-toggle-json': HTMLButtonElement;
  'db-inspector-close-btn': HTMLButtonElement;
  'db-fname-copy-btn': HTMLButtonElement;
}

export const dbDom = createScope<DbDomMap>('db');
