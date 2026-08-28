import { createScope } from '../core/query';

export interface DbDomMap {
  'db-view': HTMLElement;
  'db-refresh-btn': HTMLButtonElement;
  'db-grid-panel': HTMLElement;
  'db-json-status': HTMLElement;
  'db-save-btn': HTMLButtonElement;
  'db-json-editor': HTMLTextAreaElement;
}

export const dbDom = createScope<DbDomMap>('db');
