import { createScope } from '../core/query';

export interface LoadOrderDomMap {
  'load-view': HTMLElement;
  'load-list-container': HTMLElement;
  'ue4ss-load-section': HTMLElement;
  'ue4ss-list-subcontainer': HTMLElement;
  'palschema-load-section': HTMLElement;
  'palschema-list-subcontainer': HTMLElement;
}

export const loadOrderDom = createScope<LoadOrderDomMap>('loadOrder');
