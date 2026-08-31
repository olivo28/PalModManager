import { createScope } from '../core/query';

export interface DependencyDomMap {
  'dependency-modal': HTMLElement;
  'dep-modal-icon': HTMLElement;
  'dep-modal-title': HTMLElement;
  'dependency-modal-close-x': HTMLButtonElement;
  'dep-current-version': HTMLElement;
  'dep-status-badge': HTMLElement;
  'dep-remote-version': HTMLElement;
  'dep-btn-download-latest': HTMLButtonElement;
  'dep-btn-custom-zip': HTMLButtonElement;
  'dep-btn-open-vault': HTMLButtonElement;
  'dep-vault-count': HTMLElement;
  'dep-vault-list': HTMLElement;
  'dep-btn-uninstall': HTMLButtonElement;
  'dependency-modal-close': HTMLButtonElement;
}

export const dependencyDom = createScope<DependencyDomMap>('dependency');
