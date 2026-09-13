import { createScope } from '../core/query';

export interface FomodDomMap {
  'fomod-modal': HTMLElement;
  'fomod-mod-title': HTMLElement;
  'fomod-mod-version': HTMLElement;
  'fomod-mod-author': HTMLElement;
  'fomod-step-badge': HTMLElement;
  'fomod-progress-fill': HTMLElement;
  'fomod-modal-close-x': HTMLButtonElement;
  'fomod-banner-container': HTMLElement;
  'fomod-banner-img': HTMLImageElement;
  'fomod-steps-nav': HTMLElement;
  'fomod-step-heading': HTMLElement;
  'fomod-step-subtitle': HTMLElement;
  'fomod-mod-desc': HTMLElement;
  'fomod-step-groups': HTMLElement;
  'fomod-btn-cancel': HTMLButtonElement;
  'fomod-btn-prev': HTMLButtonElement;
  'fomod-btn-next': HTMLButtonElement;
  'fomod-next-label': HTMLElement;
}

export const fomodDom = createScope<FomodDomMap>('fomod');
